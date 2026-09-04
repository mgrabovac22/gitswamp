import type { CommitFileInfo, CommitInfo } from "@/types";

export interface GitRpgRoleDefinition {
  id: string;
  title: string;
  shortTitle: string;
  mark: string;
  accent: string;
  description: string;
  signal: string;
}

export interface GitRpgCommitStats {
  additions: number;
  deletions: number;
  filesChanged: number;
  docsFiles: number;
  testFiles: number;
  configFiles: number;
  lockFiles: number;
  uiFiles: number;
  backendFiles: number;
  automationFiles: number;
  securityFiles: number;
  dataFiles: number;
}

export interface GitRpgAuthorProfile {
  author: string;
  commits: number;
  statCommits: number;
  additions: number;
  deletions: number;
  filesChanged: number;
  avgChurn: number;
  deletionRatio: number;
  nightShare: number;
  weekendShare: number;
  smallCommitShare: number;
  largeCommitShare: number;
  vagueMessageShare: number;
  oneWordMessageShare: number;
  mergeShare: number;
  docsShare: number;
  testShare: number;
  configShare: number;
  refactorShare: number;
  fixShare: number;
  featureShare: number;
  choreShare: number;
  releaseShare: number;
  dependencyShare: number;
  uiShare: number;
  backendShare: number;
  performanceShare: number;
  securityShare: number;
  automationShare: number;
  conflictShare: number;
  dataShare: number;
  primaryRole: GitRpgRoleDefinition;
  roleReason: string;
  roles: GitRpgRoleDefinition[];
}

export interface GitRpgProfile {
  repoPath: string;
  generatedAt: number;
  scannedCommits: number;
  scannedStatCommits: number;
  primaryRole: GitRpgRoleDefinition;
  primaryAuthor: string;
  primaryReason: string;
  authorProfiles: GitRpgAuthorProfile[];
}

interface BuildGitRpgProfileInput {
  repoPath: string;
  commits: CommitInfo[];
  statsBySha: Map<string, GitRpgCommitStats>;
}

interface AuthorAccumulator {
  author: string;
  commits: number;
  nightCommits: number;
  weekendCommits: number;
  oneWordMessages: number;
  vagueMessages: number;
  mergeCommits: number;
  refactorCommits: number;
  fixCommits: number;
  featureCommits: number;
  choreCommits: number;
  releaseCommits: number;
  dependencyCommits: number;
  uiCommits: number;
  backendCommits: number;
  performanceCommits: number;
  securityCommits: number;
  automationCommits: number;
  conflictCommits: number;
  dataCommits: number;
  statCommits: number;
  additions: number;
  deletions: number;
  filesChanged: number;
  smallCommits: number;
  largeCommits: number;
  docsCommits: number;
  testCommits: number;
  configCommits: number;
}

interface RoleScore {
  role: GitRpgRoleDefinition;
  score: number;
  reason: string;
}

export const GIT_RPG_ROLES: GitRpgRoleDefinition[] = [
  {
    id: "surgeon",
    title: "Code Surgeon",
    shortTitle: "Surgeon",
    mark: "CS",
    accent: "#10b981",
    description: "Deletes more than they add, trims legacy code, and leaves the codebase smaller and sharper.",
    signal: "High deleted-to-added line ratio.",
  },
  {
    id: "night-cowboy",
    title: "Night Cowboy",
    shortTitle: "Night",
    mark: "NC",
    accent: "#8b5cf6",
    description: "Shows up most often when the rest of the team is probably asleep.",
    signal: "Large share of commits between midnight and 4 AM.",
  },
  {
    id: "tiny-sculptor",
    title: "Tiny Sculptor",
    shortTitle: "Sculpt",
    mark: "TS",
    accent: "#22d3ee",
    description: "Ships small, frequent, easy-to-review changes that rarely explode in review.",
    signal: "Low average churn per commit.",
  },
  {
    id: "ghost-rider",
    title: "Ghost Rider",
    shortTitle: "Ghost",
    mark: "GR",
    accent: "#f97316",
    description: "Large commits with tiny messages such as fix, update, or wip.",
    signal: "Large churn paired with short or vague messages.",
  },
  {
    id: "architect",
    title: "Architect",
    shortTitle: "Architect",
    mark: "AR",
    accent: "#38bdf8",
    description: "Shapes broad parts of the system and often changes repository structure.",
    signal: "Many files per commit or planned feature/refactor work.",
  },
  {
    id: "firefighter",
    title: "Firefighter",
    shortTitle: "Fire",
    mark: "FF",
    accent: "#ef4444",
    description: "Often jumps in to put out bugs, hotfixes, and regressions.",
    signal: "High share of fix, bug, patch, or hotfix commits.",
  },
  {
    id: "gardener",
    title: "Repository Gardener",
    shortTitle: "Garden",
    mark: "RG",
    accent: "#84cc16",
    description: "Keeps the repository tidy through cleanup, chore, and refactor habits.",
    signal: "Chore/refactor/cleanup signals without chaotic churn.",
  },
  {
    id: "cartographer",
    title: "Cartographer",
    shortTitle: "Docs",
    mark: "CT",
    accent: "#06b6d4",
    description: "Maps the project through documentation, README updates, and explanations.",
    signal: "Frequent docs files or documentation-oriented commits.",
  },
  {
    id: "test-guardian",
    title: "Test Guardian",
    shortTitle: "Test",
    mark: "TG",
    accent: "#14b8a6",
    description: "Protects stability through tests, specs, and safety nets.",
    signal: "High share of test files or test commit messages.",
  },
  {
    id: "refactor-monk",
    title: "Refactor Monk",
    shortTitle: "Monk",
    mark: "RM",
    accent: "#a78bfa",
    description: "Quietly reshapes code without drama and keeps technical debt under control.",
    signal: "Frequent refactor work with moderate churn.",
  },
  {
    id: "release-captain",
    title: "Release Captain",
    shortTitle: "Release",
    mark: "RC",
    accent: "#f59e0b",
    description: "Handles versions, changelogs, release branches, and delivery preparation.",
    signal: "Release, version, tag, or changelog signals.",
  },
  {
    id: "dependency-tamer",
    title: "Dependency Tamer",
    shortTitle: "Deps",
    mark: "DT",
    accent: "#ec4899",
    description: "Keeps the dependency stack alive by updating packages and lockfiles.",
    signal: "Dependency messages or lock/config file changes.",
  },
  {
    id: "merge-diplomat",
    title: "Merge Diplomat",
    shortTitle: "Merge",
    mark: "MD",
    accent: "#6366f1",
    description: "Often merges branches and keeps team history moving.",
    signal: "High share of merge-shaped commits.",
  },
  {
    id: "explorer",
    title: "Explorer",
    shortTitle: "Explore",
    mark: "EX",
    accent: "#0ea5e9",
    description: "Opens new paths through feature, spike, experiment, and prototype work.",
    signal: "Feature or experimental commit messages.",
  },
  {
    id: "stabilizer",
    title: "Stabilizer",
    shortTitle: "Stable",
    mark: "ST",
    accent: "#22c55e",
    description: "Ships small fix/test changes that reduce risk instead of expanding surface area.",
    signal: "Small churn with a high fix or test share.",
  },
  {
    id: "builder",
    title: "Builder",
    shortTitle: "Build",
    mark: "BD",
    accent: "#3b82f6",
    description: "Adds visible product volume and new functionality.",
    signal: "High added-line ratio and feature commits.",
  },
  {
    id: "ui-alchemist",
    title: "UI Alchemist",
    shortTitle: "UI",
    mark: "UI",
    accent: "#d946ef",
    description: "Turns panels, buttons, layouts, and visual flows into a cleaner product surface.",
    signal: "Strong frontend, component, layout, or styling footprint.",
  },
  {
    id: "backend-smith",
    title: "Backend Smith",
    shortTitle: "Backend",
    mark: "BS",
    accent: "#0f766e",
    description: "Works close to commands, services, API boundaries, and native backend code.",
    signal: "Backend, Rust, Tauri, service, or API-heavy changes.",
  },
  {
    id: "performance-sprinter",
    title: "Performance Sprinter",
    shortTitle: "Perf",
    mark: "PS",
    accent: "#06b6d4",
    description: "Chases faster loading, lower memory use, caching, and smoother interaction.",
    signal: "Performance, cache, memory, RAM, or loading-speed signals.",
  },
  {
    id: "security-sentinel",
    title: "Security Sentinel",
    shortTitle: "Security",
    mark: "SS",
    accent: "#f43f5e",
    description: "Works around auth, tokens, credentials, permissions, SSH, or secret handling.",
    signal: "Security-sensitive paths or commit messages.",
  },
  {
    id: "automation-pilot",
    title: "Automation Pilot",
    shortTitle: "Auto",
    mark: "AP",
    accent: "#f97316",
    description: "Keeps builds, scripts, workflows, CI, and tooling lanes moving.",
    signal: "CI, scripts, Docker, build, workflow, or automation changes.",
  },
  {
    id: "conflict-whisperer",
    title: "Conflict Whisperer",
    shortTitle: "Conflict",
    mark: "CW",
    accent: "#a855f7",
    description: "Spends time around merges, rebases, conflict handling, and history repair.",
    signal: "Conflict, merge resolution, or rebase-heavy commits.",
  },
  {
    id: "data-steward",
    title: "Data Steward",
    shortTitle: "Data",
    mark: "DS",
    accent: "#65a30d",
    description: "Changes schemas, migrations, models, queries, or data flow foundations.",
    signal: "Database, schema, migration, model, SQL, or data paths.",
  },
  {
    id: "janitor",
    title: "Janitor",
    shortTitle: "Clean",
    mark: "JN",
    accent: "#94a3b8",
    description: "Sweeps away noise, dead code, and repository clutter.",
    signal: "Cleanup/chore messages with visible deletions.",
  },
  {
    id: "weekend-sprinter",
    title: "Weekend Sprinter",
    shortTitle: "Weekend",
    mark: "WS",
    accent: "#fb7185",
    description: "Gets most active on weekends, often through short focused bursts.",
    signal: "Large share of commits on Saturday or Sunday.",
  },
  {
    id: "storyteller",
    title: "Storyteller",
    shortTitle: "Story",
    mark: "SL",
    accent: "#eab308",
    description: "Writes commit messages that explain intent, not only the mechanical change.",
    signal: "Longer messages and a low vague-message share.",
  },
  {
    id: "risk-juggler",
    title: "Risk Juggler",
    shortTitle: "Risk",
    mark: "RJ",
    accent: "#dc2626",
    description: "Handles unusually large changes that deserve extra careful review.",
    signal: "Very large churn or unusually wide file spread.",
  },
  {
    id: "balanced-adventurer",
    title: "Balanced Adventurer",
    shortTitle: "Balanced",
    mark: "BA",
    accent: "#64748b",
    description: "Has no single extreme style and shows a balanced mix of repository work.",
    signal: "Used when no clear dominant signal wins.",
  },
];

const roleById = new Map(GIT_RPG_ROLES.map((role) => [role.id, role]));
const fallbackRole: GitRpgRoleDefinition = roleById.get("balanced-adventurer") || GIT_RPG_ROLES[GIT_RPG_ROLES.length - 1]!;

export function getGitRpgRole(id: string): GitRpgRoleDefinition {
  return roleById.get(id) || fallbackRole;
}

export function summarizeCommitFilesForRpg(files: CommitFileInfo[]): GitRpgCommitStats {
  const summary: GitRpgCommitStats = {
    additions: 0,
    deletions: 0,
    filesChanged: files.length,
    docsFiles: 0,
    testFiles: 0,
    configFiles: 0,
    lockFiles: 0,
    uiFiles: 0,
    backendFiles: 0,
    automationFiles: 0,
    securityFiles: 0,
    dataFiles: 0,
  };

  for (const file of files) {
    const path = file.path.replace(/\\/g, "/").toLowerCase();
    summary.additions += Number(file.additions) || 0;
    summary.deletions += Number(file.deletions) || 0;
    if (isDocsPath(path)) summary.docsFiles += 1;
    if (isTestPath(path)) summary.testFiles += 1;
    if (isConfigPath(path)) summary.configFiles += 1;
    if (isLockPath(path)) summary.lockFiles += 1;
    if (isUiPath(path)) summary.uiFiles += 1;
    if (isBackendPath(path)) summary.backendFiles += 1;
    if (isAutomationPath(path)) summary.automationFiles += 1;
    if (isSecurityPath(path)) summary.securityFiles += 1;
    if (isDataPath(path)) summary.dataFiles += 1;
  }

  return summary;
}

export function buildGitRpgProfile(input: BuildGitRpgProfileInput): GitRpgProfile {
  const byAuthor = new Map<string, AuthorAccumulator>();
  const repoAccumulator = createAccumulator("Repository");

  for (const commit of input.commits) {
    const author = (commit.author_name || "Unknown").trim() || "Unknown";
    const acc = getAccumulator(byAuthor, author);
    const stats = input.statsBySha.get(commit.sha);
    applyCommitToAccumulator(acc, commit, stats);
    applyCommitToAccumulator(repoAccumulator, commit, stats);
  }

  const authorProfiles = Array.from(byAuthor.values())
    .map(authorProfileFromAccumulator)
    .sort((a, b) => b.commits - a.commits || a.author.localeCompare(b.author))
    .slice(0, 6);

  const repoProfile = repoAccumulator.commits > 0
    ? authorProfileFromAccumulator(repoAccumulator)
    : emptyAuthorProfile();
  return {
    repoPath: input.repoPath,
    generatedAt: Date.now(),
    scannedCommits: input.commits.length,
    scannedStatCommits: input.statsBySha.size,
    primaryRole: repoProfile.primaryRole,
    primaryAuthor: repoProfile.author,
    primaryReason: repoProfile.roleReason,
    authorProfiles,
  };
}

export function gitRpgProfileDetailLines(profile: GitRpgProfile | null): string[] {
  if (!profile) {
    return [
      "Role: Git RPG profile is still warming up.",
      "Explanation: open a repository with commit history and GitSwamp will calculate a lightweight style badge in the background.",
    ];
  }

  const lines = [
    `Role: ${profile.primaryRole.title}.`,
    `Explanation: ${profile.primaryRole.description}`,
    `Signal: ${profile.primaryReason}`,
    `Sample: ${profile.scannedCommits} commits, ${profile.scannedStatCommits} commits with line/file stats.`,
  ];

  if (profile.authorProfiles.length > 0) {
    lines.push("Team roll call:");
    for (const author of profile.authorProfiles.slice(0, 4)) {
      lines.push(`- ${author.author}: ${author.primaryRole.title} (${author.commits} commits, avg churn ${Math.round(author.avgChurn)}).`);
    }
  }

  return lines;
}

function getAccumulator(byAuthor: Map<string, AuthorAccumulator>, author: string): AuthorAccumulator {
  const existing = byAuthor.get(author);
  if (existing) return existing;
  const next = createAccumulator(author);
  byAuthor.set(author, next);
  return next;
}

function createAccumulator(author: string): AuthorAccumulator {
  return {
    author,
    commits: 0,
    nightCommits: 0,
    weekendCommits: 0,
    oneWordMessages: 0,
    vagueMessages: 0,
    mergeCommits: 0,
    refactorCommits: 0,
    fixCommits: 0,
    featureCommits: 0,
    choreCommits: 0,
    releaseCommits: 0,
    dependencyCommits: 0,
    uiCommits: 0,
    backendCommits: 0,
    performanceCommits: 0,
    securityCommits: 0,
    automationCommits: 0,
    conflictCommits: 0,
    dataCommits: 0,
    statCommits: 0,
    additions: 0,
    deletions: 0,
    filesChanged: 0,
    smallCommits: 0,
    largeCommits: 0,
    docsCommits: 0,
    testCommits: 0,
    configCommits: 0,
  };
}

function applyCommitToAccumulator(acc: AuthorAccumulator, commit: CommitInfo, stats?: GitRpgCommitStats): void {
  const message = commit.message || "";
  const normalizedMessage = normalizeMessage(message);
  const words = wordCount(normalizedMessage);
  const timestamp = toMillis(commit.timestamp);
  const date = new Date(timestamp);
  const hour = date.getHours();
  const day = date.getDay();

  acc.commits += 1;
  if (hour >= 0 && hour < 4) acc.nightCommits += 1;
  if (day === 0 || day === 6) acc.weekendCommits += 1;
  if (words <= 1) acc.oneWordMessages += 1;
  if (isVagueMessage(normalizedMessage)) acc.vagueMessages += 1;
  if (isMergeMessage(normalizedMessage) || commit.parent_shas.length > 1) acc.mergeCommits += 1;
  if (isRefactorMessage(normalizedMessage)) acc.refactorCommits += 1;
  if (isFixMessage(normalizedMessage)) acc.fixCommits += 1;
  if (isFeatureMessage(normalizedMessage)) acc.featureCommits += 1;
  if (isChoreMessage(normalizedMessage)) acc.choreCommits += 1;
  if (isReleaseMessage(normalizedMessage)) acc.releaseCommits += 1;
  if (isDependencyMessage(normalizedMessage)) acc.dependencyCommits += 1;
  if (isDocsMessage(normalizedMessage)) acc.docsCommits += 1;
  if (isUiMessage(normalizedMessage)) acc.uiCommits += 1;
  if (isBackendMessage(normalizedMessage)) acc.backendCommits += 1;
  if (isPerformanceMessage(normalizedMessage)) acc.performanceCommits += 1;
  if (isSecurityMessage(normalizedMessage)) acc.securityCommits += 1;
  if (isAutomationMessage(normalizedMessage)) acc.automationCommits += 1;
  if (isConflictMessage(normalizedMessage)) acc.conflictCommits += 1;
  if (isDataMessage(normalizedMessage)) acc.dataCommits += 1;

  if (!stats) return;

  const churn = stats.additions + stats.deletions;
  acc.statCommits += 1;
  acc.additions += stats.additions;
  acc.deletions += stats.deletions;
  acc.filesChanged += stats.filesChanged;
  if (churn > 0 && churn <= 10) acc.smallCommits += 1;
  if (churn >= 600 || stats.filesChanged >= 18) acc.largeCommits += 1;
  if (stats.docsFiles > 0) acc.docsCommits += 1;
  if (stats.testFiles > 0) acc.testCommits += 1;
  if (stats.configFiles > 0 || stats.lockFiles > 0) acc.configCommits += 1;
  if (stats.lockFiles > 0) acc.dependencyCommits += 1;
  if (stats.uiFiles > 0) acc.uiCommits += 1;
  if (stats.backendFiles > 0) acc.backendCommits += 1;
  if (stats.automationFiles > 0) acc.automationCommits += 1;
  if (stats.securityFiles > 0) acc.securityCommits += 1;
  if (stats.dataFiles > 0) acc.dataCommits += 1;
}

function authorProfileFromAccumulator(acc: AuthorAccumulator): GitRpgAuthorProfile {
  const commitBase = Math.max(1, acc.commits);
  const statBase = Math.max(1, acc.statCommits);
  const avgChurn = acc.statCommits > 0 ? (acc.additions + acc.deletions) / acc.statCommits : 0;
  const profileBase = {
    author: acc.author,
    commits: acc.commits,
    statCommits: acc.statCommits,
    additions: acc.additions,
    deletions: acc.deletions,
    filesChanged: acc.filesChanged,
    avgChurn,
    deletionRatio: acc.additions > 0 ? acc.deletions / acc.additions : acc.deletions > 0 ? 99 : 0,
    nightShare: acc.nightCommits / commitBase,
    weekendShare: acc.weekendCommits / commitBase,
    smallCommitShare: acc.smallCommits / statBase,
    largeCommitShare: acc.largeCommits / statBase,
    vagueMessageShare: acc.vagueMessages / commitBase,
    oneWordMessageShare: acc.oneWordMessages / commitBase,
    mergeShare: acc.mergeCommits / commitBase,
    docsShare: clampShare(acc.docsCommits / commitBase),
    testShare: acc.testCommits / statBase,
    configShare: acc.configCommits / statBase,
    refactorShare: acc.refactorCommits / commitBase,
    fixShare: acc.fixCommits / commitBase,
    featureShare: acc.featureCommits / commitBase,
    choreShare: acc.choreCommits / commitBase,
    releaseShare: acc.releaseCommits / commitBase,
    dependencyShare: clampShare(acc.dependencyCommits / commitBase),
    uiShare: clampShare(acc.uiCommits / commitBase),
    backendShare: clampShare(acc.backendCommits / commitBase),
    performanceShare: clampShare(acc.performanceCommits / commitBase),
    securityShare: clampShare(acc.securityCommits / commitBase),
    automationShare: clampShare(acc.automationCommits / commitBase),
    conflictShare: clampShare(acc.conflictCommits / commitBase),
    dataShare: clampShare(acc.dataCommits / commitBase),
  };

  const scored = scoreRoles(profileBase);
  const primary = scored[0] || { role: fallbackRole, reason: "No single style dominated the sample." };
  return {
    ...profileBase,
    primaryRole: primary.role,
    roleReason: primary.reason,
    roles: scored.slice(0, 3).map((item) => item.role),
  };
}

function emptyAuthorProfile(): GitRpgAuthorProfile {
  return {
    author: "Repository",
    commits: 0,
    statCommits: 0,
    additions: 0,
    deletions: 0,
    filesChanged: 0,
    avgChurn: 0,
    deletionRatio: 0,
    nightShare: 0,
    weekendShare: 0,
    smallCommitShare: 0,
    largeCommitShare: 0,
    vagueMessageShare: 0,
    oneWordMessageShare: 0,
    mergeShare: 0,
    docsShare: 0,
    testShare: 0,
    configShare: 0,
    refactorShare: 0,
    fixShare: 0,
    featureShare: 0,
    choreShare: 0,
    releaseShare: 0,
    dependencyShare: 0,
    uiShare: 0,
    backendShare: 0,
    performanceShare: 0,
    securityShare: 0,
    automationShare: 0,
    conflictShare: 0,
    dataShare: 0,
    primaryRole: fallbackRole,
    roleReason: "No commit history was available yet.",
    roles: [fallbackRole],
  };
}

function scoreRoles(profile: Omit<GitRpgAuthorProfile, "primaryRole" | "roleReason" | "roles">): RoleScore[] {
  const scores: RoleScore[] = [];
  const add = (id: string, score: number, reason: string) => {
    if (score <= 0) return;
    scores.push({ role: getGitRpgRole(id), score, reason });
  };

  const avgFiles = profile.filesChanged / Math.max(1, profile.statCommits);
  const maintenanceShare = profile.choreShare + profile.refactorShare;
  const reliabilityShare = profile.fixShare + profile.testShare;
  const veryLargeWork =
    profile.statCommits >= 8
    && (profile.largeCommitShare >= 0.48 || profile.avgChurn >= 900 || avgFiles >= 18);

  add("surgeon", profile.deletionRatio >= 1.35 && profile.deletions >= 35 ? 72 + Math.min(profile.deletionRatio * 9, 24) : 0, `${profile.deletions} deleted vs ${profile.additions} added lines.`);
  add("night-cowboy", profile.nightShare >= 0.24 ? 62 + profile.nightShare * 82 : 0, `${percent(profile.nightShare)} of commits landed between midnight and 4 AM.`);
  add("tiny-sculptor", profile.statCommits >= 5 && profile.avgChurn > 0 && profile.avgChurn <= 16 ? 78 - profile.avgChurn : 0, `Average churn is ${Math.round(profile.avgChurn)} lines per stat-sampled commit.`);
  add("ghost-rider", profile.largeCommitShare >= 0.18 && (profile.vagueMessageShare >= 0.18 || profile.oneWordMessageShare >= 0.12) ? 82 + profile.largeCommitShare * 42 : 0, `${percent(profile.largeCommitShare)} large commits with ${percent(profile.vagueMessageShare)} vague messages.`);
  add("architect", avgFiles >= 8 || (profile.largeCommitShare >= 0.24 && profile.featureShare >= 0.16) ? 68 + Math.min(avgFiles * 3, 26) + profile.featureShare * 24 : 0, `Touches ${Math.round(avgFiles)} files per stat-sampled commit.`);
  add("firefighter", profile.fixShare >= 0.22 ? 64 + profile.fixShare * 86 : 0, `${percent(profile.fixShare)} of commit messages look like fixes.`);
  add("gardener", maintenanceShare >= 0.28 && profile.largeCommitShare < 0.38 ? 70 + maintenanceShare * 55 : 0, `Chore/refactor signals cover ${percent(maintenanceShare)} of commits.`);
  add("cartographer", profile.docsShare >= 0.16 ? 68 + profile.docsShare * 74 : 0, `${percent(profile.docsShare)} of the sample touched docs.`);
  add("test-guardian", profile.testShare >= 0.14 ? 72 + profile.testShare * 82 : 0, `${percent(profile.testShare)} of stat-sampled commits touched tests.`);
  add("refactor-monk", profile.refactorShare >= 0.18 ? 72 + profile.refactorShare * 70 : 0, `${percent(profile.refactorShare)} of messages mention refactor work.`);
  add("release-captain", profile.releaseShare >= 0.1 ? 76 + profile.releaseShare * 74 : 0, `${percent(profile.releaseShare)} of messages mention release/version/changelog work.`);
  add("dependency-tamer", profile.dependencyShare >= 0.12 || profile.configShare >= 0.28 ? 72 + Math.max(profile.dependencyShare, profile.configShare) * 66 : 0, `Dependency/config signals appear in ${percent(Math.max(profile.dependencyShare, profile.configShare))} of the sample.`);
  add("merge-diplomat", profile.mergeShare >= 0.18 ? 72 + profile.mergeShare * 66 : 0, `${percent(profile.mergeShare)} of commits are merge-shaped.`);
  add("explorer", profile.featureShare >= 0.22 ? 68 + profile.featureShare * 70 : 0, `${percent(profile.featureShare)} of messages look like feature/exploration work.`);
  add("stabilizer", reliabilityShare >= 0.32 && profile.avgChurn <= 110 ? 72 + reliabilityShare * 45 : 0, `Fix/test signals are high while avg churn stays near ${Math.round(profile.avgChurn)}.`);
  add("builder", profile.additions > profile.deletions * 1.7 && profile.featureShare >= 0.12 ? 70 + profile.featureShare * 64 : 0, `${profile.additions} added lines vs ${profile.deletions} deleted lines.`);
  add("ui-alchemist", profile.uiShare >= 0.24 ? 74 + profile.uiShare * 58 : 0, `${percent(profile.uiShare)} of commits touched UI, view, component, or styling areas.`);
  add("backend-smith", profile.backendShare >= 0.22 ? 74 + profile.backendShare * 56 : 0, `${percent(profile.backendShare)} of commits touched backend/native/service areas.`);
  add("performance-sprinter", profile.performanceShare >= 0.08 ? 82 + profile.performanceShare * 80 : 0, `${percent(profile.performanceShare)} of commits mention performance, caching, memory, or loading speed.`);
  add("security-sentinel", profile.securityShare >= 0.06 ? 86 + profile.securityShare * 86 : 0, `${percent(profile.securityShare)} of commits touch security-sensitive paths or messages.`);
  add("automation-pilot", profile.automationShare >= 0.12 ? 76 + profile.automationShare * 72 : 0, `${percent(profile.automationShare)} of commits touch CI, scripts, Docker, builds, or workflows.`);
  add("conflict-whisperer", profile.conflictShare >= 0.08 ? 78 + profile.conflictShare * 76 : 0, `${percent(profile.conflictShare)} of commits mention conflict, rebase, or merge resolution work.`);
  add("data-steward", profile.dataShare >= 0.1 ? 78 + profile.dataShare * 74 : 0, `${percent(profile.dataShare)} of commits touch schema, migration, database, model, or data paths.`);
  add("janitor", (profile.choreShare >= 0.2 || profile.deletionRatio >= 1) && profile.deletions >= 25 ? 64 + profile.choreShare * 58 + Math.min(profile.deletionRatio * 6, 16) : 0, `Cleanup/chore signals with ${profile.deletions} deleted lines.`);
  add("weekend-sprinter", profile.weekendShare >= 0.28 ? 62 + profile.weekendShare * 72 : 0, `${percent(profile.weekendShare)} of commits landed on weekends.`);
  add("storyteller", profile.commits >= 5 && profile.vagueMessageShare <= 0.08 && profile.oneWordMessageShare <= 0.04 ? 60 + (1 - profile.vagueMessageShare) * 18 : 0, `Only ${percent(profile.vagueMessageShare)} of messages look vague.`);
  add("risk-juggler", veryLargeWork ? 58 + profile.largeCommitShare * 42 + Math.min(profile.avgChurn / 90, 18) + Math.min(avgFiles, 18) + profile.vagueMessageShare * 18 : 0, `${percent(profile.largeCommitShare)} very large commits, avg churn ${Math.round(profile.avgChurn)}, ${Math.round(avgFiles)} files per stat-sampled commit.`);

  if (scores.length === 0) {
    add("balanced-adventurer", 1, "Signals are balanced across the scanned history.");
  }

  return scores.sort((a, b) => b.score - a.score || a.role.title.localeCompare(b.role.title));
}

function toMillis(timestamp: number): number {
  return Math.abs(timestamp) < 1000000000000 ? timestamp * 1000 : timestamp;
}

function normalizeMessage(message: string): string {
  return message.split("\n")[0]?.trim().toLowerCase() || "";
}

function wordCount(message: string): number {
  return message.split(/\s+/).filter(Boolean).length;
}

function percent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function clampShare(value: number): number {
  return Math.max(0, Math.min(1, value));
}

function isVagueMessage(message: string): boolean {
  return /^(fix|update|changes|change|wip|work|stuff|misc|todo|test|final|cleanup)$/i.test(message)
    || wordCount(message) <= 1;
}

function isMergeMessage(message: string): boolean {
  return message.startsWith("merge ") || message.includes(" merge ");
}

function isRefactorMessage(message: string): boolean {
  return /\b(refactor|rework|cleanup|clean up|simplify|split|rename)\b/.test(message);
}

function isFixMessage(message: string): boolean {
  return /\b(fix|bug|hotfix|patch|repair|resolve|regression)\b/.test(message);
}

function isFeatureMessage(message: string): boolean {
  return /\b(feat|feature|add|implement|new|prototype|experiment|spike)\b/.test(message);
}

function isChoreMessage(message: string): boolean {
  return /\b(chore|maintenance|cleanup|format|lint|style|tidy|organize)\b/.test(message);
}

function isReleaseMessage(message: string): boolean {
  return /\b(release|version|tag|changelog|publish|deploy)\b/.test(message);
}

function isDocsMessage(message: string): boolean {
  return /\b(doc|docs|documentation|readme|guide|manual|changelog)\b/.test(message);
}

function isDependencyMessage(message: string): boolean {
  return /\b(deps|dependency|dependencies|package|bump|upgrade|update packages|lockfile)\b/.test(message);
}

function isUiMessage(message: string): boolean {
  return /\b(ui|ux|frontend|view|panel|button|layout|style|theme|visual|canvas|responsive|animation|modal|dialog)\b/.test(message);
}

function isBackendMessage(message: string): boolean {
  return /\b(backend|rust|tauri|api|command|service|server|worker|thread|process)\b/.test(message);
}

function isPerformanceMessage(message: string): boolean {
  return /\b(perf|performance|optimize|optimise|faster|speed|cache|caching|memory|ram|lazy|loading|latency|smooth)\b/.test(message);
}

function isSecurityMessage(message: string): boolean {
  return /\b(security|secure|auth|token|secret|permission|ssh|key|credential|encrypt|password)\b/.test(message);
}

function isAutomationMessage(message: string): boolean {
  return /\b(ci|workflow|build|script|docker|pipeline|automation|release job|github action)\b/.test(message);
}

function isConflictMessage(message: string): boolean {
  return /\b(conflict|resolve conflicts|merge conflict|rebase|cherry-pick|cherry pick)\b/.test(message);
}

function isDataMessage(message: string): boolean {
  return /\b(data|database|schema|migration|model|query|sql|store|storage|index)\b/.test(message);
}

function isDocsPath(path: string): boolean {
  return path.endsWith(".md")
    || path.includes("/docs/")
    || path.includes("/documentation/")
    || path.includes("readme")
    || path.includes("changelog");
}

function isTestPath(path: string): boolean {
  return path.includes("/test/")
    || path.includes("/tests/")
    || path.includes("__tests__")
    || path.includes(".spec.")
    || path.includes(".test.");
}

function isConfigPath(path: string): boolean {
  return path.includes("config")
    || path.endsWith(".json")
    || path.endsWith(".toml")
    || path.endsWith(".yml")
    || path.endsWith(".yaml")
    || path.endsWith(".ini");
}

function isLockPath(path: string): boolean {
  return path.endsWith("package-lock.json")
    || path.endsWith("pnpm-lock.yaml")
    || path.endsWith("yarn.lock")
    || path.endsWith("cargo.lock")
    || path.endsWith("poetry.lock");
}

function isUiPath(path: string): boolean {
  return path.endsWith(".vue")
    || path.endsWith(".tsx")
    || path.endsWith(".jsx")
    || path.endsWith(".css")
    || path.endsWith(".scss")
    || path.endsWith(".sass")
    || path.endsWith(".slint")
    || path.includes("/components/")
    || path.includes("/view/")
    || path.includes("/views/")
    || path.includes("/ui/")
    || path.includes("/style/")
    || path.includes("/styles/")
    || path.includes("/theme/");
}

function isBackendPath(path: string): boolean {
  return path.endsWith(".rs")
    || path.endsWith(".go")
    || path.includes("/src-tauri/")
    || path.includes("/backend/")
    || path.includes("/server/")
    || path.includes("/services/")
    || path.includes("/domain/")
    || path.includes("/commands/");
}

function isAutomationPath(path: string): boolean {
  return path.includes("/.github/")
    || path.includes("/workflows/")
    || path.includes("/scripts/")
    || path.includes("/ci/")
    || path.includes("/build/")
    || path.endsWith("dockerfile")
    || path.endsWith("docker-compose.yml")
    || path.endsWith("docker-compose.yaml")
    || path.includes("vite.config")
    || path.includes("tauri.conf");
}

function isSecurityPath(path: string): boolean {
  return path.includes("auth")
    || path.includes("token")
    || path.includes("secret")
    || path.includes("credential")
    || path.includes("permission")
    || path.includes("security")
    || path.includes("ssh")
    || path.includes("keychain")
    || path.includes("keyring");
}

function isDataPath(path: string): boolean {
  return path.endsWith(".sql")
    || path.includes("/migrations/")
    || path.includes("/schema/")
    || path.includes("/models/")
    || path.includes("/database/")
    || path.includes("/db/")
    || path.includes("/store/")
    || path.includes("/storage/")
    || path.includes("prisma");
}
