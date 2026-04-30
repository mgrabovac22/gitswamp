# Commit Intelligence Panels

## 1. Scope

GitSwamp exposes three history-focused panels from the View menu and repository menu:

- **Productivity Arena** - Alt+2
- **Time Machine** - Alt+3
- **Usual Conflict Suspects** - Alt+4

They are implemented as view modes inside the repository workspace, not as floating dialogs. The active panel is selected through `historyViewMode` in `RepositoryWorkspace.vue`, and each panel loads its own data independently.

This document explains how each panel works, which backend commands it uses, how state is cached, and what the user can do inside each view.

## 2. Shared Architecture

All three panels follow the same high-level pattern:

1. The workspace switches into a dedicated history mode.
2. The Vue panel triggers one or more Tauri `invoke(...)` calls.
3. Results are stored in reactive state and rendered through scoped sections.
4. Caching keys are used to avoid repeated repository scans.
5. A run token is used to discard stale responses when the user changes repository or filters quickly.

The panels do not block the entire workspace. Instead, they load their metrics section by section so the UI remains responsive while data is still arriving.

### 2.1 Shared backend sources

The panels rely on these backend commands:

- `get_commits`
- `get_author_deletion_stats`
- `get_conflict_hotspots`
- `get_conflict_pairs`
- `get_repository_tree_paths`
- `get_commit_files`
- `get_commit_tree_paths`
- `get_file_content`

The backend data is routed through Tauri commands and backed by Rust services in the Git service layer.

## 3. Productivity Arena

### 3.1 Purpose

Productivity Arena is the repository-level analytics view for commit rhythm, contributor activity, and stability signals. It is designed to answer questions such as:

- How active is the repository right now?
- Who contributes most of the commits?
- Are changes concentrated in bursts or spread across the week?
- Are there warning signs such as off-hours activity, merge pressure, or regression-like commit messages?

### 3.2 Data loading strategy

The panel starts in preview mode and initially loads a limited commit window. The user can then switch to full history by pressing **Load all**.

It loads three streams in parallel:

- commit history for timeline metrics
- author deletion statistics for change-volume metrics
- conflict hotspot data for stability scoring

Each stream has its own loading flag and error message. That means the panel can still render partial analytics even if one source fails.

### 3.3 Main metrics

The panel computes and displays:

- current streak and longest streak
- total commits in the active author filter
- arena health score derived from streak, merge ratio, and activity volume
- unique authors
- merge ratio
- weekend commit ratio
- peak coding hour
- top weekday
- average commits per day
- deleted lines and delete intensity
- balance score based on contribution distribution
- top contributor share
- bottleneck score based on burstiness, off-hours commits, ownership concentration, and throughput volatility
- stability risk score based on regression-like commit messages and conflict pressure
- collaboration intensity score

### 3.4 Author filtering

The author selector filters every derived metric. When a specific author is selected, the charts and scores recalculate on that author only. If the selected author disappears from the loaded data after a repository change, the view automatically falls back to **All contributors**.

### 3.5 Visual model

The panel uses card-based sections with per-section loader overlays. The UI groups the analytics into:

- headline metrics
- team metrics
- flow metrics
- delivery pressure
- stability pulse

This makes the panel readable even when the repository has a large amount of history.

### 3.6 Practical interpretation

The panel is most useful when you want a quick qualitative read of repository motion:

- a high streak with balanced authors usually indicates healthy momentum
- a high bottleneck score usually means work is arriving in bursts or a single contributor dominates output
- a high stability risk score points to regression-like activity or repeated conflict pressure

## 4. Time Machine

### 4.1 Purpose

Time Machine is a frame-by-frame historical inspector. It lets the user move through commit history and inspect the repository exactly as it looked at a selected commit.

### 4.2 Core workflow

The panel loads a full commit history and exposes a scrubber for selecting a frame. From there the user can:

- move backward and forward one commit at a time
- enable autoplay to step through history automatically
- reverse autoplay direction
- search commits by SHA
- inspect changed files for the selected commit
- browse the repository tree snapshot for that commit
- preview the content of a file at that historical point in time
- copy a rollback command for the selected commit or selected file

### 4.3 Snapshot explorer

The snapshot explorer is the key part of the panel. For every selected commit, the panel requests:

- changed files in that commit
- repository tree paths for that commit
- file content for the currently selected snapshot file

The explorer is split into two panes:

- a directory tree that mirrors the repository state at the selected commit
- a file preview pane that renders the chosen snapshot content as plain text

Directory navigation is breadcrumb-based and the view keeps the selected directory/file if the same path still exists in the next frame.

### 4.4 Autoplay behavior

Autoplay is intentionally throttled. When the user steps through frames quickly, the panel schedules snapshot refreshes with a small delay so the preview does not flicker between commits.

The view also caches commit files, commit tree paths, and file contents by repository and commit SHA. That keeps repeated frame navigation fast after the data has been loaded once.

### 4.5 SHA search

SHA search matches both full and short hashes. The panel keeps match statistics so the user can jump between hits and see:

- total matches
- distinct authors
- oldest match timestamp
- newest match timestamp

### 4.6 Rollback guidance

The panel generates a rollback command from the selected commit and optional file path:

- whole commit rollback uses a checkout command for the selected frame
- file-scoped rollback adds the selected file path to the command

The command is exposed as copyable text so the user can paste it into a terminal if they want to reproduce the historical state manually.

### 4.7 Practical interpretation

Time Machine is best used for:

- forensic debugging of how a bug entered the codebase
- inspecting legacy implementation choices without switching branches
- reproducing older snapshots of a file quickly
- verifying how a specific file evolved over time

## 5. Usual Conflict Suspects

### 5.1 Purpose

Usual Conflict Suspects is the conflict-risk analysis panel. It highlights files and file pairs that repeatedly collide during merge windows, then rolls that data up into a repository tree heatmap.

### 5.2 Data sources

The panel loads three datasets:

- conflict hotspots for individual file risk
- conflict pairs for cross-file collision pressure
- repository tree paths for hierarchical roll-up

The merge window can be switched between 300, 500, 1000, and all history. Changing the window reloads the hotspot and pair analysis.

### 5.3 Main diagnostics

The panel calculates:

- total hotspot count
- high-risk file count
- total merge touches
- total conflict mentions
- total collision index
- average collision per file
- conflict mentions per touch
- risky pair count
- top pair coupling ratio
- dominant risk domain based on repository root grouping

It also provides a short reason for the top hotspot by comparing the file against the panel-wide averages.

### 5.4 Search and tree scoping

The user can search hotspot paths directly. The repository tree adds another filter layer: clicking a tree node scopes the hotspot list to that directory subtree.

The tree is not a flat file list. It is built from repository paths, then each file node receives hotspot risk and collision index data. Directory nodes inherit the highest child risk and the sum of child collision pressure, which makes the heatmap roll-up work naturally at folder level.

### 5.5 Risk visualization

The panel uses two visual surfaces:

- a suspect-file list with risk bands and score bars
- a repository tree heatmap with expandable folders

Files with higher risk get stronger color and a stronger badge. Directories surface child risk so the user can see where conflict pressure is concentrated even when the exact file is nested deeply.

### 5.6 Coupled risk pairs

The pair section shows file pairs that repeatedly collide in the same merge window. This is useful when two files are individually moderate-risk but become problematic when changed together.

### 5.7 Practical interpretation

Use this panel when you want to plan merges or refactors. It is especially useful for spotting:

- files that need ownership coordination
- directories with repeated merge pressure
- pairs of files that should not be edited together without review

## 6. How the Three Panels Fit Together

The three panels are intentionally complementary:

- Productivity Arena answers “how is the repository moving?”
- Time Machine answers “what did the repository look like at a specific point in history?”
- Usual Conflict Suspects answers “where are merges likely to hurt us?”

Together they form the commit-intelligence layer of GitSwamp.

## 7. Implementation Notes

### 7.1 Vue state model

Each panel keeps its own reactive state and uses run tokens to prevent stale async responses from replacing current data. This is important because users can switch repositories or filters while a load is still in flight.

### 7.2 Cache model

The panel caches are keyed by repository and the relevant scope:

- Productivity Arena: repository + active commit window
- Time Machine: repository + commit SHA (+ file path for file preview)
- Conflict Suspects: repository + merge window

This avoids repeating expensive scans when the user revisits the same data.

### 7.3 UX behavior

All three panels use loader overlays rather than full-screen blocking states. The UI keeps the structure visible so the user understands what is loading and what is already available.

### 7.4 Keyboard access

The panels are reachable through the View menu and the following shortcuts:

- Alt+2 - Productivity Arena
- Alt+3 - Time Machine
- Alt+4 - Usual Conflict Suspects

## 8. Related Files

- [RepositoryWorkspace.vue](../software/gitswamp/src/view/repository/RepositoryWorkspace.vue)
- [RepositoryTabs.vue](../software/gitswamp/src/view/repository/RepositoryTabs.vue)
- [CommitProductivityPanel.vue](../software/gitswamp/src/view/commit/CommitProductivityPanel.vue)
- [CommitTimeMachinePanel.vue](../software/gitswamp/src/view/commit/CommitTimeMachinePanel.vue)
- [CommitConflictHeatmapPanel.vue](../software/gitswamp/src/view/commit/CommitConflictHeatmapPanel.vue)
- [Backend overview](DOCUMENTATION_08_BACKEND_OVERVIEW.md)
- [Frontend overview](DOCUMENTATION_04_FRONTEND_OVERVIEW.md)
- [Core features](DOCUMENTATION_11_CORE_FEATURES.md)