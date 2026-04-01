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

function parseRemoteHost(url: string): { host: string; hostWithPort: string } | null {
  const value = url.trim();
  if (!value) return null;

  const scpLikeMatch = /^[^@]+@([^:]+):(.+)$/.exec(value);
  if (scpLikeMatch) {
    const host = scpLikeMatch[1].toLowerCase();
    return { host, hostWithPort: host };
  }

  try {
    const parsed = new URL(value);
    return {
      host: parsed.hostname.toLowerCase(),
      hostWithPort: parsed.host.toLowerCase(),
    };
  } catch {
    return null;
  }
}

function normalizeDomainInput(value: string): { host: string; hostWithPort: string } | null {
  const trimmed = value.trim();
  if (!trimmed) return null;

  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  try {
    const parsed = new URL(candidate);
    if (!parsed.hostname) return null;
    return {
      host: parsed.hostname.toLowerCase(),
      hostWithPort: parsed.host.toLowerCase(),
    };
  } catch {
    const withoutProtocol = trimmed.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "");
    const firstSegment = withoutProtocol.split("/")[0].trim().toLowerCase();
    if (!firstSegment) return null;

    const host = firstSegment.split(":")[0];
    if (!host) return null;

    return {
      host,
      hostWithPort: firstSegment,
    };
  }
}

function parseGitlabSelfToken(storedValue: string | null | undefined): { domain: string; token: string } | null {
  if (!storedValue) return null;
  const separatorIndex = storedValue.indexOf("|");
  if (separatorIndex <= 0) return null;

  const domain = storedValue.slice(0, separatorIndex).trim();
  const token = storedValue.slice(separatorIndex + 1).trim();
  if (!domain || !token) return null;

  return { domain, token };
}

function domainMatchesRemote(
  remote: { host: string; hostWithPort: string },
  domainRaw: string,
): boolean {
  const normalizedDomain = normalizeDomainInput(domainRaw);
  if (!normalizedDomain) return false;

  return (
    normalizedDomain.hostWithPort === remote.hostWithPort
    || normalizedDomain.host === remote.host
  );
}

function fallbackTokenByUrl(state: GitState, urlLower: string): string | null {
  if (urlLower.includes("bitbucket.org")) return state.providerTokens.value["bitbucket"] || null;
  if (urlLower.includes("dev.azure.com") || urlLower.includes("visualstudio.com")) {
    return state.providerTokens.value["azure"] || null;
  }

  return getTokenParam(state);
}

function gitlabTokenForRemote(
  state: GitState,
  remote: { host: string; hostWithPort: string },
  urlLower: string,
): string | null | undefined {
  if (remote.host === "gitlab.com") {
    return state.providerTokens.value["gitlab"] || null;
  }

  if (!(remote.host.includes("gitlab.") || urlLower.includes("/gitlab"))) {
    return undefined;
  }

  const selfHostedRaw = state.providerTokens.value["gitlab-self"];
  const selfHostedParsed = parseGitlabSelfToken(selfHostedRaw);

  if (selfHostedParsed && domainMatchesRemote(remote, selfHostedParsed.domain)) {
    return selfHostedParsed.token;
  }

  if (selfHostedRaw && !selfHostedParsed) {
    return selfHostedRaw;
  }

  return state.providerTokens.value["gitlab"] || null;
}

function providerTokenByHost(
  state: GitState,
  remote: { host: string; hostWithPort: string },
): string | null | undefined {
  if (remote.host.includes("bitbucket.org")) return state.providerTokens.value["bitbucket"] || null;
  if (remote.host.includes("dev.azure.com") || remote.host.includes("visualstudio.com")) {
    return state.providerTokens.value["azure"] || null;
  }

  return undefined;
}

export function getTokenForUrl(state: GitState, url?: string): string | null {
  if (!url) return getTokenParam(state);

  const urlLower = url.toLowerCase();
  const remote = parseRemoteHost(url);
  if (!remote) {
    return fallbackTokenByUrl(state, urlLower);
  }

  const gitlabToken = gitlabTokenForRemote(state, remote, urlLower);
  if (gitlabToken !== undefined) {
    return gitlabToken;
  }

  const providerToken = providerTokenByHost(state, remote);
  if (providerToken !== undefined) {
    return providerToken;
  }

  return fallbackTokenByUrl(state, urlLower);
}
