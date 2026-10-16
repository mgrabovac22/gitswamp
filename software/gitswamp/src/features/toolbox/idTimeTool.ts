export function generateUuid(): string {
  try {
    if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
      return crypto.randomUUID();
    }
  } catch {
    // fall through
  }
  const bytes = new Uint8Array(16);
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    crypto.getRandomValues(bytes);
  } else {
    for (let i = 0; i < bytes.length; i += 1) bytes[i] = Math.floor(Math.random() * 256);
  }
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0"));
  return `${hex.slice(0, 4).join("")}-${hex.slice(4, 6).join("")}-${hex.slice(6, 8).join("")}-${hex.slice(8, 10).join("")}-${hex.slice(10, 16).join("")}`;
}

export function randomBytes(length: number): Uint8Array {
  const bytes = new Uint8Array(Math.max(1, Math.floor(length)));
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    crypto.getRandomValues(bytes);
  } else {
    for (let i = 0; i < bytes.length; i += 1) bytes[i] = Math.floor(Math.random() * 256);
  }
  return bytes;
}

export function generateHexToken(bytes = 16): string {
  return Array.from(randomBytes(bytes), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export function generateBase64UrlToken(bytes = 24): string {
  const raw = randomBytes(bytes);
  let binary = "";
  raw.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function generateNumericCode(digits = 6): string {
  const size = Math.max(1, Math.min(digits, 18));
  let out = "";
  for (let i = 0; i < size; i += 1) out += Math.floor(Math.random() * 10).toString();
  return out;
}

export interface TimeParseResult {
  valid: boolean;
  ms: number | null;
  error: string | null;
}

export function parseTimestamp(input: string): TimeParseResult {
  const text = input.trim();
  if (!text) return { valid: false, ms: null, error: null };
  if (/^-?\d+$/.test(text)) {
    const numeric = Number(text);
    const ms = Math.abs(numeric) < 1e12 ? numeric * 1000 : numeric;
    return { valid: true, ms, error: null };
  }
  const parsed = Date.parse(text);
  if (Number.isNaN(parsed)) return { valid: false, ms: null, error: "Could not understand that date or timestamp." };
  return { valid: true, ms: parsed, error: null };
}

export function relativeFromNow(ms: number): string {
  const diff = Date.now() - ms;
  const future = diff < 0;
  const abs = Math.abs(diff);
  const units: [string, number][] = [
    ["year", 31536000000],
    ["month", 2592000000],
    ["day", 86400000],
    ["hour", 3600000],
    ["minute", 60000],
    ["second", 1000],
  ];
  for (const [label, size] of units) {
    if (abs >= size) {
      const count = Math.floor(abs / size);
      return `${count} ${label}${count === 1 ? "" : "s"} ${future ? "from now" : "ago"}`;
    }
  }
  return "just now";
}

export interface TimeParts {
  epochSeconds: string;
  epochMillis: string;
  iso: string;
  local: string;
  relative: string;
}

export function describeTimestamp(ms: number): TimeParts {
  const date = new Date(ms);
  return {
    epochSeconds: String(Math.floor(ms / 1000)),
    epochMillis: String(ms),
    iso: date.toISOString(),
    local: date.toLocaleString(),
    relative: relativeFromNow(ms),
  };
}
