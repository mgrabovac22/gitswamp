# Frontend Overview

## 1. Frontend Architecture

GitSwamp's frontend is built with Vue 3, TypeScript, and Tailwind CSS. It provides a modern, responsive interface for Git repository management with real-time updates and intuitive workflow.

### 1.1 Technology Stack

| Technology | Version | Purpose |
|-----------|---------|---------|
| Vue 3 | 3.5.13 | Progressive JavaScript framework |
| TypeScript | 5.6.2 | Static type checking |
| Vite | 6.0.3 | Development server & build tool |
| Tailwind CSS | 4.1.12 | Utility-first CSS framework |
| Lucide Vue | 0.487.0 | Icon system |
| CVA | 0.7.1 | Component styling |

## 2. Project Structure

### 2.1 Directory Layout

```
src/
├── app/                           # Bootstrap and app startup
│   └── bootstrap.ts
├── domain/                        # Domain logic and composables
│   └── git/
│       ├── UseGit.ts
│       └── composables/
├── shared/                        # Shared UI, notifications, config
│   ├── config/
│   ├── notifications/
│   ├── ui/
│   ├── codeView.ts
│   └── themePreferences.ts
├── styles/                        # Global styling and theme tokens
│   ├── index.css
│   ├── tailwind.css
│   └── theme.css
├── types/                         # TypeScript data models
│   └── models/
├── view/                          # Main feature views and screens
│   ├── repository/
│   ├── commit/
│   └── shell/
├── App.vue                        # Root component
├── main.ts                        # Entry point
└── vite-env.d.ts                  # Vite type definitions
```

## 3. Vue Components

### 3.1 Component Overview

**Total Components:** 41 Vue components (across `src/view`, `src/shared/ui`, and `src/view/shell`)

#### Layout & Shell Components (7) - src/view/shell/

1. **TitleBar.vue**
   - Custom window title bar (Tauri)
   - Window control buttons (minimize, maximize, close)
   - Logo display with dynamic theme support
   - Drag support for window movement
   - Custom styling with light/dark theme toggle

2. **AppHeader.vue**
   - Top navigation bar with app branding
   - Primary action buttons: Pull, Push, Fetch
   - Branch quick actions (Checkout, Ghost branch)
   - Repository information display
   - Loading state indicators for async operations
   - Integrated BranchQuickActions menu

3. **SettingsDialog.vue**
   - User preferences modal with tabs
   - Theme selection: Light/Dark/System modes with palette choices
   - Font size adjustment (small/medium/large)
   - View options: Compact mode, Avatar display, Motion preferences
   - Diff view options: Line wrapping, line numbers
   - Commit analyzer settings
   - Session persistence options
   - Fullscreen and session restore controls
   - Smart .gitignore Wizard preference
   - Background maintenance and reminder preferences

4. **TerminalPanel.vue**
   - Integrated terminal for manual Git commands
   - Command input with history
   - Collapsible panel with toggle button
   - Output display with syntax highlighting
   - Error message display
   - Clear button and refresh functionality
   - Allow All Commands toggle for security control
   - Real-time output streaming
   - Destructive Git command safety prompts
   - Optional safety stash before dangerous command execution

5. **BranchQuickActions.vue**
   - Dropdown menu for quick branch operations
   - Create new branch option
   - Switch to ghost branch (experimental feature)
   - Materialize ghost branch
   - Discard ghost branch
   - Loading state handling

6. **CommandPalette.vue**
   - Ctrl+K searchable command launcher
   - Repository, view, terminal, settings, logs, and analysis actions
   - Keyboard-friendly filtering and execution

7. **LogsPanel.vue**
   - Optional startup/runtime logs view
   - Toggleable from settings/help/debug actions
   - Useful for diagnosing background Git and integration behavior

#### Repository Components (9) - src/view/repository/

6. **LandingPage.vue**
   - Welcome screen displayed when no repository is open
   - Open existing repository button with file browser
   - Clone repository option with URL input
   - Initialize new repository option
   - Recent repositories list (if available)
   - Hosts the landing dashboard
   - Visual GitSwamp branding and feature overview
   - Quick access to online guide

7. **LandingDashboard.vue**
   - Start-screen work dashboard
   - Lists GitHub pull requests authored by the authenticated user
   - Lists GitHub issues assigned to the authenticated user
   - Falls back to local open/recent repository summaries when no token is available
   - Search/filter controls for pull requests and issues

8. **RepositoryTabs.vue** (Menu System)
   - Multi-tab interface for managing multiple repositories
   - Tab switching and close functionality
   - Active tab indication and labels
   - **Comprehensive Hamburger Menu with 4 sections:**
     - **File Menu:**
       - New Tab (Ctrl+T)
       - Open Repository (Ctrl+O)
       - Close Current Tab (Ctrl+W)
       - Reopen Closed Tab (Ctrl+Shift+T)
       - Create a Gist (Ctrl+Shift+G)
     - **Edit Menu:**
       - Copy Repository Path (Ctrl+Shift+C)
       - Refresh Repository (Ctrl+Shift+R)
       - Open in VS Code (Ctrl+Shift+O)
     - **View Menu:**
       - Toggle Terminal (Ctrl+`)
       - Visualize Commit History/Graph (Alt+1)
       - Galaxy View (Alt+2)
       - Productivity Arena (Alt+3)
       - Time Machine (Alt+4)
       - Usual Conflict Suspects/Heatmap (Alt+5)
       - Burnout Analytics (Alt+6)
       - Open in Folder Explorer (Alt+O)
       - Open integrations/options (Ctrl+Shift+I)
       - Open Settings (Ctrl+,)
     - **Help Menu:**
       - Help and Shortcuts (F1)
       - About GitSwamp
       - Open Online Guide
       - Report Issue
   - Context-aware menu item disabling based on repo state
   - Keyboard shortcut support
   - Ctrl+Tab next-tab behavior and middle-click tab close
   - Menu positioning relative to button
   - ESC key closes menu and help panels

9. **RepositoryWorkspace.vue** (Main Content Area)
   - Main workspace container for repository content
   - Integrates all visualization and editing panels
   - Manages layout: Sidebar, Graph/Panels, Details
   - Handles multiple history view modes (graph, galaxy, productivity, time-machine, conflict-heatmap, burnout, remote-insights, conflict-resolve)
   - Terminal panel integration
   - Diff viewer modal
   - Conflict resolver modal
   - Coordinates state between child components

10. **RepositorySidebar.vue**
   - Main navigation sidebar for repository metadata
   - Collapsible sections with icons:
     - **Branches** - Local and remote branches
     - **Stashes** - Saved work items
     - **Tags** - Repository tags
     - **Remotes** - Remote configurations
     - **Issues** - GitHub/GitLab issues (if linked)
     - **Pull Requests** - Open PRs from current branches
   - Branch creation button
   - Search functionality across sections
   - Right-click context menus
   - Section expanded/collapsed state persistence

11. **RepositorySidebarSection.vue**
    - Reusable collapsible section component
    - Icon display with section label
    - Item count display
    - Expand/collapse chevron icon
    - Click handler for toggle events
    - auto animations

12. **Repository Action & Auth Dialogs**
    - **CloneDialog.vue**: Clone repository from URL
      - Repository URL/HTTPS input
      - Directory selection
      - Clone progress indication
      - GitHub/GitLab repository search
      - Provider authentication

    - **InitDialog.vue**: Initialize new repository
      - Directory selection
      - Repository initialization
      - Initial configuration
      - Success confirmation

    - **GhostBranchDialog.vue**: Ghost branch operations
      - Create experimental/ghost branch
      - Materialize ghost branch to real branch
      - Discard ghost changes
      - Preview of ghost changes

    - **MultiPlatformPushDialog.vue**: Multi-remote push
      - Select multiple remotes for push
      - Configure push strategies
      - Batch push operations

    - **RepositoryActionDialogs.vue**: Generic action dialogs
      - Merge operations
      - Rebase operations
      - Cherry-pick operations
      - Generic confirmation and parameter dialogs

    - **RepositoryAuthDialogs.vue**: Authentication
      - Credential input (username/password)
      - Token input dialogs
      - SSH key management
      - OAuth authentication flows

13. **RemoteInsightsPanel.vue**
    - GitHub/GitLab integration panel
    - Pull request visualization and management
    - Issue tracking and display
    - Repository statistics
    - Contributor information
    - Activity timeline
    - Links to create new PRs/issues

#### Commit Visualization & Analytics (8) - src/view/commit/

14. **CommitGraph.vue** (Core Feature)
    - Interactive commit history visualization
    - Lane-based graph rendering (branch flow visualization)
    - Zoomable and pannable interface
    - Branch merge/fork visualization
    - Hover information on commits
    - Row selection for viewing commit details
    - Virtual scrolling for performance with large repositories
    - Configurable row height (compact/normal modes)
    - Keyboard navigation support
    - Responsive to window resizing

15. **CommitDetails.vue**
    - Collapsible details panel for selected commit
    - Commit metadata: Author, Date, Email, Message
    - Files changed in commit with statistics
    - File change icons (added, modified, deleted, renamed)
    - Quick file actions (view diff, stage, unstage)
   - Staged vs Unstaged file grouping
   - Smart .gitignore Wizard integration for untracked generated/private files
   - File Change Map by folder for changed files
   - Visual Commit Builder and analyzer popovers
   - Empty directory `.gitkeep` helper
   - Commit message editing
   - Copy operations (hash, author, message)
   - Conflict status indicators

16. **FileItem.vue**
    - Individual file entry in commit or staging area
    - File type icon based on extension
    - Change statistics (insertions/deletions)
    - Status indicator (added/modified/deleted/renamed)
    - Hover state for interactivity
    - Click to view diff viewer
    - Context menu for file operations

17. **CommitGalaxyPanel.vue** (Canvas Visualization)
  - Canvas-rendered commit/branch visualization
  - Spiral, layers, and constellation layouts
  - 3D, balanced, and circling motion modes
  - Tree mode for branch hierarchy visualization
  - Zoom, pan, click-to-focus, and hover details
  - Memory-aware drawing based on loaded commit data

18. **CommitProductivityPanel.vue** (Analytics)
  - Preview-mode metrics with optional full-history loading
  - Author filtering for per-contributor analytics
  - Commit frequency, streak, and rhythm analysis
  - Weekend/off-hours activity and bottleneck indicators
  - Stability and regression-signal scoring
  - Balance score, arena health score, and contributor comparison
  - Section-level loader overlays and caching
  - Metrics grouped into headline, flow, pressure, and stability cards

19. **CommitTimeMachinePanel.vue** (Time Navigation)
  - Full-history frame navigation through commit snapshots
  - Autoplay and reverse-autoplay stepping
  - SHA search with prev/next match navigation
  - Repository tree snapshot explorer and file preview pane
  - Copyable rollback command for commit- or file-scoped restore workflows
  - Cached commit files, tree paths, and file contents for smooth playback
  - Timeline scrubber and selected-frame progress tracking
  - Loader overlays that avoid flicker while frames are changing

20. **CommitConflictHeatmapPanel.vue** (Conflict Analytics)
  - Merge-window hotspot analysis with 300/500/1000/all scopes
  - Files with highest conflict frequency and risk band indicators
  - Conflict pair diagnostics for recurring cross-file collisions
  - Repository tree heatmap with folder roll-up from child hotspots
  - Search-based filtering and tree-scope filtering
  - Collision index, merge-touch, and conflict-mention summaries
  - Risk scoring cards and top-suspect explanations
  - Expandable tree visualization for conflict pressure hotspots

21. **CommitBurnoutAnalyticsPanel.vue** (Team Focus Analytics)
  - Repository burnout pulse with readable week labels
  - After-hours and weekend work windows
  - Contributor focus and workload indicators
  - Hot-file and bottleneck context not duplicated from Productivity Arena
  - Background-friendly loading to keep scrolling responsive

22. **SmartGitignoreWizard.vue** (Working Tree Assistant)
  - Detects generated/private untracked files from existing status data
  - Groups dependency folders, build output, caches, logs, local secrets, private keys, temp files, and local DBs
  - Adds missing `.gitignore` patterns while avoiding duplicates
  - Supports Ignore selected, Open `.gitignore`, and Keep tracking

#### UI Components (8) - src/shared/ui/

23. **AppButton.vue**
    - Styled button component with CVA variants
    - Multiple style variants:
      - default (primary colored)
      - destructive (red/danger)
      - outline (bordered)
      - secondary (muted)
      - ghost (no background)
      - link (text with underline)
    - Size options: sm, md (default), lg
    - Disabled state with visual feedback
    - Loading state with spinner
    - Icon support with proper spacing
    - Focus visible states for accessibility
    - Keyboard support (Enter/Space)

24. **AppInput.vue**
    - Styled text input component
    - Supports multiple input types (text, password, email, etc.)
    - Placeholder support
    - Disabled state
    - Error state with visual indication
    - Two-way binding with v-model
    - Custom class support
    - Validation-ready structure
    - Accessible with proper labels

25. **FileDiffViewer.vue** (Advanced)
    - Complex diff viewing component
    - Multiple view modes:
      - Unified diff (traditional format)
      - Side-by-side diff (left/right comparison)
    - Syntax highlighting for 50+ languages
   - Hunk-level operations (view/apply/revert)
   - Micro-staging with hunk-level stage, unstage, and discard
   - Edit mode for direct file modification
    - Staged vs working directory diffs
    - Scroll synchronization between sides (side-by-side mode)
    - Line numbers with click highlighting
    - Context line expansion (+/- buttons)
    - Search and replace functionality
    - Collaboration highlight (co-edited lines)
    - Copy/paste operations
    - Responsive to window resizing
    - Virtual scrolling for large files
   - Loading states with animated logo
   - Optional File Journey panel with quick authorship/change context

26. **ConflictResolver.vue**
    - Three-way merge conflict resolution
    - Visual display: Current (ours) | Original | Incoming (theirs)
    - Conflict marker highlighting
    - Hunk-by-hunk resolution
    - Resolution buttons per hunk:
      - Use Ours (keep current branch version)
      - Use Theirs (accept incoming version)
      - Use Both (combine both versions)
      - Manual edit mode
    - Preview of final resolved content
    - Real-time validation
    - Save and commit on resolution
    - Navigation between conflict hunks
    - Statistics (hunks resolved/remaining)
    - Embedded mode (inline in diff viewer)

27. **ToastContainer.vue**
    - Toast notification management
    - Multiple toast support with queue
    - Type-based styling:
      - success (green with checkmark)
      - error (red with X)
      - info (blue with circle)
      - warning (yellow with triangle)
      - loading (spinner animation)
    - Auto-dismiss with configurable duration
    - Manual dismiss buttons
    - Position management (top/bottom, center/side)
    - Animated entry/exit transitions
   - Action buttons support (undo, retry, etc.)
   - Progress bar for auto-dismiss countdown

28. **CloseIconButton.vue**
   - Standardized icon-only close button
   - Shared centering, size, hover, and title behavior for panels/popovers

29. **GitCommitIcon.vue**
   - Reusable GitSwamp commit marker icon

30. **Provider Icons**
   - `BitbucketIcon.vue` and `AzureDevOpsIcon.vue`
   - Provider-specific icons used in clone/options/auth UI

### 3.2 History View Modes

GitSwamp provides 8 history/workspace modes, accessible from the **View** menu and selected shortcuts:

#### 1. **Graph Mode** (Default) - Alt+1
- Interactive commit history visualization
- Lane-based graph rendering showing branch structure
- Displays branches, merges, and commits chronologically
- Primary view for most Git workflows
- Supports zooming, panning, and navigation
- Virtual scrolling for large repositories

#### 2. **Galaxy View** - Alt+2
- Canvas-based commit and branch visualization
- Spiral, layer, and constellation layouts
- 3D, balanced, and circling motion options
- Tree mode for branch hierarchy
- Zoom, pan, hover details, and click-to-focus selection
- Useful for exploratory branch understanding without leaving the repository workspace

#### 3. **Productivity Arena** - Alt+3
- Developer activity and metrics visualization
- Displays productivity patterns across time
- Shows commit frequency by contributor
- Preview window plus full-history load switch
- Author filter that recalculates the dashboard for one contributor
- Time-based activity patterns, streaks, weekend/off-hours analysis
- File change statistics and deletion intensity
- Team productivity comparison and balance scoring
- Arena health, bottleneck, and stability indicators
- Useful for performance analysis and sprint planning

#### 4. **Time Machine** - Alt+4
- Navigate through repository history frame-by-frame
- View repository state at specific commits
- File snapshots at different points in history
- Timeline-based scrubber navigation
- SHA search with next/prev match navigation
- Commit file snapshots, repository tree snapshots, and file preview explorer
- Copyable rollback command for whole-commit or file-scoped rollback
- Diff between historical versions
- Inspect legacy code without checking out
- Smooth autoplay and arrow-step transitions (reduced visual flicker)
- Snapshot panels keep context and update progressively with section loaders and caches
- Useful for forensic analysis and understanding evolution

#### 5. **Usual Conflict Suspects (Conflict Heatmap)** - Alt+5
- Merge conflict hotspot visualization
- Identifies files with highest conflict frequency
- Conflict risk assessment and patterns
- Team collaboration conflict analytics
- Time-based conflict trend analysis
- Merge-window selector (300 / 500 / 1000 / all)
- Suggests risky merge areas
- Search-based filtering plus tree-scope filtering
- Helps prevent problematic merges

#### 6. **Burnout Analytics** - Alt+6
- Repository burnout pulse for recent weeks
- After-hours and weekend activity signals
- Contributor focus and workload risk summaries
- Hot-file ownership and bottleneck context
- Designed to avoid duplicating Productivity Arena metrics

#### 7. **Remote Insights** - Remote sync
- GitHub/GitLab integration panel
- Pull request management and overview
- Issue tracking and display
- Repository statistics and metrics
- Contributor information
- Activity timelines
- Direct links to repository web interface

#### 8. **Conflict Resolve** - Automatic on conflict
- Activated automatically when merge conflicts exist
- Three-way merge visualization (ours | original | theirs)
- Interactive conflict resolution interface
- Hunk-by-hunk resolution controls
- Real-time conflict status
- Saves and commits on resolution

### 3.3 Component Communication

```
App.vue (Root)
    │
    ├─> TitleBar
    │   └─> Window controls
    │
    ├─> AppHeader
    │   ├─> Pull/Push/Fetch buttons
    │   └─> BranchQuickActions
    │       ├─ Create Branch
    │       ├─ Ghost Operations
    │       └─ Branch Switching
    │
    ├─> RepositoryTabs
    │   ├─ Tab management
    │   └─ Hamburger Menu
    │       ├─ File Actions (New Tab, Open, Close, Gist)
    │       ├─ Edit Actions (Copy, Refresh, Open in VS Code)
    │       ├─ View Actions (Terminal, History Modes, Explorer, Settings)
    │       └─ Help Actions (Shortcuts, About, Issues)
    │
    ├─> RepositoryWorkspace (when repo open)
    │   ├─> RepositorySidebar
    │   │   ├─ RepositorySidebarSection (Branches)
    │   │   ├─ RepositorySidebarSection (Stashes)
    │   │   ├─ RepositorySidebarSection (Tags)
    │   │   ├─ RepositorySidebarSection (Remotes)
    │   │   ├─ RepositorySidebarSection (Issues)
    │   │   └─ RepositorySidebarSection (Pull Requests)
    │   │
    │   ├─ CommitGraph (mode: graph)
    │   ├─ CommitGalaxyPanel (mode: galaxy)
    │   ├─ CommitProductivityPanel (mode: productivity)
    │   ├─ CommitTimeMachinePanel (mode: time-machine)
    │   ├─ CommitConflictHeatmapPanel (mode: conflict-heatmap)
    │   ├─ CommitBurnoutAnalyticsPanel (mode: burnout)
    │   ├─ RemoteInsightsPanel (mode: remote-insights)
    │   │
    │   ├─> CommitDetails
    │   │   └─> FileItem
    │   │       └─ emits: view-diff
    │   │           └─> FileDiffViewer (modal)
    │   │
    │   ├─> ConflictResolver (modal, mode: conflict-resolve)
    │   │
    │   └─> TerminalPanel (when enabled)
    │
    ├─> LandingPage (when no repo)
    │   ├─ Open Repository
    │   ├─ Clone Repository
    │   ├─ Initialize Repository
    │   └─ LandingDashboard
    │
    ├─> CloneDialog (modal)
    ├─> InitDialog (modal)
    ├─> GhostBranchDialog (modal)
    ├─> MultiPlatformPushDialog (modal)
    ├─> RepositoryActionDialogs (modal - merge/rebase/cherry-pick)
    ├─> RepositoryAuthDialogs (modal - credentials/tokens)
    ├─> SettingsDialog (modal)
    ├─> CommandPalette (global overlay)
    ├─> LogsPanel (optional overlay)
    └─> ToastContainer (global notifications)
```

## 4. Composables

### 4.1 useGit Composable

**File:** `src/domain/git/UseGit.ts`

**Purpose:** Core Git operations wrapper providing all Git functionality to Vue components.

**Key Features:**
- Repository management (open, close, info)
- Commit operations (fetch, search, details)
- Branch operations (get, create, delete, checkout)
- File operations (stage, unstage, diff, edit)
- Stash operations (push, pop, apply, drop)
- Tag operations (create, delete, get)
- Remote operations (pull, push, fetch)
- GitHub/GitLab integration
- Real-time file watcher
- Token management
- Terminal command execution and safety stash orchestration
- Hunk stage/unstage/discard support
- Provider-token helpers for GitHub, GitLab, Bitbucket, and Azure DevOps

**Reactive State:**
```typescript
const currentRepo = ref<RepoInfo | null>(null);
const commits = ref<CommitInfo[]>([]);
const branches = ref<BranchInfo[]>([]);
const status = ref<FileStatusInfo[]>([]);
const currentBranch = ref<string>("");
const diffs = ref<Map<string, FileDiff>>(new Map());
const stashes = ref<StashInfo[]>([]);
const tags = ref<TagInfo[]>([]);
const isLoading = ref<boolean>(false);
```

**Methods (70+):**
- `openRepository(path: string): Promise<void>`
- `getRepoInfo(): Promise<RepoInfo>`
- `getCommits(offset: number, limit: number): Promise<CommitInfo[]>`
- `checkoutBranch(name: string): Promise<void>`
- `createBranch(name: string): Promise<void>`
- `stageFile(path: string): Promise<void>`
- `createCommit(message: string): Promise<void>`
- And 63 more methods...

### 4.2 useToast Composable

**File:** `src/shared/notifications/useToast.ts`

**Purpose:** Toast notification system management.

**Key Features:**
- Add toast notifications
- Remove specific toasts
- Clear all toasts
- Auto-dismiss support
- Multiple types (success, error, warning, info)

**Methods:**
```typescript
function addToast(message: string, type?: string, duration?: number): void
function removeToast(id: string): void
function clearAll(): void
```

## 5. Type System

### 5.1 TypeScript Configuration

**File:** `tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  }
}
```

### 5.2 Type Definitions

**File:** `src/types/index.ts`

Exports all data types used throughout the frontend:
- `CommitInfo`, `BranchInfo`, `FileStatusInfo`
- `RepoInfo`, `RemoteInfo`
- `CommitFileInfo`, `StashInfo`, `TagInfo`
- `GraphNode`, `GraphEdge`
- `GithubRepo`, `GitlabRepo`
- `DiffLine`, `DiffHunk`, `FileDiff`

## 6. Styling System

### 6.1 Tailwind CSS

**Configuration:** Integrated via `@tailwindcss/vite` plugin in Vite.

**Features:**
- Utility-first CSS
- Dark mode support
- Responsive design
- Custom theme extensions

### 6.2 CSS Architecture

```
styles/
├── index.css              # Main stylesheet
├── tailwind.css          # Tailwind imports
└── theme.css             # Theme variables
```

**Theme Variables:**
- Color scheme (light/dark)
- Font sizes
- Spacing scales
- Border colors
- Background colors

### 6.3 Component Styling

Uses Tailwind utility classes combined with:
- **CVA (Class Variance Authority)** - Component styling variants
- **CLSX** - Conditional class names
- **Tailwind Merge** - Smart class merging

Example:
```typescript
const buttonStyles = cva(
  "px-4 py-2 rounded font-semibold transition",
  {
    variants: {
      variant: {
        primary: "bg-blue-500 text-white hover:bg-blue-600",
        secondary: "bg-gray-200 text-gray-800 hover:bg-gray-300",
        danger: "bg-red-500 text-white hover:bg-red-600",
      },
    },
  }
);
```

## 7. Build Configuration

### 7.1 Vite Configuration

**File:** `vite.config.ts`

```typescript
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [
    vue(),
    tailwindcss()
  ],
  resolve: {
    alias: {
      "@": "/src"
    }
  },
  server: {
    port: 1420
  }
});
```

### 7.2 Build Targets

- **Development:** `npm run dev` - Starts Vite dev server on port 1420
- **Production:** `npm run build` - TypeScript check + Vite build to `dist/`
- **Preview:** `npm run preview` - Preview built output

## 8. State Management Pattern

### 8.1 Composable-Based State

```
Component
    │
    └─> useGit() [Composable]
            │
            ├─ Reactive State (ref, computed)
            │  ├─ currentRepo
            │  ├─ commits
            │  ├─ branches
            │  └─ more...
            │
            └─ Methods
               ├─ getCommits()
               ├─ checkoutBranch()
               └─ more...
                    │
                    └─> invoke("tauri_command")
                            │
                            └─> Backend
```

### 8.2 Data Flow

1. User interacts with component
2. Component calls composable method
3. Composable invokes Tauri command
4. Backend processes and returns data
5. Composable updates reactive state
6. Vue automatically re-renders component

## 9. Lifecycle Patterns

### 9.1 Component Initialization

```typescript
onMounted(async () => {
  // Load data when component mounts
  await loadCommits();
  await loadBranches();

  // Set up watchers
  watchRepositoryChanges();
});
```

### 9.2 Cleanup

```typescript
onUnmounted(() => {
  // Clean up watchers
  stopWatchingFiles();

  // Cancel pending operations
  cancelPendingRequests();
});
```

## 10. Performance Optimization

### 10.1 Virtual Scrolling

Long lists use virtual scrolling via Tailwind's responsive utilities:
- Viewport-limited height
- Scrollable container
- Efficient rendering

### 10.2 Lazy Loading

```typescript
const AsyncComponent = defineAsyncComponent(() =>
  import("./HeavyComponent.vue")
);
```

### 10.3 Memoization

Computed properties automatically memoize results:
```typescript
const filteredCommits = computed(() => {
  return commits.value.filter(c => c.message.includes(searchTerm.value));
});
```

### 10.4 Debouncing

File watcher uses debounced updates:
```typescript
const debouncedUpdate = debounce(() => {
  updateFileStatus();
}, 500);
```

### 10.5 Progressive and Bounded Loading

Current heavy views follow a bounded loading contract:
- Keep the current useful UI visible while new repository data loads
- Use stale-run tokens to discard old async responses
- Use page/section-level loading for dashboards and analytics rather than blocking the whole app
- Reuse existing commit/status state before launching new Git scans
- Cache by stable repo/commit/file keys with small, explicit limits
- Skip optional file journey or assistant work when it cannot be computed quickly

## 11. Error Handling

### 11.1 Try-Catch Pattern

```typescript
async function operationName() {
  try {
    const result = await gitOperation();
    useToast().addToast("Success!", "success");
  } catch (error) {
    useToast().addToast(error.message, "error");
    console.error(error);
  }
}
```

### 11.2 Error Recovery

- User-friendly error messages
- Toast notifications
- Retry mechanisms
- Fallback options

## 12. Accessibility

### 12.1 Semantic HTML

Components use proper semantic HTML elements:
- `<button>` for buttons
- `<input>` for forms
- `<nav>` for navigation
- Proper heading hierarchy

### 12.2 ARIA Attributes

- `aria-label` for icon buttons
- `role` attributes where needed
- `aria-expanded` for collapsibles
- Keyboard navigation support

## 13. Commit Intelligence Module Placement (2026-04 Update)

This section defines where the updated analytics modules belong and what each module owns.

### 13.1 Frontend Module Map (Required Locations)

| Layer | Module File | Responsibility |
|------|-------------|----------------|
| History panel UI | `src/view/commit/CommitConflictHeatmapPanel.vue` | Conflict suspects UI, merge-window filtering, repository tree heatmap, conflict pair analytics, per-section loaders, caching and stream orchestration |
| History panel UI | `src/view/commit/CommitProductivityPanel.vue` | Productivity metrics, author filter, preview/full-history switching, diagnostics cards, per-section loader overlays, stream-level async loading |
| History panel UI | `src/view/commit/CommitTimeMachinePanel.vue` | Timeline scrubber, autoplay navigation, SHA search, snapshot explorer, rollback command copy, no-flicker frame transitions with cached snapshots |
| Graph layout helper | `src/view/commit/graph/useCommitGraphLayout.ts` | Commit graph lane/path layout including conflict node/edge alignment behavior |
| Domain action orchestration | `src/domain/git/composables/gitBranchActions.ts` | Merge workflow orchestration, merge preflight risk check, confirmation and toast messaging |
| Frontend data model | `src/types/models/conflictHotspot.ts` | Conflict hotspot contract (including collision index fields used by heatmap/tree) |
| Frontend data model | `src/types/models/conflictAnalytics.ts` | Risk pair model and merge preflight model contracts |
| Frontend model export | `src/types/models/index.ts` | Canonical exports so panels import from one stable entry point |

### 13.2 Placement Rules (Where New Code Should Go)

1. Panel-specific rendering logic must stay inside the matching panel under `src/view/commit/`.
2. Cross-panel Git actions (merge, checkout, branch workflows) must stay in `src/domain/git/composables/`.
3. Shared DTO/type contracts must stay in `src/types/models/` and be re-exported in `src/types/models/index.ts`.
4. Pure graph math and lane calculations must stay in `src/view/commit/graph/`.
5. UI-only helpers can live in panel-local functions; domain or reusable helpers must not be embedded in unrelated view files.

### 13.3 Loading and Performance Contract

For the commit intelligence views, use these frontend rules:

1. Each visual section owns its loader overlay (do not use one global blocking loader for the whole panel).
2. Keep previous successful data visible until replacement data is ready, then swap atomically.
3. Use stream tokens (run-token pattern) to discard stale async results.
4. Cache by key:
  - Conflict hotspots/pairs by `repo + commitWindow`
  - Conflict tree paths by `repo`
  - Time Machine snapshot files/tree by `repo + commitSha`
  - Time Machine file preview by `repo + commitSha + filePath`
5. Autoplay in Time Machine must schedule snapshot loads with small delay to avoid flicker during rapid frame changes.

---

**Related Documentation:**
- [05_COMPONENTS_GUIDE.md](./DOCUMENTATION_05_COMPONENTS_GUIDE.md) - Detailed component documentation
- [06_COMPOSABLES_GUIDE.md](./DOCUMENTATION_06_COMPOSABLES_GUIDE.md) - Composables reference
- [07_UI_SYSTEM.md](./DOCUMENTATION_07_UI_SYSTEM.md) - UI system and styling
