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
  {path:'src/auth/login.ts',status:'modified',staged:true,partial:true,added:5,removed:2,kind:'source',chunk:{oldStart:10,oldLines:5,newStart:10,newLines:8,scope:'restoreSession'}},
  {path:'tests/login.test.ts',status:'added',staged:false,partial:false,added:4,removed:0,kind:'test'},
  {path:'src/auth/login.ts',status:'modified',staged:true,partial:false,added:5,removed:2,kind:'source',chunk:{oldStart:30,oldLines:5,newStart:33,newLines:8,scope:'configureTimeout'}},
];
const units=[{id:0,changes:[0]},{id:1,changes:[2]},{id:2,changes:[1]}];
window.testInvoke = async (command, args)=>{
  if (command==='plan_commit_composition') {
    window.plans = (window.plans || 0) + 1;
    return {
    branch:'main',snapshot:'fixture',changes,units,limited:false,
    groups:[
      {subject:'Fix(auth): Handle missing login state',description:'Handle a missing session.',units:[0,2]},
      {subject:'Docs: Document account setup',description:'Describe the login flow.',units:[1]},
    ],
    review:{findings:[],skipped:[],large_files:[]},
    };
  }
  if (command==='preview_commit_composition_change') {
    window.previewed=args;
    return {binary:false, limited:false, lines:[
      {line_type:'deletion',old_line_no:10,content:'const session = oldSession;'},
      {line_type:'addition',new_line_no:10,content:'const session = restoreSession();'},
    ]};
  }
  if (command==='apply_commit_composition') {
    window.applied=args;
    return args.drafts.map((_, i)=>String(i));
  }
  if (command==='find_buildable_commit_groups') {
    window.regrouped=args;
    return {
      groups:[{subject:'Fix(auth): Keep dependent session changes together',description:'Keep login and timeout handling together so the build passes.',units:args.drafts.flatMap(draft=>draft.units)}],
      review:{findings:[],skipped:[],large_files:[]},
    };
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
  await modal.getByText("Experimental feature", { exact: true }).waitFor();
  const list = modal.getByRole("region", { name: "Proposed commits" });
  const docs = list.getByRole("button", { name: /Document account setup/ });
  const fix = list.getByRole("button", { name: /Handle missing login state/ });
  await docs.waitFor();
  assert.equal(await list.locator("li").first().locator("button").getAttribute("title"), "Docs: Document account setup");
  await list.getByRole("button", { name: "Commit order", exact: true }).hover();
  await list.getByRole("tooltip").getByText(/created from bottom to top/).waitFor();
  assert.equal(await page.evaluate(() => window.previewed), undefined);
  await docs.click();
  await modal.getByText("configureTimeout · L33–40", { exact: true }).waitFor();
  assert.equal(await docs.getAttribute("aria-current"), "step");
  assert.match(await subject.inputValue(), /Document account setup/);
  await fix.click();
  await modal.getByText("Staged only", { exact: true }).waitFor();
  await modal.getByText("restoreSession · L10–17", { exact: true }).waitFor();
  const changed = modal.getByRole("list", { name: "Changed files" });
  await changed.getByRole("button", { name: /Modified.*src\/auth\/login.ts/ }).click();
  const diff = modal.getByRole("region", { name: "File diff" });
  await diff.getByText("const session = restoreSession();", { exact: true }).waitFor();
  assert.equal(await page.evaluate(() => window.previewed.change), 0);
  assert.equal(await diff.locator('[data-line="deletion"]').count(), 1);
  const moveFile = changed.getByRole("combobox", { name: "Move tests/login.test.ts", exact: true });
  await moveFile.selectOption({ label: "2. Docs: Document account setup" });
  assert.equal(await changed.getByRole("button", { name: /tests\/login.test.ts/ }).count(), 0);
  assert.equal(await changed.getByRole("button", { name: /src\/auth\/login.ts/ }).count(), 1);
  await docs.click();
  await changed.getByRole("button", { name: /Added.*tests\/login.test.ts/ }).waitFor();
  await fix.click();
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
  assert.equal(await modal.getByRole("checkbox", { name: /Close issue/ }).count(), 0);
  await modal.getByLabel("Commit tag", { exact: true }).fill("#123");
  await list.getByRole("button", { name: /Fix\(account\): #123 Preserve session after login/ }).waitFor();
  assert.match(await docs.innerText(), /#release/);
  await modal.getByLabel("Description").fill("Keep the existing session when the login state changes.");
  await modal.getByRole("checkbox", { name: "Close issue #123", exact: true }).check();
  assert.match(await modal.getByText("Closes #123", { exact: true }).innerText(), /Closes #123/);
  await modal.getByLabel("Commit tag", { exact: true }).fill("#456");
  assert.equal(await modal.getByRole("checkbox", { name: "Close issue #456", exact: true }).isChecked(), false);
  await modal.getByLabel("Commit tag", { exact: true }).fill("#123");
  assert.equal(await modal.getByRole("checkbox", { name: "Close issue #123", exact: true }).isChecked(), false);
  await modal.getByRole("checkbox", { name: "Close issue #123", exact: true }).check();
  await modal.getByRole("button", { name: "Move commit up", exact: true }).click();
  assert.match(await list.locator("li").first().innerText(), /Preserve session after login/);
  await changed.getByRole("button", { name: /src\/auth\/login.ts/ }).click();
  await diff.getByText("const session = restoreSession();", { exact: true }).waitFor();
  assert.ok(await modal.evaluate(el => el.scrollWidth <= el.clientWidth && el.scrollHeight <= el.clientHeight));
  await diff.evaluate(el => new Promise((resolve, reject) => {
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.intersectionRatio > 0.9) { observer.disconnect(); clearTimeout(timeout); resolve(); }
    }, { root: el.closest(".composer-editor"), threshold: [0.9, 1] });
    const timeout = setTimeout(() => { observer.disconnect(); reject(new Error("The selected diff stayed below the editor viewport.")); }, 2000);
    observer.observe(el);
  }));
  await page.screenshot({ path: path.join(root, "src-tauri/target/commit-composer-desktop.png") });
  await page.evaluate(() => document.documentElement.classList.remove("light"));
  assert.equal(await list.getByRole("button", { name: /Preserve session after login/ }).evaluate(el => getComputedStyle(el).cursor), "pointer");
  await page.screenshot({ path: path.join(root, "src-tauri/target/commit-composer-dark.png"), animations: "disabled" });
  await page.evaluate(() => document.documentElement.classList.add("light"));
  await page.setViewportSize({ width: 390, height: 760 });
  assert.ok(await modal.evaluate(el => el.scrollWidth <= el.clientWidth));
  await page.screenshot({ path: path.join(root, "src-tauri/target/commit-composer-compact.png") });
  await modal.getByRole("checkbox", { name: "Verify every commit", exact: true }).check();
  assert.equal(await confirm.isDisabled(), true);
  await modal.getByRole("textbox", { name: "Build command", exact: true }).fill("cargo check --locked");
  await modal.getByRole("button", { name: "Find buildable groups", exact: true }).click();
  await modal.getByRole("button", { name: "Create 1 commit", exact: true }).waitFor();
  assert.equal(await page.evaluate(() => window.applied), undefined);
  assert.equal(await page.evaluate(() => window.regrouped.buildCommand), "cargo check --locked");
  assert.match(await modal.getByLabel("Description").inputValue(), /so the build passes/);
  assert.ok(await modal.evaluate(el => el.scrollWidth <= el.clientWidth));
  await modal.getByRole("button", { name: "Undo regroup", exact: true }).click();
  await confirm.waitFor();
  assert.match(await modal.getByLabel("Description").inputValue(), /Keep the existing session/);
  await confirm.click();
  await modal.waitFor({ state: "hidden" });
  const data = await page.evaluate(() => window.applied);
  assert.equal(data.buildCommand, "cargo check --locked");
  assert.equal(data.snapshot, "fixture");
  assert.equal(data.drafts[0].subject, "Docs: #release Document account setup");
  assert.deepEqual(data.drafts[0].units, [1, 2]);
  assert.doesNotMatch(data.drafts[0].description, /Closes/);
  assert.doesNotMatch(data.drafts[1].subject, /Closes/);
  assert.match(data.drafts[1].description, /\n\nCloses #123$/);
  assert.equal(data.drafts[1].description.match(/Closes #123/g).length, 1);
  assert.equal(await page.evaluate(() => window.refreshed), true);
  await page.setViewportSize({ width: 1120, height: 800 });
  await page.getByTitle("Visual commit builder", { exact: true }).click();
  await page.getByPlaceholder("#123", { exact: true }).fill("123");
  await page.getByPlaceholder("Optional body...").fill("Keep repository checks intact.\n\nFixes #123");
  await page.getByPlaceholder("discard only unstaged file changes").fill("Protect existing changes");
  await page.getByRole("checkbox", { name: "Close issue #123", exact: true }).check();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  assert.doesNotMatch(await page.getByPlaceholder("Commit message...", { exact: true }).inputValue(), /Closes/);
  assert.equal(await page.getByPlaceholder("Description (optional)...", { exact: true }).inputValue(), "Keep repository checks intact.\n\nCloses #123");
  assert.deepEqual(errors, []);
} finally {
  const closing = Promise.all([browser?.close(), server.close()]);
  server.httpServer?.closeAllConnections();
  await Promise.race([closing, new Promise((_, reject) => {
    setTimeout(() => reject(new Error("Playwright fixture did not shut down.")), 10000).unref();
  })]).catch(error => { console.error(error); process.exit(1); });
}
console.log("PASS: graph order, per-file moves and diff, editing, regrouping and compact layout.");
