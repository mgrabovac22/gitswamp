# Commands Reference

## 1. Command Overview

GitSwamp exposes Tauri commands organized across multiple modules. All commands are asynchronous and use type-safe serialization. Modules include: repository, commits, commit_files, branches, status, diff, stash, tags, clone_init, operations, credentials, conflicts, and ghost.

## 2. Repository Commands

### repository.rs

#### get_repo_info

```typescript
// Function signature
invoke("get_repo_info", { repo_path: string }): Promise<RepoInfo>

// Parameters
{
  repo_path: string;  // Absolute path to repository
}

// Response
{
  path: string;              // Repository .git path
  is_bare: boolean;          // Is bare repository
  head: string;              // Current HEAD reference
  current_branch: string;    // Current branch name
  remotes: RemoteInfo[];     // Remote configurations
  has_conflicts: boolean;    // Has merge conflicts
  workdir: string | null;    // Working directory path
}

// Example
const repo = await invoke("get_repo_info", {
  repo_path: "/home/user/my-project"
});
console.log(repo.current_branch); // "main"
```

## 3. Commit Commands

### commits.rs

#### get_commits

```typescript
invoke("get_commits", {
  repo_path: string;
  offset: number;
  limit: number;
}): Promise<CommitInfo[]>

// Parameters
{
  repo_path: string;    // Repository path
  offset: number;       // Skip first N commits
  limit: number;        // Maximum commits to return
}

// Response - Array of CommitInfo
[
  {
    id: string;              // SHA-1 hash
    author: string;          // Author name
    email: string;           // Author email
    timestamp: number;       // Unix timestamp
    message: string;         // Commit message
    parent_ids: string[];    // Parent commit SHAs
    branch: string | null;   // Associated branch
    tags: string[];          // Associated tags
  },
  // ... more commits
]

// Example
const commits = await invoke("get_commits", {
  repo_path: "/home/user/my-project",
  offset: 0,
  limit: 50
});
console.log(commits[0].message); // First commit message
```

#### search_commits

```typescript
invoke("search_commits", {
  repo_path: string;
  query: string;
}): Promise<CommitInfo[]>

// Parameters
{
  repo_path: string;    // Repository path
  query: string;        // Search query (message or author)
}

// Response
CommitInfo[]

// Example
const results = await invoke("search_commits", {
  repo_path: "/home/user/my-project",
  query: "fix: memory leak"
});
```

### commit_files.rs

#### get_commit_files

```typescript
invoke("get_commit_files", {
  repo_path: string;
  commit_id: string;
}): Promise<CommitFileInfo[]>

// Parameters
{
  repo_path: string;     // Repository path
  commit_id: string;     // Commit SHA
}

// Response - Array of CommitFileInfo
[
  {
    path: string;           // File path
    status: string;         // "added", "deleted", "modified"
    old_path: string | null; // Original path if renamed
    insertions: number;     // Lines added
    deletions: number;      // Lines deleted
  },
  // ... more files
]

// Example
const files = await invoke("get_commit_files", {
  repo_path: "/home/user/my-project",
  commit_id: "abc123def456..."
});
console.log(files.length); // Number of changed files
```

## 4. Branch Commands

### branches.rs

#### get_branches

```typescript
invoke("get_branches", {
  repo_path: string;
}): Promise<BranchInfo[]>

// Response - Array of BranchInfo
[
  {
    name: string;            // Full branch name (refs/heads/main)
    is_local: boolean;       // Is local branch
    is_current: boolean;     // Is currently checked out
    upstream: string | null; // Upstream branch
    shorthand: string;       // Short name (main)
  },
  // ... more branches
]

// Example
const branches = await invoke("get_branches", {
  repo_path: "/home/user/my-project"
});
```

#### checkout_branch

```typescript
invoke("checkout_branch", {
  repo_path: string;
  branch_name: string;
}): Promise<void>

// Parameters
{
  repo_path: string;     // Repository path
  branch_name: string;   // Branch name or reference
}

// Example
await invoke("checkout_branch", {
  repo_path: "/home/user/my-project",
  branch_name: "develop"
});
```

#### create_branch

```typescript
invoke("create_branch", {
  repo_path: string;
  branch_name: string;
  from?: string;
}): Promise<void>

// Parameters
{
  repo_path: string;     // Repository path
  branch_name: string;   // New branch name
  from?: string;         // Source commit/branch (default: HEAD)
}

// Example
await invoke("create_branch", {
  repo_path: "/home/user/my-project",
  branch_name: "feature/new-ui",
  from: "main"
});
```

#### delete_branch

```typescript
invoke("delete_branch", {
  repo_path: string;
  branch_name: string;
  force?: boolean;
}): Promise<void>

// Parameters
{
  repo_path: string;     // Repository path
  branch_name: string;   // Branch to delete
  force?: boolean;       // Force delete (default: false)
}

// Example
await invoke("delete_branch", {
  repo_path: "/home/user/my-project",
  branch_name: "old-feature",
  force: false
});
```

#### rename_branch

```typescript
invoke("rename_branch", {
  repo_path: string;
  old_name: string;
  new_name: string;
}): Promise<void>

// Example
await invoke("rename_branch", {
  repo_path: "/home/user/my-project",
  old_name: "feature",
  new_name: "feature/improved"
});
```

## 5. Status Commands

### status.rs

#### get_status

```typescript
invoke("get_status", {
  repo_path: string;
}): Promise<FileStatusInfo[]>

// Response - Array of FileStatusInfo
[
  {
    path: string;           // File path
    status: string;         // Status string
    staged: boolean;        // In staging area
    untracked: boolean;     // Untracked file
  },
  // ... more files
]

// Example
const status = await invoke("get_status", {
  repo_path: "/home/user/my-project"
});
```

#### stage_file

```typescript
invoke("stage_file", {
  repo_path: string;
  file_path: string;
}): Promise<void>

// Example
await invoke("stage_file", {
  repo_path: "/home/user/my-project",
  file_path: "src/main.ts"
});
```

#### unstage_file

```typescript
invoke("unstage_file", {
  repo_path: string;
  file_path: string;
}): Promise<void>
```

#### create_commit

```typescript
invoke("create_commit", {
  repo_path: string;
  message: string;
}): Promise<string>  // Returns commit SHA

// Example
const sha = await invoke("create_commit", {
  repo_path: "/home/user/my-project",
  message: "feat: add new feature\n\nDetailed description"
});
console.log("Commit created:", sha);
```

#### discard_file

```typescript
invoke("discard_file", {
  repo_path: string;
  file_path: string;
}): Promise<void>
```

#### resolve_conflict_file

```typescript
invoke("resolve_conflict_file", {
  repo_path: string;
  file_path: string;
  resolution: string;  // "ours" | "theirs" | "both"
}): Promise<void>
```

#### resolve_all_conflicts

```typescript
invoke("resolve_all_conflicts", {
  repo_path: string;
}): Promise<void>
```

## 6. Diff Commands

### diff.rs

#### get_working_diff

```typescript
invoke("get_working_diff", {
  repo_path: string;
  file_path?: string;
}): Promise<FileDiff>

// Response
{
  path: string;           // File path
  status: string;         // "modified", "added", "deleted"
  hunks: DiffHunk[];      // Diff hunks
  lines: DiffLine[];      // Individual diff lines
  is_binary: boolean;     // Is binary file
  old_size: number;       // Original size
  new_size: number;       // New size
}

// Example
const diff = await invoke("get_working_diff", {
  repo_path: "/home/user/my-project",
  file_path: "src/main.ts"
});
```

#### get_commit_diff

```typescript
invoke("get_commit_diff", {
  repo_path: string;
  commit_id: string;
  file_path?: string;
}): Promise<FileDiff>
```

#### get_file_content

```typescript
invoke("get_file_content", {
  repo_path: string;
  commit_id: string;
  file_path: string;
}): Promise<string>

// Returns file content at specific commit
```

#### save_file_content

```typescript
invoke("save_file_content", {
  repo_path: string;
  file_path: string;
  content: string;
}): Promise<void>
```

#### revert_hunk

```typescript
invoke("revert_hunk", {
  repo_path: string;
  file_path: string;
  hunk_index: number;
}): Promise<void>
```

## 7. Stash Commands

### stash.rs

#### stash_list

```typescript
invoke("stash_list", {
  repo_path: string;
}): Promise<StashInfo[]>

// Response
[
  {
    id: string;              // Stash identifier
    index: number;           // Stash index
    message: string;         // Stash message
    timestamp: number;       // Creation time
    files: CommitFileInfo[]; // Changed files
  },
  // ... more stashes
]
```

#### stash_push

```typescript
invoke("stash_push", {
  repo_path: string;
  message: string;
}): Promise<void>
```

#### stash_pop

```typescript
invoke("stash_pop", {
  repo_path: string;
  stash_index: number;
}): Promise<void>
```

#### stash_apply

```typescript
invoke("stash_apply", {
  repo_path: string;
  stash_index: number;
}): Promise<void>
```

#### stash_drop

```typescript
invoke("stash_drop", {
  repo_path: string;
  stash_index: number;
}): Promise<void>
```

#### stash_files

```typescript
invoke("stash_files", {
  repo_path: string;
  stash_index: number;
}): Promise<CommitFileInfo[]>
```

## 8. Tag Commands

### tags.rs

#### get_tags

```typescript
invoke("get_tags", {
  repo_path: string;
}): Promise<TagInfo[]>

// Response
[
  {
    name: string;              // Tag name
    commit_id: string;         // Associated commit SHA
    is_annotated: boolean;     // Is annotated tag
    message?: string;          // Tag message
    tagger?: string;           // Tagger name
    timestamp?: number;        // Creation timestamp
  },
  // ... more tags
]
```

#### create_tag_at

```typescript
invoke("create_tag_at", {
  repo_path: string;
  tag_name: string;
  target: string;  // Commit SHA or ref
}): Promise<void>
```

#### delete_tag

```typescript
invoke("delete_tag", {
  repo_path: string;
  tag_name: string;
}): Promise<void>
```

#### create_annotated_tag

```typescript
invoke("create_annotated_tag", {
  repo_path: string;
  tag_name: string;
  message: string;
}): Promise<void>
```

## 9. Clone/Init Commands

### clone_init.rs

#### clone_repo

```typescript
invoke("clone_repo", {
  url: string;
  path: string;
}): Promise<void>

// Parameters
{
  url: string;   // Repository URL (HTTPS or SSH)
  path: string;  // Local destination path
}

// Example
await invoke("clone_repo", {
  url: "https://github.com/user/repo.git",
  path: "/home/user/my-project"
});
```

#### init_repo

```typescript
invoke("init_repo", {
  path: string;
}): Promise<void>

// Example
await invoke("init_repo", {
  path: "/home/user/new-project"
});
```

## 10. Operations Commands

### operations.rs

#### pull

```typescript
invoke("pull", {
  repo_path: string;
}): Promise<void>
```

#### push

```typescript
invoke("push", {
  repo_path: string;
  force?: boolean;
}): Promise<void>
```

#### fetch_all

```typescript
invoke("fetch_all", {
  repo_path: string;
}): Promise<void>
```

#### cherry_pick

```typescript
invoke("cherry_pick", {
  repo_path: string;
  commit_id: string;
}): Promise<void>
```

#### revert_commit

```typescript
invoke("revert_commit", {
  repo_path: string;
  commit_id: string;
}): Promise<void>
```

#### reset_to_commit

```typescript
invoke("reset_to_commit", {
  repo_path: string;
  commit_id: string;
  mode?: "soft" | "mixed" | "hard";
}): Promise<void>
```

#### checkout_commit

```typescript
invoke("checkout_commit", {
  repo_path: string;
  commit_id: string;
}): Promise<void>
```

#### reset_branch_to_remote

```typescript
invoke("reset_branch_to_remote", {
  repo_path: string;
  branch: string;
  remote: string;
}): Promise<void>
```

#### set_upstream

```typescript
invoke("set_upstream", {
  repo_path: string;
  branch: string;
  upstream: string;
}): Promise<void>
```

#### edit_commit_message

```typescript
invoke("edit_commit_message", {
  repo_path: string;
  commit_id: string;
  new_message: string;
}): Promise<void>
```

#### get_git_path

```typescript
invoke("get_git_path"): Promise<string>

// Returns path to system git executable
```

#### run_git_command

```typescript
invoke("run_git_command", {
  repo_path: string;
  args: string[];
}): Promise<string>

// Execute raw git command
```

#### run_shell_command

```typescript
invoke("run_shell_command", {
  path: string;
  command: string;
}): Promise<string>

// Execute shell command in repository context
```

#### get_available_external_tools

```typescript
invoke("get_available_external_tools"): Promise<string[]>

// Example response
["vscode", "visualstudio", "androidstudio", "intellij", "explorer"]
```

#### open_path_with_tool

```typescript
invoke("open_path_with_tool", {
  path: string;
  tool: string;
}): Promise<void>

// Supported tool ids
// vscode, visualstudio, androidstudio, intellij, explorer

// Examples
await invoke("open_path_with_tool", { path: repoPath, tool: "vscode" });
await invoke("open_path_with_tool", { path: repoPath, tool: "explorer" });
```

`open_path_with_tool` is the command used by the View -> Open in Folder Explorer action and Alt+O shortcut.

## 11. Credentials Commands

### credentials.rs

#### save_token

```typescript
invoke("save_token", {
  provider: string;  // "github" | "gitlab"
  token: string;
}): Promise<void>
```

#### load_token

```typescript
invoke("load_token", {
  provider: string;
}): Promise<string | null>
```

#### delete_token

```typescript
invoke("delete_token", {
  provider: string;
}): Promise<void>
```

#### save_provider_token

```typescript
invoke("save_provider_token", {
  provider: string;
  username: string;
  token: string;
  expires_at?: number;
}): Promise<void>
```

#### load_provider_token

```typescript
invoke("load_provider_token", {
  provider: string;
}): Promise<ProviderToken | null>

// Response
{
  provider: string;       // "github" | "gitlab"
  token: string;          // API token
  username: string;       // Associated username
  expires_at?: number;    // Expiration timestamp
}
```

#### delete_provider_token

```typescript
invoke("delete_provider_token", {
  provider: string;
}): Promise<void>
```

## 12. GitHub/GitLab Commands

### operations.rs (GitHub/GitLab functions)

#### search_github_repos

```typescript
invoke("search_github_repos", {
  query: string;
  page?: number;
}): Promise<GithubRepo[]>

// Response
[
  {
    id: number;              // GitHub repo ID
    name: string;            // Repository name
    full_name: string;       // owner/repo
    description?: string;    // Description
    url: string;             // HTTPS URL
    ssh_url: string;         // SSH URL
    stars: number;           // Star count
    language?: string;       // Primary language
    is_fork: boolean;        // Is fork
    owner: string;           // Owner login
  },
  // ... more repos
]
```

#### verify_github_token

```typescript
invoke("verify_github_token", {
  token: string;
}): Promise<boolean>
```

#### search_gitlab_repos

```typescript
invoke("search_gitlab_repos", {
  query: string;
  page?: number;
}): Promise<GitlabRepo[]>

// Response
[
  {
    id: number;                 // GitLab repo ID
    name: string;               // Project name
    path_with_namespace: string; // group/project
    description?: string;       // Description
    http_url: string;           // HTTPS URL
    ssh_url: string;            // SSH URL
    star_count: number;         // Stars
    language?: string;          // Primary language
    visibility: string;         // "public" | "private"
  },
  // ... more projects
]
```

#### verify_gitlab_token

```typescript
invoke("verify_gitlab_token", {
  token: string;
}): Promise<boolean>
```

#### generate_ssh_key

```typescript
invoke("generate_ssh_key"): Promise<string>

// Returns public key
```

#### add_gitlab_ssh_key

```typescript
invoke("add_gitlab_ssh_key", {
  token: string;
  public_key: string;
}): Promise<void>
```

## 13. Ghost Branch Commands

### ghost.rs

Ghost branches are experimental/temporary branches that allow you to test changes without creating permanent commits.

#### get_ghost_branch_state

```typescript
invoke("get_ghost_branch_state", {
  path: string;
}): Promise<GhostBranchState>

// Response
{
  is_active: boolean;        // Is ghost branch currently active
  changes: string[];         // List of changed files
  commit_message?: string;   // Staged commit message if any
}

// Example
const state = await invoke("get_ghost_branch_state", {
  path: "/home/user/my-project"
});
```

#### start_ghost_branch

```typescript
invoke("start_ghost_branch", {
  path: string;
}): Promise<GhostBranchState>

// Activate experimental ghost branch mode
// Example
await invoke("start_ghost_branch", {
  path: "/home/user/my-project"
});
```

#### materialize_ghost_branch

```typescript
invoke("materialize_ghost_branch", {
  path: string;
  name: string;
}): Promise<string>

// Convert ghost branch to a real, permanent branch
// Parameters
{
  path: string;   // Repository path
  name: string;   // Name for the new permanent branch
}

// Returns: Branch name
// Example
const branchName = await invoke("materialize_ghost_branch", {
  path: "/home/user/my-project",
  name: "feature/experimental-ui"
});
```

#### discard_ghost_branch

```typescript
invoke("discard_ghost_branch", {
  path: string;
}): Promise<string>

// Discard all changes in the ghost branch without creating a real branch
// Example
await invoke("discard_ghost_branch", {
  path: "/home/user/my-project"
});
```

## 14. Conflict, Productivity, and Time-Machine Commands

### conflicts.rs

These commands power conflict suspects, conflict pairs, repository tree heatmap, and merge preflight checks.

#### get_conflict_hotspots

```typescript
invoke("get_conflict_hotspots", {
  path: string;
  maxCount?: number;       // Frontend style (maps to max_count)
  lookbackMonths?: number; // Frontend style (maps to lookback_months)
}): Promise<ConflictHotspot[]>

// Response
[
  {
    path: string;               // file path
    merge_touches: number;      // merges touching file
    conflict_mentions: number;  // conflict-related mentions
    score: number;              // hotspot score
    collision_index: number;    // compounded risk indicator
  }
]
```

#### get_conflict_pairs

```typescript
invoke("get_conflict_pairs", {
  path: string;
  maxCount?: number;
  lookbackMonths?: number;
}): Promise<ConflictPair[]>

// Response
[
  {
    left_path: string;
    right_path: string;
    co_touches: number;
    conflict_touches: number;
    score: number;
  }
]
```

#### get_repository_tree_paths

```typescript
invoke("get_repository_tree_paths", {
  path: string;
  maxCount?: number;
}): Promise<string[]>

// Returns normalized repository-relative paths for building tree UI.
```

#### get_merge_preflight_risk

```typescript
invoke("get_merge_preflight_risk", {
  path: string;
  sourceBranch: string;
  sourceRemote?: boolean;
  targetBranch: string;
  maxCount?: number;
  lookbackMonths?: number;
}): Promise<MergeRiskPreflight>

// Response
{
  source_ref: string;
  target_ref: string;
  shared_change_count: number;
  suspect_count: number;
  risk_level: "low" | "moderate" | "high" | "critical";
  suspect_files: Array<{
    path: string;
    score: number;
    collision_index: number;
  }>;
}
```

### commits.rs (analytics/time-machine support)

#### get_author_deletion_stats

```typescript
invoke("get_author_deletion_stats", {
  path: string;
  maxCount?: number;
}): Promise<[string, number, number][]>

// Tuple format per row:
// [authorName, deletedLines, commitCount]
```

#### get_commit_tree_paths

```typescript
invoke("get_commit_tree_paths", {
  path: string;
  sha: string;
}): Promise<string[]>

// Returns file paths visible at that commit snapshot.
```

### Parameter Naming Note

Rust command handlers use `snake_case` parameter names while frontend invoke calls may use `camelCase` keys. Keep frontend examples aligned with current UI usage (camelCase), and keep backend signatures in Rust as snake_case.

## 15. Error Handling

All commands can throw errors with descriptive messages:

```typescript
try {
  await invoke("some_command", { ... });
} catch (error: any) {
  const errorMessage: string = error;
  console.error("Command failed:", errorMessage);
  
  // Handle specific errors
  if (errorMessage.includes("not found")) {
    // Repository not found
  } else if (errorMessage.includes("conflict")) {
    // Merge conflict
  } else if (errorMessage.includes("Permission denied")) {
    // Access error
  }
}
```

## 16. Type Definitions

All commands are fully type-safe with TypeScript definitions available in `src/types/index.ts`.

---

**Related Documentation:**
- [02_DATA_MODELS.md](./DOCUMENTATION_02_DATA_MODELS.md) - Data model definitions
- [08_BACKEND_OVERVIEW.md](./DOCUMENTATION_08_BACKEND_OVERVIEW.md) - Backend architecture
