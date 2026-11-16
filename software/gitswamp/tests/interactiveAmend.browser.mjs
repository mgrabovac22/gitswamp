import assert from "node:assert/strict";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const root = fileURLToPath(new URL("..", import.meta.url));
const entry = String.raw`
import { createApp, h, ref } from 'vue';
import Graph from '/src/view/commit/CommitGraph.vue';
import Amend from '/src/features/repository/InteractiveAmendDialog.vue';
import Patch from '/src/features/repository/HistoryPatchDialog.vue';
import Details from '/src/view/commit/CommitDetails.vue';
import '/src/styles/index.css';
const ids = ['3','2','1'].map(n=>n.repeat(40));
const commits = ids.map((sha,i)=>({sha,short_sha:sha.slice(0,7),message:['Latest commit','Review selected commit','Parent commit'][i],parent_shas:ids[i+1]?[ids[i+1]]:[],refs:i===0?['HEAD -> main']:[],author_name:'Test',author_email:'test@example.invalid',timestamp:1793380000-i}));
const requested = ref(null), patch = ref(null), headAmend = ref(false), showDetails = ref(false);
window.toggleHeadAmend = () => { showDetails.value=true; setTimeout(()=>{headAmend.value=true;},0); };
window.gitMock = async (command,args) => {
  if (command === 'get_interactive_amend_plan') { window.review = args; return {branch:'main',head:ids[0],target:commits[1],parent:commits[2],replay:1,dirty:!!window.dirty,published:true}; }
  if (command === 'amend_removal_choices') { window.removalLoaded=true; return [{path:'keep.txt',status:'modified'}, {path:'move.txt',status:'added'}]; }
  if (command === 'get_available_external_editors' || command === 'get_empty_directories') return [];
  if (command === 'get_staged_diff_summary') return null;
  return null;
};
createApp({render:()=>h('main',{style:'height:100%;display:flex;flex-direction:column'},[
  showDetails.value ? h(Details,{headCommit:commits[0],repoPath:'fixture',isWorkingChanges:true,commit:null,commitFiles:[],stagedFiles:[{path:'keep.txt',status:'modified',staged:true}],unstagedFiles:[],amendModeRequested:headAmend.value,onAmendCommit:payload=>window.normalAmend=payload}) : h(Graph,{repoPath:'fixture',commits,selected:null,hasWorkingChanges:true,currentBranch:'main',headSha:ids[0],onRequestInteractiveAmend:request=>{requested.value=request;},onRequestPatch:request=>{window.patchRequest=request; if (!request.sha) patch.value=request;}}),
  requested.value ? h(Amend,{repoPath:'fixture',...requested.value,busy:false,executionError:'',onClose:()=>requested.value=null,onStart:payload=>{window.confirmation=payload;requested.value=null;}}):null,
  patch.value ? h(Patch,{kind:patch.value.kind,file:patch.value.kind==='import'?'C:/temp/change.patch':undefined,branch:'main',busy:false,error:'',onClose:()=>patch.value=null,onImport:value=>{window.patchStage=value;patch.value=null;},onExport:value=>{window.patchScope=value;patch.value=null;}}):null,
])}).mount('#app');`;

const output = await build({
  root, configFile: false, publicDir: false, logLevel: "error",
  plugins: [{name:"amend-test",enforce:"pre",
    resolveId(id) { if (id === "virtual:amend") return "\0amend-entry"; if (id === "@tauri-apps/api/core") return "\0amend-core"; },
    load(id) { if (id === "\0amend-entry") return entry; if (id === "\0amend-core") return "export const invoke=(...args)=>window.gitMock(...args);"; },
  },vue(),tailwindcss()],
  resolve:{alias:{"@":path.join(root,"src")}},build:{write:false,minify:false,cssCodeSplit:false,rollupOptions:{input:"virtual:amend"}},
});
const assets = new Map(output.output.map(asset=>["/"+asset.fileName,asset]));
const script = "/" + output.output.find(asset=>asset.type==="chunk" && asset.isEntry).fileName;
const css = "/" + output.output.find(asset=>asset.fileName.endsWith(".css")).fileName;
const server = createServer((req,res)=>{
  const route = new URL(req.url,"http://localhost").pathname;
  if (route === "/") {
    res.setHeader("Content-Type","text/html");
    return res.end(`<!doctype html><html class="light"><head><link rel="stylesheet" href="${css}"><style>html,body,#app{height:100%;margin:0}</style></head><body><div id="app"></div><script type="module" src="${script}"></script></body></html>`);
  }
  const asset = assets.get(route);
  if (!asset) return res.writeHead(404).end();
  res.setHeader("Content-Type", asset.type === "chunk" ? "application/javascript" : "text/css");
  res.end(asset.type === "chunk" ? asset.code : asset.source);
});

let browser;
try {
  await new Promise(resolve=>server.listen(0,"127.0.0.1",resolve));
  browser = await chromium.launch({headless:true,...(process.env.CHROME_PATH ? {executablePath:process.env.CHROME_PATH} : {})});
  const page = await browser.newPage({viewport:{width:1050,height:780}});
  page.setDefaultTimeout(12000);
  const errors = [];
  page.on("pageerror",error=>errors.push(error.message));
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  const selected = page.locator(`#commit-row-${"2".repeat(40)}`);
  const amendMenu = page.getByRole("button",{name:"Interactive Amend",exact:true});
  await selected.click({button:"right",position:{x:450,y:14}});
  assert.equal(await amendMenu.getAttribute("aria-expanded"),"false");
  await amendMenu.click();
  for (const label of ["Edit or add files…","Reword…","Squash into parent…","Fixup into parent…","Drop commit…"]) {
    assert.ok(await page.getByRole("button",{name:label,exact:true}).isVisible());
  }
  await page.evaluate(()=>{window.dirty=true;});
  await page.getByRole("button",{name:"Reword…",exact:true}).click();
  const dialog = page.getByRole("dialog",{name:"Interactive Amend"});
  await dialog.getByText(/Commit or stash working changes/).waitFor();
  const confirm = dialog.getByRole("button",{name:"Confirm amend",exact:true});
  assert.ok(await confirm.isDisabled());
  await page.evaluate(()=>{window.dirty=false;});
  await dialog.getByRole("button",{name:"Review again",exact:true}).click();
  const message = dialog.getByRole("textbox",{name:"New commit message"});
  await message.fill("Clarify the selected commit");
  assert.ok(await confirm.isDisabled());
  await dialog.getByRole("checkbox",{name:/I understand/}).check();
  await page.setViewportSize({width:390,height:740});
  assert.ok(await dialog.evaluate(el=>el.clientWidth>=el.scrollWidth));
  await page.screenshot({path:path.join(root,"src-tauri/target/interactive-amend-compact.png")});
  await confirm.click();
  assert.equal(await page.evaluate(()=>window.review.sha),"2".repeat(40));
  assert.equal(await page.evaluate(()=>window.confirmation.message),"Clarify the selected commit");
  await page.setViewportSize({width:1050,height:780});
  await selected.click({button:"right",position:{x:450,y:14}});
  await page.getByRole("button",{name:"Save as patch…",exact:true}).click();
  assert.deepEqual(await page.evaluate(()=>window.patchRequest),{kind:"export",sha:"2".repeat(40)});

  await page.getByText(/^● Working(?: Changes|\.\.\.)$/).click({button:"right"});
  await page.getByRole("button",{name:"Import patch…",exact:true}).click();
  const patch = page.getByRole("dialog",{name:"Import patch"});
  await patch.getByRole("checkbox",{name:"Stage changes after applying"}).check();
  await patch.getByRole("button",{name:"Apply patch",exact:true}).click();
  assert.equal(await page.evaluate(()=>window.patchStage),true);

  await page.evaluate(()=>window.toggleHeadAmend());
  assert.equal(await page.evaluate(()=>window.removalLoaded),undefined);
  await page.getByText("Move files to Working Changes",{exact:true}).click();
  assert.ok(await page.getByRole("checkbox",{name:"Move keep.txt to Working Changes"}).isDisabled());
  await page.getByRole("checkbox",{name:"Move move.txt to Working Changes"}).check();
  await page.getByRole("button",{name:"Amend Last Commit",exact:true}).click();
  assert.deepEqual(await page.evaluate(()=>window.normalAmend.removal),{head:"3".repeat(40),paths:["move.txt"]});
  assert.deepEqual(errors,[]);
} finally {
  const closing = Promise.all([browser?.close(),new Promise(resolve=>server.close(resolve))]);
  server.closeAllConnections();
  await closing;
}
console.log("PASS: Interactive Amend submenu, review and confirmation, patch commands, and selective HEAD amend.");
