# Backend Overview

## 1. Backend Architecture

GitSwamp's backend is built with Rust and Tauri, providing type-safe, high-performance Git operations through an Inter-Process Communication (IPC) protocol with the frontend.

### 1.1 Technology Stack

| Technology | Version | Purpose |
|-----------|---------|---------|
| Tauri | 2.0 | Desktop application framework |
| Rust | Latest | Systems programming language |
| git2-rs | 0.19 | Rust bindings for libgit2 |
| serde | 1.0 | Serialization framework |
| ureq | 2.0 | HTTP client for APIs |

## 2. Project Structure

### 2.1 Cargo Workspace

```
src-tauri/
├── src/
│   ├── commands/                  # Tauri command handlers (active modules)
│   │   ├── mod.rs                # Module exports
│   │   ├── repository.rs         # Repository metadata
│   │   ├── commits.rs            # Commit history
│   │   ├── commit_files.rs       # Files in commits
│   │   ├── conflicts.rs          # Conflict analytics and preflight
│   │   ├── branches.rs           # Branch operations
│   │   ├── status.rs             # Working directory status
│   │   ├── diff.rs               # Diff generation
│   │   ├── stash.rs              # Stash management
│   │   ├── tags.rs               # Tag operations
│   │   ├── clone_init.rs         # Repo creation
│   │   ├── operations.rs         # Advanced operations
│   │   ├── ghost.rs              # Ghost branch workflow
│   │   ├── credentials.rs        # Token management
│   │   └── logs.rs               # Application logs
│   │
│   ├── models/                    # Data models (core + analytics)
│   │   ├── mod.rs
│   │   ├── commit.rs
│   │   ├── branch.rs
│   │   ├── file_status.rs
│   │   ├── repository.rs
│   │   ├── commit_file.rs
│   │   ├── conflict_hotspot.rs   # Conflict hotspot/pair/preflight models
│   │   ├── stash.rs
│   │   ├── tag.rs
│   │   ├── diff.rs
│   │   ├── github.rs
│   │   └── gitlab.rs
│   │
│   ├── services/                  # Service layer
│   │   ├── mod.rs
│   │   └── git_service.rs        # Git2-rs wrapper
│   │
│   ├── lib.rs                     # Library entry point
│   └── main.rs                    # Binary entry point
│
├── Cargo.toml                      # Rust manifest
├── Cargo.lock                      # Dependency lock
├── tauri.conf.json                # Tauri configuration
├── build.rs                        # Build script
└── icons/                          # App icons
```

## 3. Tauri Configuration

### 3.1 Core Configuration

**File:** `tauri.conf.json`

```json
{
  "appIdentifier": "com.wortex.gitswamp",
  "appName": "GitSwamp",
  "build": {
    "beforeBuildCommand": "",
    "beforeDevCommand": "",
    "devUrl": "http://localhost:1420",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "GitSwamp",
        "width": 1400,
        "height": 900,
        "minWidth": 800,
        "minHeight": 600,
        "decorations": false,
        "transparent": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "tauri": {
    "cli": {},
    "bundle": {
      "active": true,
      "targets": ["deb", "appimage", "msi", "dmg"]
    }
  }
}
```

### 3.2 Window Configuration

- **Size:** 1400x900 pixels
- **Minimum:** 800x600 pixels
- **Custom Titlebar:** No decorations (custom implementation)
- **Transparency:** Disabled
- **Resizable:** Yes

## 4. Command System

### 4.1 Command Architecture

```
Vue Component.invoke("command_name")
    │
    └─> Tauri IPC Layer
            │
            └─> Rust Async Runtime
                    │
                    └─> Command Handler
                            │
                            ├─ Parse arguments
                            ├─ Validate input
                            ├─ Perform operation
                            └─ Serialize result
                                    │
                                    └─ Return to Frontend (JSON)
```

### 4.2 Command Registration

Commands are registered in `lib.rs`:

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::repository::get_repo_info,
            commands::commits::get_commits,
            commands::branches::get_branches,
            // ... more commands
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## 5. Command Modules (12 Total)

### 5.1 Repository Commands (repository.rs)

**Functions:**
- `get_repo_info(path: String) -> Result<RepoInfo, String>`

**Purpose:** Retrieve metadata about a Git repository

**Returns:**
```rust
RepoInfo {
    path: String,
    is_bare: bool,
    head: String,
    current_branch: String,
    remotes: Vec<RemoteInfo>,
    has_conflicts: bool,
    workdir: Option<String>,
}
```

### 5.2 Commits Commands (commits.rs)

**Functions:**
- `get_commits(repo_path: String, offset: usize, limit: usize) -> Result<Vec<CommitInfo>, String>`
- `search_commits(repo_path: String, query: String) -> Result<Vec<CommitInfo>, String>`

**Purpose:** Fetch and search commit history

### 5.3 Commit Files Commands (commit_files.rs)

**Functions:**
- `get_commit_files(repo_path: String, commit_id: String) -> Result<Vec<CommitFileInfo>, String>`

**Purpose:** Get files changed in a specific commit

### 5.4 Branches Commands (branches.rs)

**Functions:**
- `get_branches(repo_path: String) -> Result<Vec<BranchInfo>, String>`
- `checkout_branch(repo_path: String, branch_name: String) -> Result<(), String>`
- `create_branch(repo_path: String, branch_name: String) -> Result<(), String>`
- `delete_branch(repo_path: String, branch_name: String) -> Result<(), String>`
- `rename_branch(repo_path: String, old_name: String, new_name: String) -> Result<(), String>`

**Purpose:** Manage branches

### 5.5 Status Commands (status.rs)

**Functions:**
- `get_status(repo_path: String) -> Result<Vec<FileStatusInfo>, String>`
- `stage_file(repo_path: String, file_path: String) -> Result<(), String>`
- `unstage_file(repo_path: String, file_path: String) -> Result<(), String>`
- `create_commit(repo_path: String, message: String) -> Result<String, String>`
- `discard_file(repo_path: String, file_path: String) -> Result<(), String>`
- `resolve_conflict_file(repo_path: String, file_path: String, resolution: String) -> Result<(), String>`
- `resolve_all_conflicts(repo_path: String) -> Result<(), String>`

**Purpose:** Manage working directory and staging area

### 5.6 Diff Commands (diff.rs)

**Functions:**
- `get_working_diff(repo_path: String, file_path: Option<String>) -> Result<FileDiff, String>`
- `get_commit_diff(repo_path: String, commit_id: String, file_path: Option<String>) -> Result<FileDiff, String>`
- `get_file_content(repo_path: String, commit_id: String, file_path: String) -> Result<String, String>`
- `save_file_content(repo_path: String, file_path: String, content: String) -> Result<(), String>`
- `revert_hunk(repo_path: String, file_path: String, hunk_index: usize) -> Result<(), String>`

**Purpose:** Generate and manage diffs

### 5.7 Stash Commands (stash.rs)

**Functions:**
- `stash_list(repo_path: String) -> Result<Vec<StashInfo>, String>`
- `stash_push(repo_path: String, message: String) -> Result<(), String>`
- `stash_pop(repo_path: String, stash_index: usize) -> Result<(), String>`
- `stash_apply(repo_path: String, stash_index: usize) -> Result<(), String>`
- `stash_drop(repo_path: String, stash_index: usize) -> Result<(), String>`
- `stash_files(repo_path: String, stash_index: usize) -> Result<Vec<CommitFileInfo>, String>`

**Purpose:** Manage stashes

### 5.8 Tags Commands (tags.rs)

**Functions:**
- `get_tags(repo_path: String) -> Result<Vec<TagInfo>, String>`
- `create_tag_at(repo_path: String, tag_name: String, target: String) -> Result<(), String>`
- `delete_tag(repo_path: String, tag_name: String) -> Result<(), String>`
- `create_annotated_tag(repo_path: String, tag_name: String, message: String) -> Result<(), String>`

**Purpose:** Manage tags

### 5.9 Clone/Init Commands (clone_init.rs)

**Functions:**
- `clone_repo(url: String, path: String) -> Result<(), String>`
- `init_repo(path: String) -> Result<(), String>`

**Purpose:** Repository initialization

### 5.10 Operations Commands (operations.rs)

**Advanced Git Operations:**
- `pull(repo_path: String) -> Result<(), String>`
- `push(repo_path: String) -> Result<(), String>`
- `fetch_all(repo_path: String) -> Result<(), String>`
- `cherry_pick(repo_path: String, commit_id: String) -> Result<(), String>`
- `revert_commit(repo_path: String, commit_id: String) -> Result<(), String>`
- `reset_to_commit(repo_path: String, commit_id: String) -> Result<(), String>`
- `checkout_commit(repo_path: String, commit_id: String) -> Result<(), String>`
- `reset_branch_to_remote(repo_path: String, branch: String, remote: String) -> Result<(), String>`
- `set_upstream(repo_path: String, branch: String, upstream: String) -> Result<(), String>`
- `edit_commit_message(repo_path: String, commit_id: String, new_message: String) -> Result<(), String>`

### 5.11 Credentials Commands (credentials.rs)

**Functions:**
- `save_token(provider: String, token: String) -> Result<(), String>`
- `load_token(provider: String) -> Result<Option<String>, String>`
- `delete_token(provider: String) -> Result<(), String>`
- `save_provider_token(provider: String, username: String, token: String, expires_at: Option<i64>) -> Result<(), String>`
- `load_provider_token(provider: String) -> Result<Option<ProviderToken>, String>`
- `delete_provider_token(provider: String) -> Result<(), String>`

**Purpose:** Token and credential management

### 5.12 GitHub/GitLab Integration (operations.rs)

**GitHub Functions:**
- `search_github_repos(query: String, page: u32) -> Result<Vec<GithubRepo>, String>`
- `verify_github_token(token: String) -> Result<bool, String>`

**GitLab Functions:**
- `search_gitlab_repos(query: String, page: u32) -> Result<Vec<GitlabRepo>, String>`
- `verify_gitlab_token(token: String) -> Result<bool, String>`
- `generate_ssh_key() -> Result<String, String>`
- `add_gitlab_ssh_key(token: String, public_key: String) -> Result<(), String>`

## 6. Services Layer

### 6.1 Service Layer Architecture

**7 Service Modules:**
1. `git_service.rs` - Core Git2-rs wrapper
2. `diff_service.rs` - Diff generation and hunk operations
3. `stash_service.rs` - Stash management operations
4. `remote_service.rs` - Remote operations (push, pull, fetch)
5. `integration_service.rs` - External tool integration (VS Code, explorer, etc.)
6. `helpers.rs` - Utility functions
7. `mod.rs` - Module exports

### 6.2 GitService

**File:** `src/services/git_service.rs`

**Purpose:** High-level wrapper around git2-rs library

**Key Methods:**
- Repository opening and validation
- Commit history retrieval with graph info
- Branch operations
- File status checking
- Diff generation
- Merge and conflict handling
- Stash operations
- Tag operations
- Remote operations (push, pull, fetch)
- Credential handling

### 6.2 Error Handling

```rust
pub type GitResult<T> = Result<T, String>;

fn handle_git_error(error: git2::Error) -> String {
    format!("Git error: {}", error.message())
}
```

## 7. Data Models

### 7.1 Model Count

**Total Models:** 11 (across 11 Rust files in models/)

1. **CommitInfo** - Commit metadata (commit.rs)
2. **BranchInfo** - Branch information (branch.rs)
3. **FileStatusInfo** - File status (file_status.rs)
4. **RepoInfo** - Repository metadata (repository.rs)
5. **CommitFileInfo** - Files in commit (commit_file.rs)
6. **StashInfo** - Stash metadata (stash.rs)
7. **TagInfo** - Tag information (tag.rs)
8. **FileDiff** - Diff data (diff.rs)
9. **GithubRepo** - GitHub API model (github.rs)
10. **GitlabRepo** - GitLab API model (gitlab.rs)
11. **mod.rs** - Model module exports

### 7.2 Model Serialization

All models use `serde` for JSON serialization:

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CommitInfo {
    pub id: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub message: String,
    pub parent_ids: Vec<String>,
    pub branch: Option<String>,
    pub tags: Vec<String>,
}
```

## 8. Cargo Dependencies

### 8.1 Key Dependencies

```toml
[dependencies]
tauri = { version = "2", features = ["shell-open", "dialog"] }
tauri-plugin-dialog = "2.6"
tauri-plugin-opener = "2"
git2 = "0.19"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ureq = "2"
tokio = { version = "1", features = ["full"] }
```

### 8.2 Build Dependencies

```toml
[build-dependencies]
tauri-build = "2"
```

## 9. Async/Await Pattern

### 9.1 Async Commands

All Tauri commands are automatically async:

```rust
#[tauri::command]
pub async fn get_commits(
    repo_path: String,
    offset: usize,
    limit: usize,
) -> Result<Vec<CommitInfo>, String> {
    // Async operation
    tokio::task::spawn_blocking(move || {
        // CPU-bound work here
    }).await.unwrap()
}
```

### 9.2 Blocking Operations

Git operations are CPU-intensive, so they're run in blocking tasks:

```rust
#[tauri::command]
pub async fn get_repo_info(path: String) -> Result<RepoInfo, String> {
    tokio::task::spawn_blocking(move || {
        git_service::get_repo_info(&path)
    })
    .await
    .map_err(|e| e.to_string())?
}
```

## 10. Error Handling

### 10.1 Error Propagation

```rust
#[tauri::command]
pub async fn checkout_branch(
    repo_path: String,
    branch_name: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let repo = git2::Repository::open(&repo_path)
            .map_err(|e| e.to_string())?;
        
        let obj = repo.revparse_single(&branch_name)
            .map_err(|e| format!("Branch not found: {}", e))?;
        
        repo.set_head_detached(obj.id())
            .map_err(|e| format!("Failed to checkout: {}", e))?;
        
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

### 10.2 Error Types

| Error Type | Handling | Example |
|-----------|----------|---------|
| Git Operation Error | Return as String | Repository not found |
| IO Error | Return as String | File not accessible |
| Serialization Error | Return as String | Invalid data format |
| API Error | Return as String | Network failure |

## 11. Security Considerations

### 11.1 Path Validation

All file paths are validated:
```rust
fn validate_path(path: &str) -> Result<(), String> {
    if path.contains("..") {
        return Err("Invalid path".into());
    }
    Ok(())
}
```

### 11.2 Credential Security

Credentials are:
- Stored encrypted on disk
- Never logged
- Cleared from memory after use
- Protected with file permissions

## 12. Performance Optimization

### 12.1 Git2-rs Efficiency

- Uses libgit2 native C library
- Efficient repository iteration
- Minimal memory allocation
- Caching where appropriate

### 12.2 Command Execution

- Async/await for non-blocking operations
- Blocking task pool for CPU-intensive work
- Efficient serialization with serde

## 13. Commit Intelligence Backend Module Placement (2026-04 Update)

This section defines where backend logic for Conflict, Productivity, and Time Machine features must live.

### 13.1 Backend Module Ownership

| Layer | Module File | Responsibility |
|------|-------------|----------------|
| Tauri command entry | `src-tauri/src/commands/conflicts.rs` | Exposes conflict analytics commands (`get_conflict_hotspots`, `get_conflict_pairs`, `get_repository_tree_paths`, `get_merge_preflight_risk`) |
| Tauri command entry | `src-tauri/src/commands/commits.rs` | Exposes history and analytics support commands (`get_commits`, `get_author_deletion_stats`, `get_commit_tree_paths`) |
| Command registration | `src-tauri/src/lib.rs` | Registers all invoke handlers and keeps command surface centralized |
| Service execution | `src-tauri/src/services/git_service.rs` | Heavy git2/libgit2 operations, scoring, filtering, preflight merge analysis, tree path extraction, deletion statistics |
| Data contracts | `src-tauri/src/models/conflict_hotspot.rs` | `ConflictHotspot`, `ConflictPair`, `MergeRiskPreflight` payload contracts |
| Model export surface | `src-tauri/src/models/mod.rs` | Canonical model exports used by command modules |

### 13.2 Placement Rules (Backend)

1. `commands/*` files must remain thin wrappers: parse input, spawn blocking task, return typed result.
2. Analytics computation and repository traversal must stay in `services/git_service.rs`.
3. New payload structs must be added under `models/` and re-exported in `models/mod.rs`.
4. New Tauri commands must be wired in `lib.rs` invoke handler list.
5. Performance-sensitive loops (history scans, diff stats) should run in blocking runtime and avoid repeated repository reopen.

### 13.3 Frontend-to-Backend Bridge for Updated Panels

| Frontend panel | Required backend commands |
|---------------|---------------------------|
| Conflict suspects panel | `get_conflict_hotspots`, `get_conflict_pairs`, `get_repository_tree_paths` |
| Merge flow pre-check | `get_merge_preflight_risk` |
| Productivity arena | `get_commits`, `get_author_deletion_stats`, `get_conflict_hotspots` |
| Time Machine | `get_commits`, `get_commit_files`, `get_commit_tree_paths`, `get_file_content` |

### 13.4 Panel-level behavior notes

- Conflict suspects uses separate repository scans for hotspot scoring, pair scoring, and tree-path hierarchy building.
- Productivity Arena loads commit history, author deletion stats, and conflict hotspots in parallel so metrics can appear independently.
- Time Machine loads full history first, then fetches file and tree snapshots lazily for the selected frame.
- Commit snapshot preview requests are keyed by repository, commit SHA, and file path to keep autoplay responsive.

---

**Related Documentation:**
- [09_COMMANDS_REFERENCE.md](./DOCUMENTATION_09_COMMANDS_REFERENCE.md) - Detailed command reference
- [10_SERVICES_GUIDE.md](./DOCUMENTATION_10_SERVICES_GUIDE.md) - Service layer documentation
- [02_DATA_MODELS.md](./DOCUMENTATION_02_DATA_MODELS.md) - Data models reference
