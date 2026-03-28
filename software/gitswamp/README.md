# GitSwamp Desktop App

GitSwamp is a desktop Git client built with Tauri, Vue 3, TypeScript, and Rust.

## Key Features

- **Multi-repository tabs** with fast switching and tab management
- **Commit graph** with visual history, branch visualization, and interactive selection
- **Branch operations**: create, delete, rename, checkout with conflict detection
- **File operations**: stage, unstage, edit, view diffs, and conflict resolution
- **Integrated terminal** with git aliases and open-tool commands (VS Code, explorer, etc.)
- **Hamburger menu (☰)** with File, Edit, View, Help sections and quick actions
- **In-app Help panel (F1)** showing keyboard shortcuts and quick start guide
- **Folder explorer integration (Alt+O)** to open repository in system explorer
- **GitHub/GitLab support** with repository search, token authentication, and gist shortcuts
- **69 Tauri commands** for comprehensive Git and system operations
- **Dark/Light theme toggle** and UI customization options

## Menu Overview (Hamburger ☰)

Top-left hamburger menu provides quick access to all main operations:

**File Section:**
- New Tab (Ctrl+T) - Open fresh repository tab
- Open Repository (Ctrl+O) - Browse and open Git repository
- Close Current Tab (Ctrl+W) - Close active tab
- Create a Gist (Ctrl+Shift+G) - Launch GitHub Gist creator

**Edit Section:**
- Copy Repository Path (Ctrl+Shift+C) - Copy active repo path to clipboard
- Refresh Repository (Ctrl+Shift+R) - Reload all repository data
- Open in VS Code (Ctrl+Shift+O) - Launch repository in VS Code

**View Section:**
- Toggle Terminal (Ctrl+`) - Show/hide integrated terminal panel
- Open Folder Explorer (Alt+O) - Open repository in system file explorer
- Open Settings (Ctrl+,) - Configure application preferences

**Help Section:**
- Features and Shortcuts (F1) - View in-app help and keyboard shortcuts
- Online Guide - Link to full user documentation
- Report Issue - Link to GitHub issue tracker

## Keyboard Shortcuts

- F1: open help and shortcuts panel
- Ctrl+`: toggle terminal panel
- Ctrl+Shift+O: open repository in VS Code
- Alt+O: open repository in system folder explorer
- Ctrl+Shift+R: refresh repository data
- Ctrl+R: focus commit search
- Ctrl+Shift+G: open GitHub Gist creator
- Ctrl+,: open settings

## Development

Requirements:

- Node.js 18+
- Rust toolchain (stable)

Install and run:

```bash
npm install
npm run tauri dev
```

Build app:

```bash
npm run build
npm run tauri build
```

## Related Documentation

- Root docs index: ../../documentation/DOCUMENTATION_00_INDEX.md
- User guide: ../../documentation/DOCUMENTATION_31_USER_GUIDE.md
- Commands reference: ../../documentation/DOCUMENTATION_09_COMMANDS_REFERENCE.md
