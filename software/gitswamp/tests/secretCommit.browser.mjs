import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createServer } from 'vite';
import vue from '@vitejs/plugin-vue';
import tailwindcss from '@tailwindcss/vite';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=fileURLToPath(new URL('..',import.meta.url));
const html=`<!doctype html><html><head><style>html,body,#app{margin:0;height:100%}</style></head><body><div id="app"></div><script type="module">
import {createApp,h,ref} from '/node_modules/.vite/deps/vue.js';
import Details from '/src/view/commit/CommitDetails.vue';
import Dialog from '/src/shared/ui/SecretCommitDialog.vue';
import {checkCommitSecrets} from '/src/domain/git/composables/secretCommitGuard.ts';
import '/src/styles/index.css';
localStorage.setItem('gitswamp-commit-analyzer-settings',JSON.stringify({enabled:false}));
localStorage.setItem('gitswamp-show-empty-dirs','false');
const repo=ref('repo'),busy=ref(false);
const findings=[{path:'src/config.ts',line:12,kind:'API token',tracked:true},{path:'.env',line:2,kind:'Password',tracked:false}];
window.calls=[];window.committed=0;window.clean=false;window.large=false;
window.testInvoke=async(command,args)=>{
 calls.push({command,args});
 if(command==='scan_secrets')return {findings:window.clean||args.includeSecrets===false?[]:findings,skipped:[],snapshot:'checked-tree',large_files:window.large?[{path:'assets/recording.mp4',size:120*1024*1024}]:[]};
 if(command==='ignore_secret_files')return;
 if(command==='get_staged_diff_summary')return {total_lines_added:2,total_lines_removed:0,files_changed:1,file_types:['ts'],has_test_changes:false,has_migration_changes:false,inferred_scope:'src'};
 if(command==='get_available_external_editors'||command==='get_empty_directories'||command==='get_commit_tree_paths')return [];
 throw Error(command);
};
window.switchRepo=()=>repo.value='another-repo';
createApp({render:()=>h('main',{style:'height:100%;max-width:680px;margin:auto;'},[
 h(Details,{repoPath:repo.value,commit:null,commitFiles:[],stagedFiles:[{path:'src/config.ts',status:'modified',staged:true,conflicted:false}],unstagedFiles:[{path:'.env',status:'untracked',staged:false,conflicted:false}],isWorkingChanges:true,operationBusy:busy.value,onCommit:async payload=>{
  busy.value=true;try{const approval=await checkCommitSecrets(repo.value,()=>true,async()=>{});if(approval)window.committed++;payload.done(Boolean(approval));}finally{busy.value=false}
 }}),h(Dialog,{repoPath:repo.value})
])}).mount('#app');
</script></body></html>`;
const server=await createServer({root,configFile:false,plugins:[{
 name:'secret-test-mocks',enforce:'pre',resolveId(id){if(id==='@tauri-apps/api/core')return '\0tauri-mock';},
 load(id){if(id==='\0tauri-mock')return 'export const invoke=(...args)=>window.testInvoke(...args);';},
 configureServer(server){server.middlewares.use((req,res,next)=>{if(req.url!=='/__secret_test__')return next();void server.transformIndexHtml(req.url,html).then(result=>{res.setHeader('Content-Type','text/html');res.end(result);});});},
},vue(),tailwindcss()],resolve:{alias:{'@':path.join(root,'src')}},optimizeDeps:{entries:[],noDiscovery:true,include:['vue','lucide-vue-next','highlight.js/lib/common']},server:{port:0,host:'127.0.0.1'}});
let browser;
try{
 await server.listen();browser=await chromium.launch({headless:true,...(process.env.CHROME_PATH?{executablePath:process.env.CHROME_PATH}:{})});
 const page=await browser.newPage({viewport:{width:1200,height:900}}), errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 await page.goto('http://127.0.0.1:'+server.httpServer.address().port+'/__secret_test__');
 const input=page.getByPlaceholder('Commit message...', {exact:true});await input.fill('Protect configuration');
 const submit=page.getByRole('button',{name:/^Commit changes/i});
 await submit.click();const dialog=page.getByRole('alertdialog');await dialog.waitFor();
 assert.equal(await page.evaluate(()=>committed),0);
 assert.match(await dialog.innerText(),/src\/config.ts:12/);
 await page.screenshot({path:path.join(root,'src-tauri/target/secret-confirm-desktop.png'),animations:'disabled'});
 await dialog.getByRole('button',{name:'Cancel',exact:true}).click();
 assert.equal(await input.inputValue(),'Protect configuration');
 await submit.click();await dialog.waitFor();await dialog.getByRole('button',{name:'Add to .gitignore',exact:true}).click();
 await page.waitForFunction(()=>calls.some(c=>c.command==='ignore_secret_files'));
 assert.equal(await page.evaluate(()=>committed),0);assert.equal(await input.inputValue(),'Protect configuration');
 assert.deepEqual(await page.evaluate(()=>calls.find(c=>c.command==='ignore_secret_files').args.paths),['.env']);
 await page.setViewportSize({width:420,height:760});await submit.click();await dialog.waitFor();
 assert.ok(await dialog.evaluate(el=>el.scrollWidth<=el.clientWidth));
 await page.screenshot({path:path.join(root,'src-tauri/target/secret-confirm-narrow.png'),animations:'disabled'});
 await dialog.getByRole('button',{name:'Commit anyway',exact:true}).click();
 await page.waitForFunction(()=>committed===1);assert.equal(await input.inputValue(),'');
 await page.evaluate(()=>window.clean=true);await input.fill('No credentials');await submit.click();
 await page.waitForFunction(()=>committed===2);assert.equal(await page.getByRole('alertdialog').count(),0);
 await page.evaluate(()=>{window.large=true;localStorage.setItem('gitswamp-secret-detector-enabled','false')});
 await input.fill('Add recording');await submit.click();await dialog.waitFor();
 assert.match(await dialog.innerText(),/Git LFS/);assert.match(await dialog.innerText(),/120.0 MiB/);
 assert.equal(await page.evaluate(()=>committed),2);
 await page.screenshot({path:path.join(root,'src-tauri/target/large-file-confirm.png'),animations:'disabled'});
 await dialog.getByRole('button',{name:'Cancel',exact:true}).click();assert.equal(await input.inputValue(),'Add recording');
 assert.deepEqual(errors,[]);console.log('PASS: staged warning, cancel preserves draft, ignore cancels and selects only new files, explicit proceed, silent clean commit, desktop/narrow layout');
}finally{await browser?.close();await server.close();}
