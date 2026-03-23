# System Architecture

## 1. Architecture Principles

GitSwamp follows a three-layer architectural pattern:

1. **Presentation Layer** - Vue 3 UI components
2. **Application Layer** - Composables and state management
3. **Service Layer** - Tauri commands and Rust services
4. **Data Layer** - Git repositories and local storage

## 2. Overall System Design

### 2.1 Layered Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Presentation Layer                        │
│  ┌─────────────┬──────────────┬──────────────┬────────────┐ │
│  │   Layout    │  Repository  │   Commits    │    UI      │ │
│  │ Components  │  Components  │ Components   │ Components │ │
│  └──────────────────────────────────────────────────────────┘ │
└────────────┬────────────────────────────────────────────────┘
             │
┌────────────▼─────────────────────────────────────────────────┐
│              Application Layer (Composables)                  │
│  ┌──────────────────┐          ┌──────────────────┐         │
│  │   useGit.ts      │          │   useToast.ts    │         │
│  │  (Core Logic)    │          │ (Notifications)  │         │
│  └──────────────────┘          └──────────────────┘         │
└────────────┬────────────────────────────────────────────────┘
             │
        Tauri IPC
   (Type-Safe RPC)
             │
┌────────────▼─────────────────────────────────────────────────┐
│              Service Layer (Rust Backend)                     │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Command Handlers (lib.rs)                 │  │
│  │  ┌──────────┬──────────┬──────────┬──────────┐        │  │
│  │  │ Repository│ Commits  │ Branches │  Status  │        │  │
│  │  └──────────┴──────────┴──────────┴──────────┘        │  │
│  │  ┌──────────┬──────────┬──────────┬──────────┐        │  │
│  │  │   Diff   │  Stash   │   Tags   │  Clone   │        │  │
│  │  └──────────┴──────────┴──────────┴──────────┘        │  │
│  │  ┌──────────┬──────────┬──────────┬──────────┐        │  │
│  │  │Operations│Credentials│ GitHub  │ GitLab   │        │  │
│  │  └──────────┴──────────┴──────────┴──────────┘        │  │
│  └────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │            Service Layer (git_service.rs)              │  │
│  │  - Git2-rs wrapper                                     │  │
│  │  - Credential handling                                 │  │
│  │  - Repository operations                               │  │
│  │  - File operations                                     │  │
│  │  - Remote operations                                   │  │
│  └────────────────────────────────────────────────────────┘  │
└────────────┬────────────────────────────────────────────────┘
             │
┌────────────▼─────────────────────────────────────────────────┐
│                  Data Access Layer                            │
│  ┌──────────────────┐  ┌──────────────────┐               │
│  │    git2-rs       │  │  System Git Exe  │               │
│  │  (libgit2)       │  │  (Fallback)      │               │
│  └──────────────────┘  └──────────────────┘               │
└────────────┬────────────────────────────────────────────────┘
             │
┌────────────▼─────────────────────────────────────────────────┐
│         File System & Git Repositories                       │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  .git/ directory, working tree, configuration, etc.  │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## 3. Component Organization

### 3.1 Frontend Components Hierarchy

```
App.vue (Root)
├── TitleBar (Custom window titlebar)
├── AppHeader (Main navigation)
├── LandingPage or RepositoryTabs
│   └── Sidebar
│       ├── SidebarSection (Branches)
│       ├── SidebarSection (Stashes)
│       └── SidebarSection (Tags)
├── CommitGraph (Main visualization)
│   └── CommitDetails
│       ├── FileItem (list)
│       └── FileDiffViewer
├── ConflictResolver (when needed)
├── SettingsDialog (modal)
├── TerminalPanel (output)
├── ToastContainer (notifications)
└── CloneDialog & InitDialog (modals)
```

### 3.2 Frontend Component Categories

**Layout Components** (4):
- `TitleBar.vue` - Window chrome
- `AppHeader.vue` - Top navigation
- `SettingsDialog.vue` - Settings modal
- `TerminalPanel.vue` - Terminal output

**Repository Management** (6):
- `LandingPage.vue` - Welcome screen
- `RepositoryTabs.vue` - Tab interface
- `Sidebar.vue` - Navigation
- `SidebarSection.vue` - Collapsible section
- `CloneDialog.vue` - Clone UI
- `InitDialog.vue` - Init UI

**Commit Visualization** (3):
- `CommitGraph.vue` - History visualization
- `CommitDetails.vue` - Commit info panel
- `FileItem.vue` - File entry

**UI Components** (6):
- `AppButton.vue` - Styled button
- `AppInput.vue` - Styled input
- `FileDiffViewer.vue` - Diff viewer
- `ConflictResolver.vue` - Merge UI
- `GitCommitIcon.vue` - Avatar/icon
- `ToastContainer.vue` - Notifications

## 4. State Management Architecture

### 4.1 Reactive State Flow

```
Vue Component
    │
    └─> useGit() Composable
            │
            ├─> Reactive State (ref, computed, reactive)
            │   ├─ currentRepo
            │   ├─ commits
            │   ├─ branches
            │   ├─ status
            │   ├─ diffs
            │   └─ more...
            │
            └─> Methods
                ├─ getRepoInfo()
                ├─ getCommits()
                ├─ checkoutBranch()
                ├─ stagFile()
                └─ more...
                    │
                    └─> invoke("command")
                            │
                            └─> Tauri IPC
                                    │
                                    └─> Rust Backend
```

### 4.2 Data Persistence

| Data Type | Storage | Mechanism |
|-----------|---------|-----------|
| Repository State | Memory | Reactive refs in composable |
| Preferences | File System | Tauri config |
| Credentials | File System | Encrypted token storage |
| Session State | Memory | Browser storage (app specific) |

## 5. Tauri Command Architecture

### 5.1 Command Handler Structure

```
User Action in Vue
    │
    └─> useGit.methodName()
            │
            └─> invoke("command_name", payload)
                    │
                    └─> Tauri IPC Serialization
                            │
                            └─> Rust Command Handler
                                    │
                                    ├─> Input Validation
                                    │
                                    ├─> Service Call
                                    │   └─> git_service.rs
                                    │       └─> git2-rs operations
                                    │
                                    ├─> Result Processing
                                    │
                                    └─> Serialization to JSON
                                            │
                                            └─> Return to Vue
                                                    │
                                                    └─> State Update
                                                            │
                                                            └─> UI Re-render
```

### 5.2 Command Organization

**12 Command Modules:**

1. `repository.rs` - Repository metadata
2. `commits.rs` - Commit history
3. `commit_files.rs` - Files in commits
4. `branches.rs` - Branch operations
5. `status.rs` - Working directory
6. `diff.rs` - Diff operations
7. `stash.rs` - Stash management
8. `tags.rs` - Tag management
9. `clone_init.rs` - Repo creation
10. `operations.rs` - Advanced Git ops
11. `credentials.rs` - Auth management
12. `github_gitlab.rs` - Provider integration

## 6. Data Model Architecture

### 6.1 Data Flow Through System

```
Git Repository
    │
    └─> git2-rs library
            │
            ├─> Extract commit data
            ├─> Extract branch data
            ├─> Extract file status
            ├─> Extract diff data
            └─ Deserialize to models
                    │
                    └─> Rust Models
                        ├─ CommitInfo
                        ├─ BranchInfo
                        ├─ FileStatusInfo
                        ├─ RepoInfo
                        ├─ FileDiff
                        └─ more...
                            │
                            └─> Serialize to JSON
                                    │
                                    └─> Send via IPC
                                            │
                                            └─> TypeScript Types
                                                ├─ CommitInfo
                                                ├─ BranchInfo
                                                ├─ FileStatusInfo
                                                ├─ RepoInfo
                                                ├─ FileDiff
                                                └─ more...
                                                    │
                                                    └─> Vue Reactive State
                                                            │
                                                            └─> Component Re-render
```

### 6.2 Model Relationships

```
RepoInfo
├─ remote_urls: Vec<String>
├─ branches: Vec<BranchInfo>
├─ stashes: Vec<StashInfo>
└─ tags: Vec<TagInfo>

CommitInfo
├─ author: String
├─ timestamp: i64
├─ message: String
├─ files: Vec<CommitFileInfo>
└─ parents: Vec<String>

FileStatusInfo
├─ path: String
├─ status: String
├─ staged: bool
└─ untracked: bool

FileDiff
├─ path: String
├─ hunks: Vec<DiffHunk>
└─ lines: Vec<DiffLine>
```

## 7. Git Operations Pipeline

### 7.1 Commit Fetch Pipeline

```
User clicks "Load Commits"
    │
    └─> CommitGraph.onMounted()
            │
            └─> useGit.getCommits()
                    │
                    └─> invoke("get_commits", { ... })
                            │
                            └─> commits::get_commits handler
                                    │
                                    ├─> Open repository
                                    ├─> Initialize revwalk
                                    ├─> Iterate commits
                                    ├─> Extract metadata
                                    ├─> Build graph structure
                                    └─> Return vector
                                            │
                                            └─> useGit updates reactive state
                                                    │
                                                    └─> CommitGraph reactively
                                                        renders visualization
```

### 7.2 File Staging Pipeline

```
User selects file + clicks "Stage"
    │
    └─> CommitDetails.stageFile()
            │
            └─> useGit.stageFile(path)
                    │
                    └─> invoke("stage_file", { path })
                            │
                            └─> status::stage_file handler
                                    │
                                    ├─> Open repository
                                    ├─> Get index
                                    ├─> Add file to index
                                    ├─> Write index
                                    └─> Return success
                                            │
                                            └─> useGit.getStatus()
                                                    │
                                                    └─> State updates
                                                            │
                                                            └─> UI reflects change
```

## 8. Communication Protocols

### 8.1 Tauri IPC Protocol

**Request Format:**
```json
{
  "cmd": "command_name",
  "callback": "callback_id",
  "error": "error_id",
  "payload": {
    "field": "value"
  }
}
```

**Response Format:**
```json
{
  "status": "ok" | "error",
  "payload": { ... },
  "error": "error message"
}
```

### 8.2 Type Safety in IPC

- Frontend sends TypeScript objects
- Serde serializes to JSON
- Rust deserializes to typed structs
- Rust methods work with typed structs
- Results serialize back to JSON
- Frontend receives typed objects

## 9. Error Handling Architecture

### 9.1 Error Flow

```
Operation Error in Rust
    │
    └─> Error Enum
            │
            └─> Display implementation
                    │
                    └─> Serialize to String
                            │
                            └─> Send to Frontend
                                    │
                                    └─> Vue catches error
                                            │
                                            └─> Toast notification or UI message
```

### 9.2 Error Types

| Level | Handling | Example |
|-------|----------|---------|
| Critical | Toast + Dialog | Repository corruption |
| High | Toast notification | Failed push |
| Medium | Inline message | Merge conflict |
| Low | Status indicator | File not found |

## 10. Performance Architecture

### 10.1 Optimization Strategies

**Frontend:**
- Virtual scrolling for large lists
- Lazy loading of components
- Memoization of expensive computations
- Debounced file watcher

**Backend:**
- Efficient git2-rs operations
- Caching where appropriate
- Parallel operations when possible
- Minimal memory allocations

**IPC:**
- Batched operations
- Pagination for large datasets
- Selective data transmission

### 10.2 Caching Strategy

```
Frontend Memory Cache
├─ Current repository info (long-lived)
├─ Current branch (session duration)
├─ Commit list (invalidated on new commit)
└─ File diffs (invalidated on file changes)
```

## 11. Threading & Concurrency

### 11.1 Tauri Command Executor

- Each command runs in Tauri's thread pool
- Commands execute asynchronously from UI thread
- Results returned via IPC when ready
- No blocking operations on main thread

### 11.2 Long-Running Operations

- Progress indication via toast notifications
- Can be canceled via IPC message
- Resources cleaned up on completion

## 12. Security Architecture

### 12.1 Credential Handling

```
User Input Credential
    │
    └─> validate
            │
            └─> encrypt (optional)
                    │
                    └─> save to secure storage
                            │
                            └─> when needed:
                                ├─> retrieve from storage
                                ├─> decrypt
                                ├─> use in operation
                                └─> forget from memory
```

### 12.2 Access Control

| Operation | Permission | Check |
|-----------|----------|-------|
| Read repo | Always allowed | Path exists |
| Write to repo | Always allowed | Has write access |
| Remote ops | Requires credentials | Auth validation |
| SSH ops | Requires SSH setup | SSH key exists |

## 13. Integration Points

### 13.1 External Services

```
GitSwamp Application
├─ GitHub API
│  ├─> Search repositories
│  ├─> Retrieve user info
│  └─> Verify tokens
├─ GitLab API
│  ├─> Search repositories
│  ├─> SSH key management
│  └─> Verify tokens
└─ System Git
   ├─> As fallback
   └─> For SSH operations
```

### 13.2 System Integration

```
Tauri Application
├─ File Dialog Plugin
│  └─> Open repository picker
├─ Opener Plugin
│  └─> Open files with system app
└─ Native Menus
   ├─> Right-click context
   └─> Top menu bar
```

## 14. Deployment Architecture

### 14.1 Build Output Structure

```
dist/                    # Web frontend build
src-tauri/target/        # Rust binary build
├─ release/              # Release build
│  ├─ gitswamp.exe      # Windows executable
│  ├─ gitswamp          # Linux binary
│  └─ GitSwamp.app      # macOS app bundle
└─ debug/                # Debug build
```

### 14.2 Distribution Artifacts

- Windows: `.msi` installer
- macOS: `.dmg` or `.app.tar.gz`
- Linux: `.AppImage` or distribution packages

---

**Related Documentation:**
- [02_DATA_MODELS.md](./DOCUMENTATION_02_DATA_MODELS.md) - Detailed data structures
- [04_FRONTEND_OVERVIEW.md](./DOCUMENTATION_04_FRONTEND_OVERVIEW.md) - Frontend details
- [08_BACKEND_OVERVIEW.md](./DOCUMENTATION_08_BACKEND_OVERVIEW.md) - Backend details
