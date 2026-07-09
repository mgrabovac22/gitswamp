export interface IdentityGuardMismatch {
  currentEmail: string;
  globalEmail: string;
  reason: string;
  remoteUrl: string;
  suggestedEmail: string | null;
}

const PERSONAL_EMAIL_DOMAINS = new Set([
  "aol.com",
  "gmx.com",
  "gmail.com",
  "googlemail.com",
  "hotmail.com",
  "icloud.com",
  "live.com",
  "mac.com",
  "mail.com",
  "me.com",
  "outlook.com",
  "pm.me",
  "proton.me",
  "protonmail.com",
  "yahoo.com",
  "yandex.com",
]);

const PUBLIC_GIT_HOSTS = new Set([
  "bitbucket.org",
  "dev.azure.com",
  "github.com",
  "gitlab.com",
  "ssh.dev.azure.com",
]);

const TOKEN_STOP_WORDS = new Set([
  "azure",
  "bitbucket",
  "com",
  "dev",
  "git",
  "github",
  "gitlab",
  "http",
  "https",
  "io",
  "net",
  "org",
  "repo",
  "repos",
  "repository",
  "ssh",
  "www",
]);

const WORK_REMOTE_HINTS = new Set([
  "agency",
  "cloud",
  "company",
  "corp",
  "enterprise",
  "group",
  "inc",
  "internal",
  "labs",
  "llc",
  "ltd",
  "product",
  "software",
  "studio",
  "studios",
  "systems",
  "team",
  "teams",
  "tech",
]);

function stripGitSuffix(path: string): string {
  return path.replace(/\.git$/i, "");
}

function parseRemoteHostAndPath(remoteUrl: string): { host: string; hostWithPort: string; path: string } | null {
  const value = remoteUrl.trim();
  if (!value) return null;

  const scpLikeMatch = value.match(/^[^@]+@([^:]+):(.+)$/);
  if (scpLikeMatch) {
    const host = scpLikeMatch[1].toLowerCase();
    return {
      host,
      hostWithPort: host,
      path: stripGitSuffix(scpLikeMatch[2].replace(/^\/+/, "")),
    };
  }

  try {
    const parsed = new URL(value);
    return {
      host: parsed.hostname.toLowerCase(),
      hostWithPort: parsed.host.toLowerCase(),
      path: stripGitSuffix(parsed.pathname.replace(/^\/+/, "")),
    };
  } catch {
    return null;
  }
}

export function normalizeIdentityEmail(value: string): string {
  const match = value.match(/[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}/i);
  return (match?.[0] || value).trim().toLowerCase();
}

function getEmailDomain(email: string): string {
  const normalized = normalizeIdentityEmail(email);
  const separatorIndex = normalized.lastIndexOf("@");
  return separatorIndex >= 0 ? normalized.slice(separatorIndex + 1) : "";
}

function getEmailLocalPart(email: string): string {
  const normalized = normalizeIdentityEmail(email);
  const separatorIndex = normalized.lastIndexOf("@");
  return separatorIndex >= 0 ? normalized.slice(0, separatorIndex) : normalized;
}

function isPersonalIdentityEmail(email: string): boolean {
  const domain = getEmailDomain(email);
  return !domain || PERSONAL_EMAIL_DOMAINS.has(domain);
}

function tokenizeIdentityText(value: string): string[] {
  return value
    .toLowerCase()
    .replace(/\.git\b/g, " ")
    .split(/[^a-z0-9]+/)
    .map((token) => token.trim())
    .filter((token) => token.length > 1 && !/^\d+$/.test(token) && !TOKEN_STOP_WORDS.has(token));
}

function getEmailLocalTokens(email: string): string[] {
  return tokenizeIdentityText(getEmailLocalPart(email));
}

function getEmailDomainTokens(email: string): string[] {
  const domain = getEmailDomain(email);
  if (!domain || PERSONAL_EMAIL_DOMAINS.has(domain)) {
    return [];
  }
  return tokenizeIdentityText(domain.split(".")[0] || domain);
}

function getRemoteIdentityTokens(remoteUrl: string): string[] {
  const parsed = parseRemoteHostAndPath(remoteUrl);
  if (!parsed) {
    return tokenizeIdentityText(remoteUrl);
  }
  return tokenizeIdentityText(`${parsed.host} ${parsed.path}`);
}

function getRemoteOwnerTokens(remoteUrl: string): string[] {
  const parsed = parseRemoteHostAndPath(remoteUrl);
  if (!parsed) {
    return [];
  }

  const segments = parsed.path.split("/").filter(Boolean);
  if (parsed.host === "dev.azure.com" && segments.length > 0) {
    return tokenizeIdentityText(segments[0]);
  }
  return tokenizeIdentityText(segments[0] || "");
}

function hasTokenOverlap(left: string[], right: string[]): boolean {
  const rightValues = new Set(right);
  return left.some((token) => rightValues.has(token));
}

function identityEmailDomainMatchesRemote(email: string, remoteUrl: string): boolean {
  const domainTokens = getEmailDomainTokens(email);
  if (domainTokens.length === 0) {
    return false;
  }
  return hasTokenOverlap(domainTokens, getRemoteIdentityTokens(remoteUrl));
}

function remoteLooksOwnedByEmail(remoteUrl: string, email: string): boolean {
  const localTokens = getEmailLocalTokens(email);
  if (localTokens.length === 0) {
    return false;
  }
  return hasTokenOverlap(localTokens, getRemoteOwnerTokens(remoteUrl));
}

function remoteLooksLikeWork(remoteUrl: string, currentEmail: string, globalEmail: string): boolean {
  const parsed = parseRemoteHostAndPath(remoteUrl);
  const host = parsed?.host || "";
  if (host && !PUBLIC_GIT_HOSTS.has(host)) {
    return true;
  }

  const tokens = getRemoteIdentityTokens(remoteUrl);
  if (tokens.some((token) => WORK_REMOTE_HINTS.has(token))) {
    return true;
  }

  if (globalEmail && !isPersonalIdentityEmail(globalEmail) && identityEmailDomainMatchesRemote(globalEmail, remoteUrl)) {
    return true;
  }

  const ownerTokens = getRemoteOwnerTokens(remoteUrl);
  return isPersonalIdentityEmail(currentEmail)
    && ownerTokens.length >= 2
    && !remoteLooksOwnedByEmail(remoteUrl, currentEmail);
}

export function detectGitIdentityMismatch(
  currentEmailRaw: string,
  globalEmailRaw: string,
  remoteUrl: string,
): IdentityGuardMismatch | null {
  const currentEmail = normalizeIdentityEmail(currentEmailRaw);
  const globalEmail = normalizeIdentityEmail(globalEmailRaw);
  const suggestedEmail = globalEmail && globalEmail !== currentEmail ? globalEmail : null;

  if (!currentEmail || !currentEmail.includes("@")) {
    return {
      currentEmail,
      globalEmail,
      reason: "No Git user.email is configured for this repository.",
      remoteUrl,
      suggestedEmail,
    };
  }

  const currentLooksPersonal = isPersonalIdentityEmail(currentEmail);
  const globalLooksPersonal = globalEmail ? isPersonalIdentityEmail(globalEmail) : false;
  const currentDomainMatchesRemote = identityEmailDomainMatchesRemote(currentEmail, remoteUrl);
  const globalDomainMatchesRemote = globalEmail ? identityEmailDomainMatchesRemote(globalEmail, remoteUrl) : false;
  const looksLikeWork = remoteLooksLikeWork(remoteUrl, currentEmail, globalEmail);

  if (currentLooksPersonal && looksLikeWork) {
    return {
      currentEmail,
      globalEmail,
      reason: "This remote looks like a team or company repository, but the active Git email looks personal.",
      remoteUrl,
      suggestedEmail: globalEmail && !globalLooksPersonal ? globalEmail : suggestedEmail,
    };
  }

  if (!currentLooksPersonal && globalEmail && globalDomainMatchesRemote && !currentDomainMatchesRemote) {
    return {
      currentEmail,
      globalEmail,
      reason: "The remote name matches another configured Git email better than the active repository email.",
      remoteUrl,
      suggestedEmail: globalEmail,
    };
  }

  if (!currentLooksPersonal && globalEmail && globalLooksPersonal && remoteLooksOwnedByEmail(remoteUrl, globalEmail) && !looksLikeWork) {
    return {
      currentEmail,
      globalEmail,
      reason: "This remote looks personal, but the active Git email looks like a work identity.",
      remoteUrl,
      suggestedEmail: globalEmail,
    };
  }

  return null;
}

export function shortenIdentityRemote(remoteUrl: string): string {
  const parsed = parseRemoteHostAndPath(remoteUrl);
  if (!parsed) {
    return remoteUrl.length > 96 ? `${remoteUrl.slice(0, 93)}...` : remoteUrl;
  }

  const value = `${parsed.host}/${parsed.path}`;
  return value.length > 96 ? `${value.slice(0, 93)}...` : value;
}
