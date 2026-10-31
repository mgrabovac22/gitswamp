import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { compileScript, parse } from "@vue/compiler-sfc";
import { createRenderer, h, nextTick, reactive, ref } from "vue";
import ts from "typescript";

const require = createRequire(import.meta.url);
function evaluate(source, mocks = {}) {
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, esModuleInterop: true },
  });
  const module = { exports: {} };
  new Function("require", "module", "exports", outputText)(id => {
    if (Object.hasOwn(mocks, id)) return mocks[id];
    if (id.endsWith(".vue") || id.endsWith(".gif")) return {};
    return require(id);
  }, module, module.exports);
  return module.exports;
}
const read = path => readFileSync(new URL(`../src/${path}`, import.meta.url), "utf8");
const codeView = evaluate(read("shared/config/code-view/index.ts"));
const renderer = createRenderer({
  patchProp() {}, insert() {}, remove() {}, createElement: () => ({}),
  createText: () => ({}), createComment: () => ({}), setText() {}, setElementText() {},
  parentNode: () => null, nextSibling: () => null,
});
globalThis.localStorage = { getItem: () => null };
globalThis.document = { addEventListener() {}, removeEventListener() {} };
globalThis.window = { addEventListener() {}, removeEventListener() {} };
globalThis.requestAnimationFrame = callback => setTimeout(callback, 0);

async function tick() { await nextTick(); await new Promise(resolve => setImmediate(resolve)); await nextTick(); }
function mountViewer() {
  const calls = [];
  const source = compileScript(parse(read("shared/ui/FileDiffViewer.vue")).descriptor, { id: "diff-test" }).content;
  const component = evaluate(source, {
    "@tauri-apps/api/core": { invoke: (command, args) => new Promise((resolve, reject) => calls.push({ command, args, resolve, reject })) },
    "@/shared/codeView": codeView,
    "@/shared/config/code-view/useDiffSyntax": { useDiffSyntax: () => ({ get: () => undefined }) },
    "@/shared/config/diffViewCache": {
      getCachedDiffText: () => null, getCachedStructuredDiff: () => null,
      pruneDiffViewerCaches() {}, setCachedDiffText() {}, setCachedStructuredDiff() {},
    },
    "@/shared/notifications/useToast": { useToast: () => ({ info() {}, error() {} }) },
    "@/domain/git/UseGit": { useGit: () => ({ providerTokens: ref({}), githubToken: ref(null) }) },
  }).default;
  component.render = () => null;
  const props = reactive({ repoPath: "repo", filePath: "a.ts", staged: false, commitSha: null });
  const app = renderer.createApp({ render: () => h(component, props) });
  app.mount({});
  const state = app._instance.subTree.component.setupState;
  const take = (command, filePath) => {
    const index = calls.findIndex(call => call.command === command && (!filePath || call.args.filePath === filePath));
    assert.notEqual(index, -1, `Missing ${command} for ${filePath}`);
    return calls.splice(index, 1)[0];
  };
  return { props, state, take, stop: () => app.unmount() };
}
const diff = path => ({ path, hunks: [], is_binary: false });

test("save A then select B in edit mode loads B", async () => {
  const v = mountViewer();
  try {
    v.take("get_working_diff").resolve(diff("a.ts"));
    await tick();
    v.state.viewMode = "edit";
    await tick();
    v.take("get_file_content", "a.ts").resolve("const a = 1;\n");
    await tick();
    v.state.editContent = "const a = 2;\n";
    v.state.onEditInput();
    const saving = v.state.saveFile();
    const pendingSave = v.take("save_file_content", "a.ts");
    assert.equal(pendingSave.args.content, "const a = 2;\n");
    pendingSave.resolve();
    await saving;
    assert.equal(v.state.hasUnsavedChanges, false);
    v.props.filePath = "b.ts";
    await tick();
    v.take("get_file_content", "b.ts").resolve("const b = 3;\n");
    await tick();
    assert.equal(v.state.editContent, "const b = 3;\n");
    assert.equal(v.state.hasUnsavedChanges, false);
  } finally { v.stop(); }
});

test("file selection reloads the editor and old reads cannot overwrite the selected file", async () => {
  const v = mountViewer();
  try {
    v.take("get_working_diff").resolve(diff("a.ts"));
    await tick();
    v.state.viewMode = "edit";
    await tick();
    const oldRead = v.take("get_file_content", "a.ts");
    v.props.filePath = "b.ts";
    await tick();
    v.take("get_file_content", "b.ts").resolve("const b = 2;");
    await tick();
    oldRead.resolve("const a = 1;");
    await tick();
    assert.equal(v.state.editContent, "const b = 2;");
    assert.equal(v.state.fileContent, "const b = 2;");
    assert.equal(v.state.loading, false);
    v.props.filePath = "c.ts";
    v.props.staged = true;
    await tick();
    v.take("get_file_content", "c.ts").resolve("const c = 3;");
    await tick();
    assert.equal(v.state.viewMode, "edit");
    assert.equal(v.state.editContent, "const c = 3;");
  } finally { v.stop(); }
});

test("late save updates neither the next file's content nor its dirty state; drafts survive switching", async () => {
  const v = mountViewer();
  try {
    v.take("get_working_diff").resolve(diff("a.ts"));
    await tick();
    v.state.viewMode = "edit";
    await tick();
    v.take("get_file_content").resolve("const a = 1;");
    await tick();
    v.state.editContent = "const a = 2;";
    v.state.onEditInput();
    const save = v.state.saveFile();
    const pendingSave = v.take("save_file_content");
    v.props.filePath = "b.ts";
    await tick();
    v.take("get_file_content", "b.ts").resolve("const b = 1;");
    await tick();
    v.state.editContent = "const b = 2;";
    v.state.onEditInput();
    pendingSave.resolve();
    await save;
    assert.equal(v.state.editContent, "const b = 2;");
    assert.equal(v.state.fileContent, "const b = 1;");
    assert.equal(v.state.hasUnsavedChanges, true);
    v.props.filePath = "a.ts";
    await tick();
    v.take("get_file_content", "a.ts").resolve("const a = 2;");
    await tick();
    assert.equal(v.state.hasUnsavedChanges, false);
    v.props.filePath = "b.ts";
    await tick();
    v.take("get_file_content", "b.ts").resolve("const b = 1;");
    await tick();
    assert.equal(v.state.editContent, "const b = 2;");
    assert.equal(v.state.hasUnsavedChanges, true);
    v.state.editContent = "const b = 3;";
    v.state.onEditInput();
    const saveB = v.state.saveFile();
    v.take("save_file_content", "b.ts").resolve();
    await saveB;
    v.props.filePath = "a.ts";
    await tick();
    v.take("get_file_content", "a.ts").resolve("const a = 2;");
    await tick();
    v.props.filePath = "b.ts";
    await tick();
    v.take("get_file_content", "b.ts").resolve("const b = 3;");
    await tick();
    assert.equal(v.state.editContent, "const b = 3;");
    assert.equal(v.state.hasUnsavedChanges, false);
  } finally { v.stop(); }
});

test("out-of-order diff requests cannot replace the current file", async () => {
  const v = mountViewer();
  try {
    const old = v.take("get_working_diff", "a.ts");
    v.props.filePath = "b.ts";
    await tick();
    v.take("get_working_diff", "b.ts").resolve(diff("b.ts"));
    await tick();
    old.resolve(diff("a.ts"));
    await tick();
    assert.equal(v.state.diff.path, "b.ts");
    assert.equal(v.state.error, null);
  } finally { v.stop(); }
});

test("preview highlighting colors known languages and escapes markup", () => {
  const html = codeView.highlightFilePreview('const title = "<img onerror=alert(1)>";', "file.ts");
  assert.match(html, /hljs-keyword/);
  assert.match(html, /&lt;img/);
  assert.doesNotMatch(html, /<img/);
});

test("large, minified, binary and unknown files use plain text without auto detection", () => {
  for (const [content, path] of [
    ["x".repeat(60_001), "file.ts"], ["x\n".repeat(1501), "file.ts"],
    ["x".repeat(2001), "file.js"], ["a\0b", "file.ts"], ["hello", "README.unknown"],
  ]) assert.equal(codeView.highlightFilePreview(content, path), null);
});
