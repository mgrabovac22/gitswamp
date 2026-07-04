<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted, nextTick, type CSSProperties } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  AlertTriangle,
  Calendar,
  Hash,
  ChevronDown,
  FileText,
  FolderPlus,
  GitBranch,
  ArrowUp,
  ArrowDown,
  Copy,
  Check,
  Plus,
  Minus,
  Trash2,
  Archive,
  Eye,
  Folder,
  File,
  FolderTree,
  Files,
  Hammer,
  Map as MapIcon,
} from "lucide-vue-next";
import AppButton from "@/shared/ui/AppButton.vue";
import CloseIconButton from "@/shared/ui/CloseIconButton.vue";
import GitCommitIcon from "@/shared/ui/GitCommitIcon.vue";
import SmartGitignoreWizard from "@/view/commit/SmartGitignoreWizard.vue";
import { splitFilePath } from "@/shared/codeView";
import { useToast } from "@/shared/notifications/useToast";
import {
  createCommitLintEngine,
  inferScopeFromPaths,
  type CommitLintFinding,
  type CommitLintResult,
} from "@/domain/analyzer/commitAnalyzer";
import {
  COMMIT_ANALYZER_SETTINGS_EVENT,
  getStoredCommitAnalyzerSettings,
} from "@/shared/config/commitAnalyzerPreferences";
import type { CommitInfo, FileStatusInfo, CommitFileInfo, StashInfo, StagedDiffSummary } from "@/types";

const props = defineProps<{
  commit: CommitInfo | null;
  selectedCommits?: CommitInfo[];
  canAmendSelectedCommit?: boolean;
  stagedFiles: FileStatusInfo[];
  unstagedFiles: FileStatusInfo[];
  conflictFiles?: FileStatusInfo[];
  hasConflicts?: boolean;
  commitFiles: CommitFileInfo[];
  isWorkingChanges: boolean;
  isStash?: boolean;
  selectedStash?: StashInfo | null;
  stashFiles?: CommitFileInfo[];
  repoPath: string;
  smartGitignoreWizardEnabled?: boolean;
}>();

const emit = defineEmits<{
  stage: [path: string];
  unstage: [path: string];
  stageAll: [];
  unstageAll: [];
  commit: [message: string];
  discard: [path: string];
  discardAll: [];
  resolveAllConflicts: [];
  resolveConflict: [path: string];
  manualResolve: [path: string];
  stashPop: [index: number];
  stashApply: [index: number];
  stashDrop: [index: number];
  viewDiff: [{ path: string; sha: string | null; staged: boolean }];
  closeDiffViewer: [];
  amendCommitMessage: [newMessage: string];
  refreshState: [];
}>();

const commitSummary = ref("");
const commitDescription = ref("");
const showCommitBuilder = ref(false);
const commitBuilderType = ref("Fix");
const commitBuilderScope = ref("");
const commitBuilderIssue = ref("");
const commitBuilderSummary = ref("");
const commitBuilderBody = ref("");
const commitLintEngine = createCommitLintEngine();
const commitAnalyzerSettings = ref(getStoredCommitAnalyzerSettings());
const commitLintResult = ref<CommitLintResult | null>(null);
const commitLintLoading = ref(false);
const stagedDiffSummary = ref<StagedDiffSummary>({
  total_lines_added: 0,
  total_lines_removed: 0,
  files_changed: 0,
  file_types: [],
  has_test_changes: false,
  has_migration_changes: false,
  inferred_scope: "general",
});
const COMMIT_LINT_DEBOUNCE_MS = 300;
let commitLintTimer: ReturnType<typeof setTimeout> | null = null;
let copiedShaTimer: ReturnType<typeof setTimeout> | null = null;
const showDiscardConfirm = ref(false);
const discardPath = ref<string | null>(null);
const copiedShaKey = ref<string | null>(null);
const selectedChangePath = ref<string | null>(null);
const commitFilesScrollContainer = ref<HTMLElement | null>(null);
const fileNavigationMode = ref(false);
const commitBuilderButtonRef = ref<HTMLElement | null>(null);
const commitBuilderPanelRef = ref<HTMLElement | null>(null);
const commitAnalyzerIndicatorRef = ref<HTMLElement | null>(null);
const commitAnalyzerPanelRef = ref<HTMLElement | null>(null);
const commitAnalyzerTooltipVisible = ref(false);
const commitBuilderPanelStyle = ref<CSSProperties>({ left: "-10000px", top: "-10000px", width: "360px", zIndex: 2147483600 });
const commitAnalyzerPanelStyle = ref<CSSProperties>({ left: "-10000px", top: "-10000px", width: "270px", zIndex: 2147483600 });

function openDiff(filePath: string, commitSha: string | null, staged: boolean) {
  closeFileContextMenu();
  selectedChangePath.value = filePath;
  emit("viewDiff", { path: filePath, sha: commitSha, staged });
}

function scrollSelectedCommitFileIntoView() {
  const container = commitFilesScrollContainer.value;
  const selectedPath = selectedChangePath.value;
  if (!container || !selectedPath) {
    return;
  }

  const escapedPath = globalThis.CSS?.escape ? globalThis.CSS.escape(selectedPath) : selectedPath;
  const selectedRow = container.querySelector<HTMLElement>(`[data-commit-file-path="${escapedPath}"]`);
  selectedRow?.scrollIntoView({ block: "nearest", behavior: "auto" });
}

function confirmDiscard(path: string | null) {
  discardPath.value = path;
  showDiscardConfirm.value = true;
}

function handleDiscardConfirm() {
  if (discardPath.value === null) {
    emit("discardAll");
  } else {
    emit("discard", discardPath.value);
  }
  showDiscardConfirm.value = false;
  discardPath.value = null;
}

function cancelDiscard() {
  showDiscardConfirm.value = false;
  discardPath.value = null;
}

function currentCommitFileIndex(): number {
  if (!selectedChangePath.value) {
    return -1;
  }

  return props.commitFiles.findIndex((file) => file.path === selectedChangePath.value);
}

function selectCommitFileAtIndex(index: number) {
  const file = props.commitFiles[index];
  if (!file) {
    return;
  }

  fileNavigationMode.value = true;
  setActiveTab("changes");
  changesViewMode.value = "files";
  selectedChangePath.value = file.path;
}

function openCommitFileAtIndex(index: number) {
  const file = props.commitFiles[index];
  if (!file) {
    return;
  }

  selectCommitFileAtIndex(index);
  openDiff(
    file.path,
    isMultiCommitSelection.value ? (file.commit_shas?.[0] || primaryCommitShaForDiff.value) : (props.commit?.sha ?? null),
    false,
  );

  void nextTick(() => {
    scrollSelectedCommitFileIntoView();
  });
}

function navigateCommitFile(direction: 1 | -1) {
  if (props.commitFiles.length === 0) {
    return;
  }

  const currentIndex = currentCommitFileIndex();
  let nextIndex = 0;
  if (currentIndex < 0) {
    nextIndex = direction > 0 ? 0 : props.commitFiles.length - 1;
  } else {
    nextIndex = Math.max(0, Math.min(props.commitFiles.length - 1, currentIndex + direction));
  }

  openCommitFileAtIndex(nextIndex);
}

function enterChangesPanelAndSelectFile(index = 0) {
  if (props.commitFiles.length === 0) {
    return;
  }

  const nextIndex = Math.max(0, Math.min(props.commitFiles.length - 1, index));
  fileNavigationMode.value = true;
  setActiveTab("changes");
  changesViewMode.value = "files";
  openCommitFileAtIndex(nextIndex);
}

function hasNavigableCommitFiles(): boolean {
  return !!props.commit && !props.isWorkingChanges && !props.isStash && props.commitFiles.length > 0;
}

function handleCommitFileArrowRight(event: KeyboardEvent) {
  if (!hasNavigableCommitFiles()) {
    return;
  }

  event.preventDefault();
  event.stopImmediatePropagation();
  enterChangesPanelAndSelectFile(Math.max(0, currentCommitFileIndex()));
}

function handleCommitFileArrowDown(event: KeyboardEvent) {
  if (!hasNavigableCommitFiles() || !fileNavigationMode.value) {
    return;
  }

  event.preventDefault();
  event.stopImmediatePropagation();

  if (activeTab.value === "changes") {
    navigateCommitFile(1);
    return;
  }

  enterChangesPanelAndSelectFile(0);
}

function handleCommitFileArrowUp(event: KeyboardEvent) {
  if (!hasNavigableCommitFiles() || !fileNavigationMode.value || activeTab.value !== "changes") {
    return;
  }

  event.preventDefault();
  event.stopImmediatePropagation();
  navigateCommitFile(-1);
}

function handleCommitFileArrowLeft(event: KeyboardEvent) {
  if (!selectedChangePath.value) {
    return;
  }

  event.preventDefault();
  event.stopImmediatePropagation();
  fileNavigationMode.value = false;
  selectedChangePath.value = null;
  emit("closeDiffViewer");
}

const activeTab = ref<"changes" | "info">("changes");
// Tracks whether the user explicitly selected the tab. When true, do not override on programmatic commit changes.
const userSelectedTab = ref(false);

function setActiveTab(tab: "changes" | "info") {
  activeTab.value = tab;
  userSelectedTab.value = true;
}
const isMultiCommitSelection = computed(() => (props.selectedCommits?.length || 0) > 1);
const primaryCommitShaForDiff = computed(() => {
  if (props.commit?.sha) {
    return props.commit.sha;
  }
  if (isMultiCommitSelection.value) {
    return props.selectedCommits?.[0]?.sha || null;
  }
  return null;
});

watch(() => props.commit, (newVal) => {
  selectedChangePath.value = null;
  fileNavigationMode.value = false;
  if (isMultiCommitSelection.value) {
    activeTab.value = "changes";
    userSelectedTab.value = false;
    return;
  }

  // Preserve user choice when stepping between commits
  if (userSelectedTab.value) {
    return;
  }

  if (newVal) {
    activeTab.value = "info";
  } else {
    activeTab.value = "changes";
  }
});

watch(selectedChangePath, async () => {
  if (activeTab.value !== "changes") {
    return;
  }

  await nextTick();
  scrollSelectedCommitFileIntoView();
});

watch(() => props.selectedStash, (newVal) => {
  if (newVal) {
    activeTab.value = "changes";
    userSelectedTab.value = false;
    fileNavigationMode.value = false;
  }
});

watch(() => isMultiCommitSelection.value, (isMulti) => {
  if (isMulti) {
    activeTab.value = "changes";
    userSelectedTab.value = false;
    fileNavigationMode.value = false;
  }
});

watch(() => props.isWorkingChanges, (val) => {
  if (val) {
    activeTab.value = "changes";
    userSelectedTab.value = false;
    fileNavigationMode.value = false;
  }
});

const expandedStaged = ref(true);
const expandedUnstaged = ref(true);
const toast = useToast();

type ExternalEditorId = "notepad" | "vscode" | "visualstudio" | "androidstudio" | "intellij";

interface ExternalEditorOption {
  id: ExternalEditorId;
  label: string;
  hint: string;
}

const editorOptionMap: Record<ExternalEditorId, ExternalEditorOption> = {
  notepad: {
    id: "notepad",
    label: "Notepad",
    hint: "Quick plain text editor",
  },
  vscode: {
    id: "vscode",
    label: "VS Code",
    hint: "Open in Visual Studio Code",
  },
  visualstudio: {
    id: "visualstudio",
    label: "Visual Studio",
    hint: "Open in Visual Studio",
  },
  androidstudio: {
    id: "androidstudio",
    label: "Android Studio",
    hint: "Open in Android Studio",
  },
  intellij: {
    id: "intellij",
    label: "IntelliJ IDEA",
    hint: "Open in IntelliJ if installed",
  },
};

const availableEditors = ref<ExternalEditorOption[]>([]);
const fileCtxVisible = ref(false);
const fileCtxX = ref(0);
const fileCtxY = ref(0);
const fileCtxPath = ref<string | null>(null);
const compactWorkingLabel = ref(globalThis.innerWidth < 1120);
const editingSubject = ref(false);
const editingDescription = ref(false);
const subjectDraft = ref("");
const descriptionDraft = ref("");
const showAmendHint = ref(false);
const workingChangesLabel = computed(() => (compactWorkingLabel.value ? "Working..." : "Working Changes"));
const GITKEEP_NOTIFY_KEY = "gitswamp-notify-gitkeep";
const VERY_LARGE_FILE_THRESHOLD_BYTES = 20 * 1024 * 1024;
const emptyDirectories = ref<string[]>([]);
const emptyDirectoriesLoading = ref(false);
const emptyDirectoriesError = ref("");
const notifyGitkeep = ref(true);
let emptyDirectoriesRunToken = 0;

const stagedFingerprint = computed(() => props.stagedFiles
  .map((file) => `${file.path}:${file.status}:${file.staged}`)
  .sort()
  .join("|"));

const hasTypedCommitMessage = computed(() => commitSummary.value.trim().length > 0);

const commitLintIndicator = computed<"none" | "warning" | "error">(() => {
  if (!hasTypedCommitMessage.value || !commitLintResult.value) {
    return "none";
  }
  if (commitLintResult.value.errors.length > 0) {
    return "error";
  }
  if (commitLintResult.value.warnings.length > 0) {
    return "warning";
  }
  return "none";
});

const commitLintTooltipFindings = computed(() => {
  if (!hasTypedCommitMessage.value || !commitLintResult.value) {
    return [];
  }

  const topErrors = commitLintResult.value.errors.slice(0, 2);
  const topWarnings = commitLintResult.value.warnings.slice(0, 2);
  return [...topErrors, ...topWarnings].slice(0, 3);
});

const commitLintIndicatorClass = computed(() => {
  if (commitLintIndicator.value === "error") {
    return "text-[#ef4444] border-[#ef4444]/50 bg-[#ef4444]/15";
  }
  if (commitLintIndicator.value === "warning") {
    return "text-[#f59e0b] border-[#f59e0b]/50 bg-[#f59e0b]/15";
  }
  return "text-[var(--muted-foreground)] border-[var(--border)] bg-[var(--secondary)]";
});

const commitBuilderTypes = ["Fix", "Feat", "Refactor", "Docs", "Style", "Test", "Chore", "Perf", "Build", "Ci", "Revert"] as const;
const commitBuilderScopes = computed(() => {
  const scopes = [
    stagedDiffSummary.value.inferred_scope,
    "frontend",
    "backend",
    "git",
    "ui",
    "ux",
    "shell",
    "tabs",
    "diff",
    "commit",
    "graph",
    "galaxy",
    "settings",
    "state",
    "repo",
    "branch",
    "stash",
    "merge",
    "rebase",
    "remote",
    "auth",
    "api",
    "data",
    "cache",
    "perf",
    "memory",
    "tests",
    "docs",
    "style",
    "build",
    "ci",
    "deps",
  ]
    .map((scope) => scope.trim().toLowerCase())
    .filter((scope) => scope.length > 0 && scope !== "general");
  return Array.from(new Set(scopes));
});

const commitBuilderPreview = computed(() => {
  const type = capitalizeCommitBuilderType(commitBuilderType.value.trim() || "Fix");
  const scope = commitBuilderScope.value.trim();
  const summary = commitBuilderSummary.value.trim() || commitSummary.value.trim() || "describe change";
  const issueTag = normalizedCommitBuilderIssueTag.value;
  const cleanSummary = issueTag ? summary.replace(/^#\d+\s+/, "") : summary;
  const taggedSummary = issueTag ? `${issueTag} ${cleanSummary}` : cleanSummary;
  return scope ? `${type}(${scope}): ${taggedSummary}` : `${type}: ${taggedSummary}`;
});

const normalizedCommitBuilderIssueTag = computed(() => normalizeIssueTag(commitBuilderIssue.value));

function capitalizeCommitBuilderType(type: string): string {
  const trimmed = type.trim();
  if (!trimmed) {
    return "Fix";
  }
  return trimmed.charAt(0).toUpperCase() + trimmed.slice(1).toLowerCase();
}

function normalizeIssueTag(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    return "";
  }

  const match = trimmed.match(/\d+/);
  return match ? `#${match[0]}` : "";
}

function parseCommitBuilderSubject(subject: string): { type: string; scope: string; issue: string; summary: string } | null {
  const match = subject.trim().match(/^([A-Za-z]+)(?:\(([^)]+)\))?:\s*(#\d+)?\s*(.*)$/);
  if (!match) {
    return null;
  }

  return {
    type: capitalizeCommitBuilderType(match[1] || "Fix"),
    scope: (match[2] || "").trim(),
    issue: match[3] || "",
    summary: (match[4] || "").trim(),
  };
}

function updateFloatingPanelPosition(
  anchor: HTMLElement | null,
  panel: HTMLElement | null,
  width: number,
  fallbackHeight: number,
  target: typeof commitBuilderPanelStyle,
) {
  if (!anchor) {
    return;
  }

  const rect = anchor.getBoundingClientRect();
  const viewportWidth = globalThis.innerWidth || document.documentElement.clientWidth || 1024;
  const viewportHeight = globalThis.innerHeight || document.documentElement.clientHeight || 768;
  const panelHeight = panel?.offsetHeight || fallbackHeight;
  const left = Math.min(Math.max(8, rect.right - width), Math.max(8, viewportWidth - width - 8));
  let top = rect.top - panelHeight - 8;

  if (top < 8) {
    top = Math.min(rect.bottom + 8, Math.max(8, viewportHeight - panelHeight - 8));
  }

  target.value = {
    left: `${left}px`,
    top: `${top}px`,
    width: `${width}px`,
    zIndex: 2147483600,
  };
}

function updateCommitFloatingPanels() {
  if (showCommitBuilder.value) {
    updateFloatingPanelPosition(commitBuilderButtonRef.value, commitBuilderPanelRef.value, 360, 340, commitBuilderPanelStyle);
  }
  if (commitAnalyzerTooltipVisible.value) {
    updateFloatingPanelPosition(commitAnalyzerIndicatorRef.value, commitAnalyzerPanelRef.value, 270, 112, commitAnalyzerPanelStyle);
  }
}

function showCommitAnalyzerTooltip() {
  commitAnalyzerTooltipVisible.value = true;
  void nextTick(updateCommitFloatingPanels);
}

function hideCommitAnalyzerTooltip() {
  commitAnalyzerTooltipVisible.value = false;
}

function openCommitBuilder() {
  const parsed = parseCommitBuilderSubject(commitSummary.value);
  commitBuilderType.value = parsed?.type || capitalizeCommitBuilderType(commitBuilderType.value || "Fix");
  commitBuilderScope.value = parsed?.scope || commitBuilderScope.value || stagedDiffSummary.value.inferred_scope || commitBuilderScopes.value[0] || "";
  commitBuilderIssue.value = parsed?.issue || commitBuilderIssue.value || "";
  commitBuilderSummary.value = parsed?.summary || commitSummary.value;
  commitBuilderBody.value = commitDescription.value;
  showCommitBuilder.value = true;
  void nextTick(updateCommitFloatingPanels);
}

function saveCommitBuilder() {
  const summary = commitBuilderSummary.value.trim();
  if (!summary) {
    toast.error("Commit builder needs a short message.");
    return;
  }
  if (commitBuilderIssue.value.trim() && !normalizedCommitBuilderIssueTag.value) {
    toast.error("Issue tag needs a number, for example 123 or #123.");
    return;
  }
  commitSummary.value = commitBuilderPreview.value;
  commitDescription.value = commitBuilderBody.value.trim();
  showCommitBuilder.value = false;
  queueCommitLintAnalysis();
}

function commitLintFindingClass(finding: CommitLintFinding): string {
  if (finding.severity === "error") {
    return "text-[#ef4444]";
  }
  if (finding.severity === "warning") {
    return "text-[#f59e0b]";
  }
  return "text-[var(--muted-foreground)]";
}

function updateCompactWorkingLabel() {
  compactWorkingLabel.value = globalThis.innerWidth < 1120;
}

function syncGitkeepNotifyPreference() {
  const saved = localStorage.getItem(GITKEEP_NOTIFY_KEY);
  if (saved === null) {
    localStorage.setItem(GITKEEP_NOTIFY_KEY, "true");
    notifyGitkeep.value = true;
    return;
  }
  notifyGitkeep.value = saved !== "false";
}

function isVeryLargeFile(file: FileStatusInfo): boolean {
  return (file.file_size_bytes || 0) >= VERY_LARGE_FILE_THRESHOLD_BYTES;
}

function formatFileSize(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${bytes} B`;
}

async function loadEmptyDirectories() {
  syncGitkeepNotifyPreference();

  if (!props.repoPath || !props.isWorkingChanges || !notifyGitkeep.value) {
    emptyDirectories.value = [];
    emptyDirectoriesError.value = "";
    emptyDirectoriesLoading.value = false;
    return;
  }

  const runToken = ++emptyDirectoriesRunToken;
  emptyDirectoriesLoading.value = true;
  emptyDirectoriesError.value = "";

  try {
    const directories = await invoke<string[]>("get_empty_directories", {
      path: props.repoPath,
      maxCount: 200,
    });

    if (runToken !== emptyDirectoriesRunToken) return;
    emptyDirectories.value = directories;
  } catch {
    if (runToken !== emptyDirectoriesRunToken) return;
    emptyDirectories.value = [];
    emptyDirectoriesError.value = "Could not scan empty folders for .gitkeep suggestions.";
  } finally {
    if (runToken === emptyDirectoriesRunToken) {
      emptyDirectoriesLoading.value = false;
    }
  }
}

async function addGitkeep(directoryPath: string) {
  if (!props.repoPath) return;

  try {
    const createdPath = await invoke<string>("add_gitkeep", {
      path: props.repoPath,
      directoryPath,
      stage: true,
    });

    toast.success(`Added and staged ${createdPath}`);
    emit("refreshState");
    await loadEmptyDirectories();
  } catch (error) {
    toast.error(`Could not add .gitkeep for ${directoryPath}: ${String(error)}`);
  }
}

async function addGitkeepToAll() {
  if (!props.repoPath || emptyDirectories.value.length === 0) return;

  const targets = [...emptyDirectories.value];
  for (const directory of targets) {
    try {
      await invoke<string>("add_gitkeep", {
        path: props.repoPath,
        directoryPath: directory,
        stage: true,
      });
    } catch (error) {
      toast.error(`Could not add .gitkeep for ${directory}: ${String(error)}`);
      return;
    }
  }

  toast.success(`Added .gitkeep to ${targets.length} folder${targets.length === 1 ? "" : "s"}`);
  emit("refreshState");
  await loadEmptyDirectories();
}

function syncAmendDraftsFromCommit() {
  const commitMessage = props.commit?.message ?? "";
  subjectDraft.value = commitSubject(commitMessage);
  descriptionDraft.value = commitBody(commitMessage);
  editingSubject.value = false;
  editingDescription.value = false;
  showAmendHint.value = false;
}

watch(() => props.commit?.sha, () => {
  syncAmendDraftsFromCommit();
}, { immediate: true });

watch(() => props.commit?.message, (newMessage) => {
  if (!newMessage) return;
  if (!editingSubject.value && !editingDescription.value) {
    syncAmendDraftsFromCommit();
  }
});

function canAmendSelectedCommit(): boolean {
  return !!props.commit && !!props.canAmendSelectedCommit;
}

function beginSubjectEdit() {
  if (!canAmendSelectedCommit()) {
    showAmendHint.value = true;
    return;
  }
  showAmendHint.value = false;
  subjectDraft.value = commitSubject(props.commit?.message ?? "");
  editingSubject.value = true;
}

function beginDescriptionEdit() {
  if (!canAmendSelectedCommit()) {
    showAmendHint.value = true;
    return;
  }
  showAmendHint.value = false;
  descriptionDraft.value = commitBody(props.commit?.message ?? "");
  editingDescription.value = true;
}

function cancelSubjectEdit() {
  subjectDraft.value = commitSubject(props.commit?.message ?? "");
  editingSubject.value = false;
}

function cancelDescriptionEdit() {
  descriptionDraft.value = commitBody(props.commit?.message ?? "");
  editingDescription.value = false;
}

function buildAmendMessage(nextSubject: string, nextBody: string): string | null {
  const cleanSubject = nextSubject.trim();
  if (!cleanSubject) {
    toast.error("Commit message subject cannot be empty.");
    return null;
  }

  const cleanBody = nextBody.trim();
  if (!cleanBody) {
    return cleanSubject;
  }

  return `${cleanSubject}\n\n${cleanBody}`;
}

function submitSubjectEdit() {
  if (!props.commit || !canAmendSelectedCommit()) return;
  const amended = buildAmendMessage(subjectDraft.value, descriptionDraft.value);
  if (!amended) return;
  editingSubject.value = false;
  emit("amendCommitMessage", amended);
}

function submitDescriptionEdit() {
  if (!props.commit || !canAmendSelectedCommit()) return;
  const amended = buildAmendMessage(subjectDraft.value, descriptionDraft.value);
  if (!amended) return;
  editingDescription.value = false;
  emit("amendCommitMessage", amended);
}

function autoExpandTextarea(textarea: HTMLTextAreaElement) {
  textarea.style.height = "auto";
  textarea.style.height = Math.min(textarea.scrollHeight, 150) + "px";
}

function onDescriptionInput(event: Event) {
  const textarea = event.target as HTMLTextAreaElement;
  autoExpandTextarea(textarea);
}

function refreshCommitAnalyzerSettings() {
  commitAnalyzerSettings.value = getStoredCommitAnalyzerSettings();
}

function buildFallbackStagedDiffSummary(): StagedDiffSummary {
  const paths = props.stagedFiles.map((file) => file.path);
  const fileTypes = new Set<string>();
  let hasTestChanges = false;
  let hasMigrationChanges = false;

  for (const rawPath of paths) {
    const path = rawPath.toLowerCase().split("\\").join("/");
    const dotIndex = path.lastIndexOf(".");
    if (dotIndex > 0 && dotIndex < path.length - 1) {
      fileTypes.add(path.slice(dotIndex));
    }

    if (path.includes("/test/") || path.includes("/tests/") || path.includes(".test.") || path.includes(".spec.")) {
      hasTestChanges = true;
    }
    if (path.includes("migration") || path.includes("/migrations/")) {
      hasMigrationChanges = true;
    }
  }

  return {
    total_lines_added: 0,
    total_lines_removed: 0,
    files_changed: paths.length,
    file_types: Array.from(fileTypes).sort(),
    has_test_changes: hasTestChanges,
    has_migration_changes: hasMigrationChanges,
    inferred_scope: inferScopeFromPaths(paths),
  };
}

async function loadStagedDiffSummary() {
  if (!props.repoPath || !props.isWorkingChanges || props.stagedFiles.length === 0) {
    stagedDiffSummary.value = buildFallbackStagedDiffSummary();
    queueCommitLintAnalysis();
    return;
  }

  commitLintLoading.value = true;
  try {
    stagedDiffSummary.value = await invoke<StagedDiffSummary>("get_staged_diff_summary", {
      path: props.repoPath,
    });
  } catch {
    stagedDiffSummary.value = buildFallbackStagedDiffSummary();
  } finally {
    commitLintLoading.value = false;
    queueCommitLintAnalysis();
  }
}

function runCommitLintAnalysis() {
  if (!props.isWorkingChanges) {
    commitLintResult.value = null;
    return;
  }

  if (!commitAnalyzerSettings.value.enabled) {
    commitLintResult.value = null;
    return;
  }

  if (!commitSummary.value.trim()) {
    commitLintResult.value = null;
    return;
  }

  commitLintResult.value = commitLintEngine.analyze({
    message: commitSummary.value,
    description: commitDescription.value,
    stagedFiles: props.stagedFiles.map((file) => ({
      path: file.path,
      status: file.status,
      staged: file.staged,
    })),
    diffSummary: {
      totalLinesAdded: stagedDiffSummary.value.total_lines_added,
      totalLinesRemoved: stagedDiffSummary.value.total_lines_removed,
      filesChanged: stagedDiffSummary.value.files_changed,
      fileTypes: stagedDiffSummary.value.file_types,
      hasTestChanges: stagedDiffSummary.value.has_test_changes,
      hasMigrationChanges: stagedDiffSummary.value.has_migration_changes,
      inferredScope: stagedDiffSummary.value.inferred_scope,
    },
    settings: commitAnalyzerSettings.value,
  });
}

function queueCommitLintAnalysis() {
  if (commitLintTimer) {
    clearTimeout(commitLintTimer);
    commitLintTimer = null;
  }

  commitLintTimer = setTimeout(() => {
    runCommitLintAnalysis();
  }, COMMIT_LINT_DEBOUNCE_MS);
}

function handleCommitAnalyzerSettingsChanged() {
  refreshCommitAnalyzerSettings();
  queueCommitLintAnalysis();
}

function formatDate(timestamp: number) {
  const d = new Date(timestamp * 1000);
  return d.toLocaleDateString("en-GB", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  }) + " @ " + d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
}

function commitSubject(msg: string): string {
  const idx = msg.indexOf("\n");
  return idx > 0 ? msg.substring(0, idx) : msg;
}
function commitBody(msg: string): string {
  const idx = msg.indexOf("\n");
  return idx > 0 ? msg.substring(idx + 1).trim() : "";
}

function parentRefs(commit: CommitInfo): string[] {
  return commit.parent_shas.map((s) => s.substring(0, 7));
}

function displayIdentityInitial(name?: string): string {
  const initial = (name || "").trim().charAt(0).toUpperCase();
  return initial || "?";
}

function isCherryPickedCommit(commit: CommitInfo): boolean {
  const message = commit.message.toLowerCase();
  return message.includes("cherry picked from commit") || message.includes("(cherry picked from commit");
}

function shouldShowCommitter(commit: CommitInfo): boolean {
  const committerName = (commit.committer_name || "").trim();
  const committerEmail = (commit.committer_email || "").trim();
  if (!committerName && !committerEmail) {
    return false;
  }

  if (isCherryPickedCommit(commit)) {
    return true;
  }

  const authorName = (commit.author_name || "").trim();
  const authorEmail = (commit.author_email || "").trim();
  return committerName !== authorName || committerEmail !== authorEmail;
}

function branchRefs(commit: CommitInfo): string[] {
  const refs = commit.refs.filter((r) => !r.includes("->"));
  const withoutHead = refs.filter((r) => r !== "HEAD");
  if (refs.includes("HEAD")) {
    return ["HEAD", ...withoutHead];
  }
  return withoutHead;
}

function refStatus(commit: CommitInfo): { local: boolean; remote: boolean } {
  const refs = commit.refs;
  const hasLocal = refs.some((r) => !r.startsWith("origin/") && !r.includes("HEAD") && !r.includes("->"));
  const hasRemote = refs.some((r) => r.startsWith("origin/"));
  return { local: hasLocal, remote: hasRemote };
}

function statusIcon(status: string): string {
  switch (status) {
    case "added": case "new": return "+";
    case "deleted": return "−";
    case "modified": return "M";
    case "renamed": return "R";
    default: return "?";
  }
}

function statusColor(status: string): string {
  switch (status) {
    case "conflicted": return "#ef4444";
    case "added": case "new": return "#10b981";
    case "deleted": return "#ef4444";
    case "modified": return "#f59e0b";
    case "renamed": return "#06b6d4";
    default: return "#64748b";
  }
}

type ChangesViewMode = "files" | "tree" | "map";

interface TreeFileMeta {
  status: string;
  hasStaged: boolean;
  hasUnstaged: boolean;
  conflicted: boolean;
}

interface ChangesTreeNode {
  key: string;
  type: "folder" | "file";
  name: string;
  path: string;
  changed: boolean;
  status: string | null;
  children: ChangesTreeNode[];
}

interface ChangesTreeRow {
  node: ChangesTreeNode;
  depth: number;
}

interface ChangeMapFile {
  path: string;
  status: string;
  staged: boolean;
  unstaged: boolean;
  conflicted: boolean;
  additions: number;
  deletions: number;
}

interface ChangeMapFolder {
  path: string;
  label: string;
  depth: number;
  fileCount: number;
  stagedCount: number;
  unstagedCount: number;
  conflictCount: number;
  additions: number;
  deletions: number;
}

const changesViewMode = ref<ChangesViewMode>("files");
const selectedChangeMapFolder = ref<string | null>(null);
const showAllFilesInTree = ref(false);
const commitTreePaths = ref<string[]>([]);
const commitTreeLoading = ref(false);
const commitTreeError = ref("");
const expandedTreeFolders = ref<Record<string, boolean>>({});
let commitTreeRunToken = 0;

const isCommitChangesView = computed(() => !!props.commit && !props.isWorkingChanges && !props.isStash);

function normalizeRepoPath(path: string): string {
  return path.replace(/\\/g, "/").replace(/^\/+|\/+$/g, "").trim();
}

function isTreeFolderExpanded(path: string): boolean {
  return expandedTreeFolders.value[path] !== false;
}

function toggleTreeFolder(path: string) {
  expandedTreeFolders.value[path] = !isTreeFolderExpanded(path);
}

function ensureTreeFolderTracked(path: string) {
  if (!(path in expandedTreeFolders.value)) {
    expandedTreeFolders.value[path] = true;
  }
}

const workingTreeMeta = computed(() => {
  const map = new Map<string, TreeFileMeta>();

  const applyMeta = (file: FileStatusInfo, staged: boolean, conflicted: boolean) => {
    const path = normalizeRepoPath(file.path);
    if (!path) return;

    const existing = map.get(path);
    if (!existing) {
      map.set(path, {
        status: conflicted ? "conflicted" : file.status,
        hasStaged: staged,
        hasUnstaged: !staged,
        conflicted,
      });
      return;
    }

    existing.hasStaged = existing.hasStaged || staged;
    existing.hasUnstaged = existing.hasUnstaged || !staged;
    existing.conflicted = existing.conflicted || conflicted;
    if (existing.conflicted) {
      existing.status = "conflicted";
    }
  };

  for (const file of props.unstagedFiles) {
    applyMeta(file, false, !!file.conflicted);
  }

  for (const file of props.stagedFiles) {
    applyMeta(file, true, !!file.conflicted);
  }

  for (const file of props.conflictFiles || []) {
    applyMeta(file, false, true);
  }

  return map;
});

const commitTreeMeta = computed(() => {
  const map = new Map<string, TreeFileMeta>();
  for (const file of props.commitFiles) {
    const path = normalizeRepoPath(file.path);
    if (!path) continue;
    map.set(path, {
      status: file.status,
      hasStaged: false,
      hasUnstaged: false,
      conflicted: false,
    });
  }
  return map;
});

const stashTreeMeta = computed(() => {
  const map = new Map<string, TreeFileMeta>();
  for (const file of props.stashFiles || []) {
    const path = normalizeRepoPath(file.path);
    if (!path) continue;
    map.set(path, {
      status: file.status,
      hasStaged: false,
      hasUnstaged: false,
      conflicted: false,
    });
  }
  return map;
});

const activeTreeMeta = computed(() => {
  if (props.isWorkingChanges) {
    return workingTreeMeta.value;
  }

  if (props.isStash) {
    return stashTreeMeta.value;
  }

  return commitTreeMeta.value;
});

const treeSourcePaths = computed(() => {
  const changedPaths = Array.from(activeTreeMeta.value.keys());

  if (!isCommitChangesView.value || !showAllFilesInTree.value) {
    return changedPaths;
  }

  const values = new Set<string>([
    ...changedPaths,
    ...commitTreePaths.value.map((path) => normalizeRepoPath(path)).filter((path) => path.length > 0),
  ]);
  return Array.from(values);
});

function sortedNodes(nodes: ChangesTreeNode[]): ChangesTreeNode[] {
  return [...nodes]
    .sort((a, b) => {
      if (a.type !== b.type) {
        return a.type === "folder" ? -1 : 1;
      }
      return a.name.localeCompare(b.name);
    })
    .map((node) => ({
      ...node,
      children: sortedNodes(node.children),
    }));
}

function refreshFolderChangedState(node: ChangesTreeNode): boolean {
  if (node.type === "file") {
    return node.changed;
  }

  let hasChangedChild = false;
  for (const child of node.children) {
    if (refreshFolderChangedState(child)) {
      hasChangedChild = true;
    }
  }
  node.changed = hasChangedChild;
  return hasChangedChild;
}

function getOrCreateChangesTreeNode(
  currentPath: string,
  segment: string,
  isFile: boolean,
  branchNodes: ChangesTreeNode[],
  byPath: Map<string, ChangesTreeNode>,
): ChangesTreeNode {
  const existing = byPath.get(currentPath);
  if (existing) {
    return existing;
  }

  const node: ChangesTreeNode = {
    key: `${isFile ? "file" : "folder"}:${currentPath}`,
    type: isFile ? "file" : "folder",
    name: segment,
    path: currentPath,
    changed: false,
    status: null,
    children: [],
  };

  byPath.set(currentPath, node);
  branchNodes.push(node);
  return node;
}

function applyChangesTreeFileMeta(node: ChangesTreeNode, normalized: string, meta: Map<string, TreeFileMeta>) {
  const fileMeta = meta.get(normalized);
  node.changed = !!fileMeta;
  node.status = fileMeta ? fileMeta.status : null;
}

function addChangesTreePath(
  rawPath: string,
  rootNodes: ChangesTreeNode[],
  byPath: Map<string, ChangesTreeNode>,
  meta: Map<string, TreeFileMeta>,
) {
  const normalized = normalizeRepoPath(rawPath);
  if (!normalized) {
    return;
  }

  const segments = normalized.split("/").filter((value) => value.length > 0);
  if (segments.length === 0) {
    return;
  }

  let branchNodes = rootNodes;
  let currentPath = "";

  for (let idx = 0; idx < segments.length; idx += 1) {
    const segment = segments[idx];
    const isFile = idx === segments.length - 1;
    currentPath = currentPath ? `${currentPath}/${segment}` : segment;

    const node = getOrCreateChangesTreeNode(currentPath, segment, isFile, branchNodes, byPath);
    if (isFile) {
      applyChangesTreeFileMeta(node, normalized, meta);
      continue;
    }

    ensureTreeFolderTracked(currentPath);
    branchNodes = node.children;
  }
}

function buildChangesTree(paths: string[], meta: Map<string, TreeFileMeta>): ChangesTreeNode[] {
  const rootNodes: ChangesTreeNode[] = [];
  const byPath = new Map<string, ChangesTreeNode>();

  for (const rawPath of paths) {
    addChangesTreePath(rawPath, rootNodes, byPath, meta);
  }

  for (const node of rootNodes) {
    refreshFolderChangedState(node);
  }

  return sortedNodes(rootNodes);
}

function flattenTreeRows(nodes: ChangesTreeNode[], depth = 0, rows: ChangesTreeRow[] = []): ChangesTreeRow[] {
  for (const node of nodes) {
    rows.push({ node, depth });
    if (node.type === "folder" && isTreeFolderExpanded(node.path)) {
      flattenTreeRows(node.children, depth + 1, rows);
    }
  }
  return rows;
}

const treeNodes = computed(() => buildChangesTree(treeSourcePaths.value, activeTreeMeta.value));
const treeRows = computed(() => flattenTreeRows(treeNodes.value));
const changeMapFiles = computed<ChangeMapFile[]>(() => {
  const byPath = new Map<string, ChangeMapFile>();

  const ensure = (path: string, status: string, additions = 0, deletions = 0) => {
    const normalized = normalizeRepoPath(path);
    if (!normalized) return null;
    const existing = byPath.get(normalized);
    if (existing) {
      if (existing.status !== "conflicted" && status === "conflicted") {
        existing.status = "conflicted";
      } else if (existing.status === "modified" && status !== "modified") {
        existing.status = status;
      }
      existing.additions += Math.max(0, additions);
      existing.deletions += Math.max(0, deletions);
      return existing;
    }

    const entry: ChangeMapFile = {
      path: normalized,
      status,
      staged: false,
      unstaged: false,
      conflicted: false,
      additions: Math.max(0, additions),
      deletions: Math.max(0, deletions),
    };
    byPath.set(normalized, entry);
    return entry;
  };

  if (props.isWorkingChanges) {
    for (const file of props.unstagedFiles) {
      const entry = ensure(file.path, file.conflicted ? "conflicted" : file.status);
      if (!entry) continue;
      entry.unstaged = true;
      entry.conflicted = entry.conflicted || !!file.conflicted;
    }
    for (const file of props.stagedFiles) {
      const entry = ensure(file.path, file.conflicted ? "conflicted" : file.status);
      if (!entry) continue;
      entry.staged = true;
      entry.conflicted = entry.conflicted || !!file.conflicted;
    }
    for (const file of props.conflictFiles || []) {
      const entry = ensure(file.path, "conflicted");
      if (!entry) continue;
      entry.unstaged = true;
      entry.conflicted = true;
    }
    return Array.from(byPath.values()).sort((a, b) => a.path.localeCompare(b.path));
  }

  const sourceFiles = props.isStash ? (props.stashFiles || []) : props.commitFiles;
  for (const file of sourceFiles) {
    const entry = ensure(file.path, file.status, file.additions || 0, file.deletions || 0);
    if (!entry) continue;
    entry.staged = true;
  }

  return Array.from(byPath.values()).sort((a, b) => a.path.localeCompare(b.path));
});

function directoryForPath(path: string): string {
  const parts = normalizeRepoPath(path).split("/").filter(Boolean);
  if (parts.length <= 1) return ".";
  return parts.slice(0, -1).join("/");
}

function directoryChain(path: string): string[] {
  const directory = directoryForPath(path);
  if (directory === ".") return ["."];
  const segments = directory.split("/");
  const chain: string[] = [];
  for (let index = 0; index < segments.length; index += 1) {
    chain.push(segments.slice(0, index + 1).join("/"));
  }
  return chain;
}

const changeMapFolders = computed<ChangeMapFolder[]>(() => {
  const folders = new Map<string, ChangeMapFolder>();

  for (const file of changeMapFiles.value) {
    for (const folderPath of directoryChain(file.path)) {
      const existing = folders.get(folderPath);
      const folder = existing || {
        path: folderPath,
        label: folderPath === "." ? "root" : `${folderPath.split("/")[folderPath.split("/").length - 1] || folderPath}/`,
        depth: folderPath === "." ? 0 : folderPath.split("/").length - 1,
        fileCount: 0,
        stagedCount: 0,
        unstagedCount: 0,
        conflictCount: 0,
        additions: 0,
        deletions: 0,
      };

      folder.fileCount += 1;
      if (file.conflicted) {
        folder.conflictCount += 1;
      } else if (file.unstaged) {
        folder.unstagedCount += 1;
      } else if (file.staged) {
        folder.stagedCount += 1;
      }
      folder.additions += file.additions;
      folder.deletions += file.deletions;
      folders.set(folderPath, folder);
    }
  }

  return Array.from(folders.values()).sort((a, b) => {
    if (a.path === ".") return -1;
    if (b.path === ".") return 1;
    const aParent = a.path.split("/").slice(0, -1).join("/");
    const bParent = b.path.split("/").slice(0, -1).join("/");
    return aParent.localeCompare(bParent) || a.depth - b.depth || a.path.localeCompare(b.path);
  });
});

const maxChangeMapFolderFiles = computed(() =>
  Math.max(1, ...changeMapFolders.value.map((folder) => folder.fileCount)),
);

const selectedChangeMapLabel = computed(() => {
  if (!selectedChangeMapFolder.value) return "All changed files";
  const folder = changeMapFolders.value.find((item) => item.path === selectedChangeMapFolder.value);
  return folder?.path === "." ? "root" : folder?.path || "Folder";
});

const filteredChangeMapFiles = computed(() => {
  const folder = selectedChangeMapFolder.value;
  if (!folder) return changeMapFiles.value;
  if (folder === ".") {
    return changeMapFiles.value.filter((file) => directoryForPath(file.path) === ".");
  }
  return changeMapFiles.value.filter((file) => file.path.startsWith(`${folder}/`));
});

function setChangeMapFolder(folderPath: string | null) {
  selectedChangeMapFolder.value = selectedChangeMapFolder.value === folderPath ? null : folderPath;
}

function changeMapFolderWidth(folder: ChangeMapFolder): string {
  const percent = Math.max(8, (folder.fileCount / maxChangeMapFolderFiles.value) * 100);
  return `${percent}%`;
}

function changeMapSegmentWidth(count: number, total: number): string {
  if (total <= 0 || count <= 0) return "0%";
  return `${Math.max(6, (count / total) * 100)}%`;
}

function openChangeMapFile(file: ChangeMapFile) {
  if (props.isWorkingChanges) {
    const meta = workingTreeMeta.value.get(file.path);
    openDiff(file.path, null, meta ? !meta.hasUnstaged && meta.hasStaged : file.staged && !file.unstaged);
    return;
  }

  if (props.isStash) {
    return;
  }

  const targetSha = primaryCommitShaForDiff.value;
  if (!targetSha) return;
  openDiff(file.path, targetSha, false);
}

function setChangesViewMode(mode: ChangesViewMode) {
  changesViewMode.value = mode;
  if (mode !== "tree") {
    showAllFilesInTree.value = false;
  }
  if (mode !== "map") {
    selectedChangeMapFolder.value = null;
  }
}

function onTreeRowClick(row: ChangesTreeRow) {
  if (row.node.type === "folder") {
    toggleTreeFolder(row.node.path);
    return;
  }

  openTreeFile(row.node.path);
}

function onTreeRowContextMenu(event: MouseEvent, row: ChangesTreeRow) {
  if (row.node.type !== "file") return;
  openFileContextMenu(event, row.node.path);
}

async function loadCommitTreePaths() {
  if (!props.repoPath || !props.commit?.sha) {
    commitTreePaths.value = [];
    commitTreeError.value = "";
    commitTreeLoading.value = false;
    return;
  }

  const runToken = ++commitTreeRunToken;
  commitTreeLoading.value = true;
  commitTreeError.value = "";

  try {
    const paths = await invoke<string[]>("get_commit_tree_paths", {
      path: props.repoPath,
      sha: props.commit.sha,
    });

    if (runToken !== commitTreeRunToken) return;
    commitTreePaths.value = paths;
  } catch {
    if (runToken !== commitTreeRunToken) return;
    commitTreePaths.value = [];
    commitTreeError.value = "Could not load full commit tree.";
  } finally {
    if (runToken === commitTreeRunToken) {
      commitTreeLoading.value = false;
    }
  }
}

function openTreeFile(path: string) {
  const normalized = normalizeRepoPath(path);

  if (props.isWorkingChanges) {
    const meta = workingTreeMeta.value.get(normalized);
    if (!meta) return;
    openDiff(normalized, null, !meta.hasUnstaged && meta.hasStaged);
    return;
  }

  if (props.isStash) {
    return;
  }

  const commitMeta = commitTreeMeta.value.get(normalized);
  const targetSha = primaryCommitShaForDiff.value;
  if (!commitMeta || !targetSha) return;
  openDiff(normalized, targetSha, false);
}

function onCommit() {
  if (!commitSummary.value.trim()) return;
  const msg = commitDescription.value.trim()
    ? `${commitSummary.value.trim()}\n\n${commitDescription.value.trim()}`
    : commitSummary.value.trim();
  emit("commit", msg);
  commitSummary.value = "";
  commitDescription.value = "";
}

async function copyToClipboard(text: string, key?: string) {
  try {
    await navigator.clipboard.writeText(text);
    if (key) {
      copiedShaKey.value = key;
      if (copiedShaTimer) {
        clearTimeout(copiedShaTimer);
      }
      copiedShaTimer = setTimeout(() => {
        if (copiedShaKey.value === key) {
          copiedShaKey.value = null;
        }
      }, 900);
    }
  } catch {
    toast.error("Could not copy to clipboard.");
  }
}

function fileParts(path: string) {
  return splitFilePath(path);
}

function formatCommitHashes(shas?: string[]): string {
  if (!shas || shas.length === 0) {
    return "";
  }
  return shas.map((sha) => sha.slice(0, 7)).join(", ");
}

function mapEditorIds(ids: string[]): ExternalEditorOption[] {
  const normalized = ids
    .map((id) => id.trim().toLowerCase())
    .filter((id): id is ExternalEditorId => id === "notepad" || id === "vscode" || id === "visualstudio" || id === "androidstudio" || id === "intellij");

  return Array.from(new Set(normalized)).map((id) => editorOptionMap[id]);
}

async function loadAvailableEditors() {
  try {
    const editorIds = await invoke<string[]>("get_available_external_editors");
    const mapped = mapEditorIds(editorIds);
    availableEditors.value = mapped.length > 0 ? mapped : [editorOptionMap.notepad];
  } catch {
    availableEditors.value = [editorOptionMap.notepad];
  }
}

function openFileContextMenu(event: MouseEvent, path: string) {
  event.preventDefault();
  event.stopPropagation();

  if (availableEditors.value.length === 0) {
    availableEditors.value = [editorOptionMap.notepad];
  }

  fileCtxPath.value = path;

  const menuWidth = 250;
  const menuHeight = 80 + Math.max(availableEditors.value.length, 1) * 42;
  let x = event.clientX;
  let y = event.clientY;

  if (x + menuWidth > window.innerWidth) {
    x = window.innerWidth - menuWidth - 8;
  }
  if (y + menuHeight > window.innerHeight) {
    y = window.innerHeight - menuHeight - 8;
  }

  fileCtxX.value = Math.max(8, x);
  fileCtxY.value = Math.max(8, y);
  fileCtxVisible.value = true;
}

function closeFileContextMenu() {
  fileCtxVisible.value = false;
  fileCtxPath.value = null;
}

function editorButtonClass(option: ExternalEditorOption): string {
  if (option.id === "notepad") {
    return "hover:border-[#94a3b8]/40 hover:bg-[#94a3b8]/10";
  }
  if (option.id === "vscode") {
    return "hover:border-[#3b82f6]/40 hover:bg-[#3b82f6]/10";
  }
  if (option.id === "visualstudio") {
    return "hover:border-[#8b5cf6]/40 hover:bg-[#8b5cf6]/10";
  }
  if (option.id === "androidstudio") {
    return "hover:border-[#22c55e]/40 hover:bg-[#22c55e]/10";
  }
  return "hover:border-[#f97316]/40 hover:bg-[#f97316]/10";
}

async function openSelectedFileWithEditor(editorId: ExternalEditorId) {
  if (!fileCtxPath.value) return;
  const selectedPath = fileCtxPath.value;

  try {
    await invoke("open_file_with_editor", {
      path: props.repoPath,
      filePath: selectedPath,
      editor: editorId,
    });
    toast.success(`Opened ${fileParts(selectedPath).fileName} in ${editorOptionMap[editorId].label}.`, 3500);
  } catch (error) {
    toast.error(`Could not open ${fileParts(selectedPath).fileName}: ${String(error)}`);
  } finally {
    closeFileContextMenu();
  }
}

function handleGlobalKeyDown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    showCommitBuilder.value = false;
    commitAnalyzerTooltipVisible.value = false;
    closeFileContextMenu();
    return;
  }

  if (event.key === "ArrowRight") {
    handleCommitFileArrowRight(event);
    return;
  }

  if (event.key === "ArrowDown") {
    handleCommitFileArrowDown(event);
    return;
  }

  if (event.key === "ArrowUp") {
    handleCommitFileArrowUp(event);
    return;
  }

  if (event.key === "ArrowLeft") {
    handleCommitFileArrowLeft(event);
  }
}

function handleGlobalPointerDown() {
  closeFileContextMenu();
}

onMounted(() => {
  void loadAvailableEditors();
  syncGitkeepNotifyPreference();
  refreshCommitAnalyzerSettings();
  globalThis.addEventListener("pointerdown", handleGlobalPointerDown);
  globalThis.addEventListener("keydown", handleGlobalKeyDown, true);
  globalThis.addEventListener("resize", updateCompactWorkingLabel);
  globalThis.addEventListener("resize", updateCommitFloatingPanels);
  globalThis.addEventListener("scroll", updateCommitFloatingPanels, true);
  globalThis.addEventListener(COMMIT_ANALYZER_SETTINGS_EVENT, handleCommitAnalyzerSettingsChanged as EventListener);
  updateCompactWorkingLabel();
});

watch(
  () => [
    props.repoPath,
    props.isWorkingChanges,
    props.stagedFiles.length,
    props.unstagedFiles.length,
    props.conflictFiles?.length || 0,
  ],
  () => {
    void loadEmptyDirectories();
  },
  { immediate: true },
);

watch(
  () => [
    props.repoPath,
    props.commit?.sha,
    props.isWorkingChanges,
    props.isStash,
    changesViewMode.value,
    showAllFilesInTree.value,
  ],
  () => {
    if (isCommitChangesView.value && changesViewMode.value === "tree" && showAllFilesInTree.value) {
      void loadCommitTreePaths();
      return;
    }

    commitTreeRunToken += 1;
    commitTreePaths.value = [];
    commitTreeError.value = "";
    commitTreeLoading.value = false;
  },
  { immediate: true },
);

watch(
  () => [props.isWorkingChanges, props.isStash, props.commit?.sha],
  () => {
    showAllFilesInTree.value = false;
    changesViewMode.value = "files";
  },
);

watch(
  () => [
    props.repoPath,
    props.isWorkingChanges,
    stagedFingerprint.value,
  ],
  () => {
    void loadStagedDiffSummary();
  },
  { immediate: true },
);

watch(
  () => [
    commitSummary.value,
    commitDescription.value,
    props.isWorkingChanges,
    stagedFingerprint.value,
    stagedDiffSummary.value.total_lines_added,
    stagedDiffSummary.value.total_lines_removed,
    stagedDiffSummary.value.files_changed,
    stagedDiffSummary.value.inferred_scope,
    commitAnalyzerSettings.value.enabled,
    commitAnalyzerSettings.value.severityThreshold,
    commitAnalyzerSettings.value.maxDiffLinesForDescWarning,
  ],
  () => {
    queueCommitLintAnalysis();
  },
  { immediate: true },
);

watch(commitLintIndicator, (indicator) => {
  if (indicator === "none") {
    commitAnalyzerTooltipVisible.value = false;
  }
});

onUnmounted(() => {
  if (commitLintTimer) {
    clearTimeout(commitLintTimer);
    commitLintTimer = null;
  }
  if (copiedShaTimer) {
    clearTimeout(copiedShaTimer);
    copiedShaTimer = null;
  }
  globalThis.removeEventListener("pointerdown", handleGlobalPointerDown);
  globalThis.removeEventListener("keydown", handleGlobalKeyDown, true);
  globalThis.removeEventListener("resize", updateCompactWorkingLabel);
  globalThis.removeEventListener("resize", updateCommitFloatingPanels);
  globalThis.removeEventListener("scroll", updateCommitFloatingPanels, true);
  globalThis.removeEventListener(COMMIT_ANALYZER_SETTINGS_EVENT, handleCommitAnalyzerSettingsChanged as EventListener);
});
</script>

<template>
  <div class="w-full bg-[var(--card)] border-l border-[var(--border)] flex flex-col h-full overflow-visible relative z-[120]">
    <div class="border-b border-[var(--border)] flex-shrink-0 relative">
      <div class="h-9 flex">
        <button
          v-if="!isMultiCommitSelection && ((commit && !isWorkingChanges) || (isStash && selectedStash))"
          @click="setActiveTab('info')"
          :class="[
            'flex-1 text-xs font-medium tracking-wide transition-colors',
            activeTab === 'info'
              ? 'text-[var(--primary)] border-b-2 border-[var(--primary)] bg-[var(--card)]'
              : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]',
          ]"
        >
          Info
        </button>
        <button
          @click="setActiveTab('changes')"
          :class="[
            'flex-1 text-xs font-medium tracking-wide transition-colors',
            activeTab === 'changes'
              ? 'text-[var(--primary)] border-b-2 border-[var(--primary)] bg-[var(--card)]'
              : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]',
          ]"
        >
          Changes
        </button>
      </div>
    </div>

    <div v-show="activeTab === 'changes' && isWorkingChanges" class="flex-1 flex flex-col overflow-hidden">
      <div ref="commitFilesScrollContainer" class="flex-1 overflow-y-auto">
        <div v-if="stagedFiles.length > 0 || unstagedFiles.length > 0 || (conflictFiles?.length || 0) > 0" class="px-3 py-2.5 border-b border-[var(--border)] bg-gradient-to-r from-[var(--primary)]/5 to-transparent">
          <div class="flex items-center justify-between gap-2 mb-1">
            <div class="flex items-center gap-2 min-w-0">
              <div class="w-2 h-2 rounded-full bg-[var(--primary)] animate-pulse" />
              <span class="text-[11px] font-semibold text-[var(--foreground)] whitespace-nowrap">{{ workingChangesLabel }}</span>
            </div>
            <div class="flex items-center gap-1 p-0.5 rounded-md border border-[var(--border)] bg-[var(--input-background)]">
              <button
                class="h-6 px-2 rounded text-[10px] font-medium transition-colors flex items-center gap-1"
                :class="changesViewMode === 'files' ? 'bg-[var(--primary)] text-white' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                @click="setChangesViewMode('files')"
              >
                <Files class="w-3 h-3" />
              </button>
              <button
                class="h-6 px-2 rounded text-[10px] font-medium transition-colors flex items-center gap-1"
                :class="changesViewMode === 'tree' ? 'bg-[var(--primary)] text-white' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                @click="setChangesViewMode('tree')"
              >
                <FolderTree class="w-3 h-3" />
              </button>
              <button
                class="h-6 px-2 rounded text-[10px] font-medium transition-colors flex items-center gap-1"
                :class="changesViewMode === 'map' ? 'bg-[var(--primary)] text-white' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                @click="setChangesViewMode('map')"
              >
                <MapIcon class="w-3 h-3" />
              </button>
            </div>
          </div>
          <div class="flex items-center gap-3 text-[10px] text-[var(--muted-foreground)]">
            <span v-if="hasConflicts && (conflictFiles?.length || 0) > 0" class="flex items-center gap-1 text-[#ef4444]">
              <span class="w-1.5 h-1.5 rounded-full bg-[#ef4444]" />
              {{ conflictFiles?.length || 0 }} unresolved
            </span>
            <span v-if="unstagedFiles.length > 0" class="flex items-center gap-1">
              <span class="w-1.5 h-1.5 rounded-full bg-[#f59e0b]" />
              {{ unstagedFiles.length }} resolved
            </span>
            <span v-if="stagedFiles.length > 0" class="flex items-center gap-1">
              <span class="w-1.5 h-1.5 rounded-full bg-[#10b981]" />
              {{ stagedFiles.length }} staged
            </span>
          </div>
        </div>

        <div
          v-if="isWorkingChanges && notifyGitkeep && (emptyDirectoriesLoading || emptyDirectoriesError || emptyDirectories.length > 0)"
          class="px-3 py-2 border-b border-[var(--border)] bg-[#eab308]/8"
        >
          <div class="flex items-center justify-between gap-2 mb-1">
            <div class="flex items-center gap-1.5 text-[11px] font-semibold text-[#eab308]">
              <AlertTriangle class="w-3.5 h-3.5" />
              Empty Folder Guardian
            </div>
            <button
              v-if="emptyDirectories.length > 1"
              class="text-[10px] px-2 py-0.5 rounded border border-[#eab308]/40 text-[#eab308] hover:bg-[#eab308]/12 transition-colors"
              @click="addGitkeepToAll"
            >
              Add .gitkeep to all
            </button>
          </div>

          <div v-if="emptyDirectoriesLoading" class="text-[10px] text-[#facc15]">Scanning for empty folders...</div>
          <div v-else-if="emptyDirectoriesError" class="text-[10px] text-[#f59e0b]">{{ emptyDirectoriesError }}</div>
          <div v-else class="space-y-1 max-h-[130px] overflow-y-auto pr-1">
            <div
              v-for="directory in emptyDirectories"
              :key="directory"
              class="flex items-center gap-2 px-2 py-1 rounded border border-[#eab308]/20 bg-[#eab308]/6"
            >
              <FolderPlus class="w-3.5 h-3.5 text-[#facc15] flex-shrink-0" />
              <span class="text-[10px] text-[var(--foreground)] truncate flex-1" :title="directory">{{ directory }}</span>
              <button
                class="text-[10px] px-2 py-0.5 rounded border border-[#eab308]/40 text-[#facc15] hover:bg-[#eab308]/14 transition-colors"
                @click="addGitkeep(directory)"
              >
                Add .gitkeep
              </button>
            </div>
          </div>
        </div>

        <template v-if="changesViewMode === 'files'">

        <div v-if="hasConflicts" class="border-b border-[var(--border)]">
          <div class="flex items-center justify-between px-3 py-2 bg-[var(--card)]">
            <span class="text-xs font-medium text-[#ef4444]">Conflicts</span>
            <button
              @click="emit('resolveAllConflicts')"
              class="text-[10px] text-[#ef4444] hover:text-[#f87171] transition-colors px-1.5 py-0.5 rounded hover:bg-[#ef4444]/10"
            >
              Resolve All
            </button>
          </div>
          <div
            v-for="f in (conflictFiles ?? [])"
            :key="'c-' + f.path"
            class="flex items-center gap-2 px-4 py-1.5 hover:bg-[#ef4444]/8 transition-all group cursor-pointer"
            @click="emit('manualResolve', f.path)"
            @contextmenu="openFileContextMenu($event, f.path)"
          >
            <span class="text-[10px] font-bold w-4 text-center text-[#ef4444]">!</span>
            <div class="flex-1 min-w-0">
              <div class="text-xs text-[var(--foreground)] truncate opacity-90">{{ fileParts(f.path).fileName }}</div>
              <div class="text-[10px] text-[var(--muted-foreground)] truncate">{{ fileParts(f.path).directory || '.' }}</div>
            </div>
            <button
              @click.stop="emit('manualResolve', f.path)"
              class="p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[#ef4444]/20 transition-all"
              title="Resolve conflict (line-by-line or simple)"
            >
              <Plus class="w-4 h-4 text-[#ef4444]" />
            </button>
          </div>
        </div>

        <SmartGitignoreWizard
          v-if="props.smartGitignoreWizardEnabled"
          :repo-path="props.repoPath"
          :unstaged-files="props.unstagedFiles"
          @applied="emit('refreshState')"
          @open-gitignore="emit('viewDiff', { path: '.gitignore', sha: null, staged: false })"
        />

        <div v-if="unstagedFiles.length > 0" class="border-b border-[var(--border)]">
          <div class="flex items-center justify-between px-3 py-2 bg-[var(--card)]">
            <button
              @click="expandedUnstaged = !expandedUnstaged"
              class="flex items-center gap-2 text-xs text-[var(--foreground)] hover:text-[var(--primary)] transition-all flex-1"
            >
              <ChevronDown
                :class="['w-3.5 h-3.5 transition-transform', !expandedUnstaged ? '-rotate-90' : '']"
              />
              <span class="font-medium">Unstaged</span>
              <span class="text-[10px] bg-[#f59e0b]/15 text-[#f59e0b] px-1.5 py-0.5 rounded-full font-semibold border border-[#f59e0b]/20">{{ unstagedFiles.length }}</span>
            </button>
            <div class="flex items-center gap-1">
              <button
                @click="confirmDiscard(null)"
                class="text-[10px] text-[#ef4444]/70 hover:text-[#ef4444] transition-colors px-1.5 py-0.5 rounded hover:bg-[#ef4444]/10"
                title="Discard all changes"
              >
                Discard All
              </button>
              <button
                @click="emit('stageAll')"
                class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors px-1.5 py-0.5 rounded hover:bg-[var(--secondary)]"
              >
                Stage All
              </button>
            </div>
          </div>
          <div v-if="expandedUnstaged">
            <div
              v-for="f in unstagedFiles.filter(x => !x.conflicted)"
              :key="'u-' + f.path"
              :class="[
                'flex items-center gap-2 px-4 py-1.5 transition-all group cursor-pointer',
                isVeryLargeFile(f)
                  ? 'bg-[#eab308]/8 hover:bg-[#eab308]/12 border-l-2 border-[#eab308]/45'
                  : 'hover:bg-[var(--primary)]/5',
              ]"
              @click="openDiff(f.path, null, false)"
              @contextmenu="openFileContextMenu($event, f.path)"
            >
              <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
              <div class="flex-1 min-w-0">
                <div class="text-xs text-[var(--foreground)] truncate opacity-90">{{ fileParts(f.path).fileName }}</div>
                <div class="text-[10px] text-[var(--muted-foreground)] truncate">{{ fileParts(f.path).directory || '.' }}</div>
                <div v-if="isVeryLargeFile(f)" class="text-[10px] text-[#eab308] truncate">
                  Size of this file is very large ({{ formatFileSize(f.file_size_bytes || 0) }})
                </div>
              </div>
              <button
                @click.stop="confirmDiscard(f.path)"
                class="p-0.5 rounded hover:bg-[#ef4444]/20 transition-all flex-shrink-0"
                title="Discard changes"
              >
                <Trash2 class="w-3 h-3 text-[#ef4444] hover:text-[#dc2626]" />
              </button>
              <button
                @click.stop="emit('stage', f.path)"
                class="p-0.5 rounded hover:bg-[var(--primary)]/20 transition-all flex-shrink-0"
                title="Stage changes"
              >
                <Plus class="w-3 h-3 text-[var(--primary)] hover:text-[var(--primary)]/80" />
              </button>
            </div>
          </div>
        </div>

        <div v-if="stagedFiles.length > 0" class="border-b border-[var(--border)]">
          <div class="flex items-center justify-between px-3 py-2 bg-[var(--card)]">
            <button
              @click="expandedStaged = !expandedStaged"
              class="flex items-center gap-2 text-xs text-[var(--foreground)] hover:text-[var(--primary)] transition-all flex-1"
            >
              <ChevronDown
                :class="['w-3.5 h-3.5 transition-transform', !expandedStaged ? '-rotate-90' : '']"
              />
              <span class="font-medium">Staged</span>
              <span class="text-[10px] bg-[#10b981]/15 text-[#10b981] px-1.5 py-0.5 rounded-full font-semibold border border-[#10b981]/20">{{ stagedFiles.length }}</span>
            </button>
            <button
              @click="emit('unstageAll')"
              class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors px-1.5 py-0.5 rounded hover:bg-[var(--secondary)]"
            >
              Unstage All
            </button>
          </div>
          <div v-if="expandedStaged">
            <div
              v-for="f in stagedFiles.filter(x => !x.conflicted)"
              :key="'s-' + f.path"
              :class="[
                'flex items-center gap-2 px-4 py-1.5 transition-all group cursor-pointer',
                isVeryLargeFile(f)
                  ? 'bg-[#eab308]/8 hover:bg-[#eab308]/12 border-l-2 border-[#eab308]/45'
                  : 'hover:bg-[var(--primary)]/5',
              ]"
              @click="openDiff(f.path, null, true)"
              @contextmenu="openFileContextMenu($event, f.path)"
            >
              <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
              <div class="flex-1 min-w-0">
                <div class="text-xs text-[var(--foreground)] truncate opacity-90">{{ fileParts(f.path).fileName }}</div>
                <div class="text-[10px] text-[var(--muted-foreground)] truncate">{{ fileParts(f.path).directory || '.' }}</div>
                <div v-if="isVeryLargeFile(f)" class="text-[10px] text-[#eab308] truncate">
                  Size of this file is very large ({{ formatFileSize(f.file_size_bytes || 0) }})
                </div>
              </div>
              <button
                @click.stop="openDiff(f.path, null, true)"
                class="p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[var(--primary)]/20 transition-all"
                title="View diff"
              >
                <Eye class="w-3 h-3 text-[var(--muted-foreground)] hover:text-[var(--primary)]" />
              </button>
              <Minus class="w-3 h-3 text-[var(--muted-foreground)] opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer" @click.stop="emit('unstage', f.path)" />
            </div>
          </div>
        </div>

        <div v-if="stagedFiles.length === 0 && unstagedFiles.length === 0 && !(conflictFiles && conflictFiles.length > 0)" class="px-4 py-12 flex flex-col items-center justify-center">
          <div class="w-14 h-14 rounded-full bg-[var(--card)] flex items-center justify-center mb-3 border border-[var(--border)]">
            <FileText class="w-6 h-6 text-[var(--muted-foreground)] opacity-40" />
          </div>
          <p class="text-xs text-[var(--muted-foreground)] font-medium">Working tree clean</p>
          <p class="text-[10px] text-[var(--muted-foreground)] opacity-60 mt-1">No uncommitted changes</p>
        </div>

        </template>

        <template v-else-if="changesViewMode === 'tree'">
          <div class="border-b border-[var(--border)]">
            <div class="flex items-center justify-between gap-2 px-3 py-2 bg-[var(--card)]">
              <span class="text-xs font-medium text-[var(--foreground)]">Changes tree</span>
              <div class="flex items-center gap-1">
                <button
                  v-if="hasConflicts"
                  class="text-[10px] text-[#ef4444] hover:text-[#f87171] transition-colors px-1.5 py-0.5 rounded hover:bg-[#ef4444]/10"
                  @click="emit('resolveAllConflicts')"
                >
                  Resolve All
                </button>
                <button
                  v-if="unstagedFiles.length > 0"
                  class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors px-1.5 py-0.5 rounded hover:bg-[var(--secondary)]"
                  @click="emit('stageAll')"
                >
                  Stage All
                </button>
                <button
                  v-if="stagedFiles.length > 0"
                  class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors px-1.5 py-0.5 rounded hover:bg-[var(--secondary)]"
                  @click="emit('unstageAll')"
                >
                  Unstage All
                </button>
                <button
                  v-if="unstagedFiles.length > 0"
                  class="text-[10px] text-[#ef4444]/70 hover:text-[#ef4444] transition-colors px-1.5 py-0.5 rounded hover:bg-[#ef4444]/10"
                  @click="confirmDiscard(null)"
                >
                  Discard All
                </button>
              </div>
            </div>

            <div v-if="treeRows.length === 0" class="px-4 py-8 text-[11px] text-[var(--muted-foreground)] text-center">
              No files to show in tree mode.
            </div>

            <div
              v-for="row in treeRows"
              :key="row.node.key"
              class="group flex items-center gap-2 py-1.5 pr-3 transition-colors"
              :class="[
                row.node.type === 'folder' ? 'hover:bg-[var(--secondary)]/45 cursor-pointer' : 'cursor-pointer hover:bg-[var(--primary)]/6',
                row.node.changed ? '' : 'opacity-70',
              ]"
              :style="{ paddingLeft: `${10 + row.depth * 14}px` }"
              @click="onTreeRowClick(row)"
              @contextmenu="onTreeRowContextMenu($event, row)"
            >
              <button
                v-if="row.node.type === 'folder'"
                class="w-3.5 h-3.5 flex items-center justify-center rounded hover:bg-[var(--secondary)]"
                @click.stop="toggleTreeFolder(row.node.path)"
              >
                <ChevronDown class="w-3 h-3 transition-transform" :class="isTreeFolderExpanded(row.node.path) ? '' : '-rotate-90'" />
              </button>
              <span v-else class="w-3.5" />

              <Folder
                v-if="row.node.type === 'folder'"
                class="w-3.5 h-3.5 flex-shrink-0"
                :class="row.node.changed ? 'text-[var(--primary)]' : 'text-[var(--muted-foreground)]/70'"
              />
              <File
                v-else
                class="w-3.5 h-3.5 flex-shrink-0"
                :style="{ color: row.node.status ? statusColor(row.node.status) : 'var(--muted-foreground)' }"
              />

              <span
                class="flex-1 min-w-0 text-xs truncate"
                :class="row.node.changed ? 'text-[var(--foreground)]' : 'text-[var(--muted-foreground)]'"
                :title="row.node.path"
              >
                {{ row.node.name }}
              </span>

              <span
                v-if="row.node.type === 'folder'"
                class="w-1.5 h-1.5 rounded-full"
                :class="row.node.changed ? 'bg-[var(--primary)]' : 'bg-[var(--muted-foreground)]/35'"
              />

              <span
                v-else-if="row.node.status"
                class="text-[10px] font-bold w-4 text-center"
                :style="{ color: statusColor(row.node.status) }"
              >
                {{ statusIcon(row.node.status) }}
              </span>
            </div>
          </div>
        </template>

        <template v-else>
          <div class="border-b border-[var(--border)]">
            <div class="flex items-center justify-between gap-2 px-3 py-2 bg-[var(--card)]">
              <div class="min-w-0">
                <div class="text-xs font-medium text-[var(--foreground)]">File Change Map</div>
                <div class="text-[10px] text-[var(--muted-foreground)] truncate">
                  {{ selectedChangeMapLabel }} · {{ filteredChangeMapFiles.length }} file{{ filteredChangeMapFiles.length === 1 ? "" : "s" }}
                </div>
              </div>
              <button
                v-if="selectedChangeMapFolder"
                class="text-[10px] px-2 py-0.5 rounded border border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]"
                @click="setChangeMapFolder(null)"
              >
                Clear
              </button>
            </div>

            <div v-if="changeMapFolders.length === 0" class="px-4 py-8 text-[11px] text-[var(--muted-foreground)] text-center">
              No folders to map.
            </div>

            <div v-else class="px-3 py-2 space-y-1.5">
              <button
                v-for="folder in changeMapFolders"
                :key="folder.path"
                class="w-full rounded border px-2 py-1.5 text-left transition-colors"
                :class="selectedChangeMapFolder === folder.path
                  ? 'border-[var(--primary)]/55 bg-[var(--primary)]/12'
                  : 'border-[var(--border)] bg-[var(--input-background)] hover:bg-[var(--secondary)]/55'"
                @click="setChangeMapFolder(folder.path)"
              >
                <div class="flex items-center gap-2">
                  <Folder class="w-3.5 h-3.5 flex-shrink-0 text-[var(--primary)]" />
                  <span
                    class="min-w-0 flex-1 truncate text-xs font-medium text-[var(--foreground)]"
                    :style="{ paddingLeft: `${folder.depth * 10}px` }"
                    :title="folder.path"
                  >
                    {{ folder.label }}
                  </span>
                  <span class="text-[10px] text-[var(--muted-foreground)]">{{ folder.fileCount }} file{{ folder.fileCount === 1 ? "" : "s" }}</span>
                </div>
                <div class="mt-1 flex h-1.5 overflow-hidden rounded bg-[var(--secondary)]">
                  <div class="bg-[#ef4444]" :style="{ width: changeMapSegmentWidth(folder.conflictCount, folder.fileCount) }" />
                  <div class="bg-[#f59e0b]" :style="{ width: changeMapSegmentWidth(folder.unstagedCount, folder.fileCount) }" />
                  <div class="bg-[#10b981]" :style="{ width: changeMapSegmentWidth(folder.stagedCount, folder.fileCount) }" />
                </div>
                <div class="mt-1 h-1 rounded bg-[var(--secondary)]">
                  <div class="h-full rounded bg-[var(--primary)]/55" :style="{ width: changeMapFolderWidth(folder) }" />
                </div>
              </button>
            </div>

            <div class="border-t border-[var(--border)]">
              <div
                v-for="file in filteredChangeMapFiles"
                :key="'map-working-' + file.path"
                class="group flex cursor-pointer items-center gap-2 px-4 py-1.5 transition-colors hover:bg-[var(--primary)]/6"
                @click="openChangeMapFile(file)"
                @contextmenu="openFileContextMenu($event, file.path)"
              >
                <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(file.status) }">{{ statusIcon(file.status) }}</span>
                <div class="min-w-0 flex-1">
                  <div class="truncate text-xs text-[var(--foreground)]">{{ fileParts(file.path).fileName }}</div>
                  <div class="truncate text-[10px] text-[var(--muted-foreground)]">{{ fileParts(file.path).directory || '.' }}</div>
                </div>
                <span v-if="file.conflicted" class="text-[10px] text-[#ef4444]">conflict</span>
                <span v-else-if="file.unstaged" class="text-[10px] text-[#f59e0b]">unstaged</span>
                <span v-else-if="file.staged" class="text-[10px] text-[#10b981]">staged</span>
              </div>
            </div>
          </div>
        </template>
      </div>

      <div class="border-t border-[var(--border)] p-3 bg-[var(--card)]/50 flex-shrink-0">
        <div class="mb-2 relative">
          <div class="flex items-center justify-between mb-1 gap-2">
            <div class="text-[10px] font-medium text-[var(--muted-foreground)]">Subject</div>
            <div class="flex items-center gap-2">
              <button
                ref="commitBuilderButtonRef"
                type="button"
                class="inline-flex h-5 w-5 items-center justify-center rounded border border-[var(--border)] bg-[var(--input-background)] text-[var(--muted-foreground)] transition-colors hover:border-[var(--primary)]/45 hover:text-[var(--primary)]"
                title="Visual commit builder"
                @click="openCommitBuilder"
              >
                <Hammer class="h-3 w-3" />
              </button>
              <span :class="[
                'text-[10px] font-medium',
                commitSummary.length > 72 ? 'text-[#ef4444]' : 'text-[var(--muted-foreground)]'
              ]">
                {{ commitSummary.length > 72 ? '-' + (commitSummary.length - 72) : (72 - commitSummary.length) }}
              </span>

              <div
                v-if="commitAnalyzerSettings.enabled && commitLintIndicator !== 'none'"
                class="relative"
              >
                <span
                  ref="commitAnalyzerIndicatorRef"
                  :class="[
                    'inline-flex items-center justify-center w-4 h-4 rounded border text-[10px] font-bold cursor-help',
                    commitLintIndicatorClass,
                  ]"
                  @mouseenter="showCommitAnalyzerTooltip"
                  @mouseleave="hideCommitAnalyzerTooltip"
                  @focus="showCommitAnalyzerTooltip"
                  @blur="hideCommitAnalyzerTooltip"
                  tabindex="0"
                >
                  {{ commitLintIndicator === 'error' ? 'X' : '!' }}
                </span>
              </div>
            </div>
          </div>
          <input
            v-model="commitSummary"
            type="text"
            placeholder="Commit message..."
            class="w-full px-3 py-2 bg-[var(--input-background)] border rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 transition-colors"
            :class="[
              'border ',
              commitSummary.length > 72 
                ? 'border-[#ef4444] bg-[#ef4444]/5' 
                : 'border-[var(--border)]'
            ]"
            @keyup.enter="onCommit"
          />
        </div>
        <textarea
          v-model="commitDescription"
          placeholder="Description (optional)..."
          rows="3"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 resize-none mb-2 overflow-y-auto"
          @input="onDescriptionInput"
        />

        <AppButton
          class="w-full bg-[var(--primary)] hover:opacity-90 text-white text-xs font-medium h-8"
          :disabled="!commitSummary.trim() || stagedFiles.length === 0"
          @click="onCommit"
        >
          Commit Changes
        </AppButton>
      </div>
    </div>

    <div v-show="activeTab === 'changes' && !isWorkingChanges && (commit || isMultiCommitSelection)" class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 overflow-y-auto">
        <div class="flex items-center justify-between gap-2 px-3 py-2 bg-[var(--card)] text-xs text-[var(--foreground)] border-b border-[var(--border)]">
          <div class="flex items-center gap-2 min-w-0">
            <span class="font-medium">{{ isMultiCommitSelection ? 'Merged changed files' : 'Changed files' }}</span>
            <span class="text-[10px] bg-[var(--primary)]/20 text-[var(--primary)] px-1.5 py-0.5 rounded-full">{{ commitFiles.length }}</span>
          </div>

          <div class="flex items-center gap-2">
            <label
              v-if="changesViewMode === 'tree'"
              class="flex items-center gap-1 text-[10px] text-[var(--muted-foreground)] cursor-pointer select-none"
            >
              <input
                type="checkbox"
                class="w-3.5 h-3.5 rounded border-[var(--border)] bg-[var(--input-background)]"
                v-model="showAllFilesInTree"
              >
              View all
            </label>

            <div class="flex items-center gap-1 p-0.5 rounded-md border border-[var(--border)] bg-[var(--input-background)]">
              <button
                class="h-6 px-2 rounded text-[10px] font-medium transition-colors flex items-center gap-1"
                :class="changesViewMode === 'files' ? 'bg-[var(--primary)] text-white' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                @click="setChangesViewMode('files')"
              >
                <Files class="w-3 h-3" />
              </button>
              <button
                class="h-6 px-2 rounded text-[10px] font-medium transition-colors flex items-center gap-1"
                :class="changesViewMode === 'tree' ? 'bg-[var(--primary)] text-white' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                @click="setChangesViewMode('tree')"
              >
                <FolderTree class="w-3 h-3" />
              </button>
              <button
                class="h-6 px-2 rounded text-[10px] font-medium transition-colors flex items-center gap-1"
                :class="changesViewMode === 'map' ? 'bg-[var(--primary)] text-white' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                @click="setChangesViewMode('map')"
              >
                <MapIcon class="w-3 h-3" />
              </button>
            </div>
          </div>
        </div>

        <template v-if="changesViewMode === 'files'">
          <div v-if="commitFiles.length > 0">
            <div
              v-for="f in commitFiles"
              :key="f.path"
              :data-commit-file-path="f.path"
              class="flex items-center gap-2 px-4 py-1.5 hover:bg-[var(--primary)]/5 transition-all cursor-pointer group"
              :class="selectedChangePath === f.path ? 'bg-[var(--primary)]/10' : ''"
              @click="openDiff(f.path, isMultiCommitSelection ? (f.commit_shas?.[0] || primaryCommitShaForDiff) : commit!.sha, false)"
              @contextmenu="openFileContextMenu($event, f.path)"
            >
              <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
              <div class="flex-1 min-w-0">
                <div class="text-xs text-[var(--foreground)] truncate opacity-90">{{ fileParts(f.path).fileName }}</div>
                <div class="text-[10px] text-[var(--muted-foreground)] truncate">{{ fileParts(f.path).directory || '.' }}</div>
                <div v-if="isMultiCommitSelection && f.commit_shas && f.commit_shas.length > 0" class="text-[10px] text-[var(--muted-foreground)] truncate">
                  commits: {{ formatCommitHashes(f.commit_shas) }}
                </div>
              </div>
              <Eye class="w-3 h-3 text-[var(--muted-foreground)] opacity-0 group-hover:opacity-100 transition-opacity" />
              <span v-if="f.additions > 0" class="text-[10px] text-[#10b981] font-mono">+{{ f.additions }}</span>
              <span v-if="f.deletions > 0" class="text-[10px] text-[#ef4444] font-mono">-{{ f.deletions }}</span>
            </div>
          </div>
          <div v-else class="px-4 py-12 flex flex-col items-center justify-center">
            <div class="w-12 h-12 rounded-full bg-[var(--card)] flex items-center justify-center mb-3 border border-[var(--border)]">
              <FileText class="w-6 h-6 text-[var(--muted-foreground)] opacity-40" />
            </div>
            <p class="text-xs text-[var(--muted-foreground)]">No file changes</p>
          </div>
        </template>

        <template v-else-if="changesViewMode === 'tree'">
          <div v-if="showAllFilesInTree && commitTreeLoading" class="px-3 py-2 text-[10px] text-[var(--muted-foreground)] border-b border-[var(--border)]">
            Loading full commit tree...
          </div>
          <div v-else-if="showAllFilesInTree && commitTreeError" class="px-3 py-2 text-[10px] text-[#f59e0b] border-b border-[var(--border)]">
            {{ commitTreeError }}
          </div>

          <div v-if="treeRows.length > 0">
            <div
              v-for="row in treeRows"
              :key="row.node.key"
              class="group flex items-center gap-2 py-1.5 pr-3 transition-colors"
              :class="[
                row.node.type === 'folder' ? 'hover:bg-[var(--secondary)]/45 cursor-pointer' : 'hover:bg-[var(--primary)]/6',
                row.node.changed ? '' : 'opacity-70',
              ]"
              :style="{ paddingLeft: `${10 + row.depth * 14}px` }"
              @click="onTreeRowClick(row)"
              @contextmenu="onTreeRowContextMenu($event, row)"
            >
              <button
                v-if="row.node.type === 'folder'"
                class="w-3.5 h-3.5 flex items-center justify-center rounded hover:bg-[var(--secondary)]"
                @click.stop="toggleTreeFolder(row.node.path)"
              >
                <ChevronDown class="w-3 h-3 transition-transform" :class="isTreeFolderExpanded(row.node.path) ? '' : '-rotate-90'" />
              </button>
              <span v-else class="w-3.5" />

              <Folder
                v-if="row.node.type === 'folder'"
                class="w-3.5 h-3.5 flex-shrink-0"
                :class="row.node.changed ? 'text-[var(--primary)]' : 'text-[var(--muted-foreground)]/70'"
              />
              <File
                v-else
                class="w-3.5 h-3.5 flex-shrink-0"
                :style="{ color: row.node.status ? statusColor(row.node.status) : 'var(--muted-foreground)' }"
              />

              <span
                class="flex-1 min-w-0 text-xs truncate"
                :class="row.node.changed ? 'text-[var(--foreground)]' : 'text-[var(--muted-foreground)]'"
                :title="row.node.path"
              >
                {{ row.node.name }}
              </span>

              <span
                v-if="row.node.type === 'folder'"
                class="w-1.5 h-1.5 rounded-full"
                :class="row.node.changed ? 'bg-[var(--primary)]' : 'bg-[var(--muted-foreground)]/35'"
              />

              <span
                v-else-if="row.node.status"
                class="text-[10px] font-bold w-4 text-center"
                :style="{ color: statusColor(row.node.status) }"
              >
                {{ statusIcon(row.node.status) }}
              </span>
            </div>
          </div>
          <div v-else class="px-4 py-12 flex flex-col items-center justify-center">
            <div class="w-12 h-12 rounded-full bg-[var(--card)] flex items-center justify-center mb-3 border border-[var(--border)]">
              <FileText class="w-6 h-6 text-[var(--muted-foreground)] opacity-40" />
            </div>
            <p class="text-xs text-[var(--muted-foreground)]">No file changes</p>
          </div>
        </template>

        <template v-else>
          <div class="border-b border-[var(--border)]">
            <div class="flex items-center justify-between gap-2 px-3 py-2 bg-[var(--card)]">
              <div class="min-w-0">
                <div class="text-xs font-medium text-[var(--foreground)]">File Change Map</div>
                <div class="text-[10px] text-[var(--muted-foreground)] truncate">
                  {{ selectedChangeMapLabel }} · {{ filteredChangeMapFiles.length }} file{{ filteredChangeMapFiles.length === 1 ? "" : "s" }}
                </div>
              </div>
              <button
                v-if="selectedChangeMapFolder"
                class="text-[10px] px-2 py-0.5 rounded border border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]"
                @click="setChangeMapFolder(null)"
              >
                Clear
              </button>
            </div>

            <div v-if="changeMapFolders.length === 0" class="px-4 py-8 text-[11px] text-[var(--muted-foreground)] text-center">
              No folders to map.
            </div>

            <div v-else class="px-3 py-2 space-y-1.5">
              <button
                v-for="folder in changeMapFolders"
                :key="folder.path"
                class="w-full rounded border px-2 py-1.5 text-left transition-colors"
                :class="selectedChangeMapFolder === folder.path
                  ? 'border-[var(--primary)]/55 bg-[var(--primary)]/12'
                  : 'border-[var(--border)] bg-[var(--input-background)] hover:bg-[var(--secondary)]/55'"
                @click="setChangeMapFolder(folder.path)"
              >
                <div class="flex items-center gap-2">
                  <Folder class="w-3.5 h-3.5 flex-shrink-0 text-[var(--primary)]" />
                  <span
                    class="min-w-0 flex-1 truncate text-xs font-medium text-[var(--foreground)]"
                    :style="{ paddingLeft: `${folder.depth * 10}px` }"
                    :title="folder.path"
                  >
                    {{ folder.label }}
                  </span>
                  <span class="text-[10px] text-[var(--muted-foreground)]">{{ folder.fileCount }} file{{ folder.fileCount === 1 ? "" : "s" }}</span>
                </div>
                <div class="mt-1 h-1 rounded bg-[var(--secondary)]">
                  <div class="h-full rounded bg-[var(--primary)]/55" :style="{ width: changeMapFolderWidth(folder) }" />
                </div>
                <div v-if="folder.additions || folder.deletions" class="mt-1 flex gap-2 text-[10px] font-mono">
                  <span v-if="folder.additions" class="text-[#10b981]">+{{ folder.additions }}</span>
                  <span v-if="folder.deletions" class="text-[#ef4444]">-{{ folder.deletions }}</span>
                </div>
              </button>
            </div>

            <div class="border-t border-[var(--border)]">
              <div
                v-for="file in filteredChangeMapFiles"
                :key="'map-commit-' + file.path"
                class="group flex cursor-pointer items-center gap-2 px-4 py-1.5 transition-colors hover:bg-[var(--primary)]/6"
                :class="selectedChangePath === file.path ? 'bg-[var(--primary)]/10' : ''"
                @click="openChangeMapFile(file)"
                @contextmenu="openFileContextMenu($event, file.path)"
              >
                <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(file.status) }">{{ statusIcon(file.status) }}</span>
                <div class="min-w-0 flex-1">
                  <div class="truncate text-xs text-[var(--foreground)]">{{ fileParts(file.path).fileName }}</div>
                  <div class="truncate text-[10px] text-[var(--muted-foreground)]">{{ fileParts(file.path).directory || '.' }}</div>
                </div>
                <span v-if="file.additions > 0" class="text-[10px] text-[#10b981] font-mono">+{{ file.additions }}</span>
                <span v-if="file.deletions > 0" class="text-[10px] text-[#ef4444] font-mono">-{{ file.deletions }}</span>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>

    <div v-show="activeTab === 'info' && commit && !isWorkingChanges && !isMultiCommitSelection" class="flex-1 overflow-y-auto">
      <div v-if="commit" class="p-4 space-y-4">
        <div>
          <div class="flex items-center justify-between gap-2 mb-1.5">
            <div class="text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider">Commit Message</div>
            <span
              v-if="showAmendHint && !canAmendSelectedCommit()"
              class="text-[9px] text-[var(--muted-foreground)]"
            >
              Amend is available only for current HEAD commit
            </span>
          </div>

          <div
            v-if="!editingSubject"
            class="text-sm text-[var(--foreground)] leading-relaxed bg-[var(--input-background)] p-3 rounded border border-[var(--border)]"
            :class="canAmendSelectedCommit() ? 'cursor-text hover:border-[var(--primary)]/45 transition-colors' : ''"
            @click="beginSubjectEdit"
          >
            {{ subjectDraft || commitSubject(commit.message) }}
          </div>

          <div v-else class="space-y-2">
            <textarea
              v-model="subjectDraft"
              rows="2"
              class="w-full px-3 py-2 text-sm text-[var(--foreground)] bg-[var(--input-background)] rounded border border-[var(--primary)]/55 focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 resize-y"
              @keydown.enter.ctrl.prevent="submitSubjectEdit"
            />
            <div class="flex items-center gap-2">
              <button
                class="px-3 py-1.5 rounded text-[11px] font-medium bg-[var(--primary)] text-white hover:opacity-90 transition-colors"
                @click="submitSubjectEdit"
              >
                Update Message
              </button>
              <button
                class="px-3 py-1.5 rounded text-[11px] font-medium border border-[var(--border)] bg-[var(--secondary)] text-[var(--foreground)] hover:opacity-85 transition-colors"
                @click="cancelSubjectEdit"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>

        <div>
          <div class="text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">Description</div>

          <div
            v-if="!editingDescription"
            class="text-xs text-[var(--foreground)] opacity-80 leading-relaxed bg-[var(--input-background)] p-3 rounded border border-[var(--border)] whitespace-pre-wrap"
            :class="canAmendSelectedCommit() ? 'cursor-text hover:border-[var(--primary)]/45 transition-colors' : ''"
            @click="beginDescriptionEdit"
          >
            {{ descriptionDraft || "Click to add a description" }}
          </div>

          <div v-else class="space-y-2">
            <textarea
              v-model="descriptionDraft"
              rows="4"
              class="w-full px-3 py-2 text-xs text-[var(--foreground)] bg-[var(--input-background)] rounded border border-[var(--primary)]/55 focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 resize-y"
              @keydown.enter.ctrl.prevent="submitDescriptionEdit"
            />
            <div class="flex items-center gap-2">
              <button
                class="px-3 py-1.5 rounded text-[11px] font-medium bg-[var(--primary)] text-white hover:opacity-90 transition-colors"
                @click="submitDescriptionEdit"
              >
                Update Message
              </button>
              <button
                class="px-3 py-1.5 rounded text-[11px] font-medium border border-[var(--border)] bg-[var(--secondary)] text-[var(--foreground)] hover:opacity-85 transition-colors"
                @click="cancelDescriptionEdit"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>

        <div class="h-px bg-[var(--border)]" />

        <div class="space-y-2.5">
          <div class="flex items-center gap-3">
            <div class="w-8 h-8 rounded-full bg-gradient-to-br from-[var(--primary)] to-[var(--accent)] flex items-center justify-center text-white text-xs font-bold flex-shrink-0">
              {{ displayIdentityInitial(commit.author_name) }}
            </div>
            <div>
              <div class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Author</div>
              <div class="text-sm text-[var(--foreground)] font-medium">{{ commit.author_name }}</div>
              <div class="text-[10px] text-[var(--muted-foreground)]">{{ commit.author_email }}</div>
            </div>
          </div>

          <div v-if="shouldShowCommitter(commit)" class="flex items-center gap-3">
            <div class="w-8 h-8 rounded-full bg-gradient-to-br from-[var(--chart-2)] to-[var(--primary)] flex items-center justify-center text-white text-xs font-bold flex-shrink-0">
              {{ displayIdentityInitial(commit.committer_name) }}
            </div>
            <div>
              <div class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Committer</div>
              <div class="text-sm text-[var(--foreground)] font-medium">{{ commit.committer_name || 'Unknown' }}</div>
              <div class="text-[10px] text-[var(--muted-foreground)]">{{ commit.committer_email || '—' }}</div>
            </div>
          </div>
        </div>

        <div class="h-px bg-[var(--border)]" />

        <div>
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <Hash class="w-3 h-3" />
            SHA
          </div>
          <div class="flex items-center gap-2">
            <code class="text-xs text-[var(--primary)] bg-[var(--input-background)] px-2 py-1 rounded font-mono break-all">{{ commit.sha }}</code>
            <button
              @click="copyToClipboard(commit.sha, 'commit-sha')"
              class="p-1 rounded hover:bg-[var(--secondary)] transition-colors flex-shrink-0"
              title="Copy SHA"
            >
              <Check v-if="copiedShaKey === 'commit-sha'" class="w-3 h-3 text-[#10b981] animate-pulse" />
              <Copy v-else class="w-3 h-3 text-[var(--muted-foreground)]" />
            </button>
          </div>
        </div>

        <div>
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <Calendar class="w-3 h-3" />
            Date
          </div>
          <div class="text-xs text-[var(--foreground)]">{{ formatDate(commit.timestamp) }}</div>
          <div class="text-[10px] text-[var(--muted-foreground)] mt-0.5">{{ commit.time_ago }}</div>
        </div>

        <div v-if="commit.parent_shas.length > 0">
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <GitCommitIcon class="w-3 h-3" />
            {{ commit.parent_shas.length > 1 ? 'Parents' : 'Parent' }}
          </div>
          <div class="flex flex-wrap gap-1.5">
            <code
              v-for="p in parentRefs(commit)"
              :key="p"
              class="text-[11px] text-[var(--primary)] bg-[var(--input-background)] px-2 py-0.5 rounded font-mono cursor-pointer hover:bg-[var(--secondary)] transition-colors"
            >{{ p }}</code>
          </div>
        </div>

        <div v-if="branchRefs(commit).length > 0">
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <GitBranch class="w-3 h-3" />
            Branches
          </div>
          <div class="flex flex-wrap gap-1.5">
            <span
              v-for="r in branchRefs(commit)"
              :key="r"
              class="px-2 py-0.5 text-[10px] font-medium rounded bg-[var(--primary)]/15 text-[var(--primary)] border border-[var(--primary)]/20"
            >
              {{ r }}
            </span>
          </div>
          <div class="mt-2 flex items-center gap-2 text-[10px]">
            <span v-if="refStatus(commit).local && refStatus(commit).remote" class="flex items-center gap-1 text-[#10b981]">
              <ArrowUp class="w-3 h-3" /><ArrowDown class="w-3 h-3" /> Local & Remote
            </span>
            <span v-else-if="refStatus(commit).local" class="flex items-center gap-1 text-[#f59e0b]">
              <ArrowUp class="w-3 h-3" /> Local only
            </span>
            <span v-else-if="refStatus(commit).remote" class="flex items-center gap-1 text-[#06b6d4]">
              <ArrowDown class="w-3 h-3" /> Remote only
            </span>
          </div>
        </div>
      </div>
    </div>

    <div v-show="activeTab === 'changes' && isStash && selectedStash" class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 overflow-y-auto">
        <div v-if="selectedStash" class="px-3 py-2.5 border-b border-[var(--border)] bg-gradient-to-r from-[#f59e0b]/10 to-transparent">
          <div class="flex items-center gap-2 mb-1">
            <Archive class="w-4 h-4 text-[#f59e0b]" />
            <span class="text-[11px] font-semibold text-[var(--foreground)]">stash@{{ '{' + selectedStash.index + '}' }}</span>
          </div>
          <div class="text-[10px] text-[var(--muted-foreground)]">{{ selectedStash.message || 'No message' }}</div>
        </div>

        <div v-if="stashFiles && stashFiles.length > 0">
          <div class="flex items-center justify-between gap-2 px-3 py-2 bg-[var(--card)] text-xs text-[var(--foreground)] border-b border-[var(--border)]">
            <div class="flex items-center gap-2 min-w-0">
              <span class="font-medium">Stashed files</span>
              <span class="text-[10px] bg-[#f59e0b]/20 text-[#f59e0b] px-1.5 py-0.5 rounded-full">{{ stashFiles.length }}</span>
            </div>
            <div class="flex items-center gap-1 p-0.5 rounded-md border border-[var(--border)] bg-[var(--input-background)]">
              <button
                class="h-6 px-2 rounded text-[10px] font-medium transition-colors flex items-center gap-1"
                :class="changesViewMode === 'files' ? 'bg-[var(--primary)] text-white' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                @click="setChangesViewMode('files')"
              >
                <Files class="w-3 h-3" />
              </button>
              <button
                class="h-6 px-2 rounded text-[10px] font-medium transition-colors flex items-center gap-1"
                :class="changesViewMode === 'tree' ? 'bg-[var(--primary)] text-white' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                @click="setChangesViewMode('tree')"
              >
                <FolderTree class="w-3 h-3" />
              </button>
              <button
                class="h-6 px-2 rounded text-[10px] font-medium transition-colors flex items-center gap-1"
                :class="changesViewMode === 'map' ? 'bg-[var(--primary)] text-white' : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'"
                @click="setChangesViewMode('map')"
              >
                <MapIcon class="w-3 h-3" />
              </button>
            </div>
          </div>

          <template v-if="changesViewMode === 'files'">
            <div
              v-for="f in stashFiles"
              :key="f.path"
              class="flex items-center gap-2 px-4 py-1.5 hover:bg-[#f59e0b]/5 transition-all cursor-pointer"
              @contextmenu="openFileContextMenu($event, f.path)"
            >
              <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(f.status) }">{{ statusIcon(f.status) }}</span>
              <div class="flex-1 min-w-0">
                <div class="text-xs text-[var(--foreground)] truncate opacity-90">{{ fileParts(f.path).fileName }}</div>
                <div class="text-[10px] text-[var(--muted-foreground)] truncate">{{ fileParts(f.path).directory || '.' }}</div>
              </div>
              <span v-if="f.additions > 0" class="text-[10px] text-[#10b981] font-mono">+{{ f.additions }}</span>
              <span v-if="f.deletions > 0" class="text-[10px] text-[#ef4444] font-mono">-{{ f.deletions }}</span>
            </div>
          </template>

          <template v-else-if="changesViewMode === 'tree'">
            <div
              v-for="row in treeRows"
              :key="row.node.key"
              class="group flex items-center gap-2 py-1.5 pr-3 transition-colors"
              :class="[
                row.node.type === 'folder' ? 'hover:bg-[var(--secondary)]/45 cursor-pointer' : 'hover:bg-[#f59e0b]/8',
                row.node.changed ? '' : 'opacity-70',
              ]"
              :style="{ paddingLeft: `${10 + row.depth * 14}px` }"
              @click="onTreeRowClick(row)"
              @contextmenu="onTreeRowContextMenu($event, row)"
            >
              <button
                v-if="row.node.type === 'folder'"
                class="w-3.5 h-3.5 flex items-center justify-center rounded hover:bg-[var(--secondary)]"
                @click.stop="toggleTreeFolder(row.node.path)"
              >
                <ChevronDown class="w-3 h-3 transition-transform" :class="isTreeFolderExpanded(row.node.path) ? '' : '-rotate-90'" />
              </button>
              <span v-else class="w-3.5" />

              <Folder
                v-if="row.node.type === 'folder'"
                class="w-3.5 h-3.5 flex-shrink-0"
                :class="row.node.changed ? 'text-[#f59e0b]' : 'text-[var(--muted-foreground)]/70'"
              />
              <File
                v-else
                class="w-3.5 h-3.5 flex-shrink-0"
                :style="{ color: row.node.status ? statusColor(row.node.status) : 'var(--muted-foreground)' }"
              />

              <span
                class="flex-1 min-w-0 text-xs truncate"
                :class="row.node.changed ? 'text-[var(--foreground)]' : 'text-[var(--muted-foreground)]'"
                :title="row.node.path"
              >
                {{ row.node.name }}
              </span>

              <span
                v-if="row.node.type === 'folder'"
                class="w-1.5 h-1.5 rounded-full"
                :class="row.node.changed ? 'bg-[#f59e0b]' : 'bg-[var(--muted-foreground)]/35'"
              />

              <span
                v-else-if="row.node.status"
                class="text-[10px] font-bold w-4 text-center"
                :style="{ color: statusColor(row.node.status) }"
              >
                {{ statusIcon(row.node.status) }}
              </span>
            </div>
          </template>

          <template v-else>
            <div class="border-b border-[var(--border)]">
              <div class="flex items-center justify-between gap-2 px-3 py-2 bg-[var(--card)]">
                <div class="min-w-0">
                  <div class="text-xs font-medium text-[var(--foreground)]">File Change Map</div>
                  <div class="text-[10px] text-[var(--muted-foreground)] truncate">
                    {{ selectedChangeMapLabel }} · {{ filteredChangeMapFiles.length }} file{{ filteredChangeMapFiles.length === 1 ? "" : "s" }}
                  </div>
                </div>
                <button
                  v-if="selectedChangeMapFolder"
                  class="text-[10px] px-2 py-0.5 rounded border border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]"
                  @click="setChangeMapFolder(null)"
                >
                  Clear
                </button>
              </div>

              <div v-if="changeMapFolders.length === 0" class="px-4 py-8 text-[11px] text-[var(--muted-foreground)] text-center">
                No folders to map.
              </div>

              <div v-else class="px-3 py-2 space-y-1.5">
                <button
                  v-for="folder in changeMapFolders"
                  :key="folder.path"
                  class="w-full rounded border px-2 py-1.5 text-left transition-colors"
                  :class="selectedChangeMapFolder === folder.path
                    ? 'border-[#f59e0b]/55 bg-[#f59e0b]/12'
                    : 'border-[var(--border)] bg-[var(--input-background)] hover:bg-[var(--secondary)]/55'"
                  @click="setChangeMapFolder(folder.path)"
                >
                  <div class="flex items-center gap-2">
                    <Folder class="w-3.5 h-3.5 flex-shrink-0 text-[#f59e0b]" />
                    <span
                      class="min-w-0 flex-1 truncate text-xs font-medium text-[var(--foreground)]"
                      :style="{ paddingLeft: `${folder.depth * 10}px` }"
                      :title="folder.path"
                    >
                      {{ folder.label }}
                    </span>
                    <span class="text-[10px] text-[var(--muted-foreground)]">{{ folder.fileCount }} file{{ folder.fileCount === 1 ? "" : "s" }}</span>
                  </div>
                  <div class="mt-1 h-1 rounded bg-[var(--secondary)]">
                    <div class="h-full rounded bg-[#f59e0b]/65" :style="{ width: changeMapFolderWidth(folder) }" />
                  </div>
                  <div v-if="folder.additions || folder.deletions" class="mt-1 flex gap-2 text-[10px] font-mono">
                    <span v-if="folder.additions" class="text-[#10b981]">+{{ folder.additions }}</span>
                    <span v-if="folder.deletions" class="text-[#ef4444]">-{{ folder.deletions }}</span>
                  </div>
                </button>
              </div>

              <div class="border-t border-[var(--border)]">
                <div
                  v-for="file in filteredChangeMapFiles"
                  :key="'map-stash-' + file.path"
                  class="group flex items-center gap-2 px-4 py-1.5 transition-colors hover:bg-[#f59e0b]/6"
                  @contextmenu="openFileContextMenu($event, file.path)"
                >
                  <span class="text-[10px] font-bold w-4 text-center" :style="{ color: statusColor(file.status) }">{{ statusIcon(file.status) }}</span>
                  <div class="min-w-0 flex-1">
                    <div class="truncate text-xs text-[var(--foreground)]">{{ fileParts(file.path).fileName }}</div>
                    <div class="truncate text-[10px] text-[var(--muted-foreground)]">{{ fileParts(file.path).directory || '.' }}</div>
                  </div>
                  <span v-if="file.additions > 0" class="text-[10px] text-[#10b981] font-mono">+{{ file.additions }}</span>
                  <span v-if="file.deletions > 0" class="text-[10px] text-[#ef4444] font-mono">-{{ file.deletions }}</span>
                </div>
              </div>
            </div>
          </template>
        </div>
        <div v-else class="px-4 py-12 flex flex-col items-center justify-center">
          <div class="w-12 h-12 rounded-full bg-[var(--card)] flex items-center justify-center mb-3 border border-[var(--border)]">
            <FileText class="w-6 h-6 text-[var(--muted-foreground)] opacity-40" />
          </div>
          <p class="text-xs text-[var(--muted-foreground)]">No stashed changes</p>
        </div>
      </div>

      <div v-if="selectedStash" class="border-t border-[var(--border)] p-3 bg-[var(--card)]/50 flex-shrink-0 space-y-2">
        <AppButton
          class="w-full bg-[#f59e0b] hover:bg-[#f59e0b]/90 text-white text-xs font-medium h-8"
          @click="emit('stashPop', selectedStash.index)"
        >
          Pop Stash
        </AppButton>
        <div class="flex gap-2">
          <AppButton
            class="flex-1 bg-[var(--secondary)] hover:bg-[var(--secondary)]/80 text-[var(--foreground)] text-xs font-medium h-8"
            @click="emit('stashApply', selectedStash.index)"
          >
            Apply
          </AppButton>
          <AppButton
            class="flex-1 bg-[#ef4444]/10 hover:bg-[#ef4444]/20 text-[#ef4444] text-xs font-medium h-8"
            @click="emit('stashDrop', selectedStash.index)"
          >
            Drop
          </AppButton>
        </div>
      </div>
    </div>

    <div v-show="activeTab === 'info' && isStash && selectedStash" class="flex-1 overflow-y-auto">
      <div v-if="selectedStash" class="p-4 space-y-4">
        <div>
          <div class="text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">Stash Reference</div>
          <div class="flex items-center gap-2">
            <code class="text-xs text-[#f59e0b] bg-[#f59e0b]/10 px-2 py-1 rounded font-mono border border-[#f59e0b]/20">stash@{{ '{' + selectedStash.index + '}' }}</code>
          </div>
        </div>

        <div v-if="selectedStash.message">
          <div class="text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">Message</div>
          <div class="text-sm text-[var(--foreground)] leading-relaxed bg-[var(--input-background)] p-3 rounded border border-[var(--border)]">
            {{ selectedStash.message }}
          </div>
        </div>

        <div class="h-px bg-[var(--border)]" />

        <div v-if="selectedStash.branch">
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <GitBranch class="w-3 h-3" />
            Original Branch
          </div>
          <span class="px-2 py-0.5 text-[10px] font-medium rounded bg-[var(--primary)]/15 text-[var(--primary)] border border-[var(--primary)]/20">
            {{ selectedStash.branch }}
          </span>
        </div>

        <div v-if="selectedStash.timestamp">
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <Calendar class="w-3 h-3" />
            Created
          </div>
          <div class="text-xs text-[var(--foreground)]">{{ selectedStash.timestamp }}</div>
        </div>

        <div v-if="selectedStash.parent_sha">
          <div class="flex items-center gap-1.5 text-[10px] text-[var(--muted-foreground)] uppercase tracking-wider mb-1.5">
            <Hash class="w-3 h-3" />
            Parent Commit
          </div>
          <div class="flex items-center gap-2">
            <code class="text-xs text-[var(--primary)] bg-[var(--input-background)] px-2 py-1 rounded font-mono">{{ selectedStash.parent_sha.substring(0, 7) }}</code>
            <button
              @click="copyToClipboard(selectedStash.parent_sha, 'stash-parent-sha')"
              class="p-1 rounded hover:bg-[var(--secondary)] transition-colors flex-shrink-0"
              title="Copy SHA"
            >
              <Check v-if="copiedShaKey === 'stash-parent-sha'" class="w-3 h-3 text-[#10b981] animate-pulse" />
              <Copy v-else class="w-3 h-3 text-[var(--muted-foreground)]" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <div v-if="!isMultiCommitSelection && !commit && !isWorkingChanges && !(isStash && selectedStash)" class="flex-1 flex items-center justify-center">
      <div class="text-center">
        <GitCommitIcon class="w-8 h-8 text-[var(--muted-foreground)] opacity-30 mx-auto mb-2" />
        <p class="text-xs text-[var(--muted-foreground)]">Select a commit to view details</p>
      </div>
    </div>

    <Teleport to="body">
      <div
        v-if="commitAnalyzerTooltipVisible && commitAnalyzerSettings.enabled && commitLintIndicator !== 'none'"
        ref="commitAnalyzerPanelRef"
        class="fixed pointer-events-none rounded-md border border-[var(--border)] bg-[var(--card)] p-2.5 shadow-2xl"
        :style="commitAnalyzerPanelStyle"
      >
        <div class="text-[11px] font-semibold text-[var(--foreground)] mb-1">Commit checks</div>
        <div v-if="commitLintLoading" class="text-[10px] text-[var(--muted-foreground)]">Analyzing staged diff...</div>
        <div v-else class="space-y-1">
          <div
            v-for="finding in commitLintTooltipFindings"
            :key="finding.id"
            class="flex items-start gap-1.5 text-[10px]"
            :class="commitLintFindingClass(finding)"
          >
            <span class="w-8 text-center font-semibold">{{ finding.severity === 'error' ? 'ERR' : 'WARN' }}</span>
            <span class="flex-1 leading-snug">{{ finding.message }}</span>
          </div>
        </div>
      </div>
    </Teleport>

    <Teleport to="body">
      <div
        v-if="showCommitBuilder"
        ref="commitBuilderPanelRef"
        class="fixed rounded-lg border border-[var(--border)] bg-[var(--popover)] p-3 shadow-2xl"
        :style="commitBuilderPanelStyle"
        @pointerdown.stop
      >
        <div class="mb-2 flex items-center justify-between gap-2">
          <div class="flex items-center gap-2">
            <Hammer class="h-3.5 w-3.5 text-[var(--primary)]" />
            <div class="text-xs font-semibold text-[var(--foreground)]">Visual Commit Builder</div>
          </div>
          <CloseIconButton size="sm" subtle title="Close commit builder" @click="showCommitBuilder = false" />
        </div>

        <div class="grid grid-cols-3 gap-2">
          <label class="space-y-1">
            <span class="text-[10px] text-[var(--muted-foreground)]">Type</span>
            <select
              v-model="commitBuilderType"
              class="h-8 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 text-xs text-[var(--foreground)] outline-none"
            >
              <option v-for="type in commitBuilderTypes" :key="type" :value="type">{{ type }}</option>
            </select>
          </label>
          <label class="space-y-1">
            <span class="text-[10px] text-[var(--muted-foreground)]">Scope</span>
            <input
              v-model="commitBuilderScope"
              list="commit-builder-scopes"
              class="h-8 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 text-xs text-[var(--foreground)] outline-none"
              placeholder="git"
            >
            <datalist id="commit-builder-scopes">
              <option v-for="scope in commitBuilderScopes" :key="scope" :value="scope" />
            </datalist>
          </label>
          <label class="space-y-1">
            <span class="text-[10px] text-[var(--muted-foreground)]">Issue</span>
            <input
              v-model="commitBuilderIssue"
              class="h-8 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 text-xs text-[var(--foreground)] outline-none"
              placeholder="#123"
              inputmode="numeric"
              @keyup.enter="saveCommitBuilder"
            >
          </label>
        </div>

        <label class="mt-2 block space-y-1">
          <span class="text-[10px] text-[var(--muted-foreground)]">Message</span>
          <input
            v-model="commitBuilderSummary"
            class="h-8 w-full rounded border border-[var(--border)] bg-[var(--input-background)] px-2 text-xs text-[var(--foreground)] outline-none"
            placeholder="discard only unstaged file changes"
            @keyup.enter="saveCommitBuilder"
          >
        </label>

        <label class="mt-2 block space-y-1">
          <span class="text-[10px] text-[var(--muted-foreground)]">Description</span>
          <textarea
            v-model="commitBuilderBody"
            rows="2"
            class="w-full resize-none rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 text-xs text-[var(--foreground)] outline-none"
            placeholder="Optional body..."
          />
        </label>

        <div class="mt-2 rounded border border-[var(--border)] bg-[var(--input-background)] px-2 py-1.5 font-mono text-[10px] text-[var(--foreground)]">
          {{ commitBuilderPreview }}
        </div>

        <div class="mt-3 flex justify-end gap-2">
          <button
            type="button"
            class="rounded border border-[var(--border)] bg-[var(--secondary)] px-2.5 py-1 text-[11px] text-[var(--foreground)] hover:opacity-85"
            @click="showCommitBuilder = false"
          >
            Cancel
          </button>
          <button
            type="button"
            class="rounded bg-[var(--primary)] px-2.5 py-1 text-[11px] font-medium text-white hover:opacity-90"
            @click="saveCommitBuilder"
          >
            Save
          </button>
        </div>
      </div>
    </Teleport>

    <Teleport to="body">
      <div
        v-if="fileCtxVisible && fileCtxPath"
        class="fixed z-[210] w-[250px] rounded-lg border border-[var(--border)] bg-[var(--popover)] shadow-2xl p-2"
        :style="{ left: fileCtxX + 'px', top: fileCtxY + 'px' }"
        @pointerdown.stop
      >
        <div class="px-1 pb-2 border-b border-[var(--border)]">
          <p class="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">Open With</p>
          <p class="text-[11px] text-[var(--foreground)] truncate mt-0.5" :title="fileCtxPath">{{ fileCtxPath }}</p>
        </div>

        <div class="pt-2 space-y-1">
          <button
            v-for="option in availableEditors"
            :key="option.id"
            class="w-full text-left px-2 py-1.5 rounded-md border border-transparent transition-colors"
            :class="editorButtonClass(option)"
            @click="openSelectedFileWithEditor(option.id)"
          >
            <div class="text-xs font-medium text-[var(--foreground)]">{{ option.label }}</div>
            <div class="file-ctx-option-hint text-[10px] text-[var(--muted-foreground)]">{{ option.hint }}</div>
          </button>
        </div>
      </div>
    </Teleport>

    <!-- Discard Confirmation Toast -->
    <Teleport to="body" v-if="showDiscardConfirm">
      <div class="fixed bottom-4 right-4 z-[200] w-80 pointer-events-auto">
        <div class="flex items-start gap-3 px-4 py-3 rounded-lg border shadow-xl backdrop-blur-md bg-[#2a1316]/96 border-[#ef4444]/75">
          <Trash2 class="w-5 h-5 flex-shrink-0 mt-0.5" style="color: #f87171" />
          <div class="flex-1 min-w-0">
            <p class="text-sm text-[var(--foreground)] font-semibold">Discard changes?</p>
            <p class="text-xs text-[#f87171] mt-1">This action cannot be undone.</p>
            <div class="mt-3 flex gap-2 justify-start">
              <button
                @click="handleDiscardConfirm"
                class="px-3 py-1.5 text-xs font-medium rounded bg-[#ef4444] text-white hover:bg-[#dc2626] transition-colors"
              >
                Yes, Discard
              </button>
              <button
                @click="cancelDiscard"
                class="px-3 py-1.5 text-xs font-medium rounded bg-[#374151] text-white hover:bg-[#4b5563] transition-colors"
              >
                No
              </button>
            </div>
          </div>
          <CloseIconButton size="sm" subtle title="Close discard confirmation" @click="cancelDiscard" />
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.file-ctx-option-hint {
  display: none;
}

:global(html.dummy-mode .file-ctx-option-hint) {
  display: block;
}
</style>
