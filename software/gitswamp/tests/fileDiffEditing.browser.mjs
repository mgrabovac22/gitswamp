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
html,body,#app{margin:0;height:100%;}button.test-file{padding:8px 16px;color:white;background:#32455e;}
</style></head><body><div id="app"></div><script type="module">
import {createApp,h,ref} from '/node_modules/.vite/deps/vue.js';
import Viewer from '/src/shared/ui/FileDiffViewer.vue';
import TimeMachine from '/src/view/commit/CommitTimeMachinePanel.vue';
import CommitDetails from '/src/view/commit/CommitDetails.vue';
import '/src/styles/index.css';
const files={'a.ts':'const alpha = 1;\\n// first file\\n','b.ts':'const beta = 2;\\n// second file\\n'};
window.testInvoke = async (command,args) => {
 if(command==='get_working_diff') return {path:args.filePath,is_binary:false,hunks:[{old_start:1,old_lines:2,new_start:1,new_lines:2,header:'@@ -1,2 +1,2 @@',lines:[{line_type:'context',old_line_no:1,new_line_no:1,content:'const syntax = true;\\n'},{line_type:'context',old_line_no:2,new_line_no:2,content:'// worker preview\\n'}]}]};
 if(command==='get_file_content') return files[args.filePath] || '';
 if(command==='save_file_content'){files[args.filePath]=args.content;return '';}
 if(command==='get_commits') return [{sha:'1234567',short_sha:'1234567',message:'Preview code',author_name:'Test',timestamp:1793380000,refs:[]}];
 if(command==='get_commit_files') return [{path:'a.ts',status:'modified',additions:1,deletions:1}];
 if(command==='get_commit_tree_paths') return ['a.ts','b.ts'];
 if(command==='get_file_blame') return [];
 if(command==='get_available_editors') return [];
 throw Error(command);
};
const selected=ref('a.ts'),machine=ref(false),details=ref(false),commitFiles=ref([]),filesLoading=ref(false);
const commit={sha:'1234567',short_sha:'1234567',message:'Large commit',author_name:'Test',author_email:'test@example.com',timestamp:1793380000,refs:[],parent_shas:[]};
createApp({render:()=>h('main',{style:'height:100%;display:flex;flex-direction:column;'},[
 h('nav',{},['a.ts','b.ts'].map(name=>h('button',{class:'test-file',onClick:()=>{details.value=false;machine.value=false;selected.value=name}},name)).concat(
 h('button',{class:'test-file',onClick:()=>{details.value=false;machine.value=true}},'Time Machine'),
 h('button',{class:'test-file',onClick:()=>{details.value=true;machine.value=false;filesLoading.value=true;setTimeout(()=>{commitFiles.value=Array.from({length:2048},(_,i)=>({path:'file-'+String(i).padStart(4,'0')+'.ts',status:'added',additions:1,deletions:0}));filesLoading.value=false;},300)}},'Large commit'))),
 h('section',{style:'flex:1;min-height:0;'},[details.value ? h(CommitDetails,{repoPath:'repo',commit,selectedCommits:[commit],commitFiles:commitFiles.value,commitFilesLoading:filesLoading.value,isWorkingChanges:false,stagedFiles:[],unstagedFiles:[]}) : machine.value ? h(TimeMachine,{repoPath:'repo'}) : h(Viewer,{repoPath:'repo',filePath:selected.value})])
])}).mount('#app');
</script></body></html>`;
const server = await createServer({
  root, configFile: false,
  plugins: [{
    name: "editing-test-mocks", enforce: "pre",
    resolveId(id) {
      if (id.replaceAll('\\', '/').endsWith('/src/domain/git/UseGit.ts')) return '\0test:@/domain/git/UseGit';
      if (mocks[id]) return `\0test:${id}`;
    },
    load(id) { if (id.startsWith("\0test:")) return mocks[id.slice(6)]; },
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url !== "/__editing_test__") return next();
        void server.transformIndexHtml(req.url, html).then(result => {
          res.setHeader("Content-Type", "text/html"); res.end(result);
        });
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
  page.on("pageerror", error => errors.push(error.message));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__editing_test__`);
  await page.waitForFunction(() => document.querySelector('.hljs-keyword'));
  await page.getByTitle("Edit file", { exact: true }).click();
  const input = page.getByRole("textbox", { name: "File content" });
  await input.waitFor();
  assert.match(await input.inputValue(), /alpha/);
  await input.fill('const alpha = 3;\n// saved file\n');
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await page.getByRole("button", { name: "b.ts", exact: true }).click();
  await page.waitForFunction(() => document.querySelector('textarea')?.value.includes('beta'));
  await page.waitForFunction(() => document.querySelector('.code-editor-colored'));
  assert.match(await input.inputValue(), /beta/);
  assert.ok(await page.locator(".code-editor-backdrop .hljs-keyword").count());
  await page.getByRole("button", { name: "a.ts", exact: true }).click();
  await page.waitForFunction(() => document.querySelector('textarea')?.value.includes('alpha = 3'));
  const longCode = Array.from({length:100},(_,i)=>'const row'+i+' = "'+('text '.repeat(80))+'";').join('\n');
  await input.fill(longCode);
  await page.waitForFunction(() => document.querySelector('.code-editor-colored'));
  await input.evaluate(el => { el.scrollTop=500;el.scrollLeft=200;el.dispatchEvent(new Event('scroll')); });
  const scrolls = await page.evaluate(() => {
    const a=document.querySelector('textarea'),b=document.querySelector('.code-editor-backdrop');
    return [a.scrollTop,b.scrollTop,a.scrollLeft,b.scrollLeft];
  });
  assert.equal(scrolls[0],scrolls[1]); assert.equal(scrolls[2],scrolls[3]);
  assert.ok(scrolls[0]>0 && scrolls[2]>0);
  await page.screenshot({path:path.join(root,'src-tauri/target/editor-desktop.png')});
  await page.setViewportSize({width:720,height:700});
  await page.screenshot({path:path.join(root,'src-tauri/target/editor-compact.png')});
  await input.fill('x'.repeat(60_001));
  await page.waitForFunction(() => !document.querySelector('.code-editor-colored'));
  assert.equal(await input.inputValue(), 'x'.repeat(60_001));
  await page.getByRole('button',{name:'Time Machine',exact:true}).click();
  await page.waitForFunction(() => document.querySelector('.snapshot-preview .hljs-keyword'));
  await page.locator('.snapshot-preview').scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(root,'src-tauri/target/time-machine-highlight.png'),fullPage:true});
  await page.setViewportSize({width:1100,height:850});
  await page.getByRole('button',{name:'Large commit',exact:true}).click();
  await page.getByRole('status').waitFor();
  await page.waitForFunction(() => document.querySelector('[data-commit-file-path]'));
  assert.ok(await page.locator('[data-commit-file-path]').count()<60);
  await page.locator('[data-commit-file-path]').first().evaluate(el=>{
    const parent=el.parentElement.parentElement;parent.scrollTop=parent.scrollHeight;parent.dispatchEvent(new Event('scroll'));
  });
  await page.locator('[data-commit-file-path="file-2047.ts"]').waitFor();
  await page.screenshot({path:path.join(root,'src-tauri/target/large-commit-window.png')});
  assert.deepEqual(errors, []);
  console.log('PASS: worker syntax coloring, save/switch, scroll alignment, large-file fallback, Time Machine and responsive 2048-file list.');
} finally {
  await browser?.close();
  await server.close();
}
