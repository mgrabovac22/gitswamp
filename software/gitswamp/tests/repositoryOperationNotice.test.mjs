import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { effectScope, nextTick, ref } from "vue";
import ts from "typescript";

const require = createRequire(import.meta.url);
function loadSource(path, mocks = {}) {
  const source = readFileSync(new URL(`../src/${path}`, import.meta.url), "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  });
  const module = { exports: {} };
  new Function("require", "module", "exports", outputText)(
    id => Object.hasOwn(mocks, id) ? mocks[id] : require(id), module, module.exports,
  );
  return module.exports;
}

const { createGitState } = loadSource("domain/git/composables/gitState.ts");
const { useRepositoryOperationNotice } = loadSource("shared/notifications/useRepositoryOperationNotice.ts");
const info = (path, kind = null) => ({
  path, name: path, current_branch: "main", is_clean: true, head_sha: "abc", remotes: [],
  operation: kind ? { kind, message: "Continue operation" } : null,
});

function notices(source) {
  const shown = [];
  const removed = [];
  const scope = effectScope();
  scope.run(() => useRepositoryOperationNotice(source, {
    action: (...args) => { shown.push(args); return shown.length; },
    remove: id => removed.push(id),
  }, () => {}, () => {}, () => {}));
  return { shown, removed, stop: () => scope.stop() };
}

function repositoryHarness() {
  const state = createGitState();
  const pending = [];
  const { createRepoActions } = loadSource("domain/git/composables/gitRepositoryActions.ts", {
    "./gitCall": { callTauri: (command, { path }) => {
      assert.equal(command, "get_repo_info");
      return new Promise(resolve => pending.push(value => resolve(value ?? info(path))));
    } },
    "./gitHelpers": { getTokenForUrl: () => null },
  });
  let finishStatus = () => {};
  const refresh = {
    refreshCommits: async () => {}, refreshBranches: async () => {},
    refreshStashes: async () => {}, refreshTags: async () => {},
    refreshStatus: () => new Promise(resolve => { finishStatus = resolve; }),
  };
  const repo = createRepoActions(state, refresh, { startFileWatcher: () => {} });
  const notice = notices({
    path: () => state.repoPath.value, kind: () => state.repositoryOperation.value?.kind,
    hasConflicts: () => state.hasConflicts.value, loading: () => state.loading.value,
  });
  return { state, repo, notice, pending, finishStatus: () => finishStatus() };
}

test("restart ignores saved cherry-pick metadata when Git reports a completed abort", async () => {
  const h = repositoryHarness();
  try {
    const opening = h.repo.openRepository("repo", { optimisticRepoInfo: info("repo", "cherry-pick") });
    await nextTick();
    assert.equal(h.state.repositoryOperation.value, null);
    assert.equal(h.notice.shown.length, 0);
    h.pending.shift()(info("repo"));
    h.finishStatus();
    await opening;
    await nextTick();
    assert.equal(h.state.repositoryOperation.value, null);
    assert.equal(h.notice.shown.length, 0);
  } finally { h.notice.stop(); }
});

test("disk operation waits for file status; unresolved conflicts do not produce resolved toast", async () => {
  const h = repositoryHarness();
  try {
    const opening = h.repo.openRepository("repo");
    h.pending.shift()(info("repo", "cherry-pick"));
    await nextTick();
    assert.equal(h.state.repositoryOperation.value, null);
    h.state.fileStatuses.value = [{ path: "file.txt", conflicted: true }];
    h.finishStatus();
    await opening;
    await nextTick();
    assert.equal(h.notice.shown.length, 0);
    h.state.fileStatuses.value = [];
    await nextTick();
    assert.equal(h.notice.shown.length, 1);
  } finally { h.notice.stop(); }
});

test("cached tab operation is not treated as current before disk validation", async () => {
  const h = repositoryHarness();
  const originalSetTimeout = globalThis.setTimeout;
  const scheduled = [];
  try {
    let opening = h.repo.openRepository("repo");
    h.pending.shift()(info("repo", "cherry-pick"));
    h.finishStatus();
    await opening;
    await nextTick();
    assert.equal(h.notice.shown.length, 1);
    opening = h.repo.openRepository("other");
    h.pending.shift()(info("other"));
    h.finishStatus();
    await opening;
    await nextTick();
    globalThis.setTimeout = callback => { scheduled.push(callback); return 1; };
    await h.repo.openRepository("repo");
    assert.equal(h.state.repositoryOperation.value, null);
    await nextTick();
    assert.equal(h.notice.shown.length, 1);
    scheduled.shift()();
    h.pending.shift()(info("repo"));
    h.finishStatus();
    await new Promise(resolve => setImmediate(resolve));
    await nextTick();
    assert.equal(h.notice.shown.length, 1);
  } finally {
    globalThis.setTimeout = originalSetTimeout;
    h.notice.stop();
  }
});

for (const kind of ["cherry-pick", "merge"]) {
  test(`${kind}: abort removes the existing toast and refresh does not show it again`, async () => {
    const path = ref("repo");
    const operation = ref();
    const loading = ref(false);
    const n = notices({ path: () => path.value, kind: () => operation.value,
      hasConflicts: () => false, loading: () => loading.value });
    try {
      operation.value = kind;
      await nextTick();
      assert.equal(n.shown.length, 1);
      loading.value = true;
      await nextTick();
      operation.value = undefined;
      await nextTick();
      assert.deepEqual(n.removed, [1]);
      loading.value = false;
      await nextTick();
      assert.equal(n.shown.length, 1);
    } finally { n.stop(); }
  });
}

test("refresh does not duplicate a valid notice; switching repos or unmounting removes it", async () => {
  const path = ref("one");
  const kind = ref();
  const loading = ref(false);
  const n = notices({ path: () => path.value, kind: () => kind.value,
    hasConflicts: () => false, loading: () => loading.value });
  try {
    kind.value = "cherry-pick";
    await nextTick();
    loading.value = true;
    await nextTick();
    loading.value = false;
    await nextTick();
    assert.equal(n.shown.length, 1);
    path.value = "two";
    kind.value = undefined;
    await nextTick();
    assert.deepEqual(n.removed, [1]);
    kind.value = "cherry-pick";
    await nextTick();
    n.stop();
    assert.deepEqual(n.removed, [1, 2]);
  } finally { n.stop(); }
});
