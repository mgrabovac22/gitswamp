import { safeStorageGet, safeStorageSet } from "@/app/storage/safeStorage";

export const strategies = [
  { id: "swamp", name: "GitSwamp Handoff", summary: "Short branches, a preflight review, then a pull request." },
  { id: "github", name: "GitHub Flow", summary: "Feature and fix branches reviewed through pull requests." },
  { id: "trunk", name: "Trunk-Based", summary: "One trunk, short-lived branches and fast-forward integration." },
  { id: "gitflow", name: "Gitflow", summary: "Development and production branches, with releases and hotfixes." },
  { id: "manual", name: "Manual Git", summary: "Keep the standard Git controls without an active strategy." },
] as const;

export type Strategy = typeof strategies[number]["id"];
export type WorkflowConfig = {
  strategy: Strategy; primary: string; development: string; feature: string; fix: string;
  release: string; hotfix: string; support: string; tag: string; shortBranches: boolean; releaseBranches: boolean;
};
export type WorkflowInspection = {
  key: string; current: string | null; local: string[]; remote: string[]; primary: string;
  development: string | null; suggestion: Strategy | null; minimal: boolean; unborn: boolean; blocked: string | null;
};
export type WorkflowAction = "feature" | "fix" | "release" | "hotfix" | "support" | "prepare-primary" | "prepare-development" | "finish" | "cleanup" | "tag" | "handoff";
export type WorkflowRequest = { action: WorkflowAction; name: string; source: string; cleanup: boolean; remote?: string };
export type Handoff = {
  source: string; target: string; published: boolean;
  remote: string | null; targetFetched: boolean;
  sourceSync: "no-remote" | "not-published" | "in-sync" | "local-ahead" | "remote-ahead" | "different" | "unknown";
  checkpoint: { changedPaths: number; testPaths: number; reviewAreas: string[]; commits: { sha: string; subject: string }[] } | null;
};
export type HandoffDraft = Pick<Handoff, "source" | "target" | "published"> & { description?: string };
export type WorkflowComparison = { target: string; ahead: number; behind: number; conflicts: boolean | null; conflictPaths?: string[] };
export type WorkflowPlan = {
  token: string; steps: string[];
  comparisons: WorkflowComparison[];
  handoff: Handoff | null;
};
export type WorkflowOutcome = { status: "applied" | "conflict" | "stopped"; completed: string[]; remaining: string[]; error: string | null };

export function workflowDefaults(strategy: Strategy, primary = "main", development = "develop"): WorkflowConfig {
  return {
    strategy, primary, development, feature: strategy === "swamp" ? "change/" : "feature/",
    fix: strategy === "swamp" ? "repair/" : strategy === "trunk" ? "bugfix/" : "fix/",
    release: "release/", hotfix: "hotfix/", support: "support/", tag: "v", shortBranches: true, releaseBranches: false,
  };
}

export function workflowName(strategy: Strategy) {
  return strategies.find(choice => choice.id === strategy)?.name ?? "Branching workflow";
}

const prefix = "gitswamp.branch-workflow.v1.";
const session = new Map<string, string>();

function read(key: string) { return session.get(key) ?? safeStorageGet(key); }
function write(key: string, value: string): boolean {
  safeStorageSet(key, value);
  if (safeStorageGet(key) === value) {
    session.delete(key);
    return true;
  }
  session.set(key, value);
  return false;
}
function configKey(key: string) { return prefix + "repo:" + key; }
function aliasKey(path: string) { return prefix + "path:" + path; }

export function workflowPreference(key: string): WorkflowConfig | null {
  const stored = read(configKey(key));
  if (!stored) return null;
  try {
    const config: WorkflowConfig = JSON.parse(stored);
    if (!config || !strategies.some(choice => choice.id === config.strategy)) return null;
    const defaults = workflowDefaults(config.strategy);
    if (Object.entries(defaults).some(([key, value]) =>
      config[key as keyof WorkflowConfig] !== undefined && typeof config[key as keyof WorkflowConfig] !== typeof value)) return null;
    return { ...defaults, ...config };
  } catch { return null; }
}

export function workflowForPath(path: string): WorkflowConfig | null {
  const key = read(aliasKey(path));
  return key ? workflowPreference(key) : null;
}

export function rememberWorkflowPath(path: string, key: string) {
  write(aliasKey(path), key);
}

export function saveWorkflow(path: string, key: string, config: WorkflowConfig): boolean {
  rememberWorkflowPath(path, key);
  return write(configKey(key), JSON.stringify(config));
}

export function workflowSources(config: WorkflowConfig, branches: string[]) {
  const prefixes = config.strategy === "gitflow" ? [config.feature, config.release, config.hotfix]
    : config.strategy === "trunk" && !config.shortBranches ? [] : [config.feature, config.fix];
  return branches.filter(branch => branch !== config.primary && (config.strategy !== "gitflow" || branch !== config.development)
    && prefixes.some(prefix => branch.startsWith(prefix) && branch.length > prefix.length));
}

export function handoffReport(handoff: Handoff, comparison: WorkflowComparison) {
  const literal = (value: string) => "`" + value.replace(/`/g, "'") + "`";
  const target = handoff.remote && handoff.targetFetched ? handoff.remote + "/" + handoff.target : handoff.target;
  const lines = [
    "## Integration handoff",
    "",
    literal(handoff.source) + " → " + literal(target) + (handoff.targetFetched ? " (last fetched)" : " (local target)"),
    "- " + comparison.ahead + " ahead, " + comparison.behind + " behind.",
    "- Merge preview: " + (comparison.conflicts === true ? "conflicts detected." : comparison.conflicts === false ? "no conflicts detected." : "not available."),
  ];
  if (handoff.checkpoint) {
    lines.push("- " + handoff.checkpoint.changedPaths + " changed paths; " + handoff.checkpoint.testPaths + " test paths touched.");
    if (handoff.checkpoint.reviewAreas.length) lines.push("- Review: " + handoff.checkpoint.reviewAreas.join("; ") + ".");
  }
  lines.push("", "## Verification", "", "- [ ] Record tests or CI results.");
  return lines.join("\n");
}
