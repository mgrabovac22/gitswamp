# Core Features

## 1. Feature Overview

GitSwamp provides comprehensive Git repository management through integrated features designed for both simple and advanced workflows.

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
- Discard all changes
- Confirmation required

**Implementation:**
```typescript
async function discardFile(path: string) {
  if (confirm("Discard changes to " + path + "?")) {
    await useGit().discardFile(path);
    await updateStatus();
  }
}
```

**Backend:** `invoke("discard_file", { repo_path, file_path })`

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
- Apply/discard hunks
- Edit hunk content
- Preview changes

**Use Cases:**
- Stage partial changes
- Review changes before commit
- Fix conflicts selectively

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

**Best Practices:**
- First line < 50 characters
- Blank line after first line
- Detailed description (72 characters per line)

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

---

**Related Documentation:**
- [12_BRANCH_MANAGEMENT.md](./DOCUMENTATION_12_BRANCH_MANAGEMENT.md) - Detailed branch workflows
- [13_COMMIT_OPERATIONS.md](./DOCUMENTATION_13_COMMIT_OPERATIONS.md) - Commit management
- [14_FILE_OPERATIONS.md](./DOCUMENTATION_14_FILE_OPERATIONS.md) - File handling
- [15_ADVANCED_FEATURES.md](./DOCUMENTATION_15_ADVANCED_FEATURES.md) - Advanced operations
