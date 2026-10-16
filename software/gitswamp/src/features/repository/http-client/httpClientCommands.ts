export type HttpClientCommand =
  | "open-cookie-manager"
  | "add-header"
  | "add-cookie"
  | "copy-curl"
  | "clear-domain-cookies"
  | "run-active"
  | "run-all";

export interface HttpClientCommandEvent {
  command: HttpClientCommand;
  repoPath: string;
}

export const HTTP_CLIENT_COMMAND_EVENT = "gitswamp:http-client-command";

let pending: HttpClientCommandEvent | null = null;

export function dispatchHttpClientCommand(command: HttpClientCommand, repoPath = ""): void {
  pending = { command, repoPath };
  window.dispatchEvent(
    new CustomEvent<HttpClientCommandEvent>(HTTP_CLIENT_COMMAND_EVENT, {
      detail: { command, repoPath },
    }),
  );
}

export function consumePendingHttpClientCommand(repoPath: string): HttpClientCommand | null {
  if (!pending || (pending.repoPath && pending.repoPath !== repoPath)) return null;
  const command = pending.command;
  pending = null;
  return command;
}
