export type RegexRuleKind =
  | "contains"
  | "startsWith"
  | "endsWith"
  | "equals"
  | "oneOf"
  | "digit"
  | "letter"
  | "wordChar"
  | "anyChar"
  | "whitespace"
  | "optional"
  | "oneOrMore"
  | "zeroOrMore"
  | "between"
  | "atLeast"
  | "group"
  | "either"
  | "custom";

export interface RegexRule {
  id: string;
  kind: RegexRuleKind;
  value: string;
  min: number;
  max: number;
  enabled: boolean;
}

export interface RegexOptions {
  wholeMatch: boolean;
  caseInsensitive: boolean;
  multiline: boolean;
  global: boolean;
}

export interface RegexBuildResult {
  pattern: string;
  flags: string;
  explanation: string[];
  error: string | null;
}

export interface RegexRuleMeta {
  kind: RegexRuleKind;
  label: string;
  hint: string;
  value?: "text" | "list" | "raw";
  placeholder?: string;
  repeatsPrevious?: boolean;
  range?: boolean;
}

export const REGEX_RULE_KINDS: RegexRuleMeta[] = [
  { kind: "contains", label: "Contains text", hint: "The text must contain the exact text you type (special characters are escaped for you).", value: "text", placeholder: "e.g. api/" },
  { kind: "startsWith", label: "Starts with", hint: "The text must begin with the value you type.", value: "text", placeholder: "e.g. https://" },
  { kind: "endsWith", label: "Ends with", hint: "The text must end with the value you type.", value: "text", placeholder: "e.g. .json" },
  { kind: "equals", label: "Is exactly", hint: "The whole text must be exactly this value.", value: "text", placeholder: "e.g. success" },
  { kind: "oneOf", label: "Is one of", hint: "The text must be one of the listed words. Separate them with commas or new lines.", value: "list", placeholder: "draft, review, published" },
  { kind: "digit", label: "Any digit", hint: "Matches a single number 0 to 9.", repeatsPrevious: false },
  { kind: "letter", label: "Any letter", hint: "Matches a single letter a to z (upper or lower case)." },
  { kind: "wordChar", label: "Any word character", hint: "Matches a letter, a digit or an underscore." },
  { kind: "anyChar", label: "Any character", hint: "Matches any single character (including spaces and symbols)." },
  { kind: "whitespace", label: "Whitespace", hint: "Matches a space, tab or line break." },
  { kind: "optional", label: "Make previous optional", hint: "The previous step may appear zero or one time.", repeatsPrevious: true },
  { kind: "oneOrMore", label: "Repeat previous 1+ times", hint: "The previous step must appear at least once.", repeatsPrevious: true },
  { kind: "zeroOrMore", label: "Repeat previous 0+ times", hint: "The previous step may appear any number of times.", repeatsPrevious: true },
  { kind: "between", label: "Repeat previous N–M times", hint: "The previous step appears between a minimum and maximum number of times.", repeatsPrevious: true, range: true },
  { kind: "atLeast", label: "Repeat previous N+ times", hint: "The previous step appears at least a set number of times.", repeatsPrevious: true, range: true },
  { kind: "group", label: "Capture group", hint: "Wrap this text in a capturing group so you can read it back later.", value: "text", placeholder: "text to capture" },
  { kind: "either", label: "Either A or B", hint: "Match one of the listed options. Separate them with commas or new lines.", value: "list", placeholder: "cat, dog" },
  { kind: "custom", label: "Custom pattern", hint: "Advanced: insert a raw regular expression fragment exactly as typed.", value: "raw", placeholder: "e.g. [a-z0-9]+" },
];

export const REGEX_RULE_MAP: Record<RegexRuleKind, RegexRuleMeta> = REGEX_RULE_KINDS.reduce(
  (acc, meta) => {
    acc[meta.kind] = meta;
    return acc;
  },
  {} as Record<RegexRuleKind, RegexRuleMeta>,
);

export function escapeRegex(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function splitList(value: string): string[] {
  return value
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

export function ruleSummary(rule: RegexRule): string {
  const value = rule.value.trim();
  switch (rule.kind) {
    case "contains":
      return value ? `contains the text “${value}”` : "contains text (nothing typed yet)";
    case "startsWith":
      return value ? `starts with “${value}”` : "starts with (nothing typed yet)";
    case "endsWith":
      return value ? `ends with “${value}”` : "ends with (nothing typed yet)";
    case "equals":
      return value ? `is exactly “${value}”` : "is exactly (nothing typed yet)";
    case "oneOf": {
      const items = splitList(rule.value);
      return items.length ? `is one of: ${items.map((item) => `“${item}”`).join(", ")}` : "is one of (nothing listed yet)";
    }
    case "digit":
      return "contains a digit (0–9)";
    case "letter":
      return "contains a letter (a–z or A–Z)";
    case "wordChar":
      return "contains a word character (letter, digit or underscore)";
    case "anyChar":
      return "contains any character";
    case "whitespace":
      return "contains whitespace (space, tab or line break)";
    case "optional":
      return "makes the previous step optional";
    case "oneOrMore":
      return "repeats the previous step one or more times";
    case "zeroOrMore":
      return "repeats the previous step zero or more times";
    case "between":
      return `repeats the previous step ${rule.min} to ${rule.max} times`;
    case "atLeast":
      return `repeats the previous step at least ${rule.min} times`;
    case "group":
      return value ? `captures the text “${value}”` : "captures a group (nothing typed yet)";
    case "either": {
      const items = splitList(rule.value);
      return items.length ? `matches either ${items.map((item) => `“${item}”`).join(" or ")}` : "matches either (nothing listed yet)";
    }
    case "custom":
      return value ? `uses your custom pattern “${value}”` : "uses a custom pattern (nothing typed yet)";
    default:
      return "";
  }
}

function wrapPrevious(fragments: string[], quantifier: string, note: string, explanation: string[]): void {
  if (fragments.length === 0) {
    explanation.push(`${note} — but there is no previous step to repeat yet.`);
    return;
  }
  const last = fragments[fragments.length - 1];
  fragments[fragments.length - 1] = `(?:${last})${quantifier}`;
  explanation.push(note);
}

export function buildRegex(rules: RegexRule[], options: RegexOptions): RegexBuildResult {
  const fragments: string[] = [];
  const explanation: string[] = [];

  for (const rule of rules) {
    if (!rule.enabled) continue;
    const value = rule.value.trim();
    switch (rule.kind) {
      case "contains":
        if (!value) break;
        fragments.push(escapeRegex(value));
        explanation.push(ruleSummary(rule));
        break;
      case "startsWith":
        if (!value) break;
        fragments.push(`^${escapeRegex(value)}`);
        explanation.push(ruleSummary(rule));
        break;
      case "endsWith":
        if (!value) break;
        fragments.push(`${escapeRegex(value)}$`);
        explanation.push(ruleSummary(rule));
        break;
      case "equals":
        if (!value) break;
        fragments.push(`^${escapeRegex(value)}$`);
        explanation.push(ruleSummary(rule));
        break;
      case "oneOf": {
        const items = splitList(rule.value).map(escapeRegex);
        if (!items.length) break;
        fragments.push(`(?:${items.join("|")})`);
        explanation.push(ruleSummary(rule));
        break;
      }
      case "digit":
        fragments.push("\\d");
        explanation.push(ruleSummary(rule));
        break;
      case "letter":
        fragments.push("[A-Za-z]");
        explanation.push(ruleSummary(rule));
        break;
      case "wordChar":
        fragments.push("\\w");
        explanation.push(ruleSummary(rule));
        break;
      case "anyChar":
        fragments.push(".");
        explanation.push(ruleSummary(rule));
        break;
      case "whitespace":
        fragments.push("\\s");
        explanation.push(ruleSummary(rule));
        break;
      case "optional":
        wrapPrevious(fragments, "?", ruleSummary(rule), explanation);
        break;
      case "oneOrMore":
        wrapPrevious(fragments, "+", ruleSummary(rule), explanation);
        break;
      case "zeroOrMore":
        wrapPrevious(fragments, "*", ruleSummary(rule), explanation);
        break;
      case "between": {
        const min = Math.max(0, Math.min(rule.min, rule.max));
        const max = Math.max(rule.min, rule.max);
        wrapPrevious(fragments, `{${min},${max}}`, ruleSummary(rule), explanation);
        break;
      }
      case "atLeast": {
        const min = Math.max(0, rule.min);
        wrapPrevious(fragments, `{${min},}`, ruleSummary(rule), explanation);
        break;
      }
      case "group":
        if (!value) break;
        fragments.push(`(${escapeRegex(value)})`);
        explanation.push(ruleSummary(rule));
        break;
      case "either": {
        const items = splitList(rule.value).map(escapeRegex);
        if (!items.length) break;
        fragments.push(`(?:${items.join("|")})`);
        explanation.push(ruleSummary(rule));
        break;
      }
      case "custom":
        if (!rule.value.trim()) break;
        fragments.push(rule.value.trim());
        explanation.push(ruleSummary(rule));
        break;
      default:
        break;
    }
  }

  let pattern = fragments.join("");
  if (options.wholeMatch && pattern) {
    pattern = `^(?:${pattern})$`;
    explanation.unshift("The whole input must match from start to finish.");
  }

  const flags = `${options.global ? "g" : ""}${options.caseInsensitive ? "i" : ""}${options.multiline ? "m" : ""}`;

  let error: string | null = null;
  if (pattern) {
    try {
      new RegExp(pattern, flags);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  return { pattern, flags, explanation, error };
}

export interface RegexMatchResult {
  matches: { index: number; text: string; groups: (string | undefined)[] }[];
  error: string | null;
}

export function testRegex(pattern: string, flags: string, sample: string): RegexMatchResult {
  if (!pattern) return { matches: [], error: null };
  const testFlags = flags.includes("g") ? flags : `${flags}g`;
  try {
    const regex = new RegExp(pattern, testFlags);
    const matches: RegexMatchResult["matches"] = [];
    let match: RegExpExecArray | null;
    let guard = 0;
    while ((match = regex.exec(sample)) !== null) {
      matches.push({ index: match.index, text: match[0], groups: match.slice(1) });
      if (match[0] === "") {
        regex.lastIndex += 1;
      }
      guard += 1;
      if (guard > 5000) break;
    }
    return { matches, error: null };
  } catch (err) {
    return { matches: [], error: err instanceof Error ? err.message : String(err) };
  }
}

export function createRule(kind: RegexRuleKind, id: string): RegexRule {
  return { id, kind, value: "", min: 1, max: 3, enabled: true };
}
