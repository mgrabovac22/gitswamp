import type { FileStatusInfo } from "@/types";

import type { GitState } from "./gitState";

export function statusHash(files: FileStatusInfo[]): string {
  return files
    .map((f) => f.path + ":" + f.status + ":" + f.staged + ":" + (!!f.conflicted))
    .join("|");
}

export function isAuthenticationError(message: string): boolean {
  const m = message.toLowerCase();
  return (
    m.includes("authentication") ||
    m.includes("auth") ||
    m.includes("permission denied") ||
    m.includes("access denied") ||
    m.includes("http 401") ||
    m.includes("http 403") ||
    m.includes("could not read username") ||
    m.includes("requires authentication") ||
    m.includes("invalid credentials")
  );
}

export function isRemoteBehindPushError(message: string): boolean {
  const m = message.toLowerCase();
  return (
    m.includes("non-fast-forward") ||
    m.includes("non fast-forward") ||
    m.includes("non-fastforward") ||
    m.includes("non fastforward") ||
    m.includes("non-fastforwardable") ||
    m.includes("non fastforwardable") ||
    m.includes("cannot push non-fastforwardable reference") ||
    m.includes("cannot push non-fast-forwardable reference") ||
    m.includes("failed to push some refs") ||
    m.includes("fetch first") ||
    m.includes("tip of your current branch is behind") ||
    (m.includes("rejected") && m.includes("push"))
  );
}

export function getOriginUrl(state: GitState): string | undefined {
  return state.repoInfo.value?.remotes?.find((r) => r.name === "origin")?.url;
}

export function getTokenParam(state: GitState): string | null {
  return state.githubToken.value || null;
}

export function getTokenForUrl(state: GitState, url?: string): string | null {
  if (!url) return getTokenParam(state);
  if (url.includes("gitlab.") || url.includes("/gitlab")) {
    const stored = state.providerTokens.value["gitlab-self"];
    if (stored && stored.includes("|")) {
      const parts = stored.split("|");
      return parts[1] || null;
    }
    return state.providerTokens.value["gitlab"] || null;
  }
  if (url.includes("gitlab.com")) return state.providerTokens.value["gitlab"] || null;
  if (url.includes("bitbucket.org")) return state.providerTokens.value["bitbucket"] || null;
  if (url.includes("dev.azure.com") || url.includes("visualstudio.com")) {
    return state.providerTokens.value["azure"] || null;
  }
  return getTokenParam(state);
}
