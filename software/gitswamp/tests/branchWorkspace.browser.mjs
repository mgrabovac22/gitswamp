import assert from "node:assert/strict";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const root = fileURLToPath(new URL("..", import.meta.url));
const html = `<!doctype html><html class="light"><head><style>body,#app{margin:0;height:100dvh}main{display:flex;flex-direction:column;height:100%;overflow:hidden}</style></head><body><div id="app"></div><script type="module">
import { createApp, defineAsyncComponent, h, ref } from '/node_modules/.vite-workspace-test/deps/vue.js';
import Sidebar from '/src/view/repository/RepositorySidebar.vue';
import RepositoryTabs from '/src/view/repository/RepositoryTabs.vue';
import SecretDialog from '/src/shared/ui/SecretCommitDialog.vue';
import RemovalDialog from '/src/features/repository/workspace/WorktreeRemovalDialog.vue';
import {useWorktreeRemoval} from '/src/features/repository/workspace/useWorktreeRemoval.ts';
import '/src/styles/index.css';
const Workspace=defineAsyncComponent(()=>import('/src/features/repository/workspace/BranchWorkspaceView.vue'));
const open=ref(false), request=ref(null), root='/example/main', repositoryTab=ref(null), selectedTab=ref('main');
const removal=useWorktreeRemoval(()=>{});
const infoTab=(name,path)=>({name,path,current_branch:name,head_sha:null,is_clean:true,remotes:[]});
const branches=['main','feature/topic','hotfix','release','spare'].map(name=>({name,is_head:name==='main',is_remote:false,ahead:0,behind:0}));
const worktrees=Object.fromEntries(branches.map((branch,index)=>[branch.name,{
 path:branch.is_head?root:'/example/worktrees/'+branch.name,
 branch:branch.name,isPrimary:branch.is_head,headSha:String(index+1).repeat(40),
 files:[{path:'shared.txt',status:'modified',staged:false}],
}]));
const byPath=Object.fromEntries(Object.values(worktrees).map(item=>[item.path,item]));
const info=(sha,message)=>({sha,short_sha:sha.slice(0,7),message,author_name:'Test',author_email:'test@example.com',timestamp:1,time_ago:'today',parent_shas:[],refs:[]});
window.calls=[];window.worktrees=worktrees;
window.testInvoke=async (command,args={})=>{
 window.calls.push({command,args});
 const target=byPath[args.path];
 switch(command){
  case 'open_branch_workspace': return worktrees[args.branchRef.slice('refs/heads/'.length)];
  case 'remove_branch_workspace': {
    const item=byPath[args.worktreePath];
    if(!item || item.isPrimary || args.path!==root || item.branch!==args.branch) throw Error('wrong worktree');
    if(item.files.length) throw Error('This worktree has untracked changes.');
    item.removed=true;
    return;
  }
  case 'open_path_with_tool': window.revealedPath=args.path; return;
  case 'get_available_external_tools': return ['explorer','vscode','androidstudio','intellij','unknown'];
  case 'open_branch_workspace_with_application': return;
  case 'get_branch_workspace_status': return {headSha:target.headSha,operation:null,files:target.files.map(file=>({...file}))};
  case 'get_branch_workspace_history': return args.offset===0?[info(target.headSha,target.message||'History of '+target.branch)]:[];
  case 'get_commit_files': return [{path:'committed.txt',status:'modified',additions:1,deletions:0}];
  case 'get_working_diff': return {path:args.filePath,old_path:null,status:'modified',hunks:[],is_binary:false};
  case 'scan_secrets': return {findings:[],skipped:[],large_files:[],snapshot:'snapshot:'+target.branch};
  case 'mutate_branch_workspace':
    if(args.branch!==target.branch)throw Error('wrong worktree');
    if(args.action.kind==='stage'||args.action.kind==='unstage')target.files=target.files.map(file=>args.action.paths.includes(file.path)?{...file,staged:args.action.kind==='stage'}:file);
    if(args.action.kind==='commit'){target.files=target.files.filter(file=>!file.staged);target.message=args.action.message;target.headSha='a'.repeat(40);}
    return;
  case 'get_available_external_editors': case 'get_empty_directories': case 'get_commit_tree_paths': return [];
  case 'get_staged_diff_summary': return {total_lines_added:1,total_lines_removed:0,files_changed:1,file_types:['txt'],has_test_changes:false,has_migration_changes:false,inferred_scope:'app'};
  default: throw Error(command);
 }
};
localStorage.setItem('gitswamp-commit-analyzer-settings',JSON.stringify({enabled:false}));
localStorage.setItem('gitswamp-show-empty-dirs','false');
createApp({render:()=>h('main',[
 h(RepositoryTabs,{tabs:[{id:'main',path:root,label:'Original Repo',repo:infoTab('main',root)},...(repositoryTab.value?[{id:'linked',path:repositoryTab.value.path,label:repositoryTab.value.branch,repo:infoTab(repositoryTab.value.branch,repositoryTab.value.path),worktree:{sourcePath:root,branch:repositoryTab.value.branch}}]:[])],activeTabId:selectedTab.value,workspaceViewOpen:open.value,onCloseWorkspaceView:()=>open.value=false,onSelectTab:id=>{if(selectedTab.value!==id){selectedTab.value=id;open.value=false;}},onCloseTab:id=>{
   if(id==='linked' && repositoryTab.value)removal.requestRemoval({sourcePath:root,location:repositoryTab.value,finish:()=>repositoryTab.value=null});
 }}),
 ...(open.value?[
 h(Workspace,{repoPath:root,currentBranch:'main',localBranches:branches,remoteBranches:[],request:request.value,onRequestConsumed:()=>request.value=null,onClose:()=>open.value=false,onRemoveWorktree:removal.requestRemoval,onOpenRepository:location=>{window.openedPath=location.path;repositoryTab.value=location;selectedTab.value='linked';open.value=false;}}),
 h(SecretDialog,{repoPath:root}),
]:[
 h('button',{onClick:()=>open.value=true},'Open workspace view'),
 h('div',{style:'height:420px;width:230px'},[h(Sidebar,{repoPath:root,branches,remoteBranches:[],currentBranch:'main',tags:[],stashes:[],onOpenBranchWorkspace:branchRef=>{request.value={branchRef};open.value=true;}})]),
 ]),
 removal.pending.value?h(RemovalDialog,{location:removal.pending.value.location,busy:removal.busy.value,error:removal.error.value,onRemove:removal.remove,onKeep:removal.closeOnly,onCancel:removal.cancel}):null,
])}).mount('#app');
</script></body></html>`;

const mocks = {
  "@tauri-apps/api/core": "export const invoke=(...args)=>window.testInvoke(...args);export class Channel{}",
  "@tauri-apps/plugin-process": "export const relaunch=async()=>{};",
  "@tauri-apps/plugin-opener": "export const openUrl=async()=>{};",
};
const server = await createServer({
  root, configFile: false, cacheDir: "node_modules/.vite-workspace-test",
  define: { __APP_VERSION__: JSON.stringify("test") },
  plugins: [{
    name: "workspace-test-mocks", enforce: "pre",
    resolveId(id) { if (Object.hasOwn(mocks, id)) return `\0${id}`; },
    load(id) { return mocks[id.slice(1)]; },
    configureServer(vite) {
      vite.middlewares.use((req, res, next) => {
        if (req.url !== "/__branch_workspace__") return next();
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
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on("pageerror", error => { errors.push(error.message); console.error(error.message); });
  page.setDefaultTimeout(20000);
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__branch_workspace__`, { waitUntil: "domcontentloaded", timeout: 60000 });
  const branch = page.locator('button[aria-haspopup="menu"]').filter({ hasText: "feature/topic" });
  await branch.waitFor();
  assert.equal(await page.evaluate(() => calls.some(call => call.command.includes("branch_workspace"))), false);
  await branch.click({ button: "right" });
  await page.getByRole("menuitem", { name: "Open Branch Workspace" }).click();

  const workspace = page.getByRole("region", { name: "Branch Workspace", exact: true });
  const topic = workspace.getByRole("region", { name: "feature/topic", exact: true });
  const main = workspace.getByRole("region", { name: "main", exact: true });
  await topic.getByRole("button", { name: /History of feature\/topic/ }).waitFor();
  const select = workspace.getByRole("combobox", { name: "Choose a branch" });
  for (const name of ["main", "hotfix", "release"]) {
    await select.selectOption(`refs/heads/${name}`);
    await workspace.getByRole("button", { name: "Open branch workspace", exact: true }).click();
    await workspace.getByRole("button", { name: new RegExp(`History of ${name}`) }).waitFor();
  }
  assert.equal(await select.isDisabled(), true);
  assert.equal(await workspace.locator(".branch-pane").count(), 4);
  const topTabs = page.locator(".tabs-scroll");
  const originalTab = topTabs.getByText("Original Repo", { exact: true });
  const activeTab = topTabs.locator('[aria-current="page"]');
  assert.equal(await topTabs.getByText("Branch Workspace", { exact: true }).count(), 0);
  assert.equal(await activeTab.innerText(), "Original Repo");
  await originalTab.click();
  assert.equal(await workspace.locator(".branch-pane").count(), 4);
  assert.equal(await workspace.getByRole("button", { name: "Close main workspace", exact: true }).count(), 0);
  const boxes = await workspace.locator(".branch-pane").evaluateAll(elements => elements.map(el => {
    const box = el.getBoundingClientRect(); return { left: box.left, right: box.right, height: box.height };
  }));
  assert.ok(boxes.every((box, index) => box.height > 600 && (index === 0 || box.left > boxes[index - 1].right)));
  await page.screenshot({ path: path.join(root, "src-tauri/target/branch-workspace-desktop.png") });

  await workspace.getByRole("navigation", { name: "Open branch workspaces" }).getByRole("button", { name: "hotfix", exact: true }).click();
  assert.match(await workspace.getByLabel("Active worktree").innerText(), /hotfix/);
  await workspace.getByTitle("Show active worktree folder").click();
  assert.equal(await page.evaluate(() => revealedPath), "/example/worktrees/hotfix");
  assert.equal(await page.evaluate(() => calls.some(call => call.command === "get_available_external_tools")), false);
  const openWith = topic.getByRole("button", { name: "Open feature/topic with", exact: true });
  const editorMenu = page.getByRole("menu", { name: "Open feature/topic with", exact: true });
  await openWith.click();
  await editorMenu.getByRole("menuitem", { name: "VS Code", exact: true }).waitFor();
  assert.equal(await editorMenu.getByRole("menuitem", { name: "Visual Studio", exact: true }).count(), 0);
  await editorMenu.getByRole("menuitem", { name: "Android Studio", exact: true }).waitFor();
  await editorMenu.getByRole("menuitem", { name: "IntelliJ IDEA", exact: true }).waitFor();
  await editorMenu.getByRole("menuitem", { name: "VS Code", exact: true }).click();
  await page.waitForFunction(() => calls.some(call => call.command === "open_path_with_tool" && call.args.tool === "vscode" && call.args.path === "/example/worktrees/feature/topic"));
  await openWith.focus();
  await page.keyboard.press("ArrowDown");
  await editorMenu.getByRole("menuitem", { name: "File Explorer" }).waitFor();
  await page.keyboard.press("Escape");
  await editorMenu.waitFor({ state: "hidden" });
  assert.equal(await openWith.evaluate(button => document.activeElement === button), true);
  await openWith.click();
  await editorMenu.getByRole("menuitem", { name: "Choose application..." }).click();
  await page.waitForFunction(() => calls.some(call => call.command === "open_branch_workspace_with_application" && call.args.path === "/example/worktrees/feature/topic" && call.args.branch === "feature/topic"));
  assert.equal(await page.evaluate(() => calls.filter(call => call.command === "get_available_external_tools").length), 1);
  const details = topic.locator(".pane-details");
  const compactHeight = (await details.boundingBox()).height;
  await topic.getByRole("button", { name: "Expand changes panel" }).click();
  assert.ok((await details.boundingBox()).height > compactHeight);
  await topic.getByRole("button", { name: "Compact changes panel" }).click();

  await topic.getByRole("button", { name: "Stage All", exact: true }).click();
  await topic.getByRole("button", { name: "Unstage All", exact: true }).waitFor();
  assert.equal(await main.getByRole("button", { name: "Unstage All", exact: true }).count(), 0);
  assert.equal(await page.evaluate(() => calls.find(call => call.command === "mutate_branch_workspace").args.path), "/example/worktrees/feature/topic");
  await topic.getByText("shared.txt", { exact: true }).click();
  await page.waitForFunction(() => calls.some(call => call.command === "get_working_diff" && call.args.path === "/example/worktrees/feature/topic" && call.args.staged));
  await workspace.getByRole("button", { name: "Close diff view" }).click();

  await topic.getByRole("button", { name: /History of feature\/topic/ }).click();
  await page.waitForFunction(() => calls.some(call => call.command === "get_commit_files" && call.args.path === "/example/worktrees/feature/topic"));
  await topic.getByRole("button", { name: /Working Changes/ }).click();
  await topic.getByPlaceholder("Commit message...", { exact: true }).fill("Fix workspace isolation");
  await topic.getByRole("button", { name: "Commit to feature/topic" }).click();
  await topic.getByRole("button", { name: /Fix workspace isolation/ }).waitFor();
  assert.equal(await page.evaluate(() => calls.find(call => call.command === "scan_secrets").args.includeSecrets), false);
  await main.getByText("shared.txt", { exact: true }).waitFor();

  const hotfixClose = workspace.getByRole("button", { name: "Close hotfix workspace", exact: true });
  const dialog = page.getByRole("dialog", { name: "Remove worktree?", exact: true });
  await hotfixClose.click();
  await dialog.waitFor();
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  assert.equal(await page.evaluate(() => calls.some(call => call.command === "remove_branch_workspace")), false);
  await hotfixClose.click();
  await dialog.getByRole("button", { name: "Remove worktree", exact: true }).click();
  await dialog.getByRole("alert").getByText(/untracked changes/).waitFor();
  await dialog.getByRole("button", { name: "Close without removing" }).click();
  assert.equal(await page.evaluate(() => Boolean(worktrees.hotfix.removed)), false);
  assert.equal(await workspace.locator(".branch-pane").count(), 3);
  await select.selectOption("refs/heads/hotfix");
  await workspace.getByRole("button", { name: "Open branch workspace", exact: true }).click();
  await workspace.getByRole("button", { name: /History of hotfix/ }).waitFor();

  await workspace.getByRole("button", { name: "Close Branch Workspace", exact: true }).click();
  await workspace.waitFor({ state: "hidden" });
  assert.equal(await activeTab.innerText(), "Original Repo");
  const requests = await page.evaluate(() => calls.filter(call => call.command === "get_branch_workspace_status").length);
  await page.waitForTimeout(2800);
  assert.equal(await page.evaluate(() => calls.filter(call => call.command === "get_branch_workspace_status").length), requests);
  await page.getByRole("button", { name: "Open workspace view", exact: true }).click();
  await topic.getByRole("button", { name: /Fix workspace isolation/ }).waitFor();
  assert.equal(await workspace.locator(".branch-pane").count(), 4);
  await page.setViewportSize({ width: 420, height: 760 });
  await workspace.getByRole("navigation", { name: "Open branch workspaces" }).getByRole("button", { name: "release", exact: true }).click();
  await page.waitForFunction(() => document.querySelector(".workspace-grid")?.scrollLeft > 0);
  assert.ok(await workspace.evaluate(el => el.clientWidth === el.scrollWidth));
  const release = workspace.getByRole("region", { name: "release", exact: true });
  assert.ok(await release.getByText("shared.txt", { exact: true }).evaluate(el => {
    const file = el.getBoundingClientRect();
    const list = el.closest(".overflow-y-auto").getBoundingClientRect();
    return Math.min(file.bottom, list.bottom) - Math.max(file.top, list.top) > 5;
  }));
  await page.screenshot({ path: path.join(root, "src-tauri/target/branch-workspace-narrow.png") });
  await release.getByRole("button", { name: "Open release with", exact: true }).click();
  const narrowMenu = page.getByRole("menu", { name: "Open release with", exact: true });
  await narrowMenu.getByRole("menuitem", { name: "VS Code", exact: true }).waitFor();
  const menuBounds = await narrowMenu.boundingBox();
  assert.ok(menuBounds.x >= 0 && menuBounds.x + menuBounds.width <= 420);
  await page.screenshot({ path: path.join(root, "src-tauri/target/branch-workspace-open-with.png") });
  await narrowMenu.getByRole("menuitem", { name: "File Explorer" }).click();
  await page.waitForFunction(() => revealedPath === "/example/worktrees/release");
  await release.getByRole("button", { name: "Open worktree in repository tab" }).click();
  assert.equal(await page.evaluate(() => openedPath), "/example/worktrees/release");
  await workspace.waitFor({ state: "hidden" });
  assert.equal(await activeTab.innerText(), "release");
  await originalTab.click();
  assert.equal(await activeTab.innerText(), "Original Repo");
  await page.getByTitle("Close release worktree tab", { exact: true }).click();
  await dialog.waitFor();
  await page.evaluate(() => { worktrees.release.files=[]; });
  await dialog.getByRole("button", { name: "Remove worktree", exact: true }).click();
  await dialog.waitFor({ state: "hidden" });
  assert.equal(await page.evaluate(() => worktrees.release.removed), true);
  assert.equal(await page.getByTitle("Close release worktree tab", { exact: true }).count(), 0);
  assert.deepEqual(errors, []);
  console.log("PASS: same-tab Branch Workspace, explicit linked tab, four worktrees, Open with, responsive panels, and confirmed removal.");
} finally {
  await browser?.close();
  await server.close();
}
