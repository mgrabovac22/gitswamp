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
const html = `<!doctype html><html class="light"><head><style>html,body,#app{margin:0;height:100%}</style></head><body><div id="app"></div><script type="module">
import {createApp,h,ref} from '/node_modules/.vite-coauthors-test/deps/vue.js';
import Details from '/src/view/commit/CommitDetails.vue';
import '/src/styles/index.css';
const selected=ref(null);
const authors={a:[{name:'Ana Test',email:'ana@example.invalid'},{name:'Ivo Test',email:'ivo@example.invalid'}],b:[{name:'Mia Test',email:'mia@example.invalid'}]};
const makeCommit=id=>({sha:id.repeat(40),short_sha:id.repeat(7),message:id==='c'?'No co-authors':'Change '+id+'\\n\\nCo-authored-by: a test',author_name:'Author Example',author_email:'author@example.invalid',committer_name:'Committer Example',committer_email:'committer@example.invalid',timestamp:1793380000,parent_shas:[],refs:[]});
window.authorReads=[];
window.testInvoke=async(command,args)=>{
  if(command==='get_commit_coauthors'){
    const id=args.sha[0];window.authorReads.push(id);
    if(window.holdAuthor===id) await new Promise(resolve=>window.releaseAuthor=resolve);
    return authors[id];
  }
  return [];
};
createApp({render:()=>h('main',{style:'display:flex;flex-direction:column;height:100%;max-width:340px'},[
 h('nav',{},['a','b','c'].map(id=>h('button',{style:'padding:8px',onClick:()=>selected.value=makeCommit(id)},'Select '+id))),
 h(Details,{repoPath:'/example/repo',commit:selected.value,commitFiles:[],isWorkingChanges:false,stagedFiles:[],unstagedFiles:[]}),
])}).mount('#app');
</script></body></html>`;
const server = await createServer({
  root, configFile: false, cacheDir: "node_modules/.vite-coauthors-test",
  define: { __APP_VERSION__: JSON.stringify("test") },
  plugins: [{
    name: "coauthors-test-mocks", enforce: "pre",
    resolveId(id) { if (id === "@tauri-apps/api/core") return "\0coauthors-test-core"; },
    load(id) { if (id === "\0coauthors-test-core") return "export const invoke=(...args)=>window.testInvoke(...args);"; },
    configureServer(vite) {
      vite.middlewares.use((req, res, next) => {
        if (req.url !== "/__commit_coauthors__") return next();
        void vite.transformIndexHtml(req.url, html).then(body => { res.setHeader("Content-Type", "text/html"); res.end(body); });
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
  const page = await browser.newPage({ viewport: { width: 360, height: 780 } });
  const errors = [];
  page.on("pageerror", error => { errors.push(error.message); console.error(error.message); });
  page.setDefaultTimeout(18000);
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__commit_coauthors__`, { waitUntil: "domcontentloaded", timeout: 60000 });
  const select = id => page.getByRole("button", { name: `Select ${id}`, exact: true }).click();
  const coauthors = page.getByRole("group", { name: "Co-authors", exact: true });
  await select("c");
  await page.getByText("Author Example", { exact: true }).waitFor();
  assert.deepEqual(await page.evaluate(() => window.authorReads), []);
  await select("a");
  await coauthors.getByText("Ana Test", { exact: true }).waitFor();
  assert.ok(await coauthors.getByText("Ivo Test", { exact: true }).isVisible());
  const committer = await page.getByText("Committer Example", { exact: true }).boundingBox();
  assert.ok((await coauthors.boundingBox()).y > committer.y + committer.height);
  await page.screenshot({ path: path.join(root, "src-tauri/target/commit-coauthors.png") });

  await page.getByRole("button", { name: "Changes", exact: true }).click();
  await select("b");
  assert.deepEqual(await page.evaluate(() => window.authorReads), ["a"]);
  await page.evaluate(() => { window.holdAuthor = "b"; });
  await page.getByRole("button", { name: "Info", exact: true }).click();
  await page.waitForFunction(() => !!window.releaseAuthor);
  await select("c");
  await page.evaluate(() => window.releaseAuthor());
  await page.waitForFunction(() => !document.querySelector('[aria-label="Co-authors"]'));
  assert.ok(!(await page.getByText("Mia Test", { exact: true }).count()));
  assert.deepEqual(errors, []);
  console.log("PASS: co-authors below committer, no reads outside Info, and stale requests ignored.");
} finally {
  await browser?.close();
  await server.close();
}
