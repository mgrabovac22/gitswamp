import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { createServer } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const root = fileURLToPath(new URL("..", import.meta.url));
const mocks = {
  "@tauri-apps/api/core": "export const invoke = (...args) => window.testInvoke(...args);",
  "@/domain/git/UseGit": "import {ref} from 'vue'; export const useGit = () => ({providerTokens:ref({}),githubToken:ref(null)});",
};
const html = `<!doctype html><html><head><style>
html,body,#app{margin:0;height:100%;}nav{display:flex;flex-wrap:wrap;gap:8px;padding:8px}nav button{padding:4px 8px;border:1px solid #555}
</style></head><body><div id="app"></div><script type="module">
import {createApp,h,ref} from '/node_modules/.vite/deps/vue.js';
import Viewer from '/src/shared/ui/FileDiffViewer.vue';
import '/src/styles/index.css';
window.liveUrls=new Set();window.created=0;window.maxLive=0;window.calls=[];
const create=URL.createObjectURL.bind(URL),revoke=URL.revokeObjectURL.bind(URL);
URL.createObjectURL=b=>{const u=create(b);liveUrls.add(u);window.created++;window.maxLive=Math.max(maxLive,liveUrls.size);return u};
URL.revokeObjectURL=u=>{liveUrls.delete(u);revoke(u)};
const image=async color=>{const c=document.createElement('canvas');c.width=320;c.height=180;const x=c.getContext('2d');x.fillStyle='white';x.fillRect(0,0,320,180);x.fillStyle=color;x.fillRect(80,40,160,100);return await new Promise(r=>c.toBlob(async b=>r(await b.arrayBuffer()),'image/png'))};
const before=await image('#e64040'),after=await image('#2077df');
const selected=ref('picture.png'),mounted=ref(true),revision=ref(0);
window.setFile=name=>{selected.value=name;mounted.value=true};window.closePreview=()=>mounted.value=false;
window.changeImage=()=>revision.value++;
const side=(id,size=before.byteLength,exists=true)=>({id,size,exists});
window.testInvoke=async(command,args)=>{
 window.calls.push({command,args});
 if(command==='get_working_diff') return {path:args.filePath,old_path:null,status:'modified',is_binary:args.filePath!=='vector.svg',hunks:[]};
 if(command==='get_binary_diff_info') return {old:side('before',before.byteLength,args.filePath!=='added.png'),new:side('after'+revision.value,args.filePath==='huge.png'?11*1024*1024:after.byteLength,args.filePath!=='deleted.png')};
 if(command==='get_file_bytes') {
  if(args.filePath==='bad.png') return new Uint8Array([1,2,3]).buffer;
  if(args.filePath==='vector.svg') return new TextEncoder().encode('<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180"><rect width="320" height="180" fill="green"/></svg>').buffer;
  return args.side==='old'?before:after;
 }
 if(command==='get_file_blame')return [];
 throw Error(command);
};
createApp({render:()=>h('main',{style:'height:100%;display:flex;flex-direction:column'},[
 h('nav',{},['picture.png','added.png','deleted.png','archive.zip','bad.png','huge.png','vector.svg'].map(name=>h('button',{onClick:()=>window.setFile(name)},name))),
 h('section',{style:'flex:1;min-height:0'},mounted.value?[h(Viewer,{repoPath:'repo',filePath:selected.value,onClose:()=>mounted.value=false})]:[])
])}).mount('#app');
</script></body></html>`;
const server = await createServer({
  root, configFile: false,
  plugins: [{
    name: "binary-test-mocks", enforce: "pre",
    resolveId(id) {
      if (id.replaceAll('\\', '/').endsWith('/src/domain/git/UseGit.ts')) return '\0test:@/domain/git/UseGit';
      if (mocks[id]) return `\0test:${id}`;
    },
    load(id) { if (id.startsWith("\0test:")) return mocks[id.slice(6)]; },
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url !== "/__binary_test__") return next();
        void server.transformIndexHtml(req.url, html).then(result => { res.setHeader("Content-Type", "text/html"); res.end(result); });
      });
    },
  }, vue(), tailwindcss()],
  resolve: { alias: { "@": path.join(root, "src") } },
  optimizeDeps: { entries: [], noDiscovery: true, include: ["vue", "lucide-vue-next", "highlight.js/lib/common"] },
  server: { port: 0, host: "127.0.0.1" },
});
let browser;
try {
  await server.listen();
  browser = await chromium.launch({ headless: true, ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}) });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on("pageerror", e => errors.push(e.message));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__binary_test__`);
  const waitImages = () => page.waitForFunction(() => {
    const images = [...document.querySelectorAll('.binary-diff-image')];
    return images.length === 2 && images.every(i => i.complete && i.naturalWidth === 320);
  });
  await waitImages();
  const colors = await page.locator('.binary-diff-image').evaluateAll(images => images.map(image => {
    const c=document.createElement('canvas');c.width=320;c.height=180;
    const x=c.getContext('2d');x.drawImage(image,0,0);return [...x.getImageData(100,80,1,1).data];
  }));
  assert.deepEqual(colors, [[230,64,64,255],[32,119,223,255]]);
  const firstReadCount = await page.evaluate(() => calls.filter(c => c.command==='get_file_bytes').length);
  await page.waitForTimeout(2000);
  assert.equal(await page.evaluate(() => calls.filter(c => c.command==='get_file_bytes').length), firstReadCount);
  await page.evaluate(() => changeImage());
  await page.waitForFunction(count => calls.filter(c => c.command==='get_file_bytes').length > count, firstReadCount);
  await waitImages();
  await page.screenshot({ path: path.join(root, 'src-tauri/target/binary-before-after.png'), animations: 'disabled' });
  await page.getByRole('button', { name: 'Slider', exact: true }).click();
  const range = page.getByRole('slider', { name: 'Image comparison position' });
  await range.focus(); await range.press('Home'); await range.press('ArrowRight');
  assert.equal(await range.inputValue(), '1');
  await page.locator('.binary-diff-stage').click({ position: { x: 300, y: 100 } });
  assert.ok(Number(await range.inputValue()) > 1);
  await page.screenshot({ path: path.join(root, 'src-tauri/target/binary-slider.png'), animations: 'disabled' });
  await page.getByRole('button', { name: 'Difference', exact: true }).click();
  await page.screenshot({ path: path.join(root, 'src-tauri/target/binary-difference.png'), animations: 'disabled' });
  await page.setViewportSize({ width: 480, height: 800 });
  await page.getByRole('button', { name: 'Side by side', exact: true }).click();
  await waitImages();
  const fit = await page.locator('.binary-diff-viewer').evaluate(el => el.scrollWidth <= el.clientWidth);
  assert.ok(fit, 'narrow preview must not overflow horizontally');
  await page.screenshot({ path: path.join(root, 'src-tauri/target/binary-narrow.png'), animations: 'disabled' });
  for (const name of ['added.png','deleted.png']) {
    await page.evaluate(name => setFile(name), name);
    await page.waitForFunction(() => document.querySelectorAll('.binary-diff-image').length === 1);
    assert.ok(await page.getByRole('button', { name: 'Slider', exact: true }).isDisabled());
  }
  await page.evaluate(() => setFile('archive.zip'));
  await page.getByText('Text diff is not available for this file type.', {exact:true}).waitFor();
  assert.equal(await page.evaluate(() => liveUrls.size), 0);
  await page.evaluate(() => setFile('bad.png'));
  await page.getByText('Preview is not available for this image format', {exact:true}).first().waitFor();
  assert.equal(await page.evaluate(() => liveUrls.size), 0);
  await page.evaluate(() => setFile('huge.png'));
  await page.getByText('File is too large to preview', {exact:true}).waitFor();
  assert.ok(!await page.evaluate(() => calls.some(c => c.command==='get_file_bytes' && c.args.filePath==='huge.png' && c.args.side==='new')));
  await page.evaluate(() => setFile('vector.svg'));
  await page.getByTitle('Visual image comparison', {exact:true}).click();
  await waitImages();
  // Repeated open/close must keep native blob ownership bounded, not cache previews.
  for (let i=0;i<30;i++) {
    await page.evaluate(() => setFile('picture.png'));
    await waitImages();
    await page.evaluate(() => closePreview());
    await page.waitForFunction(() => liveUrls.size===0);
  }
  const stats = await page.evaluate(() => ({created,maxLive,remaining:liveUrls.size}));
  assert.ok(stats.maxLive <= 2); assert.equal(stats.remaining, 0);
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({ result: 'passed', ...stats, checks: 'before/after pixels, slider keyboard/pointer, difference, narrow layout, added/deleted/binary/invalid/large/SVG, 30 open-close cycles' }));
} finally {
  await browser?.close(); await server.close();
}
