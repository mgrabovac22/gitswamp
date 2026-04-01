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

**Total Components:** 27 (across src/view, src/shared/ui, src/view/shell)

#### Layout & Shell Components (5) - src/view/shell/

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

4. **TerminalPanel.vue**
   - Integrated terminal for manual Git commands
   - Command input with history
   - Collapsible panel with toggle button
   - Output display with syntax highlighting
   - Error message display
   - Clear button and refresh functionality
   - Allow All Commands toggle for security control
   - Real-time output streaming

5. **BranchQuickActions.vue**
   - Dropdown menu for quick branch operations
   - Create new branch option
   - Switch to ghost branch (experimental feature)
   - Materialize ghost branch
   - Discard ghost branch
   - Loading state handling

#### Repository Components (8) - src/view/repository/

6. **LandingPage.vue**
   - Welcome screen displayed when no repository is open
   - Open existing repository button with file browser
   - Clone repository option with URL input
   - Initialize new repository option
   - Recent repositories list (if available)
   - Visual GitSwamp branding and feature overview
   - Quick access to online guide

7. **RepositoryTabs.vue** (Menu System)
   - Multi-tab interface for managing multiple repositories
   - Tab switching and close functionality
   - Active tab indication and labels
   - **Comprehensive Hamburger Menu with 4 sections:**
     - **File Menu:**
       - New Tab (Ctrl+T)
       - Open Repository (Ctrl+O)
       - Close Current Tab (Ctrl+W)
       - Create a Gist (Ctrl+Shift+G)
     - **Edit Menu:**
       - Copy Repository Path (Ctrl+Shift+C)
       - Refresh Repository (Ctrl+Shift+R)
       - Open in VS Code (Ctrl+Shift+O)
     - **View Menu:**
       - Toggle Terminal (Ctrl+`)
       - Visualize Commit History/Graph (Alt+1)
       - Productivity Arena (Alt+2)
       - Time Machine (Alt+3)
       - Usual Conflict Suspects/Heatmap (Alt+4)
       - Open in Folder Explorer (Alt+O)
       - Open Settings (Ctrl+,)
     - **Help Menu:**
       - Help and Shortcuts (F1)
       - About GitSwamp
       - Open Online Guide
       - Report Issue
   - Context-aware menu item disabling based on repo state
   - Keyboard shortcut support
   - Menu positioning relative to button
   - ESC key closes menu and help panels

8. **RepositoryWorkspace.vue** (Main Content Area)
   - Main workspace container for repository content
   - Integrates all visualization and editing panels
   - Manages layout: Sidebar, Graph/Panels, Details
   - Handles multiple history view modes (graph, productivity, time-machine, conflict-heatmap, remote-insights, conflict-resolve)
   - Terminal panel integration
   - Diff viewer modal
   - Conflict resolver modal
   - Coordinates state between child components

9. **RepositorySidebar.vue**
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

10. **RepositorySidebarSection.vue**
    - Reusable collapsible section component
    - Icon display with section label
    - Item count display
    - Expand/collapse chevron icon
    - Click handler for toggle events
    - Smooth animations

11. **Repository Action & Auth Dialogs**
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

12. **RemoteInsightsPanel.vue**
    - GitHub/GitLab integration panel
    - Pull request visualization and management
    - Issue tracking and display
    - Repository statistics
    - Contributor information
    - Activity timeline
    - Links to create new PRs/issues

#### Commit Visualization & Analytics (5) - src/view/commit/

13. **CommitGraph.vue** (Core Feature)
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

14. **CommitDetails.vue**
    - Collapsible details panel for selected commit
    - Commit metadata: Author, Date, Email, Message
    - Files changed in commit with statistics
    - File change icons (added, modified, deleted, renamed)
    - Quick file actions (view diff, stage, unstage)
    - Staged vs Unstaged file grouping
    - Commit message editing
    - Copy operations (hash, author, message)
    - Conflict status indicators

15. **FileItem.vue**
    - Individual file entry in commit or staging area
    - File type icon based on extension
    - Change statistics (insertions/deletions)
    - Status indicator (added/modified/deleted/renamed)
    - Hover state for interactivity
    - Click to view diff viewer
    - Context menu for file operations

16. **CommitProductivityPanel.vue** (Analytics)
    - Productivity metrics visualization
    - Developer activity statistics
    - Commit frequency analysis
    - File change patterns
    - Time-based productivity trends
    - Contributor comparison charts
    - Activity heatmap
    - Export statistics functionality

17. **CommitTimeMachinePanel.vue** (Time Navigation)
    - Navigate through repository history frames
    - File snapshots at different commits
    - Timeline scrubber for quick navigation
    - Show file content at specific commits
    - Diff between historical versions
    - Time-based filtering
    - Commit selection from timeline
    - Side-by-side version comparison

18. **CommitConflictHeatmapPanel.vue** (Conflict Analytics)
    - Merge conflict hotspot visualization
    - Files with highest conflict frequency
    - Conflict risk assessment
    - Team collaboration conflict patterns
    - Time-based conflict trends
    - Risk level indicators
    - Suggested conflict resolution strategies
    - Historical conflict analysis

#### UI Components (5) - src/shared/ui/

19. **AppButton.vue**
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

20. **AppInput.vue**
    - Styled text input component
    - Supports multiple input types (text, password, email, etc.)
    - Placeholder support
    - Disabled state
    - Error state with visual indication
    - Two-way binding with v-model
    - Custom class support
    - Validation-ready structure
    - Accessible with proper labels

21. **FileDiffViewer.vue** (Advanced)
    - Complex diff viewing component
    - Multiple view modes:
      - Unified diff (traditional format)
      - Side-by-side diff (left/right comparison)
    - Syntax highlighting for 50+ languages
    - Hunk-level operations (view/apply/revert)
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

22. **ConflictResolver.vue**
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

23. **ToastContainer.vue**
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

### 3.2 History View Modes

GitSwamp provides 6 different history visualization modes, accessible from the **View** menu:

#### 1. **Graph Mode** (Default) - Alt+1
- Interactive commit history visualization
- Lane-based graph rendering showing branch structure
- Displays branches, merges, and commits chronologically
- Primary view for most Git workflows
- Supports zooming, panning, and navigation
- Virtual scrolling for large repositories

#### 2. **Productivity Arena** - Alt+2
- Developer activity and metrics visualization
- Displays productivity patterns across time
- Shows commit frequency by contributor
- Visualizes file change patterns
- Time-based activity heatmaps
- Team productivity comparison
- Useful for performance analysis and sprint planning

#### 3. **Time Machine** - Alt+3
- Navigate through repository history frame-by-frame
- View repository state at specific commits
- File snapshots at different points in history
- Timeline-based scrubber navigation
- Diff between historical versions
- Inspect legacy code without checking out
- Useful for forensic analysis and understanding evolution

#### 4. **Usual Conflict Suspects (Conflict Heatmap)** - Alt+4
- Merge conflict hotspot visualization
- Identifies files with highest conflict frequency
- Conflict risk assessment and patterns
- Team collaboration conflict analytics
- Time-based conflict trend analysis
- Suggests risky merge areas
- Helps prevent problematic merges

#### 5. **Remote Insights** - Remote sync
- GitHub/GitLab integration panel
- Pull request management and overview
- Issue tracking and display
- Repository statistics and metrics
- Contributor information
- Activity timelines
- Direct links to repository web interface

#### 6. **Conflict Resolve** - Automatic on conflict
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
    │       ├─ View Actions (Terminal, History Modes, Explorer)
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
    │   ├─ CommitProductivityPanel (mode: productivity)
    │   ├─ CommitTimeMachinePanel (mode: time-machine)
    │   ├─ CommitConflictHeatmapPanel (mode: conflict-heatmap)
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
    │   └─ Initialize Repository
    │
    ├─> CloneDialog (modal)
    ├─> InitDialog (modal)
    ├─> GhostBranchDialog (modal)
    ├─> MultiPlatformPushDialog (modal)
    ├─> RepositoryActionDialogs (modal - merge/rebase/cherry-pick)
    ├─> RepositoryAuthDialogs (modal - credentials/tokens)
    ├─> SettingsDialog (modal)
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

---

**Related Documentation:**
- [05_COMPONENTS_GUIDE.md](./DOCUMENTATION_05_COMPONENTS_GUIDE.md) - Detailed component documentation
- [06_COMPOSABLES_GUIDE.md](./DOCUMENTATION_06_COMPOSABLES_GUIDE.md) - Composables reference
- [07_UI_SYSTEM.md](./DOCUMENTATION_07_UI_SYSTEM.md) - UI system and styling
