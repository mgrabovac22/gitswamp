import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

export function createTerminalActions(state: GitState) {
  async function runTerminalCommand(command: string) {
    if (!state.repoPath.value) return;
    const args = command.trim().split(/\s+/);
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
