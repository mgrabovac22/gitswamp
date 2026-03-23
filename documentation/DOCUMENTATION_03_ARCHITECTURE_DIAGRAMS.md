# Architecture Diagrams

## 1. High-Level System Architecture

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃                     GitSwamp Application                    ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃                                                              ┃
┃  ┌─────────────────────────────────────────────────────┐  ┃
┃  │         Vue 3 Frontend (TypeScript)                  │  ┃
┃  │  ┌──────────────────────────────────────────────┐   │  ┃
┃  │  │  Components (19 total)                       │   │  ┃
┃  │  │  ├─ Layout (TitleBar, Header, etc.)         │   │  ┃
┃  │  │  ├─ Repository (Sidebar, Dialogs)           │   │  ┃
┃  │  │  ├─ Commits (Graph, Details, FileItem)      │   │  ┃
┃  │  │  └─ UI (Button, Input, DiffViewer, etc.)    │   │  ┃
┃  │  └──────────────────────────────────────────────┘   │  ┃
┃  │  ┌──────────────────────────────────────────────┐   │  ┃
┃  │  │  Composables                                 │   │  ┃
┃  │  │  ├─ useGit() - Core operations              │   │  ┃
┃  │  │  └─ useToast() - Notifications              │   │  ┃
┃  │  └──────────────────────────────────────────────┘   │  ┃
┃  └─────────────────────────────────────────────────────┘  ┃
┃                         ↓↑                                   ┃
┃                  Tauri IPC Channel                           ┃
┃                  (Type-Safe RPC)                             ┃
┃                         ↓↑                                   ┃
┃  ┌─────────────────────────────────────────────────────┐  ┃
┃  │      Rust Backend (Command Handlers)                │  ┃
┃  │  ┌──────────────────────────────────────────────┐   │  ┃
┃  │  │  12 Command Modules (47 commands)           │   │  ┃
┃  │  │  ├─ Repository, Commits, Branches           │   │  ┃
┃  │  │  ├─ Status, Diff, Stash, Tags               │   │  ┃
┃  │  │  ├─ Clone/Init, Operations                  │   │  ┃
┃  │  │  ├─ Credentials, GitHub/GitLab              │   │  ┃
┃  │  │  └─ More...                                 │   │  ┃
┃  │  └──────────────────────────────────────────────┘   │  ┃
┃  │  ┌──────────────────────────────────────────────┐   │  ┃
┃  │  │  Services Layer                             │   │  ┃
┃  │  │  └─ GitService (git2-rs wrapper)            │   │  ┃
┃  │  └──────────────────────────────────────────────┘   │  ┃
┃  │  ┌──────────────────────────────────────────────┐   │  ┃
┃  │  │  Data Models (10+)                          │   │  ┃
┃  │  │  ├─ CommitInfo, BranchInfo                  │   │  ┃
┃  │  │  ├─ FileStatusInfo, FileDiff                │   │  ┃
┃  │  │  └─ More...                                 │   │  ┃
┃  │  └──────────────────────────────────────────────┘   │  ┃
┃  └─────────────────────────────────────────────────────┘  ┃
┃                         ↓↑                                   ┃
┃  ┌─────────────────────────────────────────────────────┐  ┃
┃  │          Git2-rs Library (libgit2)                 │  ┃
┃  │  ├─ Repository operations                        │  ┃
┃  │  ├─ Commit history traversal                     │  ┃
┃  │  ├─ Branch/Tag management                        │  ┃
┃  │  └─ Diff generation                              │  ┃
┃  └─────────────────────────────────────────────────────┘  ┃
┃                         ↓↑                                   ┃
┃  ┌─────────────────────────────────────────────────────┐  ┃
┃  │        Git Repository (File System)                │  ┃
┃  │  ├─ .git/ directory                              │  ┃
┃  │  ├─ Working directory                            │  ┃
┃  │  └─ Local configuration                          │  ┃
┃  └─────────────────────────────────────────────────────┘  ┃
┃                         ↓↑                                   ┃
┃  ┌─────────────────────────────────────────────────────┐  ┃
┃  │       Remote Repositories                          │  ┃
┃  │  ├─ GitHub (HTTPS/SSH)                           │  ┃
┃  │  ├─ GitLab (HTTPS/SSH)                           │  ┃
┃  │  └─ Other Git Remotes                            │  ┃
┃  └─────────────────────────────────────────────────────┘  ┃
┃                                                              ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

## 2. Frontend Component Hierarchy

```
App.vue (Root)
├── TitleBar
│   └── Custom window titlebar with drag support
├── AppHeader
│   ├── Repository name/path
│   ├── Current branch
│   └── Navigation buttons
├── Main Content (Either):
│   ├── LandingPage
│   │   ├── Open Repository button
│   │   ├── Clone Repository button
│   │   └── Initialize Repository button
│   │
│   └── RepositoryTabs (Active Repository)
│       ├── Sidebar
│       │   ├── SidebarSection "Branches"
│       │   │   └── Branch items
│       │   ├── SidebarSection "Stashes"
│       │   │   └── Stash items
│       │   └── SidebarSection "Tags"
│       │       └── Tag items
│       │
│       ├── CommitGraph (Main Area)
│       │   └── Visual commit history graph
│       │
│       └── CommitDetails (Right Panel)
│           ├── Commit information
│           ├── Commit message
│           └── File list
│               └── FileItem (for each file)
│                   └── FileItem details
│
├── SettingsDialog (Modal)
│   ├── Theme selection
│   ├── Font size adjustment
│   ├── Compact mode toggle
│   └── Avatar settings
│
├── CloneDialog (Modal)
│   ├── Repository URL input
│   ├── Directory selection
│   └── Provider search (GitHub/GitLab)
│
├── InitDialog (Modal)
│   └── Directory selection
│
├── ConflictResolver (Modal, when needed)
│   ├── Conflict display
│   ├── Three-way view
│   └── Resolution controls
│
├── TerminalPanel
│   └── Terminal output display
│
└── ToastContainer
    └── Toast notifications
```

## 3. Data Flow Architecture

```
User Interaction
    │
    └──> Vue Component
         │
         ├──> Template / Event Handler
         │    └──> @click, @input, etc.
         │
         └──> Composable (useGit, useToast)
              │
              ├──> Reactive State (ref, computed)
              │    ├─ currentRepo
              │    ├─ commits
              │    ├─ branches
              │    └─ status
              │
              └──> Methods
                   │
                   └──> invoke("tauri_command", params)
                        │
                        └──> Tauri IPC Layer
                             │
                             └──> JSON Serialization
                                  │
                                  └──> Rust Backend (async)
                                       │
                                       ├──> Validate Input
                                       │
                                       ├──> Call Service
                                       │    │
                                       │    └──> git2-rs operations
                                       │
                                       ├──> Process Result
                                       │
                                       └──> Serialize to JSON
                                            │
                                            └──> Send to Frontend
                                                 │
                                                 └──> Promise resolves
                                                      │
                                                      └──> Update reactive state
                                                           │
                                                           └──> Vue reactivity triggers
                                                                │
                                                                └──> Component re-renders
                                                                     │
                                                                     └──> UI updates
```

## 4. Command Processing Pipeline

```
┌─────────────────────────────────────────────────────┐
│   Frontend Component Method Call                     │
│   await useGit().checkoutBranch("main")             │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   Composable Method                                  │
│   invoke("checkout_branch", {                       │
│     repo_path: string,                              │
│     branch_name: string                             │
│   })                                                │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   Tauri IPC Serialization                           │
│   JSON.stringify({ repo_path, branch_name })       │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   Rust Handler: branches::checkout_branch           │
│   async fn checkout_branch(                         │
│     repo_path: String,                              │
│     branch_name: String,                            │
│   ) -> Result<(), String>                           │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   Async Task Execution                              │
│   tokio::task::spawn_blocking(move || {             │
│     // CPU-intensive Git operations                 │
│   })                                                │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   Service Call: GitService                          │
│   - Open repository                                 │
│   - Get branch object                               │
│   - Update HEAD                                     │
│   - Update working directory                        │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   git2-rs Operations                                │
│   - Repository::open()                              │
│   - find_branch()                                   │
│   - set_head()                                      │
│   - checkout()                                      │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   Result: Ok(()) or Err(String)                     │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   Serialize Result to JSON                          │
│   Return to Frontend                                │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   Promise Resolves with Result                      │
│   Composable updates reactive state                 │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   Vue Reactivity Triggers                           │
│   Components depending on state update              │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│   Component Re-renders                              │
│   UI displays new branch as current                 │
└─────────────────────────────────────────────────────┘
```

## 5. State Management Flow

```
┌──────────────────────────────────────────────────┐
│   App Level State                                │
│  (useGit composable in root)                     │
│                                                  │
│  ├── currentRepo: RepoInfo                       │
│  ├── commits: CommitInfo[]                       │
│  ├── branches: BranchInfo[]                      │
│  ├── status: FileStatusInfo[]                    │
│  ├── currentBranch: string                       │
│  ├── diffs: Map<string, FileDiff>                │
│  ├── stashes: StashInfo[]                        │
│  ├── tags: TagInfo[]                             │
│  ├── isLoading: boolean                          │
│  │                                               │
│  └── Methods (70+):                              │
│      ├── openRepository()                        │
│      ├── getCommits()                            │
│      ├── checkoutBranch()                        │
│      ├── stageFile()                             │
│      └── ... more methods                        │
└──────────────────────────────────────────────────┘
         ↑                           ↓
         │                           │
    Provide              (Inject in child components)
         │                           ↓
         │              ┌────────────────────────┐
         │              │   Component Props      │
         │              │  (derived from state)  │
         │              └────────────────────────┘
         │                           ↓
         │              ┌────────────────────────┐
         │              │   Component State      │
         │              │  (local to component)  │
         │              └────────────────────────┘
         │                           ↓
         │              ┌────────────────────────┐
         │              │   Computed Properties  │
         │              │  (derived state)       │
         │              └────────────────────────┘
         │                           ↓
         │              ┌────────────────────────┐
         │              │   Render Output        │
         │              │  (component template)  │
         │              └────────────────────────┘
         │
    User Interaction
         │
    Updates state
         ↓
    Triggers watchers/effects
         ↓
    Components re-render
```

## 6. Tauri Command Architecture

```
Frontend         │         Backend
(TypeScript)     │         (Rust)
─────────────────┼──────────────────
                 │
  Vue Component   │
       │          │
       │ calls    │
       │ method   │
       │          │
       ▼          │
  useGit.ts       │
       │          │
       │ invoke() │
       │          │
       └──────────┼──────────────┐
                  │              │
              IPC Channel    Command Handler
                  │              │
              JSON serial    (async fn)
                  │              │
              Type-safe          ├─ validate_input()
              RPC call           │
                  │              ├─ call_service()
                  │              │
                  │              ├─ process_result()
                  │              │
                  └──────────────┼──────────────┐
                                 │              │
                            Serialize      Error handling
                            to JSON        (return Err)
                                 │              │
                                 └──────────────┘
                                       │
                                   JSON response
                                       │
                  ┌────────────────────┘
                  │
              Promise
              resolves
                  │
              Type-safe
              result
                  │
                  ▼
            Update state
                  │
                  ▼
            Vue reactivity
                  │
                  ▼
            Component
            re-renders
```

## 7. Git Operation Flow

```
User Action                Git Operation                 Result
───────────                ──────────────                 ──────
  │                              │                          │
  ├─ Create commit ─────────────>├─ Stage files             │
  │                              ├─ Create commit           │
  │                              └─ Update HEAD ──────────>├─ New commit
  │                                                         │
  ├─ Checkout branch ───────────>├─ Validate branch        │
  │                              ├─ Update HEAD             │
  │                              └─ Update working dir ───>├─ Changed branch
  │                                                         │
  ├─ Merge ─────────────────────>├─ Create merge commit    │
  │                              ├─ Handle conflicts        │
  │                              └─ Update refs ──────────>├─ Merged state
  │                                                         │
  ├─ Push ──────────────────────>├─ Connect to remote      │
  │                              ├─ Transfer objects        │
  │                              └─ Update remote ────────>├─ Remote updated
  │                                                         │
  └─ Pull ──────────────────────>├─ Fetch remote            │
                                  ├─ Create merge commit    │
                                  └─ Update local ────────>└─ Local updated
```

## 8. Component Communication

```
┌─────────────────────────────────────────────┐
│            App.vue (Root)                   │
│  Provides: useGit()                         │
│           useToast()                        │
└──────────┬────────────────────────────────┬─┘
           │                                │
    ┌──────▼──────┐            ┌────────────▼──────┐
    │ RepositoryTabs  │            │ SettingsDialog │
    │   (emits)    │            │   (modal)        │
    └──────┬──────┘            └────────────────────┘
           │
    ┌──────▼──────┐
    │   Sidebar   │
    │  (emits)    │
    └──────┬──────┘
           │
    ┌──────▼──────┐            ┌────────────────────┐
    │CommitGraph  │            │ConflictResolver    │
    │  (emits)    │            │   (modal)          │
    └──────┬──────┘            └────────────────────┘
           │
    ┌──────▼──────┐
    │CommitDetails│
    │  (emits)    │
    └──────┬──────┘
           │
    ┌──────▼──────┐
    │FileItem     │
    │  (emits)    │
    └──────┬──────┘
           │
    ┌──────▼──────┐
    │FileDiffViewer
    │   (final)    │
    └──────────────┘

Legend:
(emits) = Component emits events
(modal) = Modal dialog component
(final) = Terminal component
```

## 9. Technology Stack Layers

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  User Interface Layer                     ┃
┃  ├─ Vue 3 Components (Reactive UI)      ┃
┃  ├─ Tailwind CSS (Styling)              ┃
┃  ├─ Lucide Icons (Icon Library)         ┃
┃  └─ TypeScript (Type Safety)            ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃  Presentation Layer                      ┃
┃  ├─ Composables (useGit, useToast)      ┃
┃  ├─ Type Definitions                    ┃
┃  ├─ State Management                    ┃
┃  └─ Event Handling                      ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃  IPC & Bridge Layer                     ┃
┃  ├─ Tauri Core                          ┃
┃  ├─ Command Serialization (serde)       ┃
┃  ├─ Type-Safe RPC                       ┃
┃  └─ Async Task Handling                 ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃  Application Logic Layer                 ┃
┃  ├─ Command Handlers (12 modules)       ┃
┃  ├─ Service Layer (GitService)          ┃
┃  ├─ Data Models                         ┃
┃  └─ Error Handling                      ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃  Git Abstraction Layer                  ┃
┃  ├─ git2-rs (Rust bindings)             ┃
┃  ├─ libgit2 (C library)                 ┃
┃  ├─ System Git (Fallback)               ┃
┃  └─ SSH & HTTPS Support                 ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃  Repository Layer                        ┃
┃  ├─ File System                          ┃
┃  ├─ .git Directory                      ┃
┃  ├─ Configuration                       ┃
┃  └─ Object Database                     ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

## 10. Request/Response Cycle

```
TIME →

Frontend          Tauri          Backend
═════════         ═════          ═══════

User clicks
  │
  ├─ Event triggered
  │
  └─ Composable method
     │
     ├─ invoke("command")
     │
     └──────────────────────► Handler receives
                              (deserialize JSON)
                              │
                              ├─ Validate input
                              │
                              ├─ Execute logic
                              │
                              ├─ Call git2-rs
                              │
                              ├─ Process result
                              │
                              └─ Serialize to JSON
                                 │
                                 └─────────────────► Promise resolves
                                                    │
                                                    ├─ Update state
                                                    │
                                                    ├─ Computed properties
                                                    │  update
                                                    │
                                                    └─ Component
                                                       re-renders
```

---

**Note:** These diagrams provide visual representations of GitSwamp's architecture. For detailed information, refer to the specific documentation files referenced throughout this guide.
