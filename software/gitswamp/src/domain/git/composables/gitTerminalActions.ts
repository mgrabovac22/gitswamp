import { callTauri } from "./gitCall";
import { getOriginUrl, getTokenForUrl } from "./gitHelpers";
import type { GitState } from "./gitState";
import type { FileStatusInfo, StashInfo } from "@/types";

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
  rmc: ["rm", "-r", "--cached", "."],
  aa: ["add", "--all"],
  unstage: ["restore", "--staged"],
};

function expandGitShortcut(args: string[]): string[] {
  if (args.length === 0) return args;
  const shortcut = GIT_SHORTCUTS[args[0].toLowerCase()];
  if (!shortcut) return args;
  return [...shortcut, ...args.slice(1)];
}

function normalizeGitArgs(args: string[]): string[] {
  if (args.length === 0) return args;

  const normalized = [...args];
  if (normalized[0].toLowerCase() !== "rm") {
    return normalized;
  }

  for (let i = 1; i < normalized.length; i += 1) {
    if (normalized[i].toLowerCase() === "cached") {
      normalized[i] = "--cached";
    }
  }

  return normalized;
}

function hasArg(args: string[], value: string): boolean {
  const target = value.toLowerCase();
  return args.some((arg) => arg.toLowerCase() === target);
}

function isFetchAllArgs(args: string[]): boolean {
  return args[0]?.toLowerCase() === "fetch" && hasArg(args.slice(1), "--all");
}

function isPullArgs(args: string[]): boolean {
  return args.length === 1 && args[0].toLowerCase() === "pull";
}

function isPushArgs(args: string[]): boolean {
  return args.length === 1 && args[0].toLowerCase() === "push";
}

function isStatusArgs(args: string[]): boolean {
  return args.length >= 1 && args[0].toLowerCase() === "status";
}

function isRmCachedAllArgs(args: string[]): boolean {
  if (args[0]?.toLowerCase() !== "rm") return false;
  const tail = args.slice(1);
  const hasRecursive = hasArg(tail, "-r") || hasArg(tail, "--recursive") || hasArg(tail, "-rf") || hasArg(tail, "-fr");
  return hasRecursive && hasArg(tail, "--cached") && hasArg(tail, ".");
}

function buildExpandedNote(rawArgs: string[], expandedArgs: string[], args: string[]): string {
  const expandedFromShortcut = expandedArgs.join(" ") !== rawArgs.join(" ");
  const normalizedFromInput = args.join(" ") !== expandedArgs.join(" ");

  let expandedNote = "";
  if (expandedFromShortcut) {
    expandedNote += `\n(alias expanded from: git ${rawArgs.join(" ")})`;
  }
  if (normalizedFromInput) {
    expandedNote += `\n(normalized to: git ${args.join(" ")})`;
  }

  return expandedNote;
}

function buildHelpText(allowAll: boolean): string {
  const modeHint = allowAll
    ? "Mode: all shell commands are allowed."
    : "Mode: git commands only (without needing to type the git prefix).";

  return [
    "$ help",
    modeHint,
    "Built-ins: clear/cls, !!, help, tools, open <tool>",
    "Git shortcuts: st->status, br->branch, co->checkout, sw->switch, lg->log graph, last->log -1 --stat, rmc->rm -r --cached .",
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

function formatNativeStatusLine(file: FileStatusInfo): string {
  if (file.conflicted) {
    return `UU ${file.path}`;
  }

  const normalized = (file.status || "").toLowerCase();
  const stageKey = file.staged ? "staged" : "unstaged";
  const lookup: Record<string, string> = {
    "added:staged": "A ",
    "new:staged": "A ",
    "added:unstaged": "??",
    "new:unstaged": "??",
    "modified:staged": "M ",
    "modified:unstaged": " M",
    "deleted:staged": "D ",
    "deleted:unstaged": " D",
    "renamed:staged": "R ",
    "renamed:unstaged": " M",
    "copied:staged": "C ",
    "copied:unstaged": " M",
  };

  const code = lookup[`${normalized}:${stageKey}`] ?? "??";

  return `${code} ${file.path}`;
}

function formatNativeStatusOutput(files: FileStatusInfo[]): string {
  if (files.length === 0) {
    return "On branch (current)\nnothing to commit, working tree clean";
  }

  return files
    .slice()
    .sort((a, b) => a.path.localeCompare(b.path))
    .map(formatNativeStatusLine)
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

  async function loadGitExecutablePath(forceRefresh = false): Promise<string | null> {
    if (!forceRefresh && cachedGitExecutable !== null) {
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

  async function runGitViaShell(path: string, args: string[], forceRefresh = false): Promise<string> {
    const executable = await loadGitExecutablePath(forceRefresh);
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

  async function runToolbarCompatibleGitCommand(
    args: string[],
    commandLabel: string,
    expandedNote: string,
  ): Promise<boolean> {
    if (isStatusArgs(args)) {
      try {
        const result = await callTauri<FileStatusInfo[]>("get_status", {
          path: state.repoPath.value,
        });
        appendOutput(commandLabel + expandedNote + "\n" + formatNativeStatusOutput(result) + "\n(native status)");
      } catch (statusError) {
        appendOutput(commandLabel + expandedNote + "\nError: " + String(statusError));
      }
      return true;
    }

    const originUrl = getOriginUrl(state);
    const token = getTokenForUrl(state, originUrl);

    if (isFetchAllArgs(args)) {
      try {
        const result = await callTauri<string>("fetch_all", {
          path: state.repoPath.value,
          token,
        });
        appendOutput(commandLabel + expandedNote + "\n" + (result || "(done)") + "\n(toolbar-compatible fetch)");
      } catch (fetchError) {
        appendOutput(commandLabel + expandedNote + "\nError: " + String(fetchError));
      }
      return true;
    }

    if (isPullArgs(args)) {
      try {
        const result = await callTauri<string>("pull", {
          path: state.repoPath.value,
          token,
        });
        appendOutput(commandLabel + expandedNote + "\n" + (result || "(done)") + "\n(toolbar-compatible pull)");
      } catch (pullError) {
        appendOutput(commandLabel + expandedNote + "\nError: " + String(pullError));
      }
      return true;
    }

    if (isPushArgs(args)) {
      try {
        const result = await callTauri<string>("push", {
          path: state.repoPath.value,
          token,
        });
        appendOutput(commandLabel + expandedNote + "\n" + (result || "(done)") + "\n(toolbar-compatible push)");
      } catch (pushError) {
        appendOutput(commandLabel + expandedNote + "\nError: " + String(pushError));
      }
      return true;
    }

    if (isRmCachedAllArgs(args)) {
      try {
        const result = await callTauri<string>("remove_cached_all", {
          path: state.repoPath.value,
        });
        appendOutput(commandLabel + expandedNote + "\n" + (result || "(done)") + "\n(index-only remove)");
      } catch (removeCachedError) {
        appendOutput(commandLabel + expandedNote + "\nError: " + String(removeCachedError));
      }
      return true;
    }

    return false;
  }

  async function runStashListCommand(commandLabel: string, expandedNote: string) {
    try {
      const stashes = await callTauri<StashInfo[]>("stash_list", { path: state.repoPath.value });
      appendOutput(commandLabel + expandedNote + "\n" + formatStashListOutput(stashes));
    } catch (stashError) {
      appendOutput(commandLabel + expandedNote + "\nError: " + String(stashError));
    }
  }

  async function runGitCommandWithFallback(args: string[], commandLabel: string, expandedNote: string) {
    try {
      const result = await callTauri<string>("run_git_command", {
        path: state.repoPath.value,
        args,
      });
      appendOutput(commandLabel + expandedNote + "\n" + (result || "(done)"));
      return;
    } catch (primaryError) {
      try {
        const fallbackResult = await runGitViaShell(state.repoPath.value, args);
        appendOutput(
          commandLabel + expandedNote + "\n" + (fallbackResult || "(done)") + "\n(shell fallback)",
        );
        return;
      } catch (fallbackError) {
        const primaryText = String(primaryError);
        const fallbackText = String(fallbackError);

        if (isGitMissingError(primaryText) || isGitMissingError(fallbackText)) {
          cachedGitExecutable = null;
          try {
            const refreshedResult = await runGitViaShell(state.repoPath.value, args, true);
            appendOutput(
              commandLabel + expandedNote + "\n" + (refreshedResult || "(done)") + "\n(refreshed git path)",
            );
            return;
          } catch {
          }
        }

        const missingGitHint = isGitMissingError(primaryText) || isGitMissingError(fallbackText)
          ? "\nHint: Git executable was not found. Check that Git is installed and available in PATH (you may need to reopen the app if PATH was changed outside)."
          : "";

        appendOutput(
          commandLabel
            + expandedNote
            + "\nError: "
            + primaryText
            + "\nFallback Error: "
            + fallbackText
            + missingGitHint,
        );
      }
    }
  }

  async function executeGitArgs(rawArgs: string[], typedCommand: string, allowAllMode: boolean) {
    const expandedArgs = expandGitShortcut(rawArgs);
    const args = normalizeGitArgs(expandedArgs);

    if (args.length === 0) {
      appendOutput("$ git\nError: Missing git arguments.");
      return;
    }

    const commandLabel = "$ git " + args.join(" ");
    const expandedNote = buildExpandedNote(rawArgs, expandedArgs, args);

    lastExecuted = { command: typedCommand, allowAll: allowAllMode };

    if (await runToolbarCompatibleGitCommand(args, commandLabel, expandedNote)) {
      return;
    }

    if (isStashListArgs(args)) {
      await runStashListCommand(commandLabel, expandedNote);
      return;
    }

    await runGitCommandWithFallback(args, commandLabel, expandedNote);
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
