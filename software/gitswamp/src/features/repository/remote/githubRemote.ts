export type DiscussionKind = "conversation" | "review";

export interface RemoteComment {
  id: number;
  author: string;
  body: string;
  createdAt: string;
  url: string;
  path: string;
  line?: number;
}

export interface RemoteDiscussionClient {
  key: string;
  canWrite: boolean;
  load(number: number, kind: DiscussionKind, page: number, signal: AbortSignal): Promise<{ comments: RemoteComment[]; more: boolean }>;
  post(number: number, body: string, signal: AbortSignal): Promise<RemoteComment>;
  authenticate(): void;
}

export type BranchComparison = { ahead: number; behind: number };
export interface RemoteBranchCatalog { names: string[]; defaultBranch: string }

type GithubContext = { apiBase: string; owner: string; repo: string; token: string };
const commentPageSize = 30;

async function request(context: GithubContext, resource: string, signal: AbortSignal, body?: string) {
  const controller = new AbortController();
  const abort = () => controller.abort();
  if (signal.aborted) abort();
  signal.addEventListener("abort", abort, { once: true });
  const timer = setTimeout(abort, 10000);
  const headers: Record<string, string> = { Accept: "application/vnd.github+json" };
  if (context.token) headers.Authorization = `Bearer ${context.token}`;
  if (body !== undefined) headers["Content-Type"] = "application/json";
  try {
    const url = `${context.apiBase}/repos/${encodeURIComponent(context.owner)}/${encodeURIComponent(context.repo)}${resource ? `/${resource}` : ""}`;
    const response = await fetch(url, {
      headers, signal: controller.signal, redirect: "error",
      ...(body === undefined ? {} : { method: "POST", body: JSON.stringify({ body }) }),
    });
    if (!response.ok) {
      const message = response.status === 401 ? "GitHub authentication expired. Reconnect in Options."
        : response.status === 403 ? "GitHub denied this request. Check repository permissions or try again later."
        : response.status === 404 ? "GitHub could not find this item with the current credentials."
        : response.status === 422 ? "GitHub rejected the comment. Check the text and whether the conversation is locked."
        : `GitHub request failed (${response.status}).`;
      throw new Error(message);
    }
    return { data: await response.json() as unknown, next: response.headers.get("link") };
  } catch (error) {
    if (controller.signal.aborted && !signal.aborted) {
      throw new Error(body === undefined ? "GitHub did not respond in time." : "No response from GitHub. Refresh the conversation before sending again.");
    }
    throw error;
  } finally {
    clearTimeout(timer);
    signal.removeEventListener("abort", abort);
  }
}

function record(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" ? value as Record<string, unknown> : {};
}

function string(value: unknown): string { return typeof value === "string" ? value : ""; }
function nextPage(link: string | null, count: number, size: number) {
  return link === null ? count === size : /;\s*rel="next"/.test(link);
}

function mapComment(payload: unknown): RemoteComment | null {
  const value = record(payload);
  if (typeof value.id !== "number") return null;
  const url = string(value.html_url);
  const line = value.line ?? value.original_line;
  return {
    id: value.id,
    author: string(record(value.user).login) || "GitHub user",
    body: string(value.body),
    createdAt: string(value.created_at),
    url: /^https:\/\//i.test(url) ? url : "",
    path: string(value.path),
    ...(typeof line === "number" ? { line } : {}),
  };
}

export function githubDiscussion(context: GithubContext, key: string, authenticate: () => void): RemoteDiscussionClient {
  return {
    key, canWrite: !!context.token, authenticate,
    async load(number, kind, page, signal) {
      const route = kind === "review" ? `pulls/${number}/comments` : `issues/${number}/comments`;
      const { data, next } = await request(context, `${route}?per_page=${commentPageSize}&page=${page}`, signal);
      if (!Array.isArray(data)) throw new Error("GitHub returned an unexpected comment list.");
      return {
        comments: data.map(mapComment).filter((item): item is RemoteComment => item !== null),
        more: nextPage(next, data.length, commentPageSize),
      };
    },
    async post(number, body, signal) {
      if (!context.token) throw new Error("Connect GitHub before commenting.");
      const { data } = await request(context, `issues/${number}/comments`, signal, body);
      const comment = mapComment(data);
      if (!comment) throw new Error("GitHub returned no comment. Refresh the conversation before sending again.");
      return comment;
    },
  };
}

export async function githubBranches(context: GithubContext, signal: AbortSignal): Promise<RemoteBranchCatalog> {
  const metadata = request(context, "", signal).then(({ data }) => string(record(data).default_branch)).catch(() => "");
  const names = new Set<string>();
  for (let page = 1; !signal.aborted; page++) {
    const { data, next } = await request(context, `branches?per_page=100&page=${page}`, signal);
    if (!Array.isArray(data)) throw new Error("GitHub returned an unexpected branch list.");
    const before = names.size;
    for (const item of data) {
      const name = string(record(item).name);
      if (name) names.add(name);
    }
    if (before === names.size || !nextPage(next, data.length, 100)) break;
  }
  return { names: [...names], defaultBranch: await metadata };
}

export async function githubBranchComparison(context: GithubContext, base: string, head: string, signal: AbortSignal): Promise<BranchComparison> {
  const { data } = await request(context, `compare/${encodeURIComponent(base)}...${encodeURIComponent(head)}?per_page=1`, signal);
  const value = record(data);
  if (typeof value.ahead_by !== "number" || typeof value.behind_by !== "number") throw new Error("Comparison unavailable.");
  return { ahead: value.ahead_by, behind: value.behind_by };
}
