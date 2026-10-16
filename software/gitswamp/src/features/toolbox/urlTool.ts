export interface UrlParam {
  id: string;
  key: string;
  value: string;
}

export interface ParsedUrl {
  protocol: string;
  username: string;
  password: string;
  host: string;
  hostname: string;
  port: string;
  pathname: string;
  hash: string;
  params: UrlParam[];
  valid: boolean;
  error: string | null;
}

let paramSeed = 0;
export function newParam(key = "", value = ""): UrlParam {
  paramSeed += 1;
  return { id: `param-${paramSeed}-${Math.random().toString(36).slice(2, 6)}`, key, value };
}

export function parseUrl(raw: string): ParsedUrl {
  const empty: ParsedUrl = {
    protocol: "", username: "", password: "", host: "", hostname: "", port: "", pathname: "", hash: "",
    params: [], valid: false, error: null,
  };
  const text = raw.trim();
  if (!text) return empty;
  try {
    const url = new URL(text);
    const params = Array.from(url.searchParams.entries()).map(([key, value]) => newParam(key, value));
    return {
      protocol: url.protocol.replace(/:$/, ""),
      username: url.username,
      password: url.password,
      host: url.host,
      hostname: url.hostname,
      port: url.port,
      pathname: url.pathname,
      hash: url.hash.replace(/^#/, ""),
      params,
      valid: true,
      error: null,
    };
  } catch (err) {
    return { ...empty, valid: false, error: err instanceof Error ? err.message : String(err) };
  }
}

export function buildUrl(base: string, params: UrlParam[]): string {
  const text = base.trim();
  if (!text) return "";
  try {
    const url = new URL(text);
    url.search = "";
    for (const param of params) {
      if (!param.key.trim()) continue;
      url.searchParams.append(param.key, param.value);
    }
    return url.toString();
  } catch {
    const query = params
      .filter((param) => param.key.trim())
      .map((param) => `${encodeURIComponent(param.key)}=${encodeURIComponent(param.value)}`)
      .join("&");
    return query ? `${text}${text.includes("?") ? "&" : "?"}${query}` : text;
  }
}

export function safeEncode(input: string): { value: string; error: string | null } {
  try {
    return { value: encodeURIComponent(input), error: null };
  } catch (err) {
    return { value: "", error: err instanceof Error ? err.message : String(err) };
  }
}

export function safeDecode(input: string): { value: string; error: string | null } {
  try {
    return { value: decodeURIComponent(input), error: null };
  } catch (err) {
    return { value: "", error: err instanceof Error ? err.message : String(err) };
  }
}

export function encodeBase64(input: string): { value: string; error: string | null } {
  try {
    const bytes = new TextEncoder().encode(input);
    let binary = "";
    bytes.forEach((byte) => {
      binary += String.fromCharCode(byte);
    });
    return { value: btoa(binary), error: null };
  } catch (err) {
    return { value: "", error: err instanceof Error ? err.message : String(err) };
  }
}

export function decodeBase64(input: string): { value: string; error: string | null } {
  try {
    const binary = atob(input.trim());
    const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
    return { value: new TextDecoder().decode(bytes), error: null };
  } catch (err) {
    return { value: "", error: err instanceof Error ? err.message : String(err) };
  }
}
