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
import {createApp,h,ref} from '/node_modules/.vite-rebase-test/deps/vue.js';
import Graph from '/src/view/commit/CommitGraph.vue';
import Rebase from '/src/features/repository/InteractiveRebaseDialog.vue';
import '/src/styles/index.css';
const base='0'.repeat(40), a='1'.repeat(40), b='2'.repeat(40), c='3'.repeat(40), d='4'.repeat(40), remote='5'.repeat(40);
const commits=[
 {sha:d,message:'Remove temporary notes',refs:['HEAD -> feature/topic']},
 {sha:c,message:'Clean up unused import',refs:[]},
 {sha:b,message:'Add focused tests',refs:['refs/heads/test']},
 {sha:a,message:'Add workspace entry point\\n\\nPreserve this detail.',refs:[]},
 {sha:base,message:'Base commit',refs:[]},
].map((commit,index,all)=>({...commit,short_sha:commit.sha.slice(0,7),author_name:'Test',author_email:'test@example.invalid',timestamp:1793380000-index,parent_shas:all[index+1]?[all[index+1].sha]:[]}));
const requested=ref(null),busy=ref(false),fastForward=ref(false);
const incoming={...commits[0],sha:remote,short_sha:remote.slice(0,7),message:'Incoming change',refs:['origin/feature/topic']};
window.setFastForward=()=>{fastForward.value=true};
window.finishRebase=()=>{busy.value=false;requested.value=null};
window.testInvoke=async (command,args)=>{
 if(command==='get_interactive_rebase_plan') {
   window.requestedBase=args.base;
   const advance=args.base===remote && fastForward.value;
   return {head:d,branch:'feature/topic',onto:args.base,upstream:advance?d:base,fastForward:advance,commits:advance?[]:commits.slice(0,4).reverse()};
 }
 throw Error(command);
};
createApp({render:()=>h('main',{style:'height:100%;display:flex;flex-direction:column'},[
 h(Graph,{repoPath:'repo',commits:[{...incoming,parent_shas:[fastForward.value?d:base]},...commits],selected:null,hasWorkingChanges:false,currentBranch:'feature/topic',headSha:d,onRequestInteractiveRebase:sha=>requested.value=sha}),
 requested.value?h(Rebase,{repoPath:'repo',base:requested.value,busy:busy.value,executionError:'',onStart:payload=>{window.payload=payload;busy.value=true},onClose:()=>requested.value=null}):null
])}).mount('#app');
</script></body></html>`;
const server = await createServer({
  root, configFile: false, cacheDir: "node_modules/.vite-rebase-test",
  plugins: [{
    name: "rebase-test-mocks", enforce: "pre",
    resolveId(id) { if (id === "@tauri-apps/api/core") return "\0rebase-test-core"; },
    load(id) { if (id === "\0rebase-test-core") return "export const invoke=(...args)=>window.testInvoke(...args);"; },
    configureServer(vite) {
      vite.middlewares.use((req, res, next) => {
        if (req.url !== "/__interactive_rebase__") return next();
        void vite.transformIndexHtml(req.url, html).then(body => { res.setHeader("Content-Type", "text/html"); res.end(body); });
      });
    },
  }, vue(), tailwindcss()],
  resolve: { alias: { "@": path.join(root, "src") } },
  optimizeDeps: { entries: [], noDiscovery: true, include: ["vue", "lucide-vue-next"] },
  server: { port: 0, host: "127.0.0.1" },
});

let browser;
try {
  await server.listen();
  browser = await chromium.launch({ headless: true, ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}) });
  const page = await browser.newPage({ viewport: { width: 1120, height: 800 } });
  const errors = [];
  page.on("pageerror", error => { errors.push(error.message); console.error(error.message); });
  page.setDefaultTimeout(18000);
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__interactive_rebase__`, { waitUntil: "domcontentloaded", timeout: 60000 });
  const menu = page.getByRole("button", { name: /Interactive Rebase from Here/ });
  const open = async () => {
    await page.locator(`#commit-row-${"0".repeat(40)}`).click({ button: "right", position: { x: 480, y: 14 } });
    await menu.click();
  };
  await open();
  const modal = page.getByRole("dialog", { name: "Interactive Rebase" });
  const list = modal.getByRole("list", { name: "Commits to rebase" });
  const start = modal.getByRole("button", { name: "Start rebase" });
  await list.waitFor();
  assert.equal(await page.evaluate(() => window.requestedBase), "0".repeat(40));
  assert.equal(await list.getByRole("listitem").count(), 4);
  assert.equal(await start.isDisabled(), true);
  await page.keyboard.press("Escape");
  await modal.waitFor({ state: "hidden" });
  const remoteSha = "5".repeat(40);
  const remoteRow = page.locator(`#commit-row-${remoteSha}`);
  await remoteRow.getByTitle("feature/topic", { exact: true }).click({ button: "right" });
  await page.getByRole("button", { name: /Interactive Rebase onto origin\/feature\/topic/ }).click();
  await list.waitFor();
  assert.equal(await page.evaluate(() => window.requestedBase), remoteSha);
  assert.equal(await list.getByRole("listitem").count(), 4);
  assert.equal(await list.getByText("Incoming change").count(), 0);
  await start.click();
  assert.deepEqual(await page.evaluate(() => window.payload.steps.map(step => step.action)), Array(4).fill("pick"));
  await page.evaluate(() => window.finishRebase());
  await modal.waitFor({ state: "hidden" });

  await page.evaluate(() => window.setFastForward());
  await remoteRow.click({ button: "right", position: { x: 480, y: 14 } });
  await page.getByRole("button", { name: /Interactive Rebase onto Here/ }).click();
  await modal.getByText(/No local commits to replay/).waitFor();
  assert.equal(await list.count(), 0);
  await modal.getByRole("button", { name: "Fast-forward", exact: true }).click();
  assert.equal(await page.evaluate(() => window.requestedBase), remoteSha);
  assert.deepEqual(await page.evaluate(() => window.payload.steps), []);
  await page.evaluate(() => window.finishRebase());
  await modal.waitFor({ state: "hidden" });

  await open();
  const choose = label => modal.getByRole("combobox", { name: `Action for ${label}`, exact: true });
  await choose("Add workspace entry point").selectOption("squash");
  await modal.getByRole("alert").getByText(/preceding commit/).waitFor();
  assert.equal(await start.isDisabled(), true);
  await choose("Add workspace entry point").selectOption("reword");
  const message = modal.getByRole("textbox", { name: "Commit message", exact: true });
  assert.match(await message.inputValue(), /Preserve this detail/);
  await message.fill("Refine workspace entry point\n\nKeep the body.");
  const handle = text => modal.getByRole("button", { name: `Reorder ${text}`, exact: true });
  const first = list.getByRole("listitem").first();
  await handle("Add focused tests").dragTo(first, { targetPosition: { x: 80, y: 7 } });
  assert.match(await first.innerText(), /Add focused tests/);
  await handle("Add focused tests").press("ArrowDown");
  assert.match(await first.innerText(), /Add workspace entry point/);
  await choose("Add focused tests").selectOption("squash");
  await choose("Clean up unused import").selectOption("fixup");
  await choose("Remove temporary notes").selectOption("drop");
  assert.equal(await list.locator('[data-grouped="true"]').count(), 3);
  assert.equal(await list.locator('[data-dropped="true"]').count(), 1);
  const combined = modal.getByRole("textbox", { name: "Combined commit message", exact: true });
  assert.match(await combined.inputValue(), /Keep the body[\s\S]*Add focused tests/);
  assert.doesNotMatch(await combined.inputValue(), /unused import/);
  await combined.fill("Workspace entry point and tests");
  await page.screenshot({ path: path.join(root, "src-tauri/target/interactive-rebase-desktop.png") });
  await page.setViewportSize({ width: 390, height: 760 });
  assert.ok(await modal.evaluate(el => el.scrollWidth <= el.clientWidth));
  await page.screenshot({ path: path.join(root, "src-tauri/target/interactive-rebase-compact.png") });
  await start.click();
  const payload = await page.evaluate(() => window.payload);
  assert.equal(payload.branch, "feature/topic");
  assert.deepEqual(payload.steps.map(step => step.action), ["reword", "squash", "fixup", "drop"]);
  assert.equal(payload.steps[0].combinedMessage, "Workspace entry point and tests");
  await page.keyboard.press("Escape");
  assert.ok(await modal.isVisible());
  assert.deepEqual(errors, []);
  console.log("PASS: graph menus, remote onto and fast-forward, reorder by drag and keyboard, validation, inline messages and squash grouping.");
} finally {
  await browser?.close();
  await server.close();
}
