import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import ts from "typescript";

const require = createRequire(import.meta.url);
const read = name => readFileSync(new URL(`../src/${name}`, import.meta.url), "utf8");
function evaluate(name, mocks = {}) {
  const { outputText } = ts.transpileModule(read(name), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  });
  const module = { exports: {} };
  new Function("require", "module", "exports", outputText)(id => mocks[id] ?? require(id), module, module.exports);
  return module.exports;
}
const { createGitState } = evaluate("domain/git/composables/gitState.ts");
const stash = (sha = "saved", index = 0) => ({ index, stash_sha: sha, message: sha, branch: "main", timestamp: "", parent_sha: "base" });
const file = (path, status = "modified", staged = false, conflicted = false) => ({ path, status, staged, conflicted });

function fixture(respond = () => "ok", updateStashes) {
  const state = createGitState();
  state.repoPath.value = "repo";
  state.stashes.value = [stash()];
  state.selectedStash.value = stash();
  const calls = [], notices = [], previews = [], selected = [];
  const refreshed = { status: 0, stashes: 0 };
  const toast = Object.fromEntries(["loading", "info", "error", "success", "warning", "remove"].map(kind => [kind, (...args) => { notices.push([kind, ...args]); return 1; }]));
  const refresh = {
    refreshStatus: async () => { refreshed.status++; },
    refreshStashes: async () => { refreshed.stashes++; updateStashes?.(state); },
    selectStash: async value => { selected.push(value); state.selectedStash.value = value; if (!value) state.selectedStashFiles.value = []; },
  };
  const { createStashActions } = evaluate("domain/git/composables/gitStashActions.ts", {
    "@/shared/notifications/useToast": { useToast: () => toast },
    "@/shared/notifications/gitOperationPreview": { publishOperationPreview: value => previews.push(value) },
    "./gitCall": { callTauri: async (command, args) => { calls.push({ command, args }); return respond(command, args); } },
  });
  return { api: createStashActions(state, refresh, toast), state, calls, notices, previews, refreshed, selected };
}

test("the public useGit facade exposes the real selective stash action", async () => {
  const actions = fixture();
  const factories = ["Branch", "Ghost", "History", "Refresh", "Remote", "Repo", "Status", "Terminal", "Token", "Watcher"];
  const mocks = {
    "@/shared/notifications/useToast": { useToast: () => ({}) },
    "@/shared/notifications/destructiveScheduler": { createDestructiveScheduler: () => ({ cancelPendingActions() {} }) },
    "@/shared/config/undoPreferences": { getStoredDestructiveUndoEnabled: () => true },
    "./composables/gitState": { createGitState },
    "./composables/gitStashActions": { createStashActions: () => actions.api },
  };
  for (const kind of factories) {
    const module = kind === "Repo" ? "Repository" : kind;
    mocks[`./composables/git${module}Actions`] = { [`create${kind}Actions`]: () => new Proxy({}, { get: () => () => {} }) };
  }
  const { useGit } = evaluate("domain/git/UseGit.ts", mocks);
  const git = useGit();
  for (const name of Object.keys(actions.api)) assert.equal(git[name], actions.api[name], `Missing public stash action: ${name}`);
  await git.restoreSelectedStash(stash(), ["new.ts"], true);
  assert.equal(actions.calls[0].command, "restore_selected_stash");
});

test("stash includes new files by default and keeps selected stash indices up to date", async () => {
  const f = fixture(() => "saved", state => { state.stashes.value = [stash("newest"), stash("saved", 1)]; });
  f.state.fileStatuses.value = [file("tracked.ts"), file("new.ts", "untracked")];
  await f.api.stashPush("All changes");
  assert.equal(f.calls[0].args.includeUntracked, true);
  assert.deepEqual(f.state.fileStatuses.value, []);
  assert.equal(f.state.selectedStash.value.index, 1);
  assert.deepEqual(f.refreshed, { status: 1, stashes: 1 });
});

test("tracked-only stash remains an explicit option and keeps new files visible", async () => {
  const f = fixture();
  f.state.fileStatuses.value = [file("tracked.ts"), file("new.ts", "new"), file("staged.ts", "new", true)];
  await f.api.stashPush("Tracked only", false);
  assert.equal(f.calls[0].args.includeUntracked, false);
  assert.deepEqual(f.state.fileStatuses.value.map(value => value.path), ["new.ts"]);
});

test("failed stash save leaves working changes visible", async () => {
  const f = fixture(() => { throw Error("Unable to save"); });
  f.state.fileStatuses.value = [file("new.ts", "new")];
  await f.api.stashPush();
  assert.equal(f.state.fileStatuses.value[0].path, "new.ts");
  assert.equal(f.state.error.value, "Error: Unable to save");
  assert.equal(f.previews.length, 0);
});

test("apply selected carries the stash identity and does not remove it", async () => {
  const f = fixture(() => ({ message: "Applied", remainder: null }));
  await f.api.restoreSelectedStash(stash(), ["new.ts"], false);
  assert.deepEqual(f.calls[0], { command: "restore_selected_stash", args: { path: "repo", index: 0, sha: "saved", paths: ["new.ts"], pop: false } });
  assert.equal(f.state.selectedStash.value.stash_sha, "saved");
  assert.equal(f.refreshed.stashes, 0);
});

test("partial pop selects the remaining stash and full pop clears the preview", async () => {
  for (const remainder of ["remaining", null]) {
    const f = fixture(() => ({ message: "Restored", remainder }), state => { state.stashes.value = remainder ? [stash(remainder)] : []; });
    await f.api.restoreSelectedStash(stash(), ["new.ts"], true);
    assert.equal(f.state.selectedStash.value?.stash_sha ?? null, remainder);
    assert.equal(f.calls[0].args.pop, true);
    assert.deepEqual(f.refreshed, { status: 1, stashes: 1 });
  }
});

test("native pop and apply failures refresh conflicts without reporting success", async () => {
  for (const action of ["stashPop", "stashApply"]) {
    const f = fixture(() => { throw Error("Conflict: original stash was kept"); });
    await f.api[action]();
    assert.deepEqual(f.refreshed, { status: 1, stashes: 1 });
    assert.match(f.state.error.value, /Conflict/);
    assert.equal(f.state.stashes.value[0].stash_sha, "saved");
    assert.ok(!f.notices.some(([kind]) => kind === "success"));
  }
});

test("failed selective pop keeps the selection and refreshes actual repository state", async () => {
  const f = fixture(() => { throw Error("Original stash kept"); });
  await f.api.restoreSelectedStash(stash(), ["new.ts"], true);
  assert.equal(f.state.selectedStash.value.stash_sha, "saved");
  assert.deepEqual(f.refreshed, { status: 1, stashes: 1 });
});

test("successful native pop and drop clear a deleted stash preview", async () => {
  for (const action of ["stashPop", "stashDrop"]) {
    const f = fixture(() => "ok", state => { state.stashes.value = []; });
    await f.api[action]();
    assert.equal(f.state.selectedStash.value, null);
    assert.deepEqual(f.selected, [null]);
  }
});

test("existing conflicts block saving, applying and popping a stash", async () => {
  const f = fixture();
  f.state.fileStatuses.value = [file("conflict.ts", "modified", false, true)];
  await f.api.stashPush();
  await f.api.stashPop();
  await f.api.stashApply();
  await f.api.restoreSelectedStash(stash(), ["conflict.ts"], true);
  assert.equal(f.calls.length, 0);
});

test("a second stash operation cannot run until the first has finished", async () => {
  let complete;
  const f = fixture(() => new Promise(resolve => { complete = resolve; }));
  const first = f.api.stashPush();
  await f.api.stashPop();
  assert.equal(f.calls.length, 1);
  complete("saved");
  await first;
});

test("switching repositories during stash completion cannot update the new repository", async () => {
  for (const action of ["stashPop", "stashApply", "stashDrop"]) {
    let complete;
    const f = fixture(() => new Promise(resolve => { complete = resolve; }));
    const pending = f.api[action]();
    f.state.repoPath.value = "other-repo";
    complete("ok");
    await pending;
    assert.deepEqual(f.refreshed, { status: 0, stashes: 0 });
    assert.equal(f.previews.length, 0);
    assert.equal(f.state.selectedStash.value.stash_sha, "saved");
  }
});
