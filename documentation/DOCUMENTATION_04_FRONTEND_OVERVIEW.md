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
├── components/                    # Vue components (19 total)
│   ├── layout/                   # Layout components
│   │   ├── TitleBar.vue
│   │   ├── AppHeader.vue
│   │   ├── SettingsDialog.vue
│   │   └── TerminalPanel.vue
│   ├── repository/               # Repository management
│   │   ├── LandingPage.vue
│   │   ├── RepositoryTabs.vue
│   │   ├── Sidebar.vue
│   │   ├── SidebarSection.vue
│   │   ├── CloneDialog.vue
│   │   └── InitDialog.vue
│   ├── commits/                  # Commit visualization
│   │   ├── CommitGraph.vue
│   │   ├── CommitDetails.vue
│   │   └── FileItem.vue
│   └── ui/                       # Reusable UI components
│       ├── AppButton.vue
│       ├── AppInput.vue
│       ├── FileDiffViewer.vue
│       ├── ConflictResolver.vue
│       ├── GitCommitIcon.vue
│       ├── ToastContainer.vue
│       └── utils.ts
├── composables/                   # Vue composables
│   ├── useGit.ts                 # Core Git operations
│   └── useToast.ts               # Toast notifications
├── types/                         # TypeScript type definitions
│   └── index.ts
├── styles/                        # CSS stylesheets
│   ├── index.css
│   ├── tailwind.css
│   └── theme.css
├── assets/                        # Static assets
│   └── vue.svg
├── App.vue                        # Root component
├── main.ts                        # Entry point
└── vite-env.d.ts                 # Vite type definitions
```

## 3. Vue Components

### 3.1 Component Overview

**Total Components:** 19

#### Layout Components (4)

1. **TitleBar.vue**
   - Custom window title bar (Tauri)
   - Drag support for window movement
   - Close/minimize/maximize buttons
   - Custom styling

2. **AppHeader.vue**
   - Top navigation bar
   - Repository selector
   - Navigation buttons
   - Settings access

3. **SettingsDialog.vue**
   - User preferences modal
   - Theme selection (light/dark)
   - Font size adjustment
   - Compact mode toggle
   - Avatar display toggle

4. **TerminalPanel.vue**
   - Terminal output display
   - Command execution results
   - Error messages
   - Collapsible panel

#### Repository Components (6)

5. **LandingPage.vue**
   - Welcome screen
   - Open repository button
   - Clone repository option
   - Initialize repository option
   - Recent repositories (if available)

6. **RepositoryTabs.vue**
   - Multi-tab interface
   - Tab switching
   - Tab close functionality
   - Active tab indication

7. **Sidebar.vue**
   - Main navigation sidebar
   - Branch list
   - Stash list
   - Tag list
   - Scrollable sections
   - Right-click context menu

8. **SidebarSection.vue**
   - Collapsible section component
   - Expandable/collapsible header
   - Dynamic content rendering
   - Icon support

9. **CloneDialog.vue**
   - Repository URL input
   - Directory selection
   - Clone progress indication
   - GitHub/GitLab repository search
   - Provider authentication

10. **InitDialog.vue**
    - Directory selection
    - Repository initialization
    - Initial configuration
    - Success confirmation

#### Commit Visualization (3)

11. **CommitGraph.vue** (Core Feature)
    - Visual commit history rendering
    - Lane-based graph layout
    - Interactive commit selection
    - Zooming and panning
    - Branch visualization
    - Responsive to size changes
    - Pagination for large histories

12. **CommitDetails.vue**
    - Commit information panel
    - Author and timestamp
    - Commit message display
    - Files changed list
    - Staging/unstaging interface
    - Commit creation form
    - Diff preview

13. **FileItem.vue**
    - Individual file entry
    - File icon based on type
    - Change statistics (insertions/deletions)
    - Status indicator
    - Clickable for diff viewing
    - Staged/unstaged indication

#### UI Components (6)

14. **AppButton.vue**
    - Styled button component
    - Multiple variants (primary, secondary, danger)
    - Size options (sm, md, lg)
    - Disabled state
    - Loading state with spinner
    - Icon support

15. **AppInput.vue**
    - Styled text input component
    - Placeholder support
    - Disabled state
    - Error state with message
    - Password field support
    - Validation feedback

16. **FileDiffViewer.vue** (Advanced)
    - Side-by-side diff viewing
    - Unified diff viewing
    - Syntax highlighting
    - Hunk-level operations
    - Edit mode for file modification
    - Scroll synchronization
    - Line numbers
    - Context line expansion

17. **ConflictResolver.vue**
    - Merge conflict display
    - Three-way merge view (original, current, incoming)
    - Conflict marker highlighting
    - Resolution buttons (use theirs/ours/both)
    - Hunk-by-hunk resolution
    - Preview of resolved content

18. **GitCommitIcon.vue**
    - Commit avatar/icon rendering
    - Author gravatar integration
    - Fallback initial display
    - Tooltip with author info

19. **ToastContainer.vue**
    - Toast notification rendering
    - Multiple toast support
    - Auto-dismiss functionality
    - Position management
    - Type-based styling (success/error/warning/info)

### 3.2 Component Communication

```
App.vue (Root)
    │
    ├─> emits: repository-changed
    ├─> emits: file-selected
    └─> emits: commit-selected
        │
        ├─> TitleBar
        ├─> AppHeader
        │   └─ emits: open-settings
        ├─> LandingPage or Main Content
        │   ├─> Sidebar
        │   │   └─ emits: branch-selected
        │   └─> CommitGraph
        │       ├─ emits: commit-clicked
        │       └─> CommitDetails
        │           ├─ emits: stage-file
        │           ├─ emits: unstage-file
        │           └─> FileItem
        │               └─ emits: view-diff
        │                   └─> FileDiffViewer
        ├─> SettingsDialog (modal)
        ├─> CloneDialog (modal)
        ├─> InitDialog (modal)
        ├─> ConflictResolver (when needed)
        ├─> TerminalPanel
        └─> ToastContainer
```

## 4. Composables

### 4.1 useGit Composable

**File:** `src/composables/useGit.ts`

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

**File:** `src/composables/useToast.ts`

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
