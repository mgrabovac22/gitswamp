import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";
import type { StashInfo } from "@/types";

type QuoteMode = '"' | "'" | null;

interface ParseState {
  args: string[];
  current: string;
  quote: QuoteMode;
  escaping: boolean;
}

function pushCurrentToken(state: ParseState) {
  if (state.current.length > 0) {
    state.args.push(state.current);
    state.current = "";
  }
}

function processEscapedChar(state: ParseState, char: string) {
  state.current += char;
  state.escaping = false;
}

function processQuotedChar(state: ParseState, char: string) {
  if (char === state.quote) {
    state.quote = null;
    return;
  }
  state.current += char;
}

function processUnquotedChar(state: ParseState, char: string) {
  if (char === "\\") {
    state.escaping = true;
    return;
  }

  if (char === '"' || char === "'") {
    state.quote = char;
    return;
  }

  if (char.trim().length === 0) {
    pushCurrentToken(state);
    return;
  }

  state.current += char;
}

function processCommandChar(state: ParseState, char: string) {
  if (state.escaping) {
    processEscapedChar(state, char);
    return;
  }

  if (state.quote !== null) {
    processQuotedChar(state, char);
    return;
  }

  processUnquotedChar(state, char);
}

function parseCommandArgs(command: string): string[] | null {
  const state: ParseState = {
    args: [],
    current: "",
    quote: null,
    escaping: false,
  };

  for (const char of command) {
    processCommandChar(state, char);
  }

  if (state.escaping || state.quote !== null) {
    return null;
  }

  pushCurrentToken(state);

  return state.args;
}

const MAX_TERMINAL_OUTPUT_ENTRIES = 500;
const EXTERNAL_TOOLS_CACHE_KEY = "gitswamp-open-tools-cache-v2";
const EXTERNAL_TOOL_IDS = ["explorer", "vscode", "visualstudio", "androidstudio", "intellij"] as const;

type ExternalToolId = (typeof EXTERNAL_TOOL_IDS)[number];

const GIT_SHORTCUTS: Record<string, string[]> = {
  st: ["status"],
  s: ["status"],
  br: ["branch"],
  co: ["checkout"],
  sw: ["switch"],
  ci: ["commit"],
  df: ["diff"],
  lg: ["log", "--oneline", "--graph", "--decorate", "-20"],
  last: ["log", "-1", "--stat"],
  pl: ["pull"],
  ps: ["push"],
  aa: ["add", "--all"],
  unstage: ["restore", "--staged"],
};

function expandGitShortcut(args: string[]): string[] {
  if (args.length === 0) return args;
  const shortcut = GIT_SHORTCUTS[args[0].toLowerCase()];
  if (!shortcut) return args;
  return [...shortcut, ...args.slice(1)];
}

function buildHelpText(allowAll: boolean): string {
  const modeHint = allowAll
    ? "Mode: all shell commands are allowed."
    : "Mode: git commands only (without needing to type the git prefix).";

  return [
    "$ help",
    modeHint,
    "Built-ins: clear/cls, !!, help, tools, open <tool>",
    "Git shortcuts: st->status, br->branch, co->checkout, sw->switch, lg->log graph, last->log -1 --stat",
    "Open tools: explorer, vscode, visualstudio, androidstudio, intellij (detected once and cached)",
    "Example: 'st' => git status, 'lg' => git log --oneline --graph --decorate -20",
  ].join("\n");
}

function quoteForShell(arg: string): string {
  if (!/[\s"'`$&|<>^()]/.test(arg)) {
    return arg;
  }
  const quote = String.fromCodePoint(34);
  const escaped = arg.split(quote).join(quote + quote);
  return quote + escaped + quote;
}

function extractGitExecutable(pathInfo: string): string {
  const marker = " (PATH has";
  const idx = pathInfo.indexOf(marker);
  if (idx > 0) {
    return pathInfo.slice(0, idx).trim();
  }
  return pathInfo.trim();
}

function normalizeToolId(value: string): ExternalToolId | null {
  const normalized = value.trim().toLowerCase();
  if (normalized === "explorer" || normalized === "folder" || normalized === "finder" || normalized === "file-explorer") {
    return "explorer";
  }
  if (normalized === "code" || normalized === "vscode") return "vscode";
  if (normalized === "vs" || normalized === "visualstudio" || normalized === "visual-studio") {
    return "visualstudio";
  }
  if (normalized === "androidstudio" || normalized === "android-studio" || normalized === "studio") {
    return "androidstudio";
  }
  if (normalized === "idea" || normalized === "intellij") return "intellij";
  return null;
}

function dedupeNormalizedTools(values: string[]): ExternalToolId[] {
  const normalized = values
    .map(normalizeToolId)
    .filter((value): value is ExternalToolId => value !== null);
  return Array.from(new Set(normalized));
}

function isGitMissingError(message: string): boolean {
  const normalized = message.toLowerCase();
  return normalized.includes("'git' is not recognized")
    || normalized.includes("git is not recognized")
    || normalized.includes("git not found")
    || normalized.includes("program not found")
    || normalized.includes("cannot find the file specified")
    || normalized.includes("no such file or directory");
}

function isStashListArgs(args: string[]): boolean {
  if (args.length === 0) return false;
  if (args[0].toLowerCase() !== "stash") return false;
  if (args.length === 1) return true;
  return args[1].toLowerCase() === "list";
}

function formatStashListOutput(stashes: StashInfo[]): string {
  if (stashes.length === 0) {
    return "(no stashes)";
  }

  return stashes
    .slice()
    .sort((a, b) => a.index - b.index)
    .map((stash) => {
      const suffix = stash.message ? `: ${stash.message}` : "";
      return `stash@{${stash.index}}: On ${stash.branch}${suffix}`;
    })
    .join("\n");
}

export function createTerminalActions(state: GitState) {
  let lastExecuted: { command: string; allowAll: boolean } | null = null;
  let cachedGitExecutable: string | null = null;
  let externalToolsCache: ExternalToolId[] | null = null;

  function appendOutput(entry: string) {
    state.terminalOutput.value.push(entry);
    if (state.terminalOutput.value.length > MAX_TERMINAL_OUTPUT_ENTRIES) {
      state.terminalOutput.value.splice(
        0,
        state.terminalOutput.value.length - MAX_TERMINAL_OUTPUT_ENTRIES,
      );
    }
  }

  async function loadGitExecutablePath(): Promise<string | null> {
    if (cachedGitExecutable !== null) {
      return cachedGitExecutable;
    }

    try {
      const info = await callTauri<string>("get_git_path");
      const executable = extractGitExecutable(info);
      cachedGitExecutable = executable.length > 0 ? executable : null;
      return cachedGitExecutable;
    } catch {
      cachedGitExecutable = null;
      return null;
    }
  }

  async function loadExternalTools(): Promise<ExternalToolId[]> {
    if (externalToolsCache) {
      return externalToolsCache;
    }

    try {
      const raw = localStorage.getItem(EXTERNAL_TOOLS_CACHE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed)) {
          const cached = dedupeNormalizedTools(parsed as string[]);
          if (cached.length > 0) {
            externalToolsCache = cached;
            return cached;
          }
        }
      }
    } catch {
    }

    try {
      const scanned = dedupeNormalizedTools(
        await callTauri<string[]>("get_available_external_tools"),
      );
      externalToolsCache = scanned;
      localStorage.setItem(EXTERNAL_TOOLS_CACHE_KEY, JSON.stringify(scanned));
      return scanned;
    } catch {
      externalToolsCache = [];
      return [];
    }
  }

  async function runGitViaShell(path: string, args: string[]): Promise<string> {
    const executable = await loadGitExecutablePath();
    const base = executable ? quoteForShell(executable) : "git";
    const command = `${base} ${args.map(quoteForShell).join(" ")}`.trim();
    return callTauri<string>("run_shell_command", {
      path,
      command,
    });
  }

  async function handleOpenToolCommand(trimmed: string): Promise<boolean> {
    if (!trimmed.toLowerCase().startsWith("open")) {
      return false;
    }

    if (!state.repoPath.value) {
      appendOutput("$ open\nError: Open a repository before using open tool commands.");
      return true;
    }

    const tokens = parseCommandArgs(trimmed);
    const toolArg = tokens?.[1] ?? "";
    const available = await loadExternalTools();

    if (available.length === 0) {
      appendOutput("$ open\nError: No supported external tools were detected.");
      return true;
    }

    const chosen = toolArg ? normalizeToolId(toolArg) : available[0];
    if (!chosen || !available.includes(chosen)) {
      appendOutput(
        `$ open ${toolArg}\nError: Tool not available. Detected tools: ${available.join(", ")}.`,
      );
      return true;
    }

    try {
      await callTauri("open_path_with_tool", {
        path: state.repoPath.value,
        tool: chosen,
      });
      appendOutput(`$ open ${chosen}\nOpened repository in ${chosen}.`);
    } catch (error) {
      appendOutput(`$ open ${chosen}\nError: ${String(error)}`);
    }

    return true;
  }

  async function handleBuiltInCommand(trimmed: string, allowAll: boolean): Promise<boolean> {
    const lower = trimmed.toLowerCase();

    if (lower === "clear" || lower === "cls") {
      state.terminalOutput.value = [];
      return true;
    }

    if (lower === "help" || lower === "githelp") {
      appendOutput(buildHelpText(allowAll));
      return true;
    }

    if (lower === "tools") {
      const tools = await loadExternalTools();
      if (tools.length === 0) {
        appendOutput("$ tools\nNo supported external tools detected.");
      } else {
        appendOutput(`$ tools\nDetected tools: ${tools.join(", ")}`);
      }
      return true;
    }

    if (await handleOpenToolCommand(trimmed)) {
      return true;
    }

    if (trimmed === "!!") {
      if (!lastExecuted) {
        appendOutput("$ !!\nError: No previous command to repeat.");
        return true;
      }
      appendOutput(`$ !!\nRepeating: ${lastExecuted.command}`);
      void runTerminalCommand(lastExecuted.command, lastExecuted.allowAll);
      return true;
    }

    return false;
  }

  async function executeGitArgs(rawArgs: string[], typedCommand: string, allowAllMode: boolean) {
    const args = expandGitShortcut(rawArgs);

    if (args.length === 0) {
      appendOutput("$ git\nError: Missing git arguments.");
      return;
    }

    const commandLabel = "$ git " + args.join(" ");
    const expandedFromShortcut = args.join(" ") !== rawArgs.join(" ");
    const expandedNote = expandedFromShortcut
      ? `\n(alias expanded from: git ${rawArgs.join(" ")})`
      : "";

    lastExecuted = { command: typedCommand, allowAll: allowAllMode };

    if (isStashListArgs(args)) {
      try {
        const stashes = await callTauri<StashInfo[]>("stash_list", { path: state.repoPath.value });
        appendOutput(commandLabel + expandedNote + "\n" + formatStashListOutput(stashes));
      } catch (stashError) {
        appendOutput(commandLabel + expandedNote + "\nError: " + String(stashError));
      }
      return;
    }

    try {
      const result = await callTauri<string>("run_git_command", {
        path: state.repoPath.value,
        args,
      });
      appendOutput(commandLabel + expandedNote + "\n" + (result || "(done)"));
    } catch (e) {
      try {
        const fallbackResult = await runGitViaShell(state.repoPath.value, args);
        appendOutput(
          commandLabel + expandedNote + "\n" + (fallbackResult || "(done)") + "\n(shell fallback)",
        );
      } catch (fallbackError) {
        const primaryError = String(e);
        const secondaryError = String(fallbackError);
        const missingGitHint = isGitMissingError(primaryError) || isGitMissingError(secondaryError)
          ? "\nHint: Git executable was not found. Install Git for Windows (or add git.exe to PATH) and restart the app."
          : "";
        appendOutput(
          commandLabel
            + expandedNote
            + "\nError: "
            + primaryError
            + "\nFallback Error: "
            + secondaryError
            + missingGitHint,
        );
      }
    }
  }

  async function runAllowAllCommand(trimmed: string, parsed: string[] | null) {
    if (parsed && parsed.length > 0) {
      const first = parsed[0].toLowerCase();
      const explicitGit = first === "git";
      const shortcutGit = !!GIT_SHORTCUTS[first];
      if (explicitGit || shortcutGit) {
        const gitArgs = explicitGit ? parsed.slice(1) : parsed;
        await executeGitArgs(gitArgs, trimmed, true);
        return;
      }
    }

    lastExecuted = { command: trimmed, allowAll: true };
    try {
      const result = await callTauri<string>("run_shell_command", {
        path: state.repoPath.value,
        command: trimmed,
      });
      appendOutput(`$ ${trimmed}\n${result || "(done)"}`);
    } catch (error) {
      appendOutput(`$ ${trimmed}\nError: ${error}`);
    }
  }

  async function runGitModeCommand(trimmed: string, parsed: string[] | null) {
    if (!parsed) {
      appendOutput(`$ ${trimmed}\nError: Invalid command syntax (check quotes/escaping).`);
      return;
    }

    const rawArgs = parsed[0]?.toLowerCase() === "git" ? parsed.slice(1) : parsed;
    await executeGitArgs(rawArgs, trimmed, false);
  }

  async function runTerminalCommand(command: string, allowAll = false) {
    const trimmed = command.trim();
    if (!trimmed) return;

    if (await handleBuiltInCommand(trimmed, allowAll)) {
      return;
    }

    if (!state.repoPath.value) {
      appendOutput("Error: Open a repository before using terminal commands.");
      return;
    }

    const parsed = parseCommandArgs(trimmed);

    if (allowAll) {
      await runAllowAllCommand(trimmed, parsed);
      return;
    }

    await runGitModeCommand(trimmed, parsed);
  }

  return {
    runTerminalCommand,
  };
}
