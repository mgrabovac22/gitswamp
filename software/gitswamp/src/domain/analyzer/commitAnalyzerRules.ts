import type { CommitRule, CommitRuleSeverity } from "./commitAnalyzerTypes";

const MESSAGE_MIN_LENGTH = 10;
const MESSAGE_MAX_LENGTH = 72;
const VAGUE_WORDS: Record<string, string[]> = {
  en: ["change", "changes", "update", "updates", "misc", "stuff", "fixes"],
  hr: ["promjena", "promjene", "izmjene", "update", "sitno", "ostalo"],
  de: ["anderung", "anderungen", "update", "sonstiges"],
  fr: ["changement", "changements", "miseajour", "divers"],
  es: ["cambio", "cambios", "actualizacion", "misc"],
  pt: ["mudanca", "mudancas", "atualizacao", "misc"],
  pl: ["zmiana", "zmiany", "aktualizacja", "inne"],
};

const GIBBERISH_REGEX = /^[a-z]{6,}$/i;
const VOWEL_REGEX = /[aeiouy]/i;
const WIP_ONLY_REGEX = /^wip$/i;
const WIP_PREFIX_REGEX = /^wip\s*[:-]/i;
const CONVENTIONAL_REGEX = /^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9\-_/.]+\))?!?:\s.+/i;
const CONVENTIONAL_SCOPE_REGEX = /^[a-z]+\(([^)]+)\)!?:/i;
const EN_PAST_TENSE_REGEX = /(ed|ing)$/i;
const HR_PAST_TENSE_REGEX = /(ao|la|li|lo)$/i;
const EMOJI_REGEX = /[\u2600-\u27BF]|[\uD83C-\uDBFF][\uDC00-\uDFFF]/;
const ISSUE_CONTEXT_REGEX = /\b(issue|bug|defect|problem|ticket|task|fix|hotfix|incident|resolve|resolved|resolves)\b/i;
const ISSUE_REF_REGEX = /(^|\s)(#\d+|[A-Z]{2,}-\d+)(?=\s|$)/;
const GOOD_DESCRIPTION_REGEX = /\b(why|because|reason|zašto|zasto|jer|kako|how)\b/i;

const SCOPE_ALIASES: Record<string, string[]> = {
  auth: ["auth", "security", "login"],
  ui: ["ui", "frontend", "view", "component"],
  api: ["api", "backend", "server", "routes"],
  docs: ["docs", "documentation"],
  tests: ["test", "tests", "qa"],
  data: ["db", "data", "migration"],
  style: ["style", "css", "theme"],
};

const RULES: CommitRule[] = [
  {
    id: "msg-empty",
    name: "Commit message cannot be empty",
    severity: "error",
    weight: 10,
    languages: ["all"],
    check: ({ message }) => ({
      passed: message.trim().length > 0,
      feedback: "Commit message is required.",
    }),
  },
  {
    id: "msg-length-min",
    name: "Commit message is too short",
    severity: "error",
    weight: 8,
    languages: ["all"],
    check: ({ message }) => ({
      passed: message.trim().length >= MESSAGE_MIN_LENGTH,
      feedback: `Use at least ${MESSAGE_MIN_LENGTH} characters in the message.`,
    }),
  },
  {
    id: "msg-length-max",
    name: "Commit message is too long",
    severity: "error",
    weight: 8,
    languages: ["all"],
    check: ({ message }) => ({
      passed: message.trim().length <= MESSAGE_MAX_LENGTH,
      feedback: `Keep subject under ${MESSAGE_MAX_LENGTH} characters.`,
    }),
  },
  {
    id: "msg-no-period",
    name: "Commit message should not end with a period",
    severity: "error",
    weight: 5,
    languages: ["all"],
    check: ({ message }) => ({
      passed: !message.trim().endsWith("."),
      feedback: "Avoid trailing period in commit subject.",
    }),
  },
  {
    id: "msg-no-wip",
    name: "WIP message needs more detail",
    severity: "error",
    weight: 6,
    languages: ["all"],
    check: ({ message, description }) => {
      const trimmed = message.trim();
      const hasWipOnly = WIP_ONLY_REGEX.test(trimmed);
      const hasWipPrefix = WIP_PREFIX_REGEX.test(trimmed);
      const hasDescription = description.trim().length > 0;
      return {
        passed: !(hasWipOnly && !hasDescription) && (hasWipOnly || hasWipPrefix ? hasDescription : true),
        feedback: "WIP commit requires a descriptive body.",
      };
    },
  },
  {
    id: "msg-no-gibberish",
    name: "Commit message appears to be gibberish",
    severity: "error",
    weight: 7,
    languages: ["all"],
    check: ({ message }) => {
      const normalized = message.trim().toLowerCase();
      if (normalized.length < 6) {
        return { passed: true };
      }
      const looksRandom = GIBBERISH_REGEX.test(normalized) && !VOWEL_REGEX.test(normalized);
      return {
        passed: !looksRandom,
        feedback: "Message looks random. Describe what changed.",
      };
    },
  },
  {
    id: "msg-imperative",
    name: "Prefer imperative commit subject",
    severity: "warning",
    weight: 4,
    languages: ["all"],
    check: ({ message, detectedLanguage }) => ({
      passed: looksImperative(message, detectedLanguage),
      feedback: "Use imperative mood, for example: Fix login token refresh.",
    }),
  },
  {
    id: "msg-no-emoji-only",
    name: "Emoji-only subject is ambiguous",
    severity: "warning",
    weight: 3,
    languages: ["all"],
    check: ({ message }) => ({
      passed: !isEmojiOnly(message),
      feedback: "Emoji-only subject is not descriptive enough.",
    }),
  },
  {
    id: "desc-missing-for-big-diff",
    name: "Description is required for larger staged changes",
    severity: "warning",
    weight: 6,
    languages: ["all"],
    check: ({ description, diffSummary, settings }) => {
      const changedLines = diffSummary.totalLinesAdded + diffSummary.totalLinesRemoved;
      const threshold = Math.max(1, settings.maxDiffLinesForDescWarning);
      const requiresDescription = changedLines > threshold;
      return {
        passed: !requiresDescription || description.trim().length > 0,
        feedback: `Large staged diff (${changedLines} lines). Add a description with why/what/how.`,
      };
    },
  },
  {
    id: "msg-capitalize",
    name: "Capitalize first letter",
    severity: "warning",
    weight: 2,
    languages: ["all"],
    check: ({ message }) => {
      const trimmed = message.trim();
      const first = trimmed.charAt(0);
      const isCapitalized = first.length === 0 || first === first.toUpperCase();
      return {
        passed: isCapitalized,
        feedback: "Start commit subject with a capital letter.",
      };
    },
  },
  {
    id: "msg-vague",
    name: "Avoid vague commit messages",
    severity: "warning",
    weight: 4,
    languages: ["all"],
    check: ({ message, detectedLanguage, settings }) => {
      const normalized = normalizeWord(message);
      const custom = settings.customVagueWords.map((value) => normalizeWord(value)).filter((value) => value.length > 0);
      const languageWords = VAGUE_WORDS[detectedLanguage] || VAGUE_WORDS.en;
      const allVagueWords = new Set([...languageWords, ...VAGUE_WORDS.en, ...custom].map(normalizeWord));
      const vague = allVagueWords.has(normalized);
      return {
        passed: !vague,
        feedback: "Message is too vague. Mention the area and purpose.",
      };
    },
  },
  {
    id: "msg-conventional-format",
    name: "Use Conventional Commits format",
    severity: "warning",
    weight: 3,
    languages: ["all"],
    check: ({ message }) => ({
      passed: CONVENTIONAL_REGEX.test(message.trim()),
      feedback: "Recommended format: type(scope): subject",
    }),
  },
  {
    id: "file-mismatch",
    name: "Message scope should match changed files",
    severity: "warning",
    weight: 4,
    languages: ["all"],
    check: ({ message, diffSummary }) => {
      const scope = extractConventionalScope(message);
      if (!scope || diffSummary.inferredScope === "general") {
        return { passed: true };
      }
      return {
        passed: scopeMatches(diffSummary.inferredScope, scope),
        feedback: `Scope "${scope}" does not match staged files (inferred: ${diffSummary.inferredScope}).`,
      };
    },
  },
  {
    id: "suggest-issue-ref",
    name: "Tag related issue",
    severity: "info",
    weight: 3,
    languages: ["all"],
    check: ({ message, description }) => {
      const combined = `${message} ${description}`;
      const hasIssueContext = ISSUE_CONTEXT_REGEX.test(combined);
      const hasIssueRef = ISSUE_REF_REGEX.test(combined);
      return {
        passed: !hasIssueContext || hasIssueRef,
        feedback: "Tag an issue related to this commit if exists.",
      };
    },
  },
  {
    id: "suggest-conventional",
    name: "Suggest Conventional Commits",
    severity: "info",
    weight: 1,
    languages: ["all"],
    check: ({ message }) => ({
      passed: CONVENTIONAL_REGEX.test(message.trim()),
      feedback: "Tip: use Conventional Commits for clearer history.",
    }),
  },
  {
    id: "desc-quality",
    name: "Description quality hint",
    severity: "info",
    weight: 1,
    languages: ["all"],
    check: ({ description }) => {
      if (description.trim().length === 0) {
        return { passed: true };
      }
      return {
        passed: GOOD_DESCRIPTION_REGEX.test(description),
        feedback: "Tip: include why/how context in description for reviewers.",
      };
    },
  },
];

export function getCommitRules(): CommitRule[] {
  return RULES;
}

export function severityRank(value: CommitRuleSeverity): number {
  switch (value) {
    case "error":
      return 0;
    case "warning":
      return 1;
    default:
      return 2;
  }
}

function normalizeWord(value: string): string {
  const normalized = value.trim().toLowerCase().normalize("NFD");
  let output = "";

  for (const char of normalized) {
    const code = char.codePointAt(0) || 0;
    const isDiacritic = code >= 0x0300 && code <= 0x036f;
    if (isDiacritic) {
      continue;
    }

    const isDigit = code >= 48 && code <= 57;
    const isAsciiLower = code >= 97 && code <= 122;
    if (isDigit || isAsciiLower) {
      output += char;
    }
  }

  return output;
}

function looksImperative(message: string, detectedLanguage: string): boolean {
  const firstWord = message.trim().split(/\s+/)[0] || "";
  if (!firstWord) return true;

  if (detectedLanguage === "hr") {
    return !HR_PAST_TENSE_REGEX.test(firstWord);
  }

  if (detectedLanguage === "en") {
    if (EN_PAST_TENSE_REGEX.test(firstWord)) {
      return false;
    }
  }

  return true;
}

function isEmojiOnly(message: string): boolean {
  const trimmed = message.trim();
  if (!trimmed) {
    return false;
  }

  let emojiCount = 0;
  let letterOrNumberCount = 0;

  for (const char of trimmed) {
    if (EMOJI_REGEX.test(char)) {
      emojiCount += 1;
      continue;
    }
    if (/\p{L}|\p{N}/u.test(char)) {
      letterOrNumberCount += 1;
    }
  }

  return emojiCount > 0 && letterOrNumberCount === 0;
}

function extractConventionalScope(message: string): string | null {
  const match = CONVENTIONAL_SCOPE_REGEX.exec(message.trim());
  if (!match) {
    return null;
  }
  return normalizeWord(match[1]);
}

function scopeMatches(inferredScope: string, scope: string): boolean {
  const normalizedInferred = normalizeWord(inferredScope);
  const normalizedScope = normalizeWord(scope);

  if (normalizedInferred === normalizedScope) {
    return true;
  }

  for (const aliases of Object.values(SCOPE_ALIASES)) {
    const normalizedAliases = new Set(aliases.map(normalizeWord));
    if (normalizedAliases.has(normalizedInferred) && normalizedAliases.has(normalizedScope)) {
      return true;
    }
  }

  return false;
}
