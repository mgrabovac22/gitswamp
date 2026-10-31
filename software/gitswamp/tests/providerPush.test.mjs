import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { reactive } from "vue";
import { compileScript, parse } from "@vue/compiler-sfc";
import ts from "typescript";

const require = createRequire(import.meta.url);
const read = path => readFileSync(new URL(`../src/${path}`, import.meta.url), "utf8");
function evaluate(source, mocks = {}) {
  const module = { exports: {} };
  const code = ts.transpileModule(source, {compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
  new Function("require", "module", "exports", code)(id => Object.hasOwn(mocks,id) ? mocks[id] : require(id), module, module.exports);
  return module.exports;
}
const helpers = evaluate(read("domain/git/composables/gitHelpers.ts"));
const { createGitState } = evaluate(read("domain/git/composables/gitState.ts"));

function pushHarness(intercept = async () => "done") {
  const state = createGitState();
  state.repoPath.value = "first-repo";
  state.repoInfo.value = {name:"repo",path:"first-repo",remotes:[],current_branch:"main",head_sha:"abc",is_clean:true};
  state.providerTokens.value = {github:"synthetic-github",gitlab:"synthetic-gitlab",azure:"synthetic-azure",bitbucket:"synthetic-bitbucket"};
  const calls=[];
  const { createRemoteActions } = evaluate(read("domain/git/composables/gitRemoteActions.ts"),{
    "./gitHelpers":helpers,
    "./gitCall":{callTauri:async(command,args)=>{calls.push({command,args});return intercept(command,args);}},
    "@/shared/notifications/useToast":{useToast:()=>({})},
  });
  const refresh = Object.fromEntries(["refreshBranches","refreshCommits","refreshStatus","refreshTags","refreshRepoInfo","refreshStashes"].map(name=>[name,async()=>{}]));
  const toast = {loading:()=>1,remove(){},success(){},error(){}};
  return {state,calls,actions:createRemoteActions(state,refresh,toast)};
}

for (const provider of ["github","gitlab","azure","bitbucket"]) {
  test(`first ${provider} push fetches only origin with its selected token`,async()=>{
    const h=pushHarness();
    await h.actions.pushToMultiplePlatforms(provider,"owner/repo");
    assert.deepEqual(h.calls.map(call=>call.command),["push_to_platform","fetch_all"]);
    assert.equal(h.calls[0].args.providerToken,`synthetic-${provider}`);
    assert.deepEqual(h.calls[1].args,{path:"first-repo",token:`synthetic-${provider}`,remoteName:"origin"});
    assert.equal(h.state.repoInfo.value.remotes.length,0,"no stale origin lookup is needed");
  });
}

test("regular push keeps the existing all-remotes fetch behavior",async()=>{
  const h=pushHarness();
  h.state.repoInfo.value.remotes=[{name:"origin",url:"https://github.com/owner/repo.git"}];
  await h.actions.push();
  assert.deepEqual(h.calls.map(call=>call.command),["push","fetch_all"]);
  assert.equal(h.calls[1].args.token,"synthetic-github");
  assert.equal(h.calls[1].args.remoteName,undefined);
});

test("changing repositories during provider push never fetches the new repo with the old token",async()=>{
  let finish;
  const h=pushHarness(command=>command==="push_to_platform"?new Promise(resolve=>{finish=resolve;}):"done");
  const pending=h.actions.pushToMultiplePlatforms("gitlab","owner/repo");
  h.state.repoPath.value="second-repo";
  finish("pushed");await pending;
  assert.equal(h.calls.length,1);
});

test("failed provider push does not start a follow-up fetch",async()=>{
  const h=pushHarness(()=>{throw Error("Push rejected");});
  await h.actions.pushToMultiplePlatforms("gitlab","owner/repo");
  assert.equal(h.calls.length,1);assert.match(h.state.error.value,/Push rejected/);
});

test("a failed push from the previous repo does not set an error on the new repo",async()=>{
  let fail;
  const h=pushHarness(()=>new Promise((_resolve,reject)=>{fail=reject;}));
  const pending=h.actions.pushToMultiplePlatforms("azure","org/project/repo@dev.azure.com");
  h.state.repoPath.value="second-repo";
  fail(Error("Authentication required"));await pending;
  assert.equal(h.state.error.value,null);assert.equal(h.calls.length,1);
});

test("Azure Enter and button handler share the required-field guard",()=>{
  const source=compileScript(parse(read("view/repository/RepositoryAuthDialogs.vue")).descriptor,{id:"auth-test"}).content;
  const component=evaluate(source).default;
  const props=reactive({pushPlatform:"azure",pushUsername:"org",pushProject:"",pushDomain:"dev.azure.com"});
  const emitted=[];
  const setup=component.setup(props,{expose(){},emit:event=>emitted.push(event)});
  assert.equal(setup.canPush.value,false);setup.submitPush();assert.deepEqual(emitted,[]);
  props.pushProject="project";
  assert.equal(setup.canPush.value,true);setup.submitPush();assert.deepEqual(emitted,["push"]);
  for(const field of ["pushUsername","pushProject","pushDomain"]){
    const value=props[field];props[field]="   ";setup.submitPush();assert.equal(emitted.length,1);props[field]=value;
  }
  props.pushPlatform="github";props.pushProject="";props.pushDomain="";
  assert.equal(setup.canPush.value,true);
});
