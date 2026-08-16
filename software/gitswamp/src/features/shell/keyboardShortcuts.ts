export type AppHelpSection = "overview" | "shortcuts" | "roles";

export interface KeyboardShortcutDefinition {
  id: string;
  label: string;
  keys: string;
}

export interface KeyboardShortcutGroup {
  id: string;
  label: string;
  shortcuts: readonly KeyboardShortcutDefinition[];
}

export const KEYBOARD_SHORTCUT_GROUPS: readonly KeyboardShortcutGroup[] = [
  {
    id: "workspace",
    label: "Workspace",
    shortcuts: [
      { id: "help", label: "Open keyboard shortcuts", keys: "F1" },
      { id: "command-palette", label: "Open command palette", keys: "Ctrl+K" },
      { id: "new-tab", label: "New tab", keys: "Ctrl+T" },
      { id: "next-tab", label: "Next tab", keys: "Ctrl+Tab" },
      { id: "previous-tab", label: "Previous tab", keys: "Ctrl+Shift+Tab" },
      { id: "open-repository", label: "Open repository", keys: "Ctrl+O" },
      { id: "close-tab", label: "Close active tab", keys: "Ctrl+W" },
      { id: "reopen-tab", label: "Reopen closed tab", keys: "Ctrl+Shift+T" },
    ],
  },
  {
    id: "graph-navigation",
    label: "Graph and Navigation",
    shortcuts: [
      { id: "jump-head", label: "Load and jump to HEAD", keys: "Ctrl+Shift+H" },
      { id: "jump-selected", label: "Jump to selected commit", keys: "Ctrl+Shift+J" },
      { id: "focus-search", label: "Focus commit search", keys: "Ctrl+R" },
      { id: "pickaxe", label: "Open Pickaxe Explorer", keys: "Ctrl+Shift+F" },
      { id: "graph-top", label: "Jump to graph top", keys: "Home" },
      { id: "graph-end", label: "Load next page and jump to graph end", keys: "End" },
      { id: "previous-commit", label: "Select previous commit", keys: "Arrow Up" },
      { id: "next-commit", label: "Select next commit", keys: "Arrow Down" },
    ],
  },
  {
    id: "changes",
    label: "Changes and Branches",
    shortcuts: [
      { id: "working-changes", label: "Show working changes", keys: "Ctrl+Shift+W" },
      { id: "create-branch", label: "Create branch", keys: "Ctrl+Shift+B" },
      { id: "stage-all", label: "Stage all unstaged files", keys: "Ctrl+Shift+S" },
      { id: "unstage-all", label: "Unstage all staged files", keys: "Ctrl+Shift+U" },
      { id: "copy-path", label: "Copy repository path", keys: "Ctrl+Shift+C" },
      { id: "refresh", label: "Refresh repository data", keys: "Ctrl+Shift+R" },
    ],
  },
  {
    id: "remote",
    label: "Remote Operations",
    shortcuts: [
      { id: "fetch", label: "Fetch all remotes", keys: "Ctrl+Alt+F" },
      { id: "pull", label: "Pull current branch", keys: "Ctrl+Alt+L" },
      { id: "push", label: "Push current branch", keys: "Ctrl+Alt+P" },
    ],
  },
  {
    id: "views",
    label: "Views",
    shortcuts: [
      { id: "terminal", label: "Toggle terminal panel", keys: "Ctrl+`" },
      { id: "graph-view", label: "Graph view", keys: "Alt+1" },
      { id: "galaxy-view", label: "Galaxy view", keys: "Alt+2" },
      { id: "productivity-view", label: "Productivity Arena", keys: "Alt+3" },
      { id: "time-machine-view", label: "Time Machine", keys: "Alt+4" },
      { id: "conflict-view", label: "Usual Conflict Suspects", keys: "Alt+5" },
      { id: "burnout-view", label: "Burnout Analytics", keys: "Alt+6" },
      { id: "city-view", label: "Repository City", keys: "Alt+7" },
    ],
  },
  {
    id: "tools",
    label: "Tools and Settings",
    shortcuts: [
      { id: "gist", label: "Open Gist creator", keys: "Ctrl+Shift+G" },
      { id: "vscode", label: "Open repository in VS Code", keys: "Ctrl+Shift+O" },
      { id: "explorer", label: "Open repository in folder explorer", keys: "Alt+O" },
      { id: "integrations", label: "Open integrations", keys: "Ctrl+Shift+I" },
      { id: "git-integration", label: "Open Git integration", keys: "Ctrl+Shift+K" },
      { id: "advanced", label: "Open advanced options", keys: "Ctrl+Shift+A" },
      { id: "organisations", label: "Open organisations", keys: "Ctrl+Shift+Y" },
      { id: "options", label: "Open options", keys: "Ctrl+," },
      { id: "logs", label: "Toggle logs panel", keys: "Ctrl+Shift+L" },
    ],
  },
] as const;
