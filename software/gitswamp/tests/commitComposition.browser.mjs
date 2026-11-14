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
import {createApp,h} from '/node_modules/.vite-composer-test/deps/vue.js';
import Details from '/src/view/commit/CommitDetails.vue';
import {extraViewShortcut} from '/src/features/shell/keyboardShortcuts.ts';
import '/src/styles/index.css';
window.shortcut = (key,code,init)=>extraViewShortcut(new KeyboardEvent('keydown',{key,code,...init}));
const changes=[
  {path:'src/auth/login.ts',status:'modified',staged:true,partial:true,added:5,removed:2,kind:'source'},
  {path:'tests/login.test.ts',status:'added',staged:false,partial:false,added:4,removed:0,kind:'test'},
  {path:'docs/usage.md',status:'modified',staged:false,partial:false,added:5,removed:2,kind:'docs'},
];
const units=[{id:0,changes:[0,1]},{id:1,changes:[2]}];
window.testInvoke = async (command, args)=>{
  if (command==='plan_commit_composition') {
    window.plans = (window.plans || 0) + 1;
    return {
    branch:'main',snapshot:'fixture',changes,units,limited:false,
    groups:[
      {subject:'Fix(auth): Handle missing login state',description:'Handle a missing session.',units:[0]},
      {subject:'Docs: Document account setup',description:'Describe the login flow.',units:[1]},
    ],
    review:{findings:[],skipped:[],large_files:[]},
    };
  }
  if (command==='apply_commit_composition') {
    window.applied=args;
    return args.drafts.map((_, i)=>String(i));
  }
  if (command==='get_staged_diff_summary') return {total_lines_added:5,total_lines_removed:2,files_changed:1,file_types:['ts'],has_test_changes:false,has_migration_changes:false,inferred_scope:'auth'};
  if (command==='get_empty_directories' || command==='get_available_external_editors') return [];
  throw Error(command);
};
createApp({render:()=>h(Details,{repoPath:'fixture',commit:null,commitFiles:[],isWorkingChanges:true,
  stagedFiles:[{path:'src/auth/login.ts',status:'modified',staged:true}],
  unstagedFiles:[{path:'docs/usage.md',status:'modified',staged:false}],
  onRefreshState:()=>window.refreshed=true})}).mount('#app');
</script></body></html>`;
const server = await createServer({
  root, configFile: false, cacheDir: "node_modules/.vite-composer-test",
  plugins: [{
    name: "composer-test", enforce: "pre",
    resolveId(id) { if (id === "@tauri-apps/api/core") return "\0composer-test-core"; },
    load(id) { if (id === "\0composer-test-core") return "export const invoke=(...args)=>window.testInvoke(...args);"; },
    configureServer(vite) {
      vite.middlewares.use((req, res, next) => {
        if (req.url !== "/__composer__") return next();
        void vite.transformIndexHtml(req.url, html).then(body => { res.setHeader("Content-Type", "text/html"); res.end(body); }).catch(next);
      });
    },
  }, vue(), tailwindcss()],
  resolve: { alias: { "@": path.join(root, "src") } },
  optimizeDeps: { entries: [], noDiscovery: true, include: ["vue", "lucide-vue-next", "class-variance-authority", "clsx", "tailwind-merge", "highlight.js/lib/common"] },
  server: { port: 0, host: "127.0.0.1" },
});

let browser;
try {
  await server.listen();
  browser = await chromium.launch({ headless: true, ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}) });
  const page = await browser.newPage({ viewport: { width: 1120, height: 800 } });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  page.on("response", response => { if (response.status() >= 400) console.error(response.status(), response.url()); });
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__composer__`, { waitUntil: "commit", timeout: 60000 });
  try { await page.getByTitle("Visual commit builder", { exact: true }).click(); }
  catch (error) { throw new Error(`Builder did not load: ${errors.join("; ")}`, { cause: error }); }
  assert.equal(await page.evaluate(() => window.plans), undefined);
  await page.getByRole("button", { name: "Compose commits automatically", exact: true }).click();
  const modal = page.getByRole("dialog", { name: "Compose commits" });
  const subject = modal.getByLabel("Subject");
  const confirm = modal.getByRole("button", { name: "Create 2 commits", exact: true });
  await modal.getByText("Staged only", { exact: true }).waitFor();
  await modal.getByText("Experimental feature", { exact: true }).waitFor();
  const list = modal.getByRole("region", { name: "Proposed commits" });
  const docs = list.getByRole("button", { name: /Document account setup/ });
  await docs.click();
  assert.equal(await docs.getAttribute("aria-current"), "step");
  assert.match(await subject.inputValue(), /Document account setup/);
  await list.getByRole("button", { name: /Handle missing login state/ }).click();
  assert.equal(await page.getByText("Visual Commit Builder", { exact: true }).count(), 0);
  assert.equal(await page.evaluate(() => window.plans), 1);
  assert.equal(await page.evaluate(() => window.shortcut("!", "Digit1", { altKey: true, shiftKey: true })), "branch-workspace");
  assert.equal(await page.evaluate(() => window.shortcut("@", "Digit2", { altKey: true, shiftKey: true })), "lfs");
  assert.equal(await page.evaluate(() => window.shortcut("2", "Digit2", { altKey: true })), undefined);
  await subject.fill("");
  assert.equal(await confirm.isDisabled(), true);
  await subject.fill("Fix(auth): Preserve login state");
  await modal.getByRole("button", { name: "Visual Commit Builder", exact: true }).click();
  const builder = modal.getByRole("group", { name: "Visual Commit Builder" });
  assert.equal(await builder.getByLabel("Type").inputValue(), "Fix");
  await builder.getByLabel("Scope").fill("account");
  await builder.getByLabel("Summary").fill("Preserve session after login");
  await builder.getByLabel("Summary").blur();
  await modal.getByLabel("Tag all commits").fill("release");
  await modal.getByRole("button", { name: "Apply to all" }).click();
  assert.match(await docs.innerText(), /#release/);
  await modal.getByLabel("Commit tag", { exact: true }).fill("#123");
  await list.getByRole("button", { name: /Fix\(account\): #123 Preserve session after login/ }).waitFor();
  assert.match(await docs.innerText(), /#release/);
  await modal.getByLabel("Description").fill("Keep the existing session when the login state changes.");
  await modal.getByRole("button", { name: "Move commit down", exact: true }).click();
  await page.screenshot({ path: path.join(root, "src-tauri/target/commit-composer-desktop.png") });
  await page.evaluate(() => document.documentElement.classList.remove("light"));
  assert.equal(await list.getByRole("button", { name: /Preserve session after login/ }).evaluate(el => getComputedStyle(el).cursor), "pointer");
  await page.screenshot({ path: path.join(root, "src-tauri/target/commit-composer-dark.png"), animations: "disabled" });
  await page.evaluate(() => document.documentElement.classList.add("light"));
  await page.setViewportSize({ width: 390, height: 760 });
  assert.ok(await modal.evaluate(el => el.scrollWidth <= el.clientWidth));
  await page.screenshot({ path: path.join(root, "src-tauri/target/commit-composer-compact.png") });
  await modal.getByLabel("Move change set").selectOption({ label: "1. Docs: Document account setup" });
  await modal.getByRole("button", { name: "Create 1 commit", exact: true }).click();
  await modal.waitFor({ state: "hidden" });
  const data = await page.evaluate(() => window.applied);
  assert.equal(data.snapshot, "fixture");
  assert.equal(data.drafts[0].subject, "Docs: #release Document account setup");
  assert.deepEqual(data.drafts[0].units, [1, 0]);
  assert.equal(await page.evaluate(() => window.refreshed), true);
  assert.deepEqual(errors, []);
  console.log("PASS: shortcuts, selection, visual editing, bulk and per-commit tags, reordering, compact layout and confirmed message.");
} finally {
  await browser?.close();
  await server.close();
}
