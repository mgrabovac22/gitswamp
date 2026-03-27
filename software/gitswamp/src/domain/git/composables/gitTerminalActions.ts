import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

export function createTerminalActions(state: GitState) {
  async function runTerminalCommand(command: string, allowAll = false) {
    if (!state.repoPath.value) return;

    const trimmed = command.trim();
    if (!trimmed) return;

    if (allowAll) {
      try {
        const result = await callTauri<string>("run_shell_command", {
          path: state.repoPath.value,
          command: trimmed,
        });
        state.terminalOutput.value.push(`$ ${trimmed}\n${result || "(done)"}`);
      } catch (e) {
        state.terminalOutput.value.push(`$ ${trimmed}\nError: ${e}`);
      }
      return;
    }

    const args = trimmed.split(/\s+/);
    if (args[0] === "git") args.shift();

    try {
      const result = await callTauri<string>("run_git_command", {
        path: state.repoPath.value,
        args,
      });
      state.terminalOutput.value.push("$ git " + args.join(" ") + "\n" + (result || "(done)"));
    } catch (e) {
      state.terminalOutput.value.push("$ git " + args.join(" ") + "\nError: " + e);
    }
  }

  return {
    runTerminalCommand,
  };
}
