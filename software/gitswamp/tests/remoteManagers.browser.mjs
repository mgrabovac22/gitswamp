import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import { build } from "vite";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const root = fileURLToPath(new URL("..", import.meta.url));
const settingsSource = readFileSync(path.join(root, "src/features/shell/optionsSearch.ts"), "utf8");
const optionsSource = readFileSync(path.join(root, "src/view/shell/OptionsDialog.vue"), "utf8");
for (const [, id] of settingsSource.matchAll(/\["([a-z-]+)", "[A-Z]/g)) {
  assert.ok(optionsSource.includes(`data-option="${id}"`), `Missing setting anchor: ${id}`);
}
const entry = String.raw`
import { createApp, h, ref } from 'vue';
import Panel from '/src/view/repository/RemoteInsightsPanel.vue';
import Options from '/src/view/shell/OptionsDialog.vue';
import { githubBranchComparison, githubBranches, githubDiscussion } from '/src/features/repository/remote/githubRemote.ts';
import { workflowDefaults } from '/src/features/repository/workflow/branchWorkflow.ts';
import '/src/styles/index.css';
const context = { apiBase:'https://api.github.com', owner:'example', repo:'project', token:'test' };
const mode = ref('pull-request-create'), options = ref(false), catalog = ref(null), handoff = ref(null);
window.showMode = value => { mode.value = value; };
window.enableHandoff = () => { handoff.value = { source:'feature/login', target:'main', description:'Reviewed in Handoff.' }; };
window.showOptions = () => { options.value = true; };
window.sent = [];
window.requests = [];
const branchNames = ['main', 'develop', 'master', 'dev', 'feature/login', 'maintenance/already', ...Array.from({length:145}, (_, i) => 'feature/topic' + i)];
const comment = (id, body, extra = {}) => ({ id, user:{login:'contributor'}, body, created_at:'2026-01-02T12:00:00Z', html_url:'https://github.com/example/project/pull/12#issuecomment-'+id, ...extra });
window.fetch = async (url, init) => {
  if (init.signal?.aborted) throw new DOMException('Aborted', 'AbortError');
  const { pathname, searchParams } = new URL(url);
  const route = decodeURIComponent(pathname.slice('/repos/example/project'.length));
  window.requests.push({route,method:init.method||'GET',page:searchParams.get('page')});
  const ok = (data, status = 200, headers) => new Response(JSON.stringify(data), {status,headers:{'Content-Type':'application/json',...headers}});
  if (route === '') return ok({default_branch:'main'});
  const page = Number(searchParams.get('page'));
  if (route === '/branches') return ok(branchNames.slice((page-1)*100,page*100).map(name=>({name})),200,page===1?{link:'<next>; rel="next"'}:{});
  if (route.startsWith('/compare/')) return ok({ahead_by:route.includes('maintenance/already')?0:2,behind_by:1});
  if (route === '/pulls/12/comments') return ok([comment(90,'Fix this line',{path:'src/login.ts',line:4})]);
  if (/^\/issues\/(12|21)\/comments$/.test(route)) {
    if (init.method === 'POST') {
      const {body} = JSON.parse(init.body);
      window.sent.push({route,body});
      return window.rejectComment ? ok({},403) : ok(comment(91,body),201);
    }
    if (route.includes('/21/')) return ok([comment(40,'Issue thread')]);
    return page === 1
      ? ok([comment(1,'<img src=x onerror=alert(1)>'),...Array.from({length:29},(_,i)=>comment(i+2,'Comment '+(i+2)))],200,{link:'<next>; rel="next"'})
      : ok([comment(31,'Second page comment')]);
  }
  return ok({},404);
};
const controller = new AbortController();
void githubBranches(context,controller.signal).then(value => { catalog.value = value; });
const discussion = githubDiscussion(context,'repo-1',()=>{window.authRequested=true;});
const issue = { number:21,id:21,title:'Login is slow',state:'open',author:'tester',description:'Steps to reproduce',url:'https://github.com/example/project/issues/21',createdAt:'2026-01-01',updatedAt:'2026-01-01' };
const pullRequest = { ...issue,id:12,number:12,title:'Improve login',description:'Speed up login',url:'https://github.com/example/project/pull/12',sourceBranch:'feature/login',targetBranch:'develop',mergeable:true,mergeableState:'clean',linkedCommits:[{sha:'123456789',title:'Validate login',url:'https://github.com/example/project/commit/123456789'}],files:[{filename:'src/login.ts',url:'https://github.com/example/project/pull/12/files',status:'modified',additions:2,deletions:1}] };
createApp({render:()=>[
  h(Panel,{mode:mode.value,issue,pullRequest,workflow:workflowDefaults('gitflow','main','develop'),workflowDraft:handoff.value,currentBranch:'feature/login',remoteName:'origin',remoteBranches:[],branchCatalog:catalog.value,discussion,
    compareBranches:(base,head,signal)=>githubBranchComparison(context,base,head,signal),onCreatePullRequest:payload=>window.pr=payload}),
  options.value ? h(Options,{onClose:()=>{ options.value = false; }}) : null,
]}).mount('#app');`;

const mock = {
  "@tauri-apps/api/core": "export const invoke=async()=>null; export const isTauri=()=>false; export class Channel {}; export class Resource {}; export const convertFileSrc=x=>x;",
  "@tauri-apps/plugin-opener": "export const openUrl=async()=>{}; export const openPath=async()=>{}; export const revealItemInDir=async()=>{};",
  "@tauri-apps/plugin-dialog": "export const open=async()=>null; export const save=async()=>null; export const confirm=async()=>false;",
  "@tauri-apps/api/event": "export const listen=async()=>()=>{}; export const emit=async()=>{};",
};
const buildResult = await build({
  root, configFile: false, publicDir: false, logLevel: "error",
  plugins: [{name:"remote-manager-test",enforce:"pre",
    resolveId(id) { if (id === "virtual:remote-test") return "\0remote-entry"; if (Object.hasOwn(mock, id)) return "\0mock:" + id; },
    load(id) { if (id === "\0remote-entry") return entry; if (id.startsWith("\0mock:")) return mock[id.slice(6)]; },
  },vue(),tailwindcss()],
  resolve:{alias:{"@":path.join(root,"src")}},
  build:{write:false,minify:false,cssCodeSplit:false,rollupOptions:{input:"virtual:remote-test"}},
});
const assets = new Map(buildResult.output.map(asset=>["/"+asset.fileName,asset]));
const script = "/" + buildResult.output.find(asset=>asset.type==="chunk" && asset.isEntry).fileName;
const css = "/" + buildResult.output.find(asset=>asset.fileName.endsWith(".css")).fileName;
const server = createServer((req,res)=>{
  const route = new URL(req.url,"http://localhost").pathname;
  if (route === "/") {
    res.setHeader("Content-Type","text/html");
    res.end(`<!doctype html><html class="light"><head><link rel="stylesheet" href="${css}"><style>html,body,#app{height:100%;margin:0}</style></head><body><div id="app"></div><script type="module" src="${script}"></script></body></html>`);
    return;
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
  const page = await browser.newPage({viewport:{width:1050,height:780},reducedMotion:"reduce"});
  const errors = [];
  page.on("pageerror", error=>errors.push(error.message));
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  await page.getByRole("button",{name:"Base: develop",exact:true}).click();
  const base = page.getByRole("combobox",{name:"Search base branches"});
  assert.match(await page.getByRole("option").first().innerText(),/^develop/);
  const display = await page.getByRole("option").allTextContents();
  assert.ok(display.some(name=>name.includes("main")) && display.includes("dev") && display.includes("master"));
  await base.press("Escape");
  const title = page.getByRole("textbox",{name:"Title",exact:true});
  await title.fill("Coordinate login rollout");
  await page.getByRole("button",{name:"Compare: feature/login",exact:true}).click();
  const compare = page.getByRole("combobox",{name:"Search compare branches"});
  await compare.fill("topic137");
  await compare.press("Enter");
  await page.getByRole("button",{name:"Compare: feature/topic137",exact:true}).waitFor();
  assert.equal(await title.inputValue(),"Coordinate login rollout");
  assert.equal(await page.evaluate(()=>window.requests.filter(request=>request.route==='/branches').length),2);
  await page.getByRole("button",{name:"Compare: feature/topic137",exact:true}).click();
  await compare.fill("already");
  await compare.press("Enter");
  await page.getByText("The compare branch has no commits ahead of the base.").waitFor();
  assert.ok(await page.getByRole("button",{name:"Create pull request",exact:true}).isDisabled());
  await page.evaluate(()=>window.enableHandoff());
  await page.getByRole("button",{name:"Base: main",exact:true}).waitFor();
  assert.equal(await page.getByRole("textbox",{name:"Description"}).inputValue(),"Reviewed in Handoff.");
  await page.getByText("2 ahead · 1 behind",{exact:true}).waitFor();
  await page.screenshot({path:path.join(root,"src-tauri/target/remote-pr-creation.png")});
  await page.getByRole("button",{name:"Create pull request",exact:true}).click();
  assert.equal(await page.evaluate(()=>window.pr.targetBranch),"main");
  await page.setViewportSize({width:390,height:760});
  await page.getByRole("button",{name:"Base: main",exact:true}).click();
  assert.ok(await page.locator(".remote-scroll").evaluate(el=>el.scrollWidth<=el.clientWidth));
  await base.press("Escape");
  await page.setViewportSize({width:1050,height:780});

  await page.evaluate(()=>window.showMode('pull-request-detail'));
  await page.getByText("<img src=x onerror=alert(1)>",{exact:true}).waitFor();
  assert.equal(await page.locator("img[src=x]").count(),0);
  await page.getByRole("button",{name:"Load more comments",exact:true}).click();
  await page.getByText("Second page comment",{exact:true}).waitFor();
  const draft = page.getByRole("textbox",{name:"Add a comment",exact:true});
  await draft.fill("Ready to review");
  await page.getByRole("button",{name:"Review comments",exact:true}).click();
  await page.getByText("src/login.ts:4",{exact:true}).waitFor();
  await page.getByRole("button",{name:"Files changed",exact:true}).click();
  await page.getByText("src/login.ts",{exact:true}).waitFor();
  await page.getByRole("button",{name:"Conversation",exact:true}).click();
  assert.equal(await draft.inputValue(),"Ready to review");
  await page.evaluate(()=>{window.rejectComment=true;});
  await page.getByRole("button",{name:"Comment",exact:true}).click();
  await page.getByRole("alert").filter({hasText:/GitHub denied/}).waitFor();
  assert.equal(await draft.inputValue(),"Ready to review");
  await page.evaluate(()=>{window.rejectComment=false;});
  await page.getByRole("button",{name:"Comment",exact:true}).click();
  await page.getByText("Ready to review",{exact:true}).waitFor();
  assert.equal(await draft.inputValue(),"");
  await page.screenshot({path:path.join(root,"src-tauri/target/remote-pr-detail.png")});

  await page.setViewportSize({width:390,height:760});
  await page.evaluate(()=>window.showMode('issue-detail'));
  await page.getByText("Issue thread",{exact:true}).waitFor();
  assert.equal(await page.getByText("Ready to review",{exact:true}).count(),0);
  await draft.fill("I can reproduce this");
  await page.getByRole("button",{name:"Comment",exact:true}).click();
  await page.getByText("I can reproduce this",{exact:true}).waitFor();
  assert.equal(await page.evaluate(()=>window.sent.at(-1).route),"/issues/21/comments");
  assert.ok(await page.locator(".remote-scroll").evaluate(el=>el.scrollWidth<=el.clientWidth));
  await page.screenshot({path:path.join(root,"src-tauri/target/remote-issue-compact.png")});

  await page.setViewportSize({width:1000,height:750});
  await page.evaluate(()=>window.showOptions());
  const search = page.getByRole("combobox",{name:"Search settings"});
  await search.fill("secrt detectr");
  await page.getByRole("option",{name:/Accidental Secret Detector/}).waitFor();
  await search.press("Enter");
  await page.waitForFunction(()=>document.activeElement?.getAttribute('data-option')==='secrets');
  assert.equal(await page.locator('[data-option="secrets"]').isChecked(),false);
  await search.fill("gitlab ssh");
  await page.getByRole("option",{name:/GitLab SSH key/}).click();
  await page.waitForFunction(()=>document.activeElement?.getAttribute('data-option')==='gitlab-ssh');
  await search.fill("aut fet");
  await page.getByRole("option",{name:/Background auto-fetch/}).waitFor();
  await search.press("Escape");
  assert.equal(await search.getAttribute("aria-expanded"),"false");
  await search.fill("visualze operatons");
  await page.getByRole("option",{name:/Visualise operations/}).waitFor();
  await page.screenshot({path:path.join(root,"src-tauri/target/options-search.png")});
  await search.press("Enter");
  await page.waitForFunction(()=>document.activeElement?.getAttribute('data-option')==='visualise');
  assert.ok(await page.locator('[data-option="visualise"]').evaluate(el=>{
    const scroll = el.closest('.overflow-y-auto');
    const bounds = scroll.getBoundingClientRect();
    const option = el.getBoundingClientRect();
    return scroll.scrollTop > 0 && option.top >= bounds.top && option.bottom <= bounds.bottom;
  }));
  assert.deepEqual(errors,[]);
} finally {
  const closing = Promise.all([browser?.close(),new Promise(resolve=>server.close(resolve))]);
  server.closeAllConnections();
  await closing;
}
console.log("PASS: published branch pagination, workflow suggestions, compare, Handoff, PR and issue comments, and Options search.");
