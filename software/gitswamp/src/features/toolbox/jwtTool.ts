export interface JwtDecoded {
  valid: boolean;
  error: string | null;
  header: unknown;
  payload: unknown;
  signature: string;
}

function base64UrlDecode(segment: string): string {
  const normalized = segment.replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized + "=".repeat((4 - (normalized.length % 4)) % 4);
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

export function decodeJwt(token: string): JwtDecoded {
  const empty: JwtDecoded = { valid: false, error: null, header: null, payload: null, signature: "" };
  const text = token.trim();
  if (!text) return empty;
  const parts = text.split(".");
  if (parts.length < 2) {
    return { ...empty, error: "A JWT needs at least two dot-separated segments." };
  }
  try {
    const header = JSON.parse(base64UrlDecode(parts[0]));
    const payload = JSON.parse(base64UrlDecode(parts[1]));
    return { valid: true, error: null, header, payload, signature: parts[2] ?? "" };
  } catch (err) {
    return { ...empty, error: err instanceof Error ? err.message : String(err) };
  }
}

export interface JwtTimeClaim {
  claim: string;
  date: string;
  relative: string;
  expired: boolean;
}

export function jwtTimeClaims(payload: unknown): JwtTimeClaim[] {
  if (!payload || typeof payload !== "object") return [];
  const record = payload as Record<string, unknown>;
  const claims: { key: string; label: string }[] = [
    { key: "iat", label: "Issued at" },
    { key: "nbf", label: "Not before" },
    { key: "exp", label: "Expires" },
  ];
  const result: JwtTimeClaim[] = [];
  const now = Date.now();
  for (const { key, label } of claims) {
    const value = record[key];
    if (typeof value !== "number") continue;
    const ms = value * 1000;
    const date = new Date(ms);
    const diff = ms - now;
    const abs = Math.abs(diff);
    const units: [string, number][] = [
      ["day", 86400000],
      ["hour", 3600000],
      ["minute", 60000],
      ["second", 1000],
    ];
    let relative = "now";
    for (const [unit, size] of units) {
      if (abs >= size) {
        const amount = Math.floor(abs / size);
        relative = `${amount} ${unit}${amount === 1 ? "" : "s"} ${diff < 0 ? "ago" : "from now"}`;
        break;
      }
    }
    result.push({ claim: label, date: date.toLocaleString(), relative, expired: key === "exp" && diff < 0 });
  }
  return result;
}
