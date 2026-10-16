function words(input: string): string[] {
  return input
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean);
}

export function toCamelCase(input: string): string {
  return words(input)
    .map((word, index) => (index === 0 ? word.toLowerCase() : capitalize(word.toLowerCase())))
    .join("");
}

export function toPascalCase(input: string): string {
  return words(input)
    .map((word) => capitalize(word.toLowerCase()))
    .join("");
}

export function toSnakeCase(input: string): string {
  return words(input)
    .map((word) => word.toLowerCase())
    .join("_");
}

export function toKebabCase(input: string): string {
  return words(input)
    .map((word) => word.toLowerCase())
    .join("-");
}

export function toConstantCase(input: string): string {
  return words(input)
    .map((word) => word.toUpperCase())
    .join("_");
}

export function toTitleCase(input: string): string {
  return words(input)
    .map((word) => capitalize(word.toLowerCase()))
    .join(" ");
}

export function toSentenceCase(input: string): string {
  const lower = input.toLowerCase();
  return lower.replace(/(^\s*\w|[.!?]\s+\w)/g, (match) => match.toUpperCase());
}

export function toDotCase(input: string): string {
  return words(input)
    .map((word) => word.toLowerCase())
    .join(".");
}

function capitalize(word: string): string {
  return word.charAt(0).toUpperCase() + word.slice(1);
}

export type LineOp = "sort-asc" | "sort-desc" | "unique" | "remove-empty" | "trim" | "reverse" | "shuffle" | "number";

export function applyLineOp(input: string, op: LineOp): string {
  const lines = input.replace(/\r\n/g, "\n").split("\n");
  switch (op) {
    case "sort-asc":
      return [...lines].sort((a, b) => a.localeCompare(b)).join("\n");
    case "sort-desc":
      return [...lines].sort((a, b) => b.localeCompare(a)).join("\n");
    case "unique": {
      const seen = new Set<string>();
      return lines.filter((line) => (seen.has(line) ? false : (seen.add(line), true))).join("\n");
    }
    case "remove-empty":
      return lines.filter((line) => line.trim().length > 0).join("\n");
    case "trim":
      return lines.map((line) => line.trim()).join("\n");
    case "reverse":
      return [...lines].reverse().join("\n");
    case "shuffle": {
      const copy = [...lines];
      for (let i = copy.length - 1; i > 0; i -= 1) {
        const j = Math.floor(Math.random() * (i + 1));
        [copy[i], copy[j]] = [copy[j], copy[i]];
      }
      return copy.join("\n");
    }
    case "number":
      return lines.map((line, index) => `${index + 1}. ${line}`).join("\n");
    default:
      return input;
  }
}

export interface TextStats {
  characters: number;
  charactersNoSpaces: number;
  words: number;
  lines: number;
  bytes: number;
}

export function textStats(input: string): TextStats {
  const characters = input.length;
  const charactersNoSpaces = input.replace(/\s/g, "").length;
  const wordCount = (input.match(/\S+/g) ?? []).length;
  const lines = input.length ? input.split(/\n/).length : 0;
  let bytes = 0;
  if (typeof TextEncoder !== "undefined") {
    bytes = new TextEncoder().encode(input).length;
  } else {
    bytes = characters;
  }
  return { characters, charactersNoSpaces, words: wordCount, lines, bytes };
}

export const CASE_TRANSFORMS: { id: string; label: string; run: (input: string) => string }[] = [
  { id: "camel", label: "camelCase", run: toCamelCase },
  { id: "pascal", label: "PascalCase", run: toPascalCase },
  { id: "snake", label: "snake_case", run: toSnakeCase },
  { id: "kebab", label: "kebab-case", run: toKebabCase },
  { id: "constant", label: "CONSTANT_CASE", run: toConstantCase },
  { id: "title", label: "Title Case", run: toTitleCase },
  { id: "sentence", label: "Sentence case", run: toSentenceCase },
  { id: "dot", label: "dot.case", run: toDotCase },
  { id: "upper", label: "UPPERCASE", run: (input) => input.toUpperCase() },
  { id: "lower", label: "lowercase", run: (input) => input.toLowerCase() },
];

export const LINE_OPS: { id: LineOp; label: string }[] = [
  { id: "sort-asc", label: "Sort A→Z" },
  { id: "sort-desc", label: "Sort Z→A" },
  { id: "unique", label: "Remove duplicates" },
  { id: "remove-empty", label: "Remove empty lines" },
  { id: "trim", label: "Trim each line" },
  { id: "reverse", label: "Reverse lines" },
  { id: "shuffle", label: "Shuffle lines" },
  { id: "number", label: "Number lines" },
];
