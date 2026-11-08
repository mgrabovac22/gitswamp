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
import {createApp,defineAsyncComponent,h,ref} from '/node_modules/.vite-polish-test/deps/vue.js';
import InitDialog from '/src/view/repository/InitDialog.vue';
import PushDialog from '/src/view/repository/MultiPlatformPushDialog.vue';
import ConflictDialog from '/src/features/repository/ConflictAppearanceDialog.vue';
import Sidebar from '/src/view/repository/RepositorySidebar.vue';
import ToastContainer from '/src/shared/ui/ToastContainer.vue';
import {useUndoableDestructiveAction} from '/src/shared/notifications/useUndoableDestructiveAction.ts';
import {useConflictVisualisation} from '/src/features/repository/useConflictVisualisation.ts';
import {useOperationVisualisation} from '/src/features/repository/useOperationVisualisation.ts';
import {storeVisualiseOperations} from '/src/shared/config/visualiseOperationsPreferences.ts';
import {publishOperationPreview,publishPendingOperationPreview} from '/src/shared/notifications/gitOperationPreview.ts';
import '/src/styles/index.css';
const repo=ref('repo'),ready=ref(true),files=ref([]),mode=ref('');
window.setFiles=(...paths)=>files.value=paths.map(path=>({path,conflicted:true,staged:false,status:'conflicted'}));
window.enable=(enabled=true)=>storeVisualiseOperations(enabled);
window.setMode=value=>mode.value=value;
window.switchRepo=(next='other')=>{repo.value=next;ready.value=false;files.value=[]};
window.loadRepo=(...paths)=>{setFiles(...paths);ready.value=true};
window.previewReads=[];
window.operationReads=[];
window.showOperation=(kind='merge')=>publishOperationPreview({kind,repoPath:repo.value,source:kind==='merge'?'origin/feature/conflict-visualisation':'deadbee',target:'main',resultSha:'caffeed'+('0'.repeat(33)),sourceSha:'deadbee'+('0'.repeat(33))});
window.showRefOperation=(kind)=>publishOperationPreview({kind,repoPath:repo.value,target:kind==='reset'?'main':'feature/topic',sourceSha:'c0ffee0',previousSha:'deadbee',resultSha:'caffeed',mode:'mixed',previousName:'feature/old-name'});
window.showTransfer=(kind,provider='gitlab')=>publishOperationPreview({kind,repoPath:repo.value,target:'feature/topic',endpoint:{label:'origin',provider},sha:'caffeed'});
window.destructiveRuns=0;
window.testInvoke=async(command,args)=>{
 if(command==='get_operation_preview'){
   window.operationReads.push(args);
   if(window.holdOperation)await new Promise(resolve=>window.releaseOperation=resolve);
   return {before:{sha:'c0ffee0',subject:'Previous tip'},result:{sha:args.resultSha,subject:'Result commit subject'},
     incoming:Array.from({length:args.kind==='merge'?6:1},(_,i)=>({sha:'deadbe'+i,subject:'Incoming commit '+i})),hasMore:args.kind==='merge'};
 }
 if(command!=='get_conflict_preview')throw Error(command);
 window.previewReads.push({path:args.filePath,time:performance.now()});
 const context={number:3,text:'// Shared context',conflicted:false};
 return {ours:args.filePath,theirs:args.filePath,ranges:[[4,10],[20,26]],partial:false,
   ours_code:[context,{number:5,text:'const color = "blue";',conflicted:true}],
   theirs_code:[context,{number:8,text:'const color = "green";',conflicted:true}]};
};
createApp({setup(){
 const {preview,close}=useConflictVisualisation(()=>repo.value,()=>ready.value,()=>files.value);
 const {preview:operation,close:closeOperation}=useOperationVisualisation(()=>repo.value,()=>Boolean(preview.value)||files.value.length>0);
 const OperationDialog=defineAsyncComponent(()=>import('/src/features/repository/OperationVisualisationDialog.vue'));
 const {scheduleDestructiveAction}=useUndoableDestructiveAction(()=>repo.value);
 window.scheduleDeletion=(withPreview=false)=>scheduleDestructiveAction({message:'Delete feature/topic in 5 seconds.',run:()=>window.destructiveRuns++,onPending:withPreview?undo=>publishPendingOperationPreview({kind:'branch-deleted',repoPath:repo.value,target:'feature/topic'},undo):undefined});
 return ()=>h('main',{class:'p-4'},[
 h('button',{onClick:()=>window.graphClicks=(window.graphClicks||0)+1},'Commit graph'),
 h(InitDialog,{visible:mode.value==='init',onInit:(path,branch)=>{window.initResult={path,branch};mode.value=''},onClose:()=>mode.value=''}),
 h(PushDialog,{visible:mode.value==='push',repoName:'test',availablePlatforms:{github:'test'},pushing:false,onClose:()=>mode.value=''}),
 mode.value==='sidebar'?h('div',{class:'h-[320px] w-[230px]'},[h(Sidebar,{repoPath:repo.value,currentBranch:'main',branches:[{name:'feature/topic',is_head:false,is_remote:false}],remoteBranches:['origin/feature/topic','upstream/feature/topic'].map(name=>({name,is_head:false,is_remote:true})),tags:[],stashes:[],onRenameBranch:name=>window.renameBranch=name,onCreateBranchAt:point=>window.branchPoint=point,onCheckout:name=>window.checkedOut=name,onDeleteBranch:()=>window.scheduleDeletion()})]):null,
 h(ToastContainer),
 preview.value?h(ConflictDialog,{preview:preview.value,onClose:close}):null,
 operation.value?h(OperationDialog,{key:operation.value.kind+':'+operation.value.target+':'+(operation.value.resultSha??'')+':'+Boolean(operation.value.pendingUndo),preview:operation.value,onClose:closeOperation}):null
 ]);
}}).mount('#app');
</script></body></html>`;
const modules = {
  "@tauri-apps/api/core": "export const invoke=(...args)=>window.testInvoke(...args);export class Channel{}",
  "@tauri-apps/api/path": "export const homeDir=async()=>'/home/test';export const sep=()=>'/';export const join=async(...p)=>p.join('/');",
  "@tauri-apps/plugin-dialog": "export const open=async()=>null;",
  "@tauri-apps/plugin-process": "export const relaunch=async()=>{};",
  "@tauri-apps/plugin-opener": "export const openUrl=async()=>{};",
};
const server = await createServer({
  root, configFile:false, cacheDir:"node_modules/.vite-polish-test",
  plugins:[{
    name:"polish-test-mocks",enforce:"pre",
    resolveId(id){if(Object.hasOwn(modules,id))return '\0'+id;},
    load(id){return modules[id.slice(1)];},
    configureServer(server){server.middlewares.use((req,res,next)=>{
      if(req.url!=="/__repository_polish__")return next();
      void server.transformIndexHtml(req.url,html).then(result=>{res.setHeader("Content-Type","text/html");res.end(result);});
    });},
  },vue(),tailwindcss()],
  resolve:{alias:{"@":path.join(root,"src")}},
  optimizeDeps:{entries:[],noDiscovery:true,include:["vue","lucide-vue-next"]},
  server:{port:0,host:"127.0.0.1"},
});
let browser;
try {
  await server.listen();
  browser=await chromium.launch({headless:true,...(process.env.CHROME_PATH?{executablePath:process.env.CHROME_PATH}:{})});
  const page=await browser.newPage({viewport:{width:1100,height:800}}), errors=[];
  page.on("pageerror",error=>{errors.push(error.message);console.error(error.message);});
  page.setDefaultTimeout(15000);
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__repository_polish__`,{waitUntil:"domcontentloaded",timeout:60000});
  await page.getByRole("button",{name:"Commit graph"}).waitFor();

  await page.evaluate(()=>setMode("init"));
  const github=page.getByRole("button",{name:"GitHub Push to GitHub",exact:true}).locator("svg");
  assert.notEqual(await github.evaluate(svg=>getComputedStyle(svg).color),"rgb(255, 255, 255)");
  await page.getByRole("button",{name:/Local Only/}).click();
  assert.equal(await page.getByLabel("Initialize in").inputValue(),"/home/test/Repozitoriji");
  await page.getByPlaceholder("my-project").fill("project");
  await page.getByRole("button",{name:"Create Repository",exact:true}).click();
  await page.waitForFunction(()=>window.initResult?.path==="/home/test/Repozitoriji/project");
  await page.evaluate(()=>{localStorage.setItem("gitswamp-clone-path","/data/repositories");setMode("init");});
  await page.getByRole("button",{name:/Local Only/}).click();
  await page.waitForFunction(()=>document.getElementById("init-repository-directory")?.value==="/data/repositories");
  await page.getByRole("button",{name:"Close init dialog"}).click();
  await page.evaluate(()=>setMode("push"));
  const push=page.getByRole("heading",{name:"Push to Platform"}).locator("xpath=../..");
  const light=await push.evaluate(el=>getComputedStyle(el).backgroundColor);
  await page.evaluate(()=>document.documentElement.classList.remove("light"));
  assert.notEqual(await push.evaluate(el=>getComputedStyle(el).backgroundColor),light);
  await page.getByRole("button",{name:"Close push dialog"}).click();

  const popup=page.getByRole("dialog");
  await page.evaluate(()=>showOperation());
  await page.evaluate(()=>showRefOperation('branch-deleted'));
  assert.equal(await page.evaluate(()=>operationReads.length),0);
  assert.equal(await popup.count(),0);
  await page.evaluate(()=>setFiles("existing.txt"));
  assert.equal(await popup.count(),0);
  assert.equal(await page.evaluate(()=>previewReads.length),0);
  await page.evaluate(()=>enable());
  assert.equal(await popup.count(),0);
  await page.evaluate(()=>setFiles("existing.txt","src/new.ts","docs/readme.md",...Array.from({length:13},(_,i)=>"extra-"+i+".ts")));
  await popup.waitFor();
  await popup.getByText("Working tree lines 4–10",{exact:true}).waitFor();
  assert.equal(await popup.getByLabel("File progress").innerText(),"1 / 15");
  const current=popup.getByRole("region",{name:"Current code"});
  const incoming=popup.getByRole("region",{name:"Incoming code"});
  assert.match(await current.innerText(),/const color = "blue"/);
  assert.match(await incoming.innerText(),/const color = "green"/);
  assert.equal(await popup.locator(".conflicted-line").count(),2);
  await page.waitForFunction(()=>{
    const row=document.querySelector(".conflicted-line");
    return row && getComputedStyle(row).borderLeftColor!=="rgba(0, 0, 0, 0)";
  });
  const a=await current.boundingBox(), b=await incoming.boundingBox();
  assert.ok(Math.min(a.x+a.width,b.x+b.width)-Math.max(a.x,b.x)>a.width*.65);
  assert.ok(Math.min(a.y+a.height,b.y+b.height)-Math.max(a.y,b.y)>a.height*.65);
  assert.equal(await page.evaluate(()=>previewReads.length),1);
  await page.screenshot({path:path.join(root,"src-tauri/target/conflict-preview-desktop.png")});
  await popup.locator(".file-name").getByText("docs/readme.md",{exact:true}).waitFor();
  await page.waitForFunction(()=>previewReads.length===2);
  const gap=await page.evaluate(()=>previewReads[1].time-previewReads[0].time);
  assert.ok(gap>=2990 && gap<4100,`File advanced after ${gap}ms`);
  await popup.getByRole("button",{name:"Close conflict preview"}).click();
  await page.getByRole("button",{name:"Commit graph"}).click();
  assert.equal(await page.evaluate(()=>graphClicks),1);
  await page.evaluate(()=>{switchRepo();loadRepo("already-unmerged.txt");});
  assert.equal(await popup.count(),0);

  await page.setViewportSize({width:420,height:760});
  await page.evaluate(()=>setFiles("already-unmerged.txt","a-very-long-directory/src/other-file-with-a-long-name.ts"));
  await popup.waitFor();
  await popup.getByText("Working tree lines 4–10",{exact:true}).waitFor();
  assert.ok(await popup.evaluate(el=>el.scrollWidth<=el.clientWidth));
  await page.waitForFunction(()=>{
    const sheet=document.querySelector(".code-sheet--theirs");
    return sheet && getComputedStyle(sheet).zIndex==="2";
  });
  await page.screenshot({path:path.join(root,"src-tauri/target/conflict-preview-narrow.png")});
  await popup.waitFor({state:"hidden",timeout:4200});
  assert.equal(await page.evaluate(()=>previewReads.length),3);
  assert.ok(await page.evaluate(()=>performance.now()-previewReads[2].time>=2990));

  await page.evaluate(()=>showOperation());
  assert.equal(await page.evaluate(()=>operationReads.length),0);
  await page.evaluate(()=>setFiles());
  await page.setViewportSize({width:1100,height:800});
  await page.evaluate(()=>showOperation());
  const merge=page.getByRole("dialog",{name:"Merge completed"});
  await merge.waitFor();
  assert.equal(await merge.getByRole("listitem").count(),6);
  await merge.getByText("origin/feature/conflict-visualisation",{exact:true}).waitFor();
  await merge.getByText("More commits in the graph",{exact:true}).waitFor();
  await page.waitForFunction(()=>document.querySelector('.incoming-commit')?.getCTM().e>52);
  await page.screenshot({path:path.join(root,"src-tauri/target/operation-preview-desktop.png")});
  await page.waitForFunction(()=>getComputedStyle(document.querySelector('.result-point')).opacity==='1');
  await page.keyboard.press("Escape");
  await popup.waitFor({state:"hidden"});

  await page.setViewportSize({width:360,height:640});
  await page.emulateMedia({reducedMotion:"reduce"});
  await page.evaluate(()=>showOperation('cherry-pick'));
  const picked=page.getByRole("dialog",{name:"Cherry-pick completed"});
  await picked.waitFor();
  assert.equal(await picked.locator("animateMotion").count(),0);
  await picked.getByRole("heading",{name:"Picked commit"}).waitFor();
  assert.ok(await picked.evaluate(el=>el.scrollWidth<=el.clientWidth && el.scrollHeight<=el.clientHeight));
  await page.screenshot({path:path.join(root,"src-tauri/target/operation-preview-narrow.png")});
  await picked.getByRole("button",{name:"Close operation preview"}).click();
  await page.evaluate(()=>showOperation('revert'));
  await page.getByRole("dialog",{name:"Revert completed"}).getByRole("heading",{name:"Reverted commit"}).waitFor();
  await popup.waitFor({state:"hidden",timeout:8100});

  await page.emulateMedia({reducedMotion:"no-preference"});
  await page.evaluate(()=>showRefOperation('reset'));
  const reset=page.getByRole("dialog",{name:"Reset completed"});
  await reset.waitFor();
  await reset.getByText("Index reset, working tree unchanged").waitFor();
  await page.waitForFunction(()=>{
    const offset=document.querySelector('.rewind-head')?.getCTM().e;
    return offset>70 && offset<240;
  });
  await page.screenshot({path:path.join(root,"src-tauri/target/operation-reset-narrow.png")});
  await page.keyboard.press("Escape");

  await page.setViewportSize({width:1100,height:800});
  await page.evaluate(()=>showRefOperation('branch-deleted'));
  const deleted=page.getByRole("dialog",{name:"Local branch deleted"});
  await deleted.getByText("Branch reference removed").waitFor();
  await page.waitForFunction(()=>document.querySelector('.discarded-ref')?.getCTM().e>80);
  await page.screenshot({path:path.join(root,"src-tauri/target/operation-delete-desktop.png")});
  await page.waitForFunction(()=>getComputedStyle(document.querySelector('.discarded-ref')).opacity==='0');
  await deleted.getByRole("button",{name:"Close operation preview"}).click();

  await page.evaluate(()=>showRefOperation('branch-created'));
  await page.getByRole("dialog",{name:"Branch created"}).getByText("c0ffee0").waitFor();
  await page.keyboard.press("Escape");
  await page.evaluate(()=>showRefOperation('branch-renamed'));
  await page.getByRole("dialog",{name:"Branch renamed"}).getByText("feature/old-name").waitFor();
  await page.evaluate(()=>enable(false));
  await popup.waitFor({state:"hidden"});
  assert.equal(await page.evaluate(()=>operationReads.length),3);
  await page.evaluate(()=>showTransfer('push'));
  assert.equal(await popup.count(),0);
  await page.evaluate(()=>enable());

  await page.evaluate(()=>setMode('sidebar'));
  const branch=page.locator('button[aria-haspopup="menu"]').filter({hasText:'feature/topic'}).first();
  await branch.click({button:'right'});
  const menu=page.getByRole('menu',{name:'Branch feature/topic'});
  await menu.getByRole('menuitem',{name:'Set upstream'}).waitFor();
  await menu.getByRole('menuitem',{name:'Reset branch to remote'}).waitFor();
  await menu.getByRole('menuitem',{name:'Rename branch'}).click();
  assert.equal(await page.evaluate(()=>renameBranch),'feature/topic');
  await branch.focus();
  await page.keyboard.press('Shift+F10');
  await menu.getByRole('menuitem',{name:'Create branch here'}).click();
  assert.equal(await page.evaluate(()=>branchPoint),'refs/heads/feature/topic');
  await page.getByRole('button',{name:/REMOTE/}).click();
  await page.locator('button[aria-haspopup="menu"]').filter({hasText:'upstream/feature/topic'}).click({button:'right'});
  const remoteMenu=page.getByRole('menu',{name:'Branch upstream/feature/topic'});
  assert.equal(await remoteMenu.getByRole('menuitem',{name:'Rename branch'}).count(),0);
  await remoteMenu.getByRole('menuitem',{name:'Create branch here'}).click();
  assert.equal(await page.evaluate(()=>branchPoint),'refs/remotes/upstream/feature/topic');

  await page.evaluate(()=>showTransfer('push'));
  const pushed=page.getByRole('dialog',{name:'Push completed'});
  await pushed.getByText('GitLab',{exact:true}).waitFor();
  await page.waitForFunction(()=>document.querySelector('.transfer-packet')?.getCTM().e>260);
  await page.screenshot({path:path.join(root,'src-tauri/target/operation-push-desktop.png')});
  await branch.click({button:'right'});
  await menu.getByRole('menuitem',{name:'Delete local branch'}).click();
  assert.equal(await pushed.isVisible(),true);
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  await pushed.waitFor({state:'hidden',timeout:1000});
  await page.getByRole('button',{name:'Undo',exact:true}).waitFor({state:'hidden'});

  await page.evaluate(()=>scheduleDeletion(true));
  const pending=page.getByRole('dialog',{name:'Branch deletion pending'});
  await pending.getByText('Scheduled',{exact:true}).waitFor();
  await pending.getByRole('button',{name:'Undo',exact:true}).click();
  await pending.waitFor({state:'hidden',timeout:1000});
  await page.getByRole('button',{name:'Undo',exact:true}).waitFor({state:'hidden'});

  await page.evaluate(()=>{window.holdOperation=true;showOperation();scheduleDeletion();});
  await page.waitForFunction(()=>typeof window.releaseOperation==='function');
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  await page.evaluate(()=>{window.releaseOperation();window.releaseOperation=undefined;window.holdOperation=false;});
  await page.waitForTimeout(100);
  assert.equal(await popup.count(),0);

  await page.setViewportSize({width:360,height:640});
  await page.evaluate(()=>{setMode('');showTransfer('pull','github');});
  const pulled=page.getByRole('dialog',{name:'Pull completed'});
  await pulled.getByText('GitHub',{exact:true}).waitFor();
  assert.ok(await pulled.evaluate(el=>el.scrollWidth<=el.clientWidth));
  await page.waitForFunction(()=>document.querySelector('.transfer-packet--in')?.getCTM().e<150);
  await page.screenshot({path:path.join(root,'src-tauri/target/operation-pull-narrow.png')});
  await page.keyboard.press('Escape');
  await page.evaluate(()=>showTransfer('stash-restore','unknown'));
  await page.getByRole('dialog',{name:'Stash restored'}).getByText('Local archive').waitFor();
  await page.keyboard.press('Escape');

  await page.evaluate(()=>{window.holdOperation=true;showOperation();});
  await page.waitForFunction(()=>typeof window.releaseOperation==='function');
  await page.evaluate(()=>{switchRepo('third-repo');window.releaseOperation();});
  await page.waitForTimeout(100);
  assert.equal(await popup.count(),0);
  await page.evaluate(()=>{enable(false);showOperation();});
  assert.equal(await page.evaluate(()=>operationReads.length),5);
  await page.waitForTimeout(5100);
  assert.equal(await page.evaluate(()=>destructiveRuns),0);
  assert.deepEqual(errors,[]);
  console.log("PASS: branch menus, Undo cancelling pending Git and previews, transfer animations, responsive layout and stale previews.");
} finally {
  await browser?.close();
  await server.close();
}
