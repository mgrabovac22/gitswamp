# Core Features

## 1. Feature Overview

GitSwamp provides comprehensive Git repository management through 102 integrated Tauri commands and a user-friendly interface designed for both simple and advanced workflows.

The current frontend combines classic Git GUI workflows with lightweight safety and intelligence layers:
- Start dashboard for local repository context plus GitHub pull requests authored by the user and issues assigned to them
- Command Palette for quickly opening views and running common repository actions
- Graph, Galaxy, Productivity Arena, Time Machine, Conflict Suspects, and Burnout Analytics history modes
- Smart .gitignore Wizard for generated/private untracked files
- Hunk-level micro-staging in the diff viewer
- Terminal safety previews for destructive manual Git commands
- Undo toast delay for destructive UI actions
- Release Notes generator after successful merge flows

## 2. Repository Management

### 2.1 Open Repository

**Purpose:** Load and display an existing Git repository

**User Flow:**
1. Click "Open Repository" on landing page
2. Select directory via file dialog
3. Application validates and loads repository
4. Displays repository information and commit history

**Implementation:**
```typescript
async function openRepository(path: string) {
  const repo = await useGit().openRepository(path);
  currentRepo.value = repo;
  await loadCommits();
  await loadBranches();
}
```

**Backend:** `invoke("open_repository", { path })`

**Success Indicators:**
- Repository path displayed in header
- Commit graph loads
- Branches appear in sidebar
- Working changes load progressively after the initial commit list

### 2.2 Clone Repository

**Purpose:** Clone a remote repository to local filesystem

**User Flow:**
1. Click "Clone Repository" on landing page
2. Enter repository URL (HTTPS or SSH)
3. Select destination directory
4. System clones repository
5. Opens new repository automatically

**Supported Sources:**
- GitHub
- GitLab
- Gitea
- Any HTTPS/SSH Git remote

**Features:**
- URL validation
- Directory selection dialog
- Progress indication
- Error handling

**Implementation:**
```typescript
async function cloneRepository(url: string, path: string) {
  await useGit().cloneRepo(url, path);
  await openRepository(path);
}
```

**Backend:** `invoke("clone_repo", { url, path })`

### 2.3 Initialize Repository

**Purpose:** Create a new Git repository

**User Flow:**
1. Click "Initialize Repository"
2. Select directory
3. System creates .git folder
4. Opens new repository

**Implementation:**
```typescript
async function initRepository(path: string) {
  await useGit().initRepo(path);
  await openRepository(path);
}
```

**Backend:** `invoke("init_repo", { path })`

### 2.4 Repository Information

**Displays:**
- Repository path
- Current HEAD
- Current branch
- Remote URLs
- Conflict status
- Working directory state

**Updated:** Real-time (via file watcher)

### 2.5 Start Dashboard

**Purpose:** Give users useful context before opening or while switching repositories.

**Displays:**
- Open repository count, recent repositories, and active local branch context
- GitHub pull requests authored by the signed-in user
- GitHub issues assigned to the signed-in user
- Search boxes for pull requests and issues
- A settings prompt when no GitHub token is configured

**Loading Strategy:**
- GitHub data is fetched in pages with cancellation support
- Pull requests and issues load independently from local recent repositories
- If no token exists, the dashboard stays useful with local repository summaries

## 3. Commit History Visualization

### 3.1 Commit Graph

**Purpose:** Visual representation of commit history with branches

**Features:**
- Lane-based graph layout
- Branch lines and merges
- Interactive commit selection
- Tag indicators
- Author avatars
- Zoom and pan support

**Performance:**
- Handles 1000+ commits efficiently
- Pagination for large repositories
- Lazy loading of older commits

**User Interactions:**
- Click commit for details
- Double-click to checkout
- Right-click context menu
- Search commits

### 3.2 Commit Information

**Displays:**
- Commit hash (clickable, copyable)
- Author name and email
- Commit timestamp
- Commit message (full)
- Parent commits
- Associated branch/tag
- File statistics

**Related Files:**
- List all files changed
- Show additions/deletions per file
- Click to view diff

### 3.3 Search Commits

**Features:**
- Search by message
- Search by author
- Search by email
- Case-sensitive/insensitive toggle
- Regex support (optional)

**Implementation:**
```typescript
async function searchCommits(query: string) {
  const results = await useGit().searchCommits(query);
  return results;
}
```

**Backend:** `invoke("search_commits", { repo_path, query })`

### 3.4 Galaxy View

**Purpose:** Visualize branches and commits as an interactive canvas map for exploratory history understanding.

**Features:**
- Spiral, layered, and constellation layouts
- 3D, balanced, and circling motion modes
- Tree mode for branch/commit hierarchy visualization
- Zoom, pan, click-to-focus, hover details, and branch lane context
- Load-all support for larger histories with memory-aware rendering

**Performance Rules:**
- Canvas drawing stays local to the panel
- Motion is lightweight and can be reduced, while head-branch circling remains available as a visual anchor
- Data is derived from already loaded commit/branch state where possible

### 3.5 Burnout Analytics

**Purpose:** Show team focus, after-hours work, repository pulse, and contributor workload signals from Git history.

**Displays:**
- Repository burnout pulse over recent weeks
- After-hours and weekend work indicators
- Contributor-level activity windows
- Hot-file and bottleneck signals not already covered by Productivity Arena

**Loading Strategy:**
- Uses fast commit metadata first
- Avoids blocking scroll while deeper analytics are computed
- Keeps labels human-readable instead of shorthand week codes

## 4. Branch Management

### 4.1 List Branches

**Shows:**
- Local branches
  - Current branch (highlighted)
  - Upstream tracking
  - Status relative to upstream
- Remote branches
  - Grouped by remote
  - Status indication

**Sidebar Display:**
- Expandable "Branches" section
- Current branch marked
- Right-click context menu
  - Checkout
  - Delete
  - Rename
  - Create new from

### 4.2 Create Branch

**Dialog Input:**
- Branch name (validated)
- Source (current HEAD or select commit)
- Track remote (optional)

**Validation:**
- Valid branch name
- No existing branch with name
- Valid source reference

**Implementation:**
```typescript
async function createBranch(name: string, from?: string) {
  await useGit().createBranch(name, from);
  await useGit().getBranches(); // Refresh
}
```

**Backend:** `invoke("create_branch", { repo_path, branch_name, from })`

### 4.3 Checkout Branch

**Purpose:** Switch to different branch

**Validation:**
- Uncommitted changes detection
- Merge conflict detection
- Warning if requires cleanup

**Scenarios:**
- Clean checkout - immediate switch
- With changes - prompt to commit or stash
- Merge conflicts - show resolver

**Implementation:**
```typescript
async function checkoutBranch(name: string) {
  if (hasUncommittedChanges.value) {
    // Prompt user
  }
  await useGit().checkoutBranch(name);
  await loadCommits(); // Refresh
}
```

**Backend:** `invoke("checkout_branch", { repo_path, branch_name })`

### 4.4 Delete Branch

**Protections:**
- Cannot delete current branch
- Confirmation prompt
- Warning if not merged to main/master
- Option to force delete

**Implementation:**
```typescript
async function deleteBranch(name: string, force = false) {
  await useGit().deleteBranch(name, force);
  await useGit().getBranches(); // Refresh
}
```

**Backend:** `invoke("delete_branch", { repo_path, branch_name, force })`

### 4.5 Rename Branch

**Features:**
- Rename local branch
- Update upstream tracking (optional)
- Handle conflicts

**Implementation:**
```typescript
async function renameBranch(oldName: string, newName: string) {
  await useGit().renameBranch(oldName, newName);
  await useGit().getBranches(); // Refresh
}
```

**Backend:** `invoke("rename_branch", { repo_path, old_name, new_name })`

### 4.6 Track Remote

**Features:**
- Set upstream branch
- View tracking status
- Unset upstream

**Display:**
- Branch showing "tracking origin/branch"
- Ahead/behind counts

**Implementation:**
```typescript
async function setUpstream(branch: string, upstream: string) {
  await useGit().setUpstream(branch, upstream);
  await useGit().getBranches(); // Refresh
}
```

**Backend:** `invoke("set_upstream", { repo_path, branch, upstream })`

## 5. File Operations

### 5.1 View File Status

**Shows:**
- Working directory changes
  - Modified
  - Added
  - Deleted
  - Renamed
  - Untracked
- Staging area changes
- Merge conflicts

**Real-time Updates:**
- File watcher monitors changes
- Updates on file save
- Debounced to avoid excessive updates

### 5.2 Stage Files

**Features:**
- Stage individual files
- Stage all changes
- Stage by hunk (in diff viewer)
- Stage new files and modified files from the changes panel

**UI:**
- Click file + "Stage" button
- Drag and drop
- Context menu

**Implementation:**
```typescript
async function stageFile(path: string) {
  await useGit().stageFile(path);
  await updateStatus();
}
```

**Backend:** `invoke("stage_file", { repo_path, file_path })`

### 5.3 Unstage Files

**Features:**
- Unstage individual files
- Unstage all
- Unstage by hunk

**UI:**
- Click staged file + "Unstage" button
- Context menu
- Diff viewer controls

**Implementation:**
```typescript
async function unstageFile(path: string) {
  await useGit().unstageFile(path);
  await updateStatus();
}
```

**Backend:** `invoke("unstage_file", { repo_path, file_path })`

### 5.4 Discard Changes

**Purpose:** Discard working directory changes

**Features:**
- Discard single file
- Discard all unstaged working-tree changes
- Does not discard staged changes when the user requests unstaged discard
- Undo toast delay before destructive UI discard runs
- Confirmation or warning for irreversible paths

**Implementation:**
```typescript
async function discardFile(path: string) {
  scheduleDestructiveAction({
    message: "Discard changes?",
    run: async () => {
      await useGit().discardFile(path);
      await updateStatus();
    },
  });
}
```

**Backend:** `invoke("discard_file", { repo_path, file_path })`

### 5.5 Smart .gitignore Wizard

**Purpose:** Prevent accidental commits of generated files, local secrets, caches, logs, and dependency folders.

**Behavior:**
- Enabled by default and configurable in Settings/Preferences
- Scans untracked files already present in the working changes list
- Groups matches such as `node_modules/`, `.env`, `*.log`, `dist/`, `target/`, framework caches, Python caches, local databases, and private key formats
- Offers **Ignore selected**, **Open .gitignore**, and **Keep tracking**
- Adds only missing ignore patterns and preserves existing `.gitignore` content

**Performance:**
- Uses existing status data, not a full filesystem crawl
- Samples only a few paths per suggestion group
- Avoids suggestions already covered by existing patterns

## 6. Diff Viewing

### 6.1 Working Directory Diff

**Features:**
- View changes in working directory
- Side-by-side view
- Unified view
- Syntax highlighting
- Line numbers
- Hunk operations

**Display:**
- File list with change indicators
- Click file to view diff
- Toggle between view modes

**Implementation:**
```typescript
async function getWorkingDiff(filePath?: string) {
  return await useGit().getWorkingDiff(filePath);
}
```

**Backend:** `invoke("get_working_diff", { repo_path, file_path })`

### 6.2 Commit Diff

**Features:**
- View changes in specific commit
- View diff between commits
- Side-by-side comparison
- File-specific diffs

**User Flow:**
1. Select commit
2. View associated files
3. Click file to see diff
4. Can compare with parent or specific commit

**Implementation:**
```typescript
async function getCommitDiff(commitId: string, filePath?: string) {
  return await useGit().getCommitDiff(commitId, filePath);
}
```

**Backend:** `invoke("get_commit_diff", { repo_path, commit_id, file_path })`

### 6.3 Hunk Operations

**Features:**
- View individual hunks
- Stage hunks
- Unstage hunks
- Discard hunks
- Edit hunk content
- Preview changes

**Use Cases:**
- Stage partial changes
- Review changes before commit
- Fix conflicts selectively
- Separate unrelated edits in the same file into different commits

### 6.4 File Editing in Diff Viewer

**Features:**
- Edit mode toggle
- Syntax highlighting
- Line numbering
- Change indicators
- Save changes back to file

**Implementation:**
```typescript
async function saveFileContent(filePath: string, content: string) {
  await useGit().saveFileContent(filePath, content);
  await updateStatus();
}
```

**Backend:** `invoke("save_file_content", { repo_path, file_path, content })`

### 6.5 File Journey

**Purpose:** Provide a small, optional summary for the currently inspected file.

**Displays:**
- Created by
- Last changed by
- Recent commit context when available quickly

**Rules:**
- Hidden by default behind a small diff-toolbar icon
- Can be closed from the panel
- Skips expensive work rather than blocking the diff viewer

## 7. Commit Creation

### 7.1 Create Commit

**Process:**
1. Stage desired files
2. Enter commit message
3. Optionally add extended description
4. Create commit

**Features:**
- Co-author support
- Commit template
- Message validation
- Sign commits (if configured)

**Implementation:**
```typescript
async function createCommit(message: string, coAuthors?: string[]) {
  await useGit().createCommit(message);
  await loadCommits(); // Refresh
  await updateStatus(); // Clear staging
}
```

**Backend:** `invoke("create_commit", { repo_path, message, co_authors })`

### 7.2 Commit Message Editor

**Features:**
- Large text area
- Line wrap
- Undo/Redo
- Message templates
- Character counter
- Visual Commit Builder with conventional type, scope, issue tag, and summary fields
- Commit Analyzer indicator and message quality feedback

**Best Practices:**
- First line < 50 characters
- Blank line after first line
- Detailed description (72 characters per line)

### 7.3 Visual Commit Builder

**Purpose:** Help users compose consistent commit subjects without memorizing commit conventions.

**Behavior:**
- Opens from the builder icon beside the commit analyzer/message area
- Uses capitalized commit types to match analyzer rules
- Supports a wide scope list plus custom scope input
- Supports issue tags such as `#123`
- Saves the generated subject back into the commit input

**Subject Shape:**
```text
Type(scope): #123 concise summary
```

## 8. Remote Operations

### 8.1 Push

**Purpose:** Push commits to remote repository

**Features:**
- Push current branch
- Push all branches
- Force push (with warning)
- Set upstream branch

**Validation:**
- Verify authentication
- Check remote exists
- Detect rejected pushes

**Implementation:**
```typescript
async function push(force = false) {
  await useGit().push(force);
  toast.addToast("Pushed successfully", "success");
}
```

**Backend:** `invoke("push", { repo_path, force })`

### 8.2 Pull

**Purpose:** Fetch and merge from remote

**Process:**
1. Fetch latest from remote
2. Merge into current branch
3. Handle conflicts if needed

**Features:**
- Auto-merge when possible
- Conflict detection
- Rebase option

**Implementation:**
```typescript
async function pull() {
  try {
    await useGit().pull();
    toast.addToast("Pulled successfully", "success");
  } catch (error) {
    if (error.includes("conflict")) {
      showConflictResolver();
    }
  }
}
```

**Backend:** `invoke("pull", { repo_path })`

### 8.3 Fetch

**Purpose:** Download latest changes from remote

**Features:**
- Fetch all remotes
- Fetch specific remote
- Fetch specific branch
- Prune deleted remote branches

**Implementation:**
```typescript
async function fetchAll() {
  await useGit().fetchAll();
  toast.addToast("Fetched latest changes", "success");
}
```

**Backend:** `invoke("fetch_all", { repo_path })`

### 8.4 Release Notes After Merge

**Purpose:** Generate a structured Markdown summary after a successful merge scenario.

**Behavior:**
- Triggered only after a positive merge completion path
- Asks whether the user wants to download release notes
- Uses commits between source and target refs
- Groups changes into Breaking Changes, Features, Bug Fixes, Performance, Documentation, and Other Changes
- Adds executive summary, readiness notes, suggested verification, contributors, highlights, and commit appendix
- Saves the Markdown file to the user-selected path

## 9. Advanced Operations

### 9.1 Cherry Pick

**Purpose:** Apply specific commits to current branch

**Process:**
1. Select commit from history
2. Choose "Cherry Pick"
3. Resolve conflicts if needed
4. Commit applied

**Implementation:**
```typescript
async function cherryPick(commitId: string) {
  await useGit().cherryPick(commitId);
}
```

**Backend:** `invoke("cherry_pick", { repo_path, commit_id })`

### 9.2 Revert Commit

**Purpose:** Create new commit that undoes changes

**Features:**
- Keep history intact
- Option to automatically commit or leave staged
- Conflict detection

**Implementation:**
```typescript
async function revertCommit(commitId: string) {
  await useGit().revertCommit(commitId);
}
```

**Backend:** `invoke("revert_commit", { repo_path, commit_id })`

### 9.3 Reset

**Purpose:** Move HEAD to specific commit

**Modes:**
- Soft: Keep changes staged
- Mixed: Keep changes unstaged
- Hard: Discard all changes (warning required)

**Implementation:**
```typescript
async function resetTo(commitId: string, mode: "soft" | "mixed" | "hard") {
  if (mode === "hard") {
    if (!confirm("This will discard all changes. Continue?")) return;
  }
  await useGit().resetToCommit(commitId, mode);
}
```

**Backend:** `invoke("reset_to_commit", { repo_path, commit_id, mode })`

### 9.4 Checkout Commit (Detached HEAD)

**Purpose:** Check out specific commit for inspection

**Features:**
- Detached HEAD mode
- Create branch from commit
- Easy return to branch

**Implementation:**
```typescript
async function checkoutCommit(commitId: string) {
  await useGit().checkoutCommit(commitId);
  toast.addToast("In detached HEAD state", "warning");
}
```

**Backend:** `invoke("checkout_commit", { repo_path, commit_id })`

## 10. Productivity UI and Help

### 10.1 Quick Menu (File/Edit/View/Help)

**Purpose:** Provide fast access to high-frequency actions from a single menu.

**Features:**
- File section: tab management and repository open actions
- Edit section: copy path, refresh repository, open in VS Code
- View section: toggle terminal, open folder explorer, open settings, and switch between Graph, Galaxy, Productivity, Time Machine, Conflict Suspects, and Burnout Analytics
- Help section: in-app help panel, online guide, issue reporting
- Middle-click closes a repository tab
- Ctrl+Tab switches to the next repository tab

### 10.2 In-App Help and Shortcuts

**Purpose:** Keep keyboard shortcuts and usage instructions available inside the app.

**Access:**
- F1 shortcut
- Hamburger menu -> Help -> Help and Shortcuts

**Contents:**
- Core feature overview
- Keyboard shortcuts table
- Quick navigation tips for repository workflows
- Command Palette shortcut and view-mode shortcuts

### 10.3 Open Repository with External Tools

**Purpose:** Open the active repository in external tools directly from UI and terminal.

**Supported Tools:**
- VS Code
- Visual Studio
- Android Studio
- IntelliJ
- System folder explorer

**Backend Command:** `invoke("open_path_with_tool", { path, tool })`

### 10.4 Command Palette

**Purpose:** Search common commands and open views with minimal clicks.

**Access:**
- Ctrl+K

**Common Actions:**
- Refresh repository
- Toggle terminal
- Open repository
- Open folder explorer
- Open settings
- Open logs
- Switch to Graph, Galaxy, or Burnout Analytics

### 10.5 Terminal Command Safety Layer

**Purpose:** Protect manual terminal usage from accidental destructive Git commands.

**Guarded Commands:**
- `git reset --hard`
- forced `git clean`
- destructive `git restore`
- checkout overwrite patterns
- `git rm` without `--cached`
- `git stash pop`, `drop`, or `clear`
- local branch delete
- force push

**Safety UX:**
- Shows a preview of likely staged, unstaged, untracked, or branch/remote impact
- Offers Continue, Cancel, and Create safety stash first when applicable
- Disables safety stash if conflicts are present or if a stash is not useful for that command type

### 10.6 Background Maintenance Preferences

**Purpose:** Optional background mechanisms for users who want proactive repository reminders without changing the normal Git workflow.

**Defaults:**
- Off by default

**Available Mechanisms:**
- Repository health refresh
- Remote hygiene
- Focus sync
- Idle-only mode
- Stale work reminder
- Behind-branch reminder
- Large change reminder with configurable threshold
- Conflict reminder
- Commit details preload

**UX Rule:** Enabled reminders surface through toasts and should stay non-invasive.

## 11. Commit Intelligence Modules (Conflict + Productivity + Time Machine)

This section documents updated module ownership and where each module must live.

For detailed scoring formulas, code ownership interpretation, conflict hotspot algorithms, merge preflight risk, Productivity Arena metrics, Burnout Analytics, and Time Machine snapshot behavior, use [36_COMMIT_INTELLIGENCE_PANELS.md](./DOCUMENTATION_36_COMMIT_INTELLIGENCE_PANELS.md) as the canonical deep dive.

### 11.1 Frontend Ownership Map

| Feature Area | Module File | Must Own |
|-------------|-------------|----------|
| Galaxy view | `src/view/commit/CommitGalaxyPanel.vue` | Canvas galaxy, tree mode, zoom/focus, layout controls, branch/commit hover details |
| Conflict suspects panel | `src/view/commit/CommitConflictHeatmapPanel.vue` | Hotspots stream, pairs stream, repository tree stream, merge-window filtering, tree roll-up, per-element loaders, conflict diagnostics cards |
| Productivity arena | `src/view/commit/CommitProductivityPanel.vue` | Preview/full-history loading, author filter, streak and rhythm metrics, stability scoring, section-level performance caching |
| Time machine | `src/view/commit/CommitTimeMachinePanel.vue` | Full-history timeline loading, autoplay, SHA search, snapshot explorer, rollback command copy, cached commit snapshots |
| Burnout analytics | `src/view/commit/CommitBurnoutAnalyticsPanel.vue` | Repository pulse, after-hours windows, contributor workload, focus and risk charts |
| Smart .gitignore assistant | `src/view/commit/SmartGitignoreWizard.vue` | Generated/private file grouping, pattern coverage checks, `.gitignore` updates |
| Release notes | `src/features/release-notes/releaseNotes.ts` | Markdown grouping, summaries, contributors, readiness and verification notes |
| Merge preflight bridge | `src/domain/git/composables/gitBranchActions.ts` | Merge-risk pre-check orchestration and user confirmation prompt flow |
| Shared analytics models | `src/types/models/conflictHotspot.ts`, `src/types/models/conflictAnalytics.ts` | Typed contracts shared between panels and Tauri payloads |

### 11.2 Backend Ownership Map

| Feature Area | Module File | Must Own |
|-------------|-------------|----------|
| Conflict command surface | `src-tauri/src/commands/conflicts.rs` | Tauri commands for hotspots, pairs, repo tree paths, merge preflight risk |
| Productivity support commands | `src-tauri/src/commands/commits.rs` | Commits, author deletion stats, commit tree paths, commit file snapshots |
| Heavy analytics logic | `src-tauri/src/services/git_service.rs` | Scoring, filtering windows, conflict pairing, preflight computation, deletion-stat optimization |
| Analytics payload contracts | `src-tauri/src/models/conflict_hotspot.rs` | `ConflictHotspot`, `ConflictPair`, `MergeRiskPreflight` |
| Command registration | `src-tauri/src/lib.rs` | `invoke_handler` registration for all above commands |

### 11.3 Placement and Change Rules

1. Keep UI rendering and section loader behavior in panel files under `src/view/commit/`.
2. Keep cross-feature branch/merge orchestration in `src/domain/git/composables/`.
3. Keep backend command handlers thin; move heavy compute to `src-tauri/src/services/git_service.rs`.
4. Add or change payload shape only in model files under `src-tauri/src/models/` and `src/types/models/`.
5. Wire every new command in `src-tauri/src/lib.rs` and update `DOCUMENTATION_09_COMMANDS_REFERENCE.md` in the same change.
6. Keep terminal safety prompt detection in `src/view/shell/TerminalPanel.vue` and command execution/safety stash orchestration in `src/domain/git/composables/gitTerminalActions.ts`.
7. Keep undoable destructive UI actions behind `src/shared/notifications/useUndoableDestructiveAction.ts`.

### 11.4 Performance Baseline for These Features

1. Use per-stream or per-section loading (avoid full-panel hard blocking unless absolutely required).
2. Cache by stable keys (`repo`, `repo + window`, `repo + sha`, `repo + sha + filePath`).
3. Use stale-run tokens for async stream safety.
4. Time Machine autoplay must use scheduled snapshot refresh to reduce frame flicker.
5. Keep conflict-tree roll-up logic aligned between the repository tree and hotspot scoring so folder nodes inherit child pressure consistently.
6. Prefer existing status/commit data for dashboard and assistant features before adding new filesystem or Git scans.
7. If a feature cannot load quickly, show the useful partial state and continue work in the background.

---

**Related Documentation:**
- [12_BRANCH_MANAGEMENT.md](./DOCUMENTATION_12_BRANCH_MANAGEMENT.md) - Detailed branch workflows
- [13_COMMIT_OPERATIONS.md](./DOCUMENTATION_13_COMMIT_OPERATIONS.md) - Commit management
- [14_FILE_OPERATIONS.md](./DOCUMENTATION_14_FILE_OPERATIONS.md) - File handling
- [15_ADVANCED_FEATURES.md](./DOCUMENTATION_15_ADVANCED_FEATURES.md) - Advanced operations
- [36_COMMIT_INTELLIGENCE_PANELS.md](./DOCUMENTATION_36_COMMIT_INTELLIGENCE_PANELS.md) - Detailed history intelligence panels
