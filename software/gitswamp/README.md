# GitSwamp Desktop App

GitSwamp is a powerful desktop Git client built with Tauri, Vue 3, TypeScript, and Rust. It provides an intuitive interface for repository management with advanced workflows, multi-platform support, and deep integration with GitHub, GitLab (including self-hosted), and other platforms.

## Key Features

### Core Git Operations
- **Repository Management**: Clone, init, open multiple repositories in tabs with session restore
- **Commit History**: Interactive commit graph with search, time-lapse history view, and expert advisor
- **Branch Operations**: Create, rename, delete, checkout, merge, set upstream, delete remote branches
- **Working Tree**: Stage/unstage files, stage all, unstage all, discard changes, stash, pop
- **Tagging**: Create annotated and lightweight tags, delete tags

### Advanced Workflows

#### Ghost Branches
Temporary experimental workflow allowing you to:
- Start a temporary "ghost" branch from your current branch
- Work freely without affecting the base branch
- Materialize the ghost branch into a permanent branch with a new name
- Discard all changes and return to base branch

#### Diff & Conflict Resolution
- **Multi-mode Diff Viewer**:
  - Hunk-based diff with line-by-line navigation
  - Full-file overlay showing entire file with change highlights
  - Edit mode for direct file editing in working tree
  - Time-lapse file history (preview file evolution through recent commits)
  - Inline diff highlighting showing character-level changes within lines
- **Conflict Resolution**:
  - One-click strategies: keep modified, keep base, delete file
  - Manual conflict resolver with visual diff
  - Bulk conflict resolution

#### Commit Analysis
- Real-time commit message quality scoring
- Commit diff analysis showing addition/deletion statistics
- Language-specific code metrics
- Custom linting rules for commit messages
- Author deletion statistics over time
- History intelligence panels for deeper repository analysis:
  - **Productivity Arena** - commit rhythm, author balance, bottleneck, and stability scoring
  - **Time Machine** - frame-by-frame history playback, snapshot explorer, and rollback command guidance
  - **Usual Conflict Suspects** - conflict hotspot heatmap, tree roll-up, and risky file-pair diagnostics

#### Productivity Insights
- **Commit Productivity Panel**: Visualization of commit activity and code change trends
- **Conflict Heatmap**: Identify files and areas with frequent conflicts
- **Expert Advisor**: Find who knows a file best based on recent commits

### Platform Integrations

#### GitHub
- OAuth token-based authentication
- Repository search and clone
- View pull requests and issues
- Create pull requests and issues
- GitHub Enterprise support

#### GitLab
- **GitLab.com**: Standard token authentication
- **Self-Hosted GitLab**: 
  - Token verification per instance
  - Repository search and clone with domain normalization
  - SSH key generation and upload to instance
  - Multi-platform push flow with credential handling
  - Support for custom domains and ports (e.g., `gitlab.company.com:8443`)

#### Other Platforms
- Bitbucket & Bitbucket Data Center (token auth)
- Azure DevOps (PAT auth)
- SSH key management

### Remote Insights
- **GitHub**: View/create pull requests and issues
- **GitLab**: Discover merge request source branches for opened MRs
- Create issues and pull requests directly from the app

### User Experience
- **Terminal Integration**: Integrated terminal panel with git command execution
- **Helper Actions**: Open repository in VS Code, open in system file explorer
- **Customizable UI**: 
  - Multiple theme palettes (default, swamp, github-dark, dark-red, emerald-night, midnight-blue, github-light, mint-light, sand-light, rose-light)
  - Font size adjustment (small, medium, large)
  - Compact mode, reduced motion, diff line wrapping, line number visibility toggles
  - Avatar toggle
  - Plain text highlighting mode for large diffs

## Tech Stack

- **Frontend**: Vue 3 + TypeScript + Vite with Tailwind CSS
- **Desktop Shell**: Tauri 2 (cross-platform native wrapper)
- **Backend**: Rust with git2 library for Git operations
- **API Integrations**: GitHub, GitLab, Bitbucket, Azure DevOps
- **UI Components**: Custom components with lucide-vue-next icons

## Project Structure

```
src/
├── App.vue                          # Root application component
├── main.ts                          # Application entry point
├── app/
│   └── bootstrap.ts                # Application initialization
├── assets/                          # Static assets
├── composables/
│   ├── useGit.ts                   # Git composable factory
│   └── useToast.ts                 # Toast notification system
├── domain/
│   ├── git/                        # Git operations layer
│   │   ├── UseGit.ts              # Main Git composable
│   │   └── composables/            # Git action composables
│   │       ├── gitBranchActions.ts
│   │       ├── gitGhostActions.ts
│   │       ├── gitHistoryActions.ts
│   │       ├── gitRefreshActions.ts
│   │       ├── gitRemoteActions.ts
│   │       ├── gitRepositoryActions.ts
│   │       ├── gitStashActions.ts
│   │       ├── gitStatusActions.ts
│   │       ├── gitTerminalActions.ts
│   │       ├── gitTokenActions.ts
│   │       ├── gitWatcherActions.ts
│   │       └── gitState.ts
│   └── analyzer/                   # Commit analysis engine
│       ├── commitAnalyzer.ts       # Main analyzer
│       ├── commitAnalyzerDiff.ts   # Diff analysis
│       ├── commitAnalyzerLanguage.ts
│       ├── commitAnalyzerRules.ts
│       ├── commitAnalyzerScorer.ts
│       ├── commitAnalyzerTypes.ts
│       ├── commitLintEngine.ts     # Message linting
├── shared/
│   ├── codeView.ts                 # Code syntax highlighting
│   ├── themePreferences.ts         # Theme system
│   ├── config/
│   │   ├── commitAnalyzerPreferences.ts
│   │   ├── diffViewCache.ts        # Diff caching strategy
│   │   ├── code-view/
│   │   └── theme/
│   ├── notifications/
│   │   └── useToast.ts
│   └── ui/                          # Vue components
│       ├── AppButton.vue
│       ├── AppInput.vue
│       ├── ConflictResolver.vue    # Manual conflict resolution UI
│       ├── FileDiffViewer.vue      # Multi-mode diff viewer
│       ├── GitCommitIcon.vue
│       └── ToastContainer.vue
├── styles/
│   ├── index.css                   # Main styles
│   ├── tailwind.css                # Tailwind configuration
│   └── theme.css                   # Theme variables and palettes
├── types/
│   ├── index.ts
│   └── models/                      # TypeScript interfaces
│       ├── branchInfo.ts
│       ├── commitFileInfo.ts
│       ├── commitInfo.ts
│       ├── conflictHotspot.ts
│       ├── diffHunk.ts
│       ├── diffLine.ts
│       ├── fileDiff.ts
│       ├── fileStatusInfo.ts
│       ├── ghostBranchState.ts
│       ├── githubRepo.ts
│       ├── gitlabRepo.ts
│       ├── graphEdge.ts
│       ├── graphNode.ts
│       ├── issueInfo.ts
│       ├── pullRequestInfo.ts
│       ├── remoteInfo.ts
│       ├── repoInfo.ts
│       ├── stagedDiffSummary.ts
│       ├── stashInfo.ts
│       └── tagInfo.ts
└── view/                            # Vue views and pages
    ├── AppView.vue                 # Main application layout
    ├── commit/                      # Commit-related views
    │   ├── CommitConflictHeatmapPanel.vue
    │   ├── CommitDetails.vue
    │   ├── CommitGraph.vue
    │   ├── CommitProductivityPanel.vue
    │   ├── CommitTimeMachinePanel.vue
    │   ├── FileItem.vue
    │   └── graph/
    └── repository/                  # Repository management views
        ├── CloneDialog.vue          # Clone from various sources
        ├── GhostBranchDialog.vue   # Ghost branch workflow
        ├── InitDialog.vue           # Initialize new repository
        ├── LandingPage.vue
        ├── MultiPlatformPushDialog.vue  # Platform selection for push
        ├── RemoteInsightsPanel.vue  # PR/Issue view
        ├── RepositoryActionDialogs.vue
        ├── RepositoryAuthDialogs.vue    # Auth credential dialogs
        ├── RepositorySidebar.vue
        ├── RepositoryTabs.vue
        ├── RepositoryWorkspace.vue
        └── shell/

src-tauri/src/
├── main.rs                         # Tauri main entry
├── lib.rs                          # Command exports
├── constants.rs                    # Platform constants
├── commands/                       # Tauri command handlers
│   ├── branches.rs
│   ├── clone_init.rs
│   ├── commit_files.rs
│   ├── commits.rs
│   ├── conflicts.rs
│   ├── credentials.rs              # Token storage
│   ├── diff.rs
│   ├── ghost.rs                    # Ghost branch operations
│   ├── operations.rs               # Major Git operations
│   ├── repository.rs
│   ├── stash.rs
│   ├── status.rs
│   └── tags.rs
├── services/                       # Business logic
│   ├── git_service.rs             # Core Git operations using git2
│   ├── helpers.rs                 # Utility functions
│   ├── remote_service.rs          # Not currently used
│   ├── stash_service.rs
│   ├── diff_service.rs
│   └── integration_service.rs     # GitHub/GitLab API integrations
├── models/                         # Data structures
├── repositories/                   # Data access layer
└── target/                         # Build output
```

## Development

### Requirements

- Node.js 18+
- Rust toolchain (stable)
- Git (for repository operations)

### Setup

```bash
# Install frontend dependencies
cd gitswamp
npm install
```

### Commands

```bash
# Run frontend dev server (Vite)
npm run dev

# Type check frontend
npm run build  # (includes vue-tsc check)

# Run desktop app in development mode
npm run tauri dev

# Build frontend assets
npm run build

# Build desktop bundle for distribution
npm run tauri build
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `F1` | Open help and shortcuts panel |
| `Ctrl+`` | Toggle terminal panel |
| `Ctrl+Shift+O` | Open repository in VS Code |
| `Alt+O` | Open repository in system file explorer |
| `Ctrl+Shift+R` | Refresh repository data |
| `Ctrl+R` | Focus commit search |
| `Ctrl+Shift+G` | Create gist from repository context |
| `Ctrl+,` | Open settings |

## Authentication & Token Management

### Supported Providers

- **GitHub** (`github` key): Personal access tokens with `repo` scope
- **GitHub Enterprise** (`github-enterprise` key): Custom domain support
- **GitLab.com** (`gitlab` key): Personal access tokens with `read_api` and `read_repository` scopes
- **GitLab Self-Hosted** (`gitlab-self` key): Custom instance support
- **Bitbucket** (`bitbucket` key): App passwords with repository read permissions
- **Bitbucket Data Center** (`bitbucket-dc` key): HTTP access tokens
- **Azure DevOps** (`azure` key): Personal access tokens with code read scope

### Token Storage

- Tokens are stored securely per provider key
- GitHub tokens have legacy compatibility mode (synced between old and new storage)
- GitLab self-hosted tokens are stored with domain information: `domain|token`

### GitLab Self-Hosted Configuration

When using GitLab self-hosted:

1. **Domain Format**: Use the instance domain without protocol or path
   - Correct: `gitlab.company.com` or `gitlab.company.com:8443`
   - Incorrect: `https://gitlab.company.com` or `gitlab.company.com/api/v4`

2. **Token Configuration**: 
   - Generate personal access token in your GitLab instance
   - Required scopes: `api`, `read_repository`, `write_repository`
   - Stored with domain normalization for reliable matching

3. **Clone & Push**: 
   - Search repositories by name or path
   - SSH key generation and upload to instance
   - Multi-platform push with automatic domain/token matching

4. **Verification**: Application verifies token validity when connecting to instance

## Settings & Customization

### Performance
- **Commit Analyzer**: Toggle real-time commit quality analysis (≈1 MB RAM per analysis)
- **Plain Text Highlighting**: Auto-enable for large diffs (500+ lines or 260K+ chars)
- **Diff View Cache**: Recent diffs cached for faster reopening

### Display
- **Theme**: 10 theme palettes with light/dark variants
- **Font Size**: Small (14px), Medium (16px), Large (18px)
- **Compact Mode**: Reduce row heights in commit graph
- **Reduced Motion**: Disable animations
- **Avatar Display**: Toggle author avatars in commit graph
- **Diff Options**:
  - Line wrapping in diff view
  - Line number visibility
  - Inline diff highlighting mode

### Output
- **Diff Simplification**: Auto-disable highlighting for very large files (35K+ lines or 1.2M+ chars)
- **Fast Path**: Plain text mode for log, txt, csv, lock, patch files

## Architecture Notes

### Git State Management
- Centralized `GitState` with reactive properties (Vue refs)
- Separate composables for domain-specific operations (branches, commits, tokens, etc.)
- Command debouncing and caching to minimize Tauri IPC overhead
- Diff viewer cache with size-limited LRU eviction

### Diff Viewer Optimization
- Virtualized rendering for large files (20px line height)
- Inline diff LCS (longest common subsequence) algorithm for character-level changes
- Syntax highlighting cache with ~2000-entry LRU
- Plain text fast path for >4KB lines
- Format-specific fast path for log/csv/txt/patch files

### Commit Analyzer
- On-demand analysis triggered during commit message composition
- Diff statistics (additions/deletions/language detection)
- Message linting with customizable rules
- Caches analysis per commit SHA

### Platform Integration
- Unified token management system with provider-specific keys
- Domain normalization for self-hosted instances (strips protocol/path/port validation)
- Separate API endpoints for GitHub (REST), GitLab (GraphQL/REST hybrid)
- SSH key generation and upload for Git over SSH support

## Notes

- **Token Storage**: Provider tokens stored per provider key. GitHub legacy token also synchronized for backward compatibility.
- **GitLab Self-Hosted**: Domain matching is normalized (case-insensitive, port-aware) to handle user input variations.
- **Offline Mode**: Application requires Git to be installed and accessible. Most operations work on local repositories without network.
- **Session Restore**: Open repositories and view state are restored on app restart (configurable in settings).
- **Diff Performance**: Very large diffs (>35K lines) automatically switch to plain text mode for performance.
