import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import ts from 'typescript';

const require = createRequire(import.meta.url);
const read = path => readFileSync(new URL(`../src/${path}`, import.meta.url), 'utf8');
function evaluate(source, mocks = {}) {
  const module = { exports: {} };
  const code = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  new Function('require', 'module', 'exports', code)(id => Object.hasOwn(mocks,id) ? mocks[id] : require(id), module, module.exports);
  return module.exports;
}
const makeReport = () => ({ findings: [{path:'config.ts',line:2,kind:'API token',tracked:true},{path:'.env',line:3,kind:'Password',tracked:false}], skipped:[], snapshot:'repo:head:index' });
function harness({report=makeReport(), choice='cancel', enabled=true, current=()=>true, fail=false} = {}) {
  const calls = [], prompts = [], refreshes = [];
  const { checkCommitSecrets } = evaluate(read('domain/git/composables/secretCommitGuard.ts'), {
    './gitCall':{callTauri:async(command,args)=>{calls.push({command,args});if(fail)throw Error('Cannot scan');return report;}},
    '@/shared/config/secretDetectorPreferences':{secretDetectorEnabled:()=>enabled},
    '@/shared/notifications/secretConfirmation':{requestSecretConfirmation:async(path,report)=>{prompts.push({path,report});return choice;}},
  });
  return {calls,prompts,refreshes,run:()=>checkCommitSecrets('repo',current,async()=>refreshes.push(true))};
}
test('secret detector is enabled by default, including unavailable storage',()=>{
  const p=evaluate(read('shared/config/secretDetectorPreferences.ts'));
  globalThis.localStorage={getItem:()=>null};assert.equal(p.secretDetectorEnabled(),true);
  globalThis.localStorage={getItem:()=>{throw Error('storage');}};assert.equal(p.secretDetectorEnabled(),true);
  globalThis.localStorage={getItem:()=> 'false'};assert.equal(p.secretDetectorEnabled(),false);
});
test('clean staged scan is silent and carries exact snapshot to commit',async()=>{
  const h=harness({report:{findings:[],skipped:[],snapshot:'checked-tree'}});
  assert.deepEqual(await h.run(),{secretCheck:true,secretApproval:'checked-tree'});
  assert.equal(h.prompts.length,0);assert.deepEqual(h.calls[0],{command:'scan_secrets',args:{path:'repo',staged:true,includeSecrets:true}});
});
test('cancel never authorizes or mutates staged files',async()=>{
  const h=harness();assert.equal(await h.run(),null);assert.equal(h.calls.length,1);
});
test('explicit confirmation approves only the reviewed snapshot',async()=>{
  const h=harness({choice:'commit'});assert.deepEqual(await h.run(),{secretCheck:true,secretApproval:'repo:head:index'});
});
test('ignore only passes new flagged files and cancels commit',async()=>{
  const h=harness({choice:'ignore'});assert.equal(await h.run(),null);
  assert.deepEqual(h.calls[1],{command:'ignore_secret_files',args:{path:'repo',paths:['.env'],snapshot:'repo:head:index'}});
  assert.equal(h.refreshes.length,1);
});
test('incomplete scans also require confirmation; scan errors fail closed',async()=>{
  const h=harness({report:{findings:[],skipped:['huge.json'],snapshot:'a'}});assert.equal(await h.run(),null);assert.equal(h.prompts.length,1);
  await assert.rejects(harness({fail:true}).run(),/Cannot scan/);
});
test('repository changes invalidate preflight and confirmation',async()=>{
  const h=harness({current:()=>false});assert.equal(await h.run(),null);assert.equal(h.prompts.length,0);
  let count=0;const switched=harness({choice:'ignore',current:()=>++count===1});assert.equal(await switched.run(),null);assert.equal(switched.calls.length,1);
});
test('disabled secret detector still checks staged sizes without scanning content',async()=>{
  const h=harness({enabled:false,report:{findings:[],skipped:[],snapshot:'tree',large_files:[]}});
  assert.deepEqual(await h.run(),{secretCheck:false,secretApproval:'tree'});assert.equal(h.calls[0].args.includeSecrets,false);
});
test('large staged files require confirmation even with secret detection disabled',async()=>{
  const h=harness({enabled:false,report:{findings:[],skipped:[],snapshot:'tree',large_files:[{path:'video.bin',size:104857601}]}});
  assert.equal(await h.run(),null);assert.equal(h.prompts.length,1);
});
test('closing or replacing secret confirmation resolves the pending request as cancel',async()=>{
  const c=evaluate(read('shared/notifications/secretConfirmation.ts'));
  const first=c.requestSecretConfirmation('repo',makeReport());
  const second=c.requestSecretConfirmation('other',makeReport());
  assert.equal(await first,'cancel');c.finishSecretConfirmation();assert.equal(await second,'cancel');assert.equal(c.secretConfirmation.value,null);
});

test('normal and amend commits both wait for guard and duplicate submissions cannot bypass it',async()=>{
  const {createGitState}=evaluate(read('domain/git/composables/gitState.ts'));
  for (const method of ['commitChanges','amendLastCommit']) {
    const state=createGitState();state.repoPath.value='repo';
    const calls=[];let finish;
    const {createStatusActions}=evaluate(read('domain/git/composables/gitStatusActions.ts'),{
      './gitCall':{callTauri:async(command,args)=>{calls.push({command,args});return {}; }},
      './secretCommitGuard':{checkCommitSecrets:()=>new Promise(resolve=>finish=resolve)},
      './gitStatusOptimisticUpdates':{},
      '@/shared/notifications/gitOperationPreview':{publishOperationPreview(){}},
    });
    const refresh=Object.fromEntries(['refreshCommits','refreshStatus','refreshBranches','refreshStashes','refreshRepoInfo'].map(name=>[name,async()=>{}]));
    const toast={loading:()=>1,remove(){},success(){},error(){}};
    const actions=createStatusActions(state,refresh,toast);
    const value=method==='commitChanges'?'Draft message':{message:'Amend',resetAuthor:false,signoff:false};
    const pending=actions[method](value);
    assert.equal(state.loading.value,true);assert.equal(await actions[method](value),false);assert.equal(calls.length,0);
    finish(null);assert.equal(await pending,false);assert.equal(calls.length,0);assert.equal(state.loading.value,false);
    const allowed=actions[method](value);finish({secretCheck:true,secretApproval:'tree'});assert.equal(await allowed,true);
    assert.equal(calls[0].args.secretApproval,'tree');assert.equal(state.loading.value,false);
  }
});
