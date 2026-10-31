import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { effectScope, nextTick, ref, shallowRef } from "vue";
import { parse } from "@vue/compiler-sfc";
import ts from "typescript";

const require = createRequire(import.meta.url);
const read = path => readFileSync(new URL(`../src/${path}`, import.meta.url), "utf8");
function load(source, values = {}) {
  const module = { exports: {} };
  const code = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  new Function("module", "exports", "require", ...Object.keys(values), code)(module, module.exports, require, ...Object.values(values));
  return module.exports;
}
const { createSyntaxQueue } = load(read("shared/config/code-view/syntaxQueue.ts"));
const { useWindowedRows } = load(read("shared/ui/useWindowedRows.ts"));
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));

test("syntax is deduplicated, batched off the render path and cached", async () => {
  const sent = [];
  let updates = 0;
  const worker = { postMessage: message => sent.push(message), terminate() {} };
  const queue = createSyntaxQueue(() => worker, () => updates++);
  try {
    queue.reset("code.ts");
    for (let i=0; i<1000; i++) queue.get(`const x${i} = 1;`);
    queue.get("const x0 = 1;");
    assert.equal(sent.length, 0, "render only enqueues, it does not tokenize");
    await wait(5);
    assert.equal(sent.length, 1);
    assert.equal(sent[0].lines.length, 64);
    await wait(5);
    assert.equal(sent.length, 1, "at most one batch in flight");
    worker.onmessage({ data: { generation: sent[0].generation, results: [["const x0 = 1;", "<span>const</span> x0 = 1;"]] } });
    await wait(5);
    assert.equal(sent.length, 2);
    assert.match(queue.get("const x0 = 1;"), /span/);
    await wait(40);
    assert.equal(updates, 1, "batch updates are coalesced");
  } finally { queue.dispose(); }
});

test("changing files cancels obsolete syntax work and ignores late replies", async () => {
  const workers = [];
  const queue = createSyntaxQueue(() => {
    const worker = { postMessage(message) { this.message = message; }, terminate() { this.stopped = true; } };
    workers.push(worker); return worker;
  }, () => {});
  try {
    queue.reset("a.ts"); queue.get("value"); await wait(5);
    const old = workers[0];
    queue.reset("b.py");
    assert.equal(old.stopped, true);
    old.onmessage({ data: { generation: old.message.generation, results: [["value", "wrong-language"]] } });
    assert.equal(queue.get("value"), undefined);
    await wait(5);
    assert.equal(workers[1].message.path, "b.py");
  } finally { queue.dispose(); }
});

test("missing workers fall back to plain text without blocking or throwing", async () => {
  const queue = createSyntaxQueue(() => { throw Error("Worker unavailable"); }, () => {});
  try { queue.get("line"); await wait(5); assert.equal(queue.get("line"), undefined); }
  finally { queue.dispose(); }
});

test("large file lists render a bounded window; small lists keep all rows", async () => {
  const scope = effectScope();
  const items = ref(Array.from({length:2048},(_,index)=>index));
  let onScroll;
  const root = { scrollTop: 0, clientHeight: 500, addEventListener(_, callback) { onScroll=callback; }, removeEventListener() {} };
  const container = shallowRef(null);
  const previous = globalThis.ResizeObserver;
  globalThis.ResizeObserver = class { observe() {} disconnect() {} };
  try {
    const rows = scope.run(() => useWindowedRows(() => items.value, container, () => 46));
    container.value=root; await nextTick();
    assert.ok(rows.value.rows.length < 40);
    assert.equal(rows.value.before+rows.value.after+rows.value.rows.length*46,2048*46);
    root.scrollTop=2048*46-500; onScroll();
    assert.equal(rows.value.rows.at(-1),2047);
    items.value=[1,2,3];
    assert.deepEqual([...rows.value.rows],[1,2,3]);
    assert.equal(rows.value.active,false);
  } finally { scope.stop(); globalThis.ResizeObserver=previous; }
});

function loaders() {
  const script = parse(read("view/AppView.vue")).descriptor.scriptSetup.content;
  const ast = ts.createSourceFile("AppView.ts", script, ts.ScriptTarget.Latest, true);
  const names = new Set(["multiCommitFilesRunToken", "commitFilesLoading", "commitFilesError", "COMMIT_FILES_CACHE_LIMIT",
    "COMMIT_FILES_CACHE_MAX_FILES", "commitFilesCache", "commitFilesRequests", "getCommitFilesCacheKey", "getCachedCommitFiles",
    "cacheCommitFiles", "getCommitFiles", "mergeCommitFileStatus", "loadSingleSelectedCommitFiles", "loadAggregatedSelectedCommitFiles"]);
  const picked = ast.statements.filter(node => ts.isFunctionDeclaration(node) ? names.has(node.name?.text)
    : ts.isVariableStatement(node) && node.declarationList.declarations.every(decl => names.has(decl.name.getText(ast))));
  const git={repoPath:ref("repo"),selectedCommits:ref([]),selectedCommitFiles:shallowRef([])};
  const calls=[];
  const api=load(picked.map(node=>node.getText(ast)).join("\n")+`\nmodule.exports={
    single:loadSingleSelectedCommitFiles,multi:loadAggregatedSelectedCommitFiles,loading:commitFilesLoading,error:commitFilesError,
    select(commits){multiCommitFilesRunToken++;git.selectedCommits.value=commits;}
  };`, {ref,git,invoke:(_command,args)=>new Promise((resolve,reject)=>calls.push({args,resolve,reject}))});
  return {git,calls,...api};
}
const commit=sha=>({sha,timestamp:1});
const files=Array.from({length:2048},(_,i)=>({path:`file-${i}.ts`,status:"added",additions:1,deletions:0}));

test("2048-file requests are deduplicated and cached; repeat selections finish loading", async () => {
  const api=loaders(); const c=commit("a");
  api.select([c]); const a=api.single(c);
  api.select([c]); const b=api.single(c);
  assert.equal(api.calls.length,1);
  assert.equal(api.loading.value,true);
  api.calls[0].resolve(files); await Promise.all([a,b]);
  assert.equal(api.loading.value,false);
  assert.equal(api.git.selectedCommitFiles.value.length,2048);
  await api.single(c);
  assert.equal(api.calls.length,1,"large commits use the cache too");
});

test("late commit responses do not replace the new selection or its loading state", async () => {
  const api=loaders(),a=commit("a"),b=commit("b");
  api.select([a]); const first=api.single(a);
  api.select([b]); const second=api.single(b);
  api.calls[0].resolve(files); await first;
  assert.equal(api.loading.value,true);
  assert.equal(api.git.selectedCommitFiles.value.length,0);
  api.calls[1].resolve([]); await second;
  assert.equal(api.loading.value,false);
});

test("failed loads clear their spinner and expose an error", async () => {
  const api=loaders(),c=commit("a"); api.select([c]);
  const result=api.single(c); api.calls[0].reject(Error("failed")); await result;
  assert.equal(api.loading.value,false); assert.match(api.error.value,/Could not load/);
});

test("multi-selection limits native requests to four concurrent jobs", async () => {
  const api=loaders(),commits=Array.from({length:8},(_,i)=>commit(String(i)));
  api.select(commits); const result=api.multi(commits);
  assert.equal(api.calls.length,4);
  for(let i=0;i<4;i++) api.calls[i].resolve([]);
  await wait(0); assert.equal(api.calls.length,8);
  for(let i=4;i<8;i++) api.calls[i].resolve([]);
  await result; assert.equal(api.loading.value,false);
});
