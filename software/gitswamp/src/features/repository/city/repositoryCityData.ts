import { invoke } from "@tauri-apps/api/core";
import type {
  RepositoryCityContributor,
  RepositoryCityFile,
  RepositoryCitySnapshot,
} from "./repositoryCity.types";

const FIX_SIGNAL_RE = /\b(fix(?:ed|es)?|bug|hotfix|regression|revert|rollback|repair)\b/i;
const MAX_VISUAL_FILES = 5000;
const SNAPSHOT_CACHE_LIMIT = 2;

interface FileActivity {
  touches: number;
  churn: number;
  fixTouches: number;
  lastAuthor: string;
  lastAuthorEmail: string;
  lastChangedAt: number;
  lastCommitSha: string;
}

interface ContributorAccumulator {
  name: string;
  email: string;
  commits: Set<string>;
  touches: number;
  lastActiveAt: number;
  pathTouches: Map<string, number>;
}

const snapshotCache = new Map<string, RepositoryCitySnapshot>();

function cacheGet(key: string): RepositoryCitySnapshot | null {
  const value = snapshotCache.get(key);
  if (!value) return null;
  snapshotCache.delete(key);
  snapshotCache.set(key, value);
  return value;
}

function cacheSet(key: string, value: RepositoryCitySnapshot) {
  snapshotCache.delete(key);
  snapshotCache.set(key, value);
  while (snapshotCache.size > SNAPSHOT_CACHE_LIMIT) {
    const oldest = snapshotCache.keys().next().value;
    if (!oldest) break;
    snapshotCache.delete(oldest);
  }
}

function parseTree(output: string): Map<string, number> {
  const files = new Map<string, number>();
  for (const entry of output.split("\0")) {
    if (!entry) continue;
    const tab = entry.indexOf("\t");
    if (tab < 0) continue;
    const metadata = entry.slice(0, tab).trim().split(/\s+/);
    const path = entry.slice(tab + 1);
    const size = Number(metadata[3]);
    if (path) files.set(path, Number.isFinite(size) ? size : 0);
  }
  return files;
}

function parsePathList(output: string): Map<string, number> {
  const files = new Map<string, number>();
  for (const path of output.split("\0")) {
    if (path) files.set(path, 0);
  }
  return files;
}

function contributorKey(name: string, email: string): string {
  return `${email.trim().toLowerCase()}::${name.trim().toLowerCase()}`;
}

function yieldToUi(): Promise<void> {
  return new Promise((resolve) => {
    globalThis.setTimeout(resolve, 0);
  });
}

async function parseActivity(output: string): Promise<{
  files: Map<string, FileActivity>;
  contributors: RepositoryCityContributor[];
  sampledCommits: number;
}> {
  const files = new Map<string, FileActivity>();
  const contributors = new Map<string, ContributorAccumulator>();
  let sampledCommits = 0;
  let processedRecords = 0;
  let cursor = 0;

  while (cursor < output.length) {
    const start = output.indexOf("\u001e", cursor);
    if (start < 0) break;
    const next = output.indexOf("\u001e", start + 1);
    const record = output.slice(start + 1, next < 0 ? output.length : next);
    cursor = next < 0 ? output.length : next;
    if (!record.trim()) continue;
    processedRecords += 1;
    if (processedRecords % 90 === 0) await yieldToUi();

    const firstLineBreak = record.indexOf("\n");
    const header = (firstLineBreak >= 0 ? record.slice(0, firstLineBreak) : record).trim();
    const [sha = "", author = "Unknown", email = "", timestampRaw = "0", subject = ""] = header.split("\u001f");
    if (!sha) continue;
    sampledCommits += 1;

    const timestamp = Number(timestampRaw) || 0;
    const isFix = FIX_SIGNAL_RE.test(subject);
    const contributorId = contributorKey(author, email);
    let contributor = contributors.get(contributorId);
    if (!contributor) {
      contributor = {
        name: author || "Unknown",
        email,
        commits: new Set<string>(),
        touches: 0,
        lastActiveAt: timestamp,
        pathTouches: new Map<string, number>(),
      };
      contributors.set(contributorId, contributor);
    }
    contributor.commits.add(sha);
    contributor.lastActiveAt = Math.max(contributor.lastActiveAt, timestamp);

    const body = firstLineBreak >= 0 ? record.slice(firstLineBreak + 1) : "";
    for (const line of body.split(/\r?\n/)) {
      const match = line.match(/^(-|\d+)\t(-|\d+)\t(.+)$/);
      if (!match) continue;
      const path = match[3];
      const additions = match[1] === "-" ? 0 : Number(match[1]) || 0;
      const deletions = match[2] === "-" ? 0 : Number(match[2]) || 0;
      const existing = files.get(path);
      if (existing) {
        existing.touches += 1;
        existing.churn += additions + deletions;
        if (isFix) existing.fixTouches += 1;
      } else {
        files.set(path, {
          touches: 1,
          churn: additions + deletions,
          fixTouches: isFix ? 1 : 0,
          lastAuthor: author || "Unknown",
          lastAuthorEmail: email,
          lastChangedAt: timestamp,
          lastCommitSha: sha,
        });
      }

      contributor.touches += 1;
      contributor.pathTouches.set(path, (contributor.pathTouches.get(path) ?? 0) + 1);
    }
  }

  const contributorRows = Array.from(contributors.entries()).map(([id, item]) => {
    let primaryPath = "";
    let primaryTouches = -1;
    item.pathTouches.forEach((touches, path) => {
      if (touches > primaryTouches) {
        primaryPath = path;
        primaryTouches = touches;
      }
    });
    return {
      id,
      name: item.name,
      email: item.email,
      commits: item.commits.size,
      touches: item.touches,
      primaryPath,
      lastActiveAt: item.lastActiveAt,
    };
  }).sort((a, b) => b.lastActiveAt - a.lastActiveAt || b.touches - a.touches);

  return { files, contributors: contributorRows, sampledCommits };
}

function folderForPath(path: string): string {
  const segments = path.split("/");
  return segments.length > 1 ? segments[0] : "Repository root";
}

function buildFiles(
  tree: Map<string, number>,
  activity: Map<string, FileActivity>,
): { files: RepositoryCityFile[]; omittedFiles: number } {
  const rows: RepositoryCityFile[] = Array.from(tree.entries()).map(([path, size]) => {
    const stats = activity.get(path);
    return {
      path,
      folder: folderForPath(path),
      size,
      touches: stats?.touches ?? 0,
      churn: stats?.churn ?? 0,
      fixTouches: stats?.fixTouches ?? 0,
      heat: 0,
      lastAuthor: stats?.lastAuthor ?? "",
      lastAuthorEmail: stats?.lastAuthorEmail ?? "",
      lastChangedAt: stats?.lastChangedAt ?? 0,
      lastCommitSha: stats?.lastCommitSha ?? "",
    };
  });

  let maxTouches = 1;
  let maxChurn = 1;
  let maxFixes = 1;
  for (const item of rows) {
    maxTouches = Math.max(maxTouches, item.touches);
    maxChurn = Math.max(maxChurn, item.churn);
    maxFixes = Math.max(maxFixes, item.fixTouches);
  }
  for (const item of rows) {
    const touchScore = Math.log1p(item.touches) / Math.log1p(maxTouches);
    const churnScore = Math.log1p(item.churn) / Math.log1p(maxChurn);
    const fixScore = item.fixTouches / maxFixes;
    item.heat = Math.min(1, touchScore * 0.54 + churnScore * 0.23 + fixScore * 0.23);
  }

  if (rows.length <= MAX_VISUAL_FILES) {
    return { files: rows, omittedFiles: 0 };
  }

  rows.sort((a, b) => {
    const activityDelta = b.touches - a.touches;
    if (activityDelta !== 0) return activityDelta;
    return b.size - a.size;
  });
  return {
    files: rows.slice(0, MAX_VISUAL_FILES),
    omittedFiles: rows.length - MAX_VISUAL_FILES,
  };
}

async function git(path: string, args: string[]): Promise<string> {
  return invoke<string>("run_git_command", { path, args });
}

function isBadConfigError(error: unknown): boolean {
  return String(error).toLowerCase().includes("bad config");
}

function normalizeGitError(error: unknown): Error {
  const message = String(error);
  if (isBadConfigError(error)) {
    return new Error(`${message}. Fix the repository .git/config file, then refresh Repository City.`);
  }
  return new Error(message);
}

async function resolveCityRevision(
  repoPath: string,
  preferredRef: string,
): Promise<{ refName: string; headSha: string } | null> {
  const candidates = [
    preferredRef,
    "HEAD",
    "refs/heads/main",
    "main",
    "refs/heads/master",
    "master",
    "refs/remotes/origin/main",
    "origin/main",
    "refs/remotes/origin/master",
    "origin/master",
  ].filter(Boolean);
  const seen = new Set<string>();

  for (const candidate of candidates) {
    if (seen.has(candidate)) continue;
    seen.add(candidate);
    try {
      const headSha = (await git(repoPath, ["rev-parse", "--verify", `${candidate}^{commit}`])).trim();
      if (headSha) return { refName: candidate, headSha };
    } catch (error) {
      if (isBadConfigError(error)) throw normalizeGitError(error);
    }
  }

  return null;
}

export async function loadRepositoryCity(
  repoPath: string,
  refName: string,
  historyLimit: number,
): Promise<RepositoryCitySnapshot> {
  const revision = await resolveCityRevision(repoPath, refName);
  if (!revision) {
    const [pathOutput, userName, userEmail] = await Promise.all([
      git(repoPath, ["ls-files", "-z", "--cached", "--others", "--exclude-standard"]).catch((error) => {
        throw normalizeGitError(error);
      }),
      git(repoPath, ["config", "user.name"]).catch(() => ""),
      git(repoPath, ["config", "user.email"]).catch(() => ""),
    ]);
    const { files, omittedFiles } = buildFiles(parsePathList(pathOutput), new Map());
    return {
      repoPath,
      refName: "working-tree",
      headSha: "",
      files,
      contributors: [],
      sampledCommits: 0,
      omittedFiles,
      userName: userName.trim(),
      userEmail: userEmail.trim(),
    };
  }

  const cacheKey = `${repoPath}::${revision.headSha}::${historyLimit <= 0 ? "all" : historyLimit}`;
  const cached = cacheGet(cacheKey);
  if (cached) return cached;

  const [treeOutput, activityOutput, userName, userEmail] = await Promise.all([
    git(repoPath, ["ls-tree", "-r", "-l", "-z", "--full-tree", revision.headSha]).catch((error) => {
      throw normalizeGitError(error);
    }),
    git(repoPath, [
      "log",
      revision.headSha,
      ...(historyLimit > 0 ? [`--max-count=${historyLimit}`] : []),
      "--date-order",
      "--format=\u001e%H\u001f%an\u001f%ae\u001f%ct\u001f%s",
      "--numstat",
      "--no-renames",
      "--",
    ]).catch((error) => {
      throw normalizeGitError(error);
    }),
    git(repoPath, ["config", "user.name"]).catch(() => ""),
    git(repoPath, ["config", "user.email"]).catch(() => ""),
  ]);

  const tree = parseTree(treeOutput);
  await yieldToUi();
  const activity = await parseActivity(activityOutput);
  await yieldToUi();
  const { files, omittedFiles } = buildFiles(tree, activity.files);
  const snapshot: RepositoryCitySnapshot = {
    repoPath,
    refName: revision.refName,
    headSha: revision.headSha,
    files,
    contributors: activity.contributors.slice(0, 24),
    sampledCommits: activity.sampledCommits,
    omittedFiles,
    userName: userName.trim(),
    userEmail: userEmail.trim(),
  };
  cacheSet(cacheKey, snapshot);
  return snapshot;
}

export async function findLatestFileCommit(
  repoPath: string,
  refName: string,
  filePath: string,
): Promise<string> {
  return (await git(repoPath, [
    "log",
    "-1",
    "--format=%H",
    refName,
    "--",
    filePath,
  ])).trim();
}

export function clearRepositoryCityCache(repoPath?: string) {
  if (!repoPath) {
    snapshotCache.clear();
    return;
  }
  for (const key of snapshotCache.keys()) {
    if (key.startsWith(`${repoPath}::`)) snapshotCache.delete(key);
  }
}
