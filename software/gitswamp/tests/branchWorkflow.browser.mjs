import assert from "node:assert/strict";
import { createServer } from "node:http";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { build } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const root = fileURLToPath(new URL("..", import.meta.url));
const html = `<!doctype html><html class="light"><head><link rel="icon" href="data:,"></head><body><main id="app"></main><script type="module">
import {createApp,h,ref} from 'vue';
import AppHeader from '/src/view/shell/AppHeader.vue';
import InitDialog from '/src/view/repository/InitDialog.vue';
import CommitGraph from '/src/view/commit/CommitGraph.vue';
import Dialog from '/src/features/repository/workflow/BranchWorkflowDialog.vue';
import {workflowForPath,workflowName} from '/src/features/repository/workflow/branchWorkflow.ts';
import {isBranchWorkflowShortcut} from '/src/features/shell/keyboardShortcuts.ts';
import '/src/styles/index.css';
window.calls=[];
window.testInvoke=async (command,args)=>{
  window.calls.push({command,args});
  if (command === 'inspect_branch_workflow') {
    if (args.path === 'repo-c') {
      return {key:args.path,current:'change/search',local:['main','change/search'],remote:['main','change/search'],primary:'main',development:null,suggestion:'swamp',unborn:false,minimal:false,blocked:null};
    }
    const [primary,development] = args.path === 'repo-a' ? ['main','dev'] : ['trunk',null];
    return {key: args.path, current: primary, local: [primary, ...(development ? [development, 'feature/existing'] : [])], remote: [],
       primary, development, suggestion: development ? 'gitflow' : 'trunk', unborn: false, minimal: false, blocked: null};
  }
  if (command === 'validate_branch_workflow') return;
  if (command === 'preview_branch_workflow') {
    if (args.request.action === 'handoff') {
      return {token:'handoff', steps:[], comparisons:[{target:'main',ahead:2,behind:1,conflicts:!window.cleanHandoff,conflictPaths:window.cleanHandoff?[]:['shared.txt']}],
        handoff:{source:'change/search',target:'main',published:true,remote:'origin',targetFetched:true,sourceSync:'in-sync',
          checkpoint:{changedPaths:4,testPaths:1,reviewAreas:['Schema and migrations'],commits:[{sha:'abcd1234',subject:'Review search'}]}}};
    }
    return {token:'reviewed', steps:['Create and switch to '+args.config.feature+args.request.name+' from '+args.config.development], comparisons:[], handoff:null};
  }
  if (command === 'verify_branch_handoff') {
    if (window.staleHandoff) throw Error('The reviewed refs changed. Preview the handoff again.');
    return;
  }
  if (command === 'execute_branch_workflow') return {status:'applied', completed:['Created feature/search'], remaining:[], error:null};
  return null;
};
const repo=ref('repo-a'), open=ref(false), init=ref(false), note=ref(0);
window.changeRepo=(value)=>{repo.value=value;};
window.showInit=()=>{init.value=true;};
window.blockStorage=()=>{Storage.prototype.setItem=()=>{throw Error('No storage');}; Storage.prototype.getItem=()=>{throw Error('No storage');};};
window.addEventListener('keydown',e=>{if(isBranchWorkflowShortcut(e))open.value=true;});
createApp({render:()=>{
  void note.value;
  const config=workflowForPath(repo.value);
  const workflowLabel=config && config.strategy!=='manual' ? workflowName(config.strategy) : undefined;
  return [
    h(AppHeader,{loading:false,ghostActive:false,workflowLabel,onBranchWorkflow:()=>open.value=true}),
    h('div',{class:'relative h-[380px]'},[
      h(CommitGraph,{repoPath:repo.value,commits:[],selected:null,hasWorkingChanges:true,currentBranch:'main',headSha:'111',workflowLabel,onBranchWorkflow:()=>open.value=true}),
      h('aside',{class:'absolute z-[120] right-0 top-0 h-full w-[360px] bg-[var(--card)]','aria-label':'Commit info'},'Commit info'),
    ]),
    open.value ? h(Dialog,{key:repo.value,repoPath:repo.value,canOpenPullRequest:repo.value==='repo-c',remoteName:'origin',
      fetchRefs:async()=>{window.cleanHandoff=true;window.fetches=(window.fetches||0)+1;},
      onClose:()=>open.value=false,onSaved:()=>note.value++,onOutcome:r=>window.outcome=r,onPullRequest:draft=>window.handoff=draft}) : null,
    h(InitDialog,{visible:init.value,onInit:(path,branch,workflow)=>{window.initialized={path,branch,workflow};init.value=false;}})
  ];
}}).mount('#app');
</script></body></html>`;
const mock = {
  "@tauri-apps/api/core": "export const invoke=(...args)=>window.testInvoke(...args);",
  "@tauri-apps/api/path": "export const join=async (...parts)=>parts.join('/'); export const homeDir=async ()=>'/home'; export const sep=()=>'/';",
  "@tauri-apps/plugin-dialog": "export const open=async ()=>null;",
};
const prefix = "\0workflow:";
const entry = html.split('<script type="module">')[1].split("</script>")[0];
const result = await build({
  root, configFile: false, publicDir: false, logLevel: "error",
  plugins: [{
    name: "workflow-test", enforce: "pre",
    resolveId(id) {
      if (id === "virtual:workflow-test") return "\0workflow-entry";
      if (Object.hasOwn(mock, id)) return prefix + id;
    },
    load(id) {
      if (id === "\0workflow-entry") return entry;
      if (id.startsWith(prefix)) return mock[id.slice(prefix.length)];
    },
  }, vue(), tailwindcss()],
  resolve: { alias: { "@": path.join(root, "src") } },
  build: { write: false, minify: false, cssCodeSplit: false, rollupOptions: { input: "virtual:workflow-test" } },
});
const assets = new Map(result.output.map(asset => ["/" + asset.fileName, asset]));
const script = "/" + result.output.find(asset => asset.type === "chunk" && asset.isEntry).fileName;
const css = "/" + result.output.find(asset => asset.fileName.endsWith(".css")).fileName;
const server = createServer((req, res) => {
  if (req.url === "/__workflow__") {
    res.setHeader("Content-Type", "text/html");
    res.end(html.split('<script type="module">')[0].replace("</head>", '<link rel="stylesheet" href="'+css+'"></head>')
      + '<script type="module" src="'+script+'"></script></body></html>');
    return;
  }
  const asset = assets.get(new URL(req.url, "http://localhost").pathname);
  if (asset) {
    if (asset.type === "chunk") res.setHeader("Content-Type", "application/javascript");
    else if (asset.fileName.endsWith(".css")) res.setHeader("Content-Type", "text/css");
    res.end(asset.type === "chunk" ? asset.code : asset.source);
    return;
  }
  res.writeHead(404).end();
});

let browser;
try {
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  browser = await chromium.launch({ headless: true, ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}) });
  const page = await browser.newPage({ viewport: { width: 1100, height: 760 } });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  page.on("response", response => { if (response.status() >= 400) console.error(response.status(), response.url()); });
  page.on("console", message => { if (message.type() === "error") errors.push(message.text()); });
  await page.goto("http://127.0.0.1:" + server.address().port + "/__workflow__");
  const branchActions = page.getByTitle("Branch actions", { exact: true });
  try { await branchActions.click(); }
  catch (error) { throw new Error("Workflow fixture did not start: " + errors.join("; "), { cause: error }); }
  assert.equal(await page.evaluate(() => window.calls.some(call => call.command === "inspect_branch_workflow")), false);
  const openWorkflow = page.getByRole("button", { name: "Branching workflow", exact: true });
  const overlaps = await openWorkflow.evaluate(button => {
    const { x, y, width, height } = button.getBoundingClientRect();
    return document.elementsFromPoint(x + width / 2, y + height / 2).some(element => element.getAttribute("aria-label") === "Commit info");
  });
  assert.ok(overlaps, "The menu should overlap the commit info panel in this fixture");
  await page.keyboard.press("Escape");
  await openWorkflow.waitFor({ state: "hidden" });
  assert.equal(await branchActions.getAttribute("aria-expanded"), "false");
  await branchActions.click();
  await openWorkflow.click();
  const modal = page.locator("dialog.workflow-dialog");
  await modal.getByRole("button", { name: /Gitflow/ }).click();
  assert.equal(await modal.getByLabel("Development branch", { exact: true }).inputValue(), "dev");
  await modal.getByRole("button", { name: "Save strategy", exact: true }).click();
  const tooltip = modal.getByRole("tooltip");
  for (const [option, description] of [
    ["Start feature", /feature branch based on 'dev'/],
    ["Start release", /release branch from 'dev'/],
    ["Start hotfix", /hotfix branch from 'main'/],
    ["Start maintenance branch", /from a release tag, or from 'main'/],
    ["Finish branch", /feature into 'dev'.*into 'main' and 'dev'/],
    ["Delete merged branch", /fully merged and has no remote counterpart/],
    ["Create release tag", /latest commit of 'main'/],
    ["Manage strategy", /active strategy, its base branches and its naming prefixes/],
  ]) {
    const trigger = modal.getByRole("button", { name: option, exact: true });
    await trigger.hover();
    assert.match(await tooltip.innerText(), description);
    assert.equal(await trigger.getAttribute("aria-describedby"), await tooltip.getAttribute("id"));
  }
  const closeWorkflow = modal.getByTitle("Close branching workflow", { exact: true });
  await closeWorkflow.hover();
  await tooltip.waitFor({ state: "hidden" });
  await closeWorkflow.focus();
  await page.keyboard.press("Tab");
  assert.match(await tooltip.innerText(), /feature branch based on 'dev'/);
  await page.keyboard.press("Escape");
  await tooltip.waitFor({ state: "hidden" });
  assert.ok(await modal.isVisible());
  await page.setViewportSize({ width: 365, height: 450 });
  await modal.getByRole("button", { name: "Manage strategy", exact: true }).hover();
  const hintBounds = await tooltip.boundingBox();
  assert.ok(hintBounds.x >= 8 && hintBounds.y >= 0 && hintBounds.x + hintBounds.width <= 357 && hintBounds.y + hintBounds.height <= 450);
  await page.setViewportSize({ width: 1100, height: 760 });
  await modal.getByRole("button", { name: "Start feature", exact: true }).click();
  await modal.getByLabel("Branch name", { exact: true }).fill("search");
  await modal.getByRole("button", { name: "Preview", exact: true }).click();
  await modal.getByText("Create and switch to feature/search from dev", { exact: true }).waitFor();
  assert.equal(await page.evaluate(() => window.outcome), undefined);
  await page.screenshot({ path: path.join(root, "src-tauri/target/branch-workflow-desktop.png") });
  await modal.getByRole("button", { name: "Confirm", exact: true }).click();
  await modal.getByRole("button", { name: "Done", exact: true }).click();
  await modal.waitFor({ state: "hidden" });
  assert.equal(await page.evaluate(() => window.calls.find(call => call.command === "execute_branch_workflow").args.token), "reviewed");
  await page.getByText(/● Working(?: Changes|\.\.\.)/).first().click({ button: "right" });
  await page.getByRole("button", { name: "Gitflow", exact: true }).click();
  await modal.getByRole("button", { name: "Manage strategy", exact: true }).click();
  await modal.getByRole("button", { name: /Manual Git/ }).click();
  await modal.getByRole("button", { name: "Save strategy", exact: true }).click();
  await modal.getByText("No active branching strategy.", { exact: true }).waitFor();
  await modal.getByTitle("Close branching workflow", { exact: true }).click();
  await page.evaluate(() => { window.changeRepo("repo-b"); window.blockStorage(); });
  await page.keyboard.press("Control+Alt+b");
  await modal.getByRole("button", { name: /Trunk-Based/ }).click();
  assert.equal(await modal.getByLabel("Trunk branch", { exact: true }).inputValue(), "trunk");
  await modal.getByRole("button", { name: "Save strategy", exact: true }).click();
  await modal.getByText("Settings are available for this session only.", { exact: true }).waitFor();
  await page.setViewportSize({ width: 390, height: 680 });
  assert.ok(await modal.evaluate(el => el.scrollWidth <= el.clientWidth));
  await page.screenshot({ path: path.join(root, "src-tauri/target/branch-workflow-compact.png") });
  await modal.getByTitle("Close branching workflow", { exact: true }).click();
  await page.evaluate(() => window.changeRepo("repo-c"));
  await page.keyboard.press("Control+Alt+b");
  await modal.getByRole("button", { name: /GitSwamp Handoff/ }).click();
  await modal.getByRole("button", { name: "Save strategy", exact: true }).click();
  await modal.getByRole("button", { name: "Review pull request handoff", exact: true }).click();
  await modal.getByRole("button", { name: "Preview", exact: true }).click();
  await modal.getByText("Resolve merge conflicts first", { exact: true }).waitFor();
  await modal.getByText("shared.txt", { exact: true }).waitFor();
  assert.ok(await modal.getByRole("button", { name: "Open pull request form", exact: true }).isDisabled());
  await modal.getByRole("button", { name: "Fetch & recheck", exact: true }).click();
  await modal.getByText("Ready for pull request review", { exact: true }).waitFor();
  assert.equal(await page.evaluate(() => window.fetches), 1);
  await modal.getByText(/4 changed paths/).waitFor();
  await modal.getByText("Recent source commits (1)", { exact: true }).click();
  await modal.getByText("Review search", { exact: false }).waitFor();
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await modal.getByRole("button", { name: "Copy notes", exact: true }).click();
  await modal.getByText("Handoff notes copied.", { exact: true }).waitFor();
  assert.match(await page.evaluate(() => navigator.clipboard.readText()), /Schema and migrations/);
  await modal.getByRole("checkbox", { name: "Include handoff notes in PR" }).check();
  assert.ok(await modal.evaluate(el => el.scrollWidth <= el.clientWidth));
  await page.screenshot({ path: path.join(root, "src-tauri/target/branch-workflow-handoff.png") });
  await page.evaluate(() => { window.staleHandoff = true; });
  await modal.getByRole("button", { name: "Open pull request form", exact: true }).click();
  await modal.getByText(/reviewed refs changed/).waitFor();
  assert.equal(await page.evaluate(() => window.handoff), undefined);
  await page.evaluate(() => { window.staleHandoff = false; });
  await modal.getByRole("button", { name: "Recheck", exact: true }).click();
  await modal.getByRole("button", { name: "Open pull request form", exact: true }).click();
  await page.waitForFunction(() => !!window.handoff);
  assert.equal(await page.evaluate(() => window.handoff.target), "main");
  const description = await page.evaluate(() => window.handoff.description);
  assert.ok(description.includes("Schema and migrations") && description.includes("- [ ]"));
  assert.ok(!description.includes("Review search"));
  await modal.getByTitle("Close branching workflow", { exact: true }).click();
  await page.setViewportSize({ width: 1100, height: 760 });
  await page.evaluate(() => window.showInit());
  await page.getByRole("button", { name: /Local Only/ }).first().click();
  await page.getByPlaceholder("my-project").fill("example");
  await page.getByRole("button", { name: "Branching strategy (optional)" }).click();
  await page.getByRole("button", { name: /GitSwamp Handoff/ }).click();
  await page.getByLabel("Primary branch", { exact: true }).fill("integration");
  await page.getByRole("button", { name: "Use strategy", exact: true }).click();
  await page.getByRole("button", { name: "Create Repository", exact: true }).click();
  await page.waitForFunction(() => !!window.initialized);
  const initial = await page.evaluate(() => window.initialized);
  assert.equal(initial.branch, "integration");
  assert.equal(initial.workflow.strategy, "swamp");
  assert.deepEqual(errors, []);
} finally {
  const closing = Promise.all([browser?.close(), new Promise(resolve => server.close(resolve))]);
  server.closeAllConnections();
  await Promise.race([closing, new Promise((_, reject) => {
    setTimeout(() => reject(new Error("Workflow test did not shut down.")), 10000).unref();
  })]).catch(error => { console.error(error); process.exit(1); });
}
console.log("PASS: branch menu, graph context, shortcut, preview and confirm, disabling, session fallback and initialization.");
