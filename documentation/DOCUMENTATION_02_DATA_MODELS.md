# Data Models and Type Definitions

## 1. Overview

GitSwamp uses a comprehensive type system across both frontend (TypeScript) and backend (Rust) to ensure type safety throughout the entire application. All data structures are carefully designed to match Git repository semantics while providing efficient serialization.

## 2. Core Data Models

### 2.1 Repository Information

#### RepoInfo (Rust & TypeScript)

```rust
pub struct RepoInfo {
    pub path: String,              // Absolute path to .git directory
    pub is_bare: bool,             // Is bare repository
    pub head: String,              // Current HEAD reference
    pub current_branch: String,    // Current branch name
    pub remotes: Vec<RemoteInfo>,  // Remote configurations
    pub has_conflicts: bool,       // Merge conflict status
    pub workdir: Option<String>,   // Working directory path
}
```

**TypeScript Equivalent:**
```typescript
interface RepoInfo {
  path: string;
  is_bare: boolean;
  head: string;
  current_branch: string;
  remotes: RemoteInfo[];
  has_conflicts: boolean;
  workdir?: string;
}
```

**Usage:**
- Stored in `useGit.currentRepo`
- Displayed in `AppHeader`
- Used to determine available operations

#### RemoteInfo (Rust & TypeScript)

```rust
pub struct RemoteInfo {
    pub name: String,              // Remote name (usually "origin")
    pub url: String,               // Remote URL
    pub fetch: Option<String>,     // Fetch refspec
    pub push: Option<String>,      // Push refspec
}
```

**Use Cases:**
- Display in sidebar
- Validate push/pull operations
- Show remote branches

### 2.2 Commit Information

#### CommitInfo (Rust & TypeScript)

```rust
pub struct CommitInfo {
    pub id: String,                // Commit SHA-1 hash
    pub author: String,            // Author name
    pub email: String,             // Author email
    pub timestamp: i64,            // Unix timestamp
    pub message: String,           // Commit message
    pub parent_ids: Vec<String>,   // Parent commit SHAs
    pub branch: Option<String>,    // Associated branch
    pub tags: Vec<String>,         // Associated tags
}
```

**TypeScript Equivalent:**
```typescript
interface CommitInfo {
  id: string;
  author: string;
  email: string;
  timestamp: number;
  message: string;
  parent_ids: string[];
  branch?: string;
  tags: string[];
}
```

**Usage:**
- Rendered in `CommitGraph`
- Displayed in `CommitDetails`
- Used for searching and filtering

#### CommitFileInfo (Rust & TypeScript)

```rust
pub struct CommitFileInfo {
    pub path: String,              // File path
    pub status: String,            // "Added", "Deleted", "Modified"
    pub old_path: Option<String>,  // Original path if renamed
    pub insertions: usize,         // Lines added
    pub deletions: usize,          // Lines deleted
}
```

**Usage:**
- Listed in `CommitDetails`
- Displayed with change statistics
- Filtered by status

### 2.3 Branch Information

#### BranchInfo (Rust & TypeScript)

```rust
pub struct BranchInfo {
    pub name: String,              // Branch name
    pub is_local: bool,            // Local vs remote
    pub is_current: bool,          // Is current branch
    pub upstream: Option<String>,  // Upstream branch
    pub shorthand: String,         // Short name
}
```

**TypeScript Equivalent:**
```typescript
interface BranchInfo {
  name: string;
  is_local: boolean;
  is_current: boolean;
  upstream?: string;
  shorthand: string;
}
```

**Usage:**
- Rendered in `Sidebar`
- Used for checkout operations
- Displayed with tracking information

### 2.4 File Status Information

#### FileStatusInfo (Rust & TypeScript)

```rust
pub struct FileStatusInfo {
    pub path: String,              // File path
    pub status: FileStatus,        // Status enum
    pub staged: bool,              // In staging area
    pub untracked: bool,           // Untracked file
}

pub enum FileStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed(String),               // Old path
    Copied(String),                // Source path
    Ignored,
    Untracked,
    Conflicted,
}
```

**Usage:**
- Displayed in `CommitDetails`
- Used for staging/unstaging
- Shows merge conflicts

### 2.5 Diff Information

#### FileDiff (Rust & TypeScript)

```rust
pub struct FileDiff {
    pub path: String,              // File path
    pub status: String,            // "modified", "added", "deleted"
    pub hunks: Vec<DiffHunk>,      // Diff hunks
    pub lines: Vec<DiffLine>,      // All diff lines
    pub is_binary: bool,           // Binary file flag
    pub old_size: usize,           // Original size
    pub new_size: usize,           // New size
}

pub struct DiffHunk {
    pub old_start: usize,          // Old file line start
    pub old_lines: usize,          // Old file lines
    pub new_start: usize,          // New file line start
    pub new_lines: usize,          // New file lines
    pub header: String,            // Hunk header
}

pub struct DiffLine {
    pub line_type: DiffLineType,   // Type of line
    pub content: String,           // Line content
    pub old_line_no: Option<usize>,// Old line number
    pub new_line_no: Option<usize>,// New line number
}

pub enum DiffLineType {
    Context,
    Addition,
    Deletion,
    Binary,
}
```

**TypeScript Equivalent:**
```typescript
interface FileDiff {
  path: string;
  status: "modified" | "added" | "deleted";
  hunks: DiffHunk[];
  lines: DiffLine[];
  is_binary: boolean;
  old_size: number;
  new_size: number;
}

interface DiffHunk {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  header: string;
}

interface DiffLine {
  line_type: "context" | "addition" | "deletion" | "binary";
  content: string;
  old_line_no?: number;
  new_line_no?: number;
}
```

**Usage:**
- Displayed in `FileDiffViewer`
- Used for hunk operations
- Editable in editor mode

### 2.6 Stash Information

#### StashInfo (Rust & TypeScript)

```rust
pub struct StashInfo {
    pub id: String,                // Stash identifier
    pub index: usize,              // Stash index
    pub message: String,           // Stash message
    pub timestamp: i64,            // Unix timestamp
    pub files: Vec<CommitFileInfo>,// Changed files
}
```

**Usage:**
- Listed in `Sidebar`
- Displayed with timestamp
- Used for apply/pop operations

### 2.7 Tag Information

#### TagInfo (Rust & TypeScript)

```rust
pub struct TagInfo {
    pub name: String,              // Tag name
    pub commit_id: String,         // Associated commit SHA
    pub is_annotated: bool,        // Lightweight vs annotated
    pub message: Option<String>,   // Tag message
    pub tagger: Option<String>,    // Tagger name
    pub timestamp: Option<i64>,    // Unix timestamp
}
```

**Usage:**
- Listed in `Sidebar`
- Displayed with commit reference
- Used for tag operations

## 3. Graph Data Structures

### 3.1 Commit Graph Visualization

#### GraphNode

```typescript
interface GraphNode {
  id: string;           // Commit SHA
  x: number;            // X position
  y: number;            // Y position
  lane: number;         // Vertical lane
  width: number;        // Node width
  height: number;       // Node height
  commit: CommitInfo;   // Associated commit
}
```

#### GraphEdge

```typescript
interface GraphEdge {
  from: string;         // Source commit SHA
  to: string;           // Target commit SHA
  lane: number;         // Lane assignment
  path: string;         // SVG path data
}
```

## 4. GitHub Integration Models

### 4.1 GitHub Repository Model

```rust
pub struct GithubRepo {
    pub id: u64,                    // GitHub repo ID
    pub name: String,               // Repository name
    pub full_name: String,          // owner/repo
    pub description: Option<String>,// Repository description
    pub url: String,                // HTTPS clone URL
    pub ssh_url: String,            // SSH clone URL
    pub stars: u32,                 // Star count
    pub language: Option<String>,   // Primary language
    pub is_fork: bool,              // Is fork
    pub owner: String,              // Owner login
}
```

## 5. GitLab Integration Models

### 5.1 GitLab Repository Model

```rust
pub struct GitlabRepo {
    pub id: u64,                    // GitLab repo ID
    pub name: String,               // Repository name
    pub path_with_namespace: String,// group/project path
    pub description: Option<String>,// Project description
    pub http_url: String,           // HTTPS clone URL
    pub ssh_url: String,            // SSH clone URL
    pub star_count: u32,            // Stars
    pub language: Option<String>,   // Primary language
    pub visibility: String,         // Public/Private
}
```

## 6. UI State Models

### 6.1 Toast Notification

```typescript
interface Toast {
  id: string;           // Unique identifier
  type: "success" | "error" | "warning" | "info";
  message: string;      // Notification message
  duration?: number;    // Auto-dismiss time (ms)
}
```

### 6.2 Dialog State

```typescript
interface DialogState {
  isOpen: boolean;
  type: "clone" | "init" | "settings" | "none";
  data?: any;           // Dialog-specific data
}
```

### 6.3 Selection State

```typescript
interface SelectionState {
  selectedFile?: string;
  selectedCommit?: string;
  selectedBranch?: string;
  selectedStash?: string;
}
```

## 7. Request/Response Models

### 7.1 Paginated Request

```rust
pub struct PaginationRequest {
    pub offset: usize,   // Starting index
    pub limit: usize,    // Result count
}
```

### 7.2 Paginated Response

```rust
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,   // Data items
    pub total: usize,    // Total count
    pub offset: usize,   // Current offset
    pub limit: usize,    // Current limit
}
```

## 8. Error Models

### 8.1 Error Response

```rust
pub struct ErrorResponse {
    pub code: String,    // Error code
    pub message: String, // Error message
    pub details: Option<String>,  // Additional details
}
```

### 8.2 Result Type

```rust
pub type GitResult<T> = Result<T, String>;
```

## 9. Configuration Models

### 9.1 User Preferences

```rust
pub struct UserPreferences {
    pub theme: String,            // "light" or "dark"
    pub font_size: u16,           // Font size in pixels
    pub compact_mode: bool,       // Compact UI mode
    pub show_avatars: bool,       // Show commit avatars
    pub default_author: String,   // Default author name
    pub default_email: String,    // Default author email
}
```

### 9.2 Provider Token

```rust
pub struct ProviderToken {
    pub provider: String,         // "github" or "gitlab"
    pub token: String,            // OAuth/API token
    pub username: String,         // Associated username
    pub expires_at: Option<i64>,  // Token expiration
}
```

## 10. Model Relationships

### 10.1 Dependency Graph

```
RepoInfo
├─ owns: Vec<RemoteInfo>
├─ references: Vec<BranchInfo>
├─ references: Vec<CommitInfo>
│  └─ owns: Vec<CommitFileInfo>
├─ references: Vec<StashInfo>
│  └─ owns: Vec<CommitFileInfo>
├─ references: Vec<TagInfo>
│  └─ references: CommitInfo
└─ references: Vec<FileStatusInfo>

FileDiff
├─ references: FileStatusInfo
├─ owns: Vec<DiffHunk>
└─ owns: Vec<DiffLine>

CommitInfo
├─ references: Vec<CommitInfo> (parents)
└─ owns: Vec<CommitFileInfo>
```

## 11. Type Safety Implementation

### 11.1 Rust Type Validation

```rust
impl CommitInfo {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.len() != 40 { // SHA-1 length
            return Err("Invalid commit ID".into());
        }
        if self.timestamp < 0 {
            return Err("Invalid timestamp".into());
        }
        Ok(())
    }
}
```

### 11.2 TypeScript Type Guards

```typescript
function isCommitInfo(obj: any): obj is CommitInfo {
  return (
    typeof obj.id === "string" &&
    typeof obj.author === "string" &&
    typeof obj.timestamp === "number" &&
    Array.isArray(obj.parent_ids)
  );
}
```

## 12. Serialization

### 12.1 JSON Serialization (serde)

All Rust models use `serde` for JSON serialization:

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CommitInfo {
    // Fields...
}
```

### 12.2 TypeScript JSON Mapping

```typescript
// Automatically mapped from Rust serde JSON
const commit: CommitInfo = JSON.parse(jsonString);
```

## 13. Model Validation Rules

| Model | Validation |
|-------|-----------|
| CommitInfo | Valid SHA, non-negative timestamp |
| FileStatusInfo | Valid file path, known status |
| DiffLine | Valid line type, non-empty content |
| BranchInfo | Non-empty name, valid shorthand |
| TagInfo | Non-empty name, valid SHA reference |
| StashInfo | Valid index, non-empty message |

---

**Related Documentation:**
- [28_FRONTEND_API.md](./DOCUMENTATION_28_FRONTEND_API.md) - Frontend API using these types
- [29_BACKEND_API.md](./DOCUMENTATION_29_BACKEND_API.md) - Backend API using these types
- [30_TYPE_DEFINITIONS.md](./DOCUMENTATION_30_TYPE_DEFINITIONS.md) - Complete type definitions
