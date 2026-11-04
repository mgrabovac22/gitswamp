import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { compileScript, parse } from "@vue/compiler-sfc";
import { createRenderer, h, nextTick, reactive } from "vue";
import ts from "typescript";

const require = createRequire(import.meta.url);
const read = name => readFileSync(new URL(`../src/${name}`, import.meta.url), "utf8");
function evaluate(source, mocks = {}) {
  const module = { exports: {} };
  const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } });
  new Function("require", "module", "exports", outputText)(id => mocks[id] ?? require(id), module, module.exports);
  return module.exports;
}
const images = evaluate(read("shared/config/imagePreview.ts"));
const preview = evaluate(read("shared/config/binaryPreview.ts"), { "./imagePreview": images });
function png(width = 32, height = 24) {
  const bytes = new Uint8Array(45);
  bytes.set([137, 80, 78, 71, 13, 10, 26, 10]);
  const view = new DataView(bytes.buffer);
  view.setUint32(8, 13); bytes.set([73, 72, 68, 82], 12);
  view.setUint32(16, width); view.setUint32(20, height);
  return bytes;
}
const side = (id = "blob", size = 45, exists = true) => ({ id, size, exists });
const both = () => ({ old: side("before"), new: side("after") });
const tick = async () => { await nextTick(); await new Promise(resolve => setImmediate(resolve)); await nextTick(); };
const renderer = createRenderer({
  patchProp() {}, insert() {}, remove() {}, createElement: () => ({}), createText: () => ({}),
  createComment: () => ({}), setText() {}, setElementText() {}, parentNode: () => null, nextSibling: () => null,
});
function mount() {
  const calls = [], live = new Set(), revoked = [];
  const create = URL.createObjectURL, revoke = URL.revokeObjectURL;
  let next = 0;
  URL.createObjectURL = () => { const url = `blob:test-${++next}`; live.add(url); return url; };
  URL.revokeObjectURL = url => { live.delete(url); revoked.push(url); };
  const source = compileScript(parse(read("shared/ui/BinaryDiffViewer.vue")).descriptor, { id: "binary-test" }).content;
  const component = evaluate(source, {
    "@tauri-apps/api/core": { invoke: (command, args) => new Promise((resolve, reject) => calls.push({ command, args, resolve, reject })) },
    "@/shared/config/imagePreview": images,
    "@/shared/config/binaryPreview": preview,
  }).default;
  component.render = () => null;
  const props = reactive({ repoPath: "repo", filePath: "a.png", staged: false, commitSha: null });
  const app = renderer.createApp({ render: () => h(component, props) });
  app.mount({});
  const state = app._instance.subTree.component.setupState;
  const take = (command, side) => {
    const index = calls.findIndex(c => c.command === command && (!side || c.args.side === side));
    assert.notEqual(index, -1, `Missing ${command} ${side ?? ""}`);
    return calls.splice(index, 1)[0];
  };
  return { props, state, calls, live, revoked, take,
    stop() { app.unmount(); URL.createObjectURL = create; URL.revokeObjectURL = revoke; },
  };
}

test("compressed images are bounded by bytes, dimensions and animation, not just file size", () => {
  assert.doesNotThrow(() => preview.validateImagePreview(png(), "a.png"));
  assert.throws(() => preview.validateImagePreview(png(20000, 20000), "a.png"), /preview limit/);
  assert.throws(() => preview.validateImagePreview(png(4096, 4096), "a.png"), /preview limit/);
  assert.throws(() => preview.validateImagePreview(new Uint8Array(images.MAX_IMAGE_PREVIEW_BYTES + 1), "a.png"), /10 MB/);
  assert.throws(() => preview.validateImagePreview(new Uint8Array(), "a.png"), /empty/);
  const animated = png(); animated.set([97, 99, 84, 76], 37);
  assert.throws(() => preview.validateImagePreview(animated, "a.png"), /Animated/);
  assert.throws(() => preview.validateImagePreview(new Uint8Array([0xff, 0xd8]), "a.jpg"), /dimensions/);
});

test("before and after load serially, unchanged refresh does not reload bytes, unmount releases URLs", async () => {
  const v = mount();
  try {
    v.take("get_binary_diff_info").resolve(both()); await tick();
    v.take("get_file_bytes", "old").resolve(png().buffer); await tick();
    v.take("get_file_bytes", "new").resolve(png(40).buffer); await tick();
    assert.equal(v.state.canCompare, true); assert.equal(v.live.size, 2);
    const oldUrl = v.state.oldUrl;
    const refresh = v.state.loadBinary(true);
    v.take("get_binary_diff_info").resolve(both()); await refresh;
    assert.equal(v.state.oldUrl, oldUrl); assert.equal(v.calls.length, 0);
  } finally { v.stop(); }
  assert.equal(v.live.size, 0);
});

test("equal Git blob IDs share one read and one URL", async () => {
  const v = mount();
  try {
    v.take("get_binary_diff_info").resolve({ old: side(), new: side() }); await tick();
    v.take("get_file_bytes").resolve(png()); await tick();
    assert.equal(v.state.oldUrl, v.state.newUrl); assert.equal(v.live.size, 1);
    assert.equal(v.calls.length, 0);
  } finally { v.stop(); }
  assert.equal(v.revoked.length, 1);
});

test("one failed side does not hide the other side or leak its URL", async () => {
  const v = mount();
  try {
    v.take("get_binary_diff_info").resolve(both()); await tick();
    v.take("get_file_bytes", "old").resolve(png()); await tick();
    v.take("get_file_bytes", "new").reject(new Error("Read failed")); await tick();
    assert.ok(v.state.oldUrl); assert.equal(v.state.newUrl, null);
    assert.equal(v.state.error, null); assert.equal(v.state.emptyMessage("new"), "Read failed");
  } finally { v.stop(); }
  assert.equal(v.live.size, 0);
});

test("stale reads cannot publish URLs or start the second side after selection/unmount", async () => {
  const v = mount();
  try {
    v.take("get_binary_diff_info").resolve(both()); await tick();
    const stale = v.take("get_file_bytes", "old");
    v.props.filePath = "b.png"; await tick();
    v.take("get_binary_diff_info").resolve(both()); await tick();
    assert.equal(v.calls.length, 0, "next read waits for the outstanding read");
    stale.resolve(png()); await tick();
    const latest = v.take("get_file_bytes", "old");
    assert.equal(latest.args.filePath, "b.png"); assert.equal(v.live.size, 0);
    v.stop(); latest.resolve(png()); await tick();
    assert.equal(v.calls.length, 0); assert.equal(v.live.size, 0);
  } catch (error) { v.stop(); throw error; }
});

test("missing, oversized and non-image sides do not read file bytes", async () => {
  const v = mount();
  try {
    v.take("get_binary_diff_info").resolve({ old: side("", 0, false), new: side("big", images.MAX_IMAGE_PREVIEW_BYTES + 1) }); await tick();
    assert.equal(v.calls.length, 0); assert.equal(v.state.canCompare, false);
    assert.equal(v.state.emptyMessage("old"), "No previous version");
    assert.match(v.state.emptyMessage("new"), /too large/);
    v.props.filePath = "archive.zip"; await tick();
    v.take("get_binary_diff_info").resolve(both()); await tick();
    assert.equal(v.calls.length, 0); assert.equal(v.live.size, 0);
  } finally { v.stop(); }
});

test("changed working file metadata triggers only a fresh bounded preview", async () => {
  const v = mount();
  try {
    v.take("get_binary_diff_info").resolve(both()); await tick();
    v.take("get_file_bytes", "old").resolve(png()); await tick();
    v.take("get_file_bytes", "new").resolve(png()); await tick();
    const oldUrls = [...v.live];
    const refresh = v.state.loadBinary(true);
    v.take("get_binary_diff_info").resolve({ old: side("before"), new: side("changed") }); await tick();
    assert.ok(oldUrls.every(url => v.revoked.includes(url)));
    v.take("get_file_bytes", "old").resolve(png()); await tick();
    v.take("get_file_bytes", "new").resolve(png()); await refresh;
    assert.equal(v.live.size, 2);
  } finally { v.stop(); }
});
