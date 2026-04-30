# GitSwamp Project Overview

## 1. Introduction

GitSwamp is a modern, feature-rich desktop Git client designed to provide developers with an intuitive and powerful interface for managing Git repositories. Built with cutting-edge technologies including Tauri, Vue 3, TypeScript, and Rust, GitSwamp combines the performance of native applications with the flexibility and user experience of modern web technologies.

## 2. Project Vision

GitSwamp aims to simplify Git repository management by providing:
- A clean, intuitive user interface
- Real-time repository status visualization
- Advanced Git operations made accessible
- Multi-repository support with easy switching
- Seamless GitHub and GitLab integration
- Cross-platform compatibility (Windows, macOS, Linux)

## 3. Project Scope

### 3.1 Core Objectives

The project focuses on delivering a comprehensive Git management solution that:

1. **Visualize Version History** - Display commit history with an interactive, visual graph representation
2. **Manage Branches** - Intuitive branch creation, deletion, switching, and renaming
3. **Handle File Operations** - Stage, unstage, and commit changes with detailed diffs
4. **Resolve Conflicts** - Built-in merge conflict resolution interface
5. **Integrate with Remote Services** - GitHub and GitLab repository search and authentication
6. **Support Advanced Operations** - Stashing, tagging, cherry-picking, rebasing, and more
7. **Analyze Repository Motion** - Productivity, Time Machine, and conflict-risk views for history intelligence

### 3.2 Target Users

- Software developers working with Git repositories
- Development teams requiring a collaborative Git interface
- Individual contributors seeking a streamlined Git experience
- Organizations migrating from other Git clients

## 4. Technical Stack

### 4.1 Frontend Stack

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Framework | Vue 3 | 3.5.13 | Progressive JavaScript framework |
| Language | TypeScript | 5.6.2 | Static typing for JavaScript |
| Build Tool | Vite | 6.0.3 | Next-generation frontend build tool |
| CSS Framework | Tailwind CSS | 4.1.12 | Utility-first CSS framework |
| Icons | Lucide Vue Next | 0.487.0 | Icon library for Vue |
| Utilities | CVA | 0.7.1 | Class Variance Authority for styling |
| Utilities | CLSX | 2.1.1 | Utility for constructing className |

### 4.2 Backend Stack

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Framework | Tauri | 2.0 | Lightweight desktop app framework |
| Language | Rust | Latest | Systems programming language |
| Git Library | git2-rs | 0.19 | Rust bindings for libgit2 |
| HTTP Client | ureq | 2.0 | Lightweight HTTP client |
| Serialization | serde/serde_json | 1.0 | Data serialization framework |
| File Dialog | tauri-plugin-dialog | 2.6.0 | Native file picker |
| File Opener | tauri-plugin-opener | 2.0 | Open files with system app |

### 4.3 Development Stack

| Tool | Version | Purpose |
|------|---------|---------|
| Node.js | 18+ | JavaScript runtime |
| npm | Latest | Package manager |
| Rust Toolchain | Latest | Rust compiler and tools |
| Cargo | Latest | Rust package manager |
| VS Code | Latest | Recommended IDE |

## 5. Project Structure Overview

```
gitswamp/
├── README.md                     # Documentation root readme
├── documentation/                # Project documentation set
└── software/
      ├── package.json              # Workspace-level npm dependencies
      └── gitswamp/
            ├── src/                  # Vue frontend (app/domain/shared/view)
            ├── src-tauri/            # Rust backend, commands, services
            ├── public/               # Static assets
            ├── package.json          # Frontend app package
            ├── vite.config.ts        # Vite configuration
            ├── tsconfig.json         # TypeScript configuration
            └── README.md             # App-specific readme
```

## 6. Key Features

### 6.1 Repository Management

- **Multi-Repository Support** - Open and switch between multiple repositories
- **Clone Repository** - Clone from GitHub, GitLab, or any Git remote
- **Initialize Repository** - Create new Git repositories locally
- **Repository Info** - View remote URLs, branch tracking, and metadata

### 6.2 Branch Management

- **View Branches** - See all local and remote branches
- **Create Branches** - Create new branches from current HEAD or specific commits
- **Delete Branches** - Remove local and remote branches
- **Rename Branches** - Rename branch names
- **Checkout Branches** - Switch between branches with conflict detection
- **Track Remote** - Set upstream branches and manage tracking

### 6.3 Commit Operations

- **Commit History** - Visual, interactive commit graph with branching visualization
- **Commit Details** - View full commit information, author, timestamp, and message
- **File Changes** - See all files modified in each commit
- **Search Commits** - Search through commit history by message or author
- **Cherry Pick** - Apply specific commits to current branch
- **Revert Commits** - Create new commits that undo changes

### 6.4 File Operations

- **Stage Files** - Add files to the staging area
- **Unstage Files** - Remove files from staging area
- **View Diffs** - Side-by-side and unified diff viewing
- **Edit Hunks** - Selectively apply or discard changes by hunk
- **Discard Changes** - Discard working directory changes
- **File History** - View file changes across commits

### 6.5 Stash Management

- **Create Stash** - Stash working directory changes
- **List Stashes** - View all saved stashes
- **Apply Stash** - Apply stash to current branch
- **Pop Stash** - Apply and remove stash
- **Delete Stash** - Remove stashed changes

### 6.6 Tag Management

- **Lightweight Tags** - Create simple reference points
- **Annotated Tags** - Create tags with descriptions and metadata
- **List Tags** - View all tags in repository
- **Delete Tags** - Remove tags

### 6.7 Merge & Conflict Resolution

- **Conflict Detection** - Automatic detection of merge conflicts
- **Conflict Resolver** - Visual interface for resolving conflicts
- **Three-Way Merge** - View original, current, and incoming changes
- **Mark Resolution** - Mark files as resolved

### 6.8 Advanced Operations

- **Pull/Push** - Synchronize with remote repositories
- **Fetch** - Retrieve updates from remotes
- **Reset** - Reset to specific commits or branches
- **Rebase** - Rebase branches with conflict handling
- **Edit Commit Messages** - Amend and modify commit messages
- **SSH Key Generation** - Generate SSH keys for GitLab

### 6.9 Remote Integration

- **GitHub Support** - Search and clone GitHub repositories, GitHub API integration
- **GitLab Support** - Search and clone GitLab repositories, SSH key management
- **Repository Search** - Search for public repositories across services
- **Provider Authentication** - Token-based authentication for API access
- **SSH Integration** - SSH key generation and management

### 6.10 User Interface Features

- **Multi-Tab Support** - Switch between open repositories with ease
- **Hamburger Menu (☰)** - Quick access to File, Edit, View, and Help sections
  - File: New Tab, Open Repository, Close Tab, Create Gist
  - Edit: Copy Repository Path, Refresh Repository, Open in VS Code
      - View: Toggle Terminal, Commit Graph, Productivity Arena, Time Machine, Usual Conflict Suspects, Open in Folder Explorer, Open Settings
  - Help: In-app Shortcuts, Online Guide, Report Issue
- **In-App Help Panel (F1)** - Shortcut guide and quick start guide
- **Folder Explorer Shortcut (Alt+O)** - Open active repository in system explorer
- **Real-Time Status** - Live file status updates
- **Dark/Light Theme** - Switchable color themes
- **Compact Mode** - Minimize UI for larger editor view
- **Custom Font Size** - Adjustable interface font
- **Toast Notifications** - System-wide notifications for all operations
- **Settings Dialog** - Configure application preferences

### 6.11 Commit Intelligence Views

- **Productivity Arena** - Commit rhythm, author balance, streaks, and stability scoring
- **Time Machine** - Commit-by-commit history playback with snapshot explorer and rollback guidance
- **Usual Conflict Suspects** - Merge-window hotspot analysis, tree heatmap, and risky pair detection

## 7. Architecture Overview

### 7.1 Three-Layer Architecture

```
┌─────────────────────────────────────────┐
│      Vue 3 Frontend (TypeScript)        │
│  Components, Composables, State         │
└─────────────────┬───────────────────────┘
                  │
        Tauri IPC Channel
                  │
┌─────────────────▼───────────────────────┐
│    Rust Backend (Commands, Services)    │
│  Command Handlers, Git Operations       │
└─────────────────┬───────────────────────┘
                  │
            git2-rs library
            System Git Exe
                  │
┌─────────────────▼───────────────────────┐
│        Git Repository / Filesystem      │
└─────────────────────────────────────────┘
```

### 7.2 Data Flow

1. **User Interaction** - User interacts with Vue component
2. **State Management** - Component uses composables (useGit, useToast)
3. **IPC Invocation** - Composable invokes Tauri command
4. **Backend Execution** - Rust handler executes command logic
5. **Git Operations** - Service layer performs Git operations using git2-rs
6. **Result Serialization** - Results converted to JSON
7. **State Update** - Frontend composable updates reactive state
8. **UI Render** - Vue reactively updates the interface

### 7.3 Component Architecture

**Frontend Components (19 total):**
- 4 Layout components for structure
- 6 Repository management components
- 3 Commit visualization components
- 6 Reusable UI components

**Backend Commands (47 total):**
- Repository operations
- Commit history and details
- Branch management
- File status and staging
- Diff generation and file operations
- Stash operations
- Tag management
- Clone and initialization
- Advanced Git operations
- GitHub and GitLab integration
- Credentials and token management

## 8. Development Status

### 8.1 Current Version

- **Version Number:** 0.1.0
- **Release Status:** Alpha/Early Development
- **Stability:** Features under active development

### 8.2 Major Components Status

| Component | Status | Completeness |
|-----------|--------|--------------|
| Frontend UI | Complete | 95% |
| Backend Commands | Complete | 95% |
| Git Operations | Complete | 95% |
| GitHub Integration | Functional | 80% |
| GitLab Integration | Functional | 80% |
| Conflict Resolution | Functional | 85% |
| Performance Optimization | Ongoing | 60% |
| Testing Suite | In Progress | 40% |
| Documentation | In Progress | 70% |

## 9. Key Metrics

### 9.1 Code Metrics

| Metric | Value |
|--------|-------|
| Frontend Components | 19 |
| Vue Composables | 2 |
| TypeScript Files | 30+ |
| Rust Command Modules | 12 |
| Total Commands | 47 |
| Data Models | 10+ |
| CSS Utilities | Tailwind |

### 9.2 Performance Targets

- **App Startup Time** - < 2 seconds
- **Commit Graph Rendering** - < 500ms for 1000 commits
- **File Staging** - Instant (< 100ms)
- **Diff Generation** - < 500ms for typical files
- **Memory Usage** - < 200MB baseline

## 10. System Requirements

### 10.1 Minimum Requirements

| Requirement | Specification |
|-------------|---------------|
| OS | Windows 10+, macOS 10.13+, Linux (Ubuntu 20.04+) |
| CPU | Dual-core 2.0 GHz or higher |
| RAM | 2 GB minimum |
| Disk | 100 MB free space |
| Git | Git 2.20+ |

### 10.2 Recommended Requirements

| Requirement | Specification |
|-------------|---------------|
| OS | Windows 11, macOS 12+, Linux (Ubuntu 22.04+) |
| CPU | Quad-core 2.5 GHz or higher |
| RAM | 4-8 GB |
| Disk | 500 MB free space |
| Git | Git 2.40+ |
| Network | Broadband connection (for remote operations) |

## 11. Dependencies Overview

### 11.1 Critical Dependencies

- **Tauri:** Core desktop application framework
- **Vue 3:** Frontend framework and reactivity
- **git2-rs:** Git library bindings
- **TypeScript:** Type safety
- **Tailwind CSS:** UI styling

### 11.2 Optional Dependencies

- **GitHub API:** Repository search and integration
- **GitLab API:** Repository search and SSH key management

## 12. Licensing & Attribution

- Project uses open-source libraries and frameworks
- Dependencies managed through npm (Node.js) and Cargo (Rust)
- Full attribution and licenses available in package manifests

## 13. Contact & Support

For questions, issues, or contributions:
- Review documentation in this folder
- Check existing GitHub issues
- Follow contribution guidelines in project repository

---

**Next Steps:**
- Review [01_SYSTEM_ARCHITECTURE.md](./DOCUMENTATION_01_SYSTEM_ARCHITECTURE.md) for detailed architecture
- Check [19_DEVELOPMENT_SETUP.md](./DOCUMENTATION_19_DEVELOPMENT_SETUP.md) for development environment setup
- Browse specific feature documentation for implementation details
