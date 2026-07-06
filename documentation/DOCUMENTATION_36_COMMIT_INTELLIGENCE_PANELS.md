# Commit Intelligence Panels

## 1. Scope

GitSwamp originally exposed three history-focused panels from the View menu and repository menu:

- **Productivity Arena** - Alt+3
- **Time Machine** - Alt+4
- **Usual Conflict Suspects** - Alt+5

They are implemented as view modes inside the repository workspace, not as floating dialogs. The active panel is selected through `historyViewMode` in `RepositoryWorkspace.vue`, and each panel loads its own data independently.

This document explains how each panel works, which backend commands it uses, how state is cached, and what the user can do inside each view.

The current app also includes two adjacent history-intelligence modes in the same workspace:

- **Galaxy View** - Alt+2, canvas-based commit/branch topology with galaxy and tree layouts
- **Burndown Analytics** - Alt+6, team focus and after-hours analytics that complement Productivity Arena

## 2. Shared Architecture

The commit-intelligence panels follow the same high-level pattern:

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

### 2.2 Code ownership map

This ownership map describes where code should live. It is separate from Git author ownership shown in analytics panels.

| Area | Owner module | Responsibility |
|------|--------------|----------------|
| Workspace routing | `RepositoryWorkspace.vue` | Owns history view mode selection, lazy-loaded panel wiring, and shared repository props |
| View menu and shortcuts | `RepositoryTabs.vue` | Owns Alt shortcuts, View menu entries, and tab-level actions |
| Productivity analytics UI | `CommitProductivityPanel.vue` | Owns rhythm, contribution balance, pressure, stability, and author-filtered metrics |
| Time Machine UI | `CommitTimeMachinePanel.vue` | Owns timeline frames, autoplay, SHA search, snapshot explorer, and rollback command preview |
| Conflict suspects UI | `CommitConflictHeatmapPanel.vue` | Owns hotspot rendering, pair rendering, tree heatmap roll-up, search, filters, and risk labels |
| Burndown and code ownership UI | `CommitBurndownAnalyticsPanel.vue` | Owns after-hours analytics, hot-file ownership, contributor risk rows, and team pulse charts |
| Tauri command boundary | `src-tauri/src/commands/conflicts.rs`, `commits.rs`, `commit_files.rs`, `diff.rs` | Owns async command wrappers, thread offloading, and payload return boundaries |
| Git analytics engine | `src-tauri/src/services/git_service.rs` | Owns canonical Git scans, conflict scoring, merge preflight scoring, deletion statistics, and tree reads |
| Payload contracts | `src-tauri/src/models/conflict_hotspot.rs`, `src/types/models/conflictHotspot.ts`, `src/types/models/conflictAnalytics.ts` | Owns shared data shape between Rust and Vue |

The rule is simple: Rust owns expensive Git facts, Vue owns presentation, normalization, cache keys, and user-facing interpretation. If a calculation needs raw repository traversal, it belongs in `GitService`. If a calculation only turns loaded facts into a score, label, color, or chart, it can stay in the panel that renders it.

### 2.3 Shared algorithm principles

All commit-intelligence views use the same performance principles:

- Load a bounded preview first, then let the user request deeper history.
- Keep old successful data visible while a newer request is loading.
- Use run tokens so stale async responses cannot overwrite the current repository.
- Cache by stable scope, usually repository, repository plus window, repository plus SHA, or repository plus SHA plus file path.
- Split expensive work into independent streams so one slow source does not block the whole view.
- Keep backend payloads factual and small, then derive UI labels and percentages in the frontend.
- Treat analytics as guidance, not as absolute truth, because Git history can be squashed, rebased, renamed, or imported from another system.

### 2.4 User-facing interpretation layer

The panels intentionally avoid raw-only numbers. Every score should answer one user question:

| Signal | User question |
|--------|---------------|
| Health score | Is the repository moving at a stable pace? |
| Balance score | Is work distributed across contributors? |
| Bottleneck score | Is too much work concentrated in bursts or one contributor? |
| Stability risk | Are recent commits showing bug-fix pressure or repeated conflict pressure? |
| Hotspot score | Which files most often show up in merge-risk situations? |
| Pair score | Which file pairs become risky when touched together? |
| Time Machine frame | What did the repository look like at this exact historical point? |
| Hot-file owner | Who currently carries most of the touches on a frequently changed file? |

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

### 3.7 Productivity algorithm

Productivity Arena starts from commit history and turns it into rhythm, ownership, and stability signals.

1. **Choose history depth.** Preview mode uses a small window for fast first render. Full mode expands to the larger history limit used by the panel.
2. **Normalize commits.** Each commit is mapped to a day key, hour, weekday, author name, timestamp, merge flag, and message text.
3. **Apply author filter.** When the user selects an author, every metric recalculates from only that author's commits.
4. **Build rhythm metrics.** The panel counts active days, current streak, longest streak, average commits per active day, peak hour, and top weekday.
5. **Build contribution metrics.** It counts commits per author, unique authors, top contributor share, and a normalized balance score.
6. **Build pressure metrics.** It calculates weekend ratio, off-hours ratio, top-three-day load share, and throughput volatility.
7. **Build stability metrics.** It looks for regression-like commit messages and combines that with conflict hotspot pressure.
8. **Render derived labels.** Scores are converted into low, moderate, high, or critical labels so the user does not have to interpret raw percentages.

The current streak is calculated from the newest active day backward until a day has no commits. The longest streak is calculated by sorting unique active days and finding the longest consecutive sequence.

### 3.8 Productivity formulas and ownership signals

The panel uses these practical formulas:

| Metric | Formula |
|--------|---------|
| Top contributor share | `topAuthorCommitCount / totalCommits * 100` |
| Balance score | normalized Shannon entropy of commit distribution, scaled to 0-100 |
| Arena health | `streakPart + mergePart + activityPart`, capped at 100 |
| Collaboration intensity | author count, merge ratio, and average commits per active day, capped at 100 |
| Bottleneck score | burst concentration, top contributor share, off-hours ratio, and daily volatility, capped at 100 |
| Stability risk | regression-message rate, conflict mention density, and bottleneck score, capped at 100 |
| Recovery pressure | weighted mix of regression-message rate and conflict mention density |
| Context-switch pressure | weighted mix of off-hours ratio and top-three-day load share |

The ownership signal in Productivity Arena is repository-wide. A low balance score or high top contributor share means one person is carrying a large part of the commit stream. That is useful for planning reviews, pairing, and vacation risk, but it does not mean that person owns every file. File-level ownership is handled by Burndown Analytics.

The score bands are intentionally coarse:

- 75 and above is critical
- 55 to 74 is high
- 30 to 54 is moderate
- below 30 is low

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

### 4.8 Time Machine algorithm

Time Machine treats history as a sequence of frames.

1. **Load commit frames.** The panel loads commit history for the active repository and stores it as an ordered timeline.
2. **Select a frame.** The selected index points at exactly one commit SHA.
3. **Load snapshot facts.** For the selected SHA, the panel requests changed files and repository tree paths in parallel.
4. **Build explorer rows.** Tree paths are split by directory, merged into a breadcrumb-style explorer, sorted with folders before files.
5. **Choose preview file.** The panel prefers the already selected file if it still exists, otherwise it falls back to the selected changed file or the first available path.
6. **Load file content.** File preview calls `get_file_content` with repository path, file path, and selected SHA.
7. **Generate rollback command.** The selected SHA and optional file path are turned into a copyable checkout command.

The panel uses three cache scopes:

| Cache | Key |
|-------|-----|
| Commit files | repository plus commit SHA |
| Commit tree paths | repository plus commit SHA |
| File preview content | repository plus commit SHA plus file path |

File contents are cached only when they are small enough to stay safe for memory. Large previews can still render, but they should not permanently inflate the cache.

### 4.9 Autoplay, search, and stale-load safety

Autoplay is designed to feel smooth without overwhelming the backend:

- Manual frame changes schedule snapshot refresh quickly.
- Autoplay frame changes use a slightly longer delay to prevent preview flicker.
- A run token invalidates older snapshot requests when the user scrubs quickly.
- The explorer keeps scroll and directory context when possible.

SHA search matches short and full hashes. The panel also reports match count, distinct authors, oldest match, and newest match, so search feels like navigation rather than a raw text filter.

Rollback guidance is intentionally explicit:

- commit-level rollback uses `git checkout <sha>`
- file-level rollback uses `git checkout <sha> -- <path>`

The panel only prepares the command. It does not execute rollback automatically.

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

### 5.8 Hotspot algorithm

Conflict hotspots are calculated from merge commits, not from every regular commit.

1. The backend chooses a scan limit from the selected merge window.
2. It loads enough recent commits to find merge commits inside that window.
3. It skips commits outside the optional lookback period.
4. For each merge commit, it loads changed files.
5. Each unique file touched in that merge gets one merge touch and one score point.
6. If the merge commit message contains `conflict`, the file gets two extra score points and one conflict mention.
7. The final `collision_index` is the number of conflict mentions.
8. Results are sorted by score, merge touches, conflict mentions, and path.

This makes the hotspot score intuitive: a file rises when it repeatedly appears in merge commits, and it rises faster when those merge commits explicitly mention conflicts.

### 5.9 Pair algorithm

Conflict pairs answer a different question: which files become risky together?

1. The backend uses the same merge-commit window and lookback period.
2. It collects unique changed paths for each merge commit.
3. Very large merge commits are capped to keep pair generation bounded.
4. Every pair of changed files receives one co-touch and one score point.
5. If the merge message mentions `conflict`, the pair receives two extra score points and one conflict touch.
6. Pairs are sorted by score, conflict touches, co-touches, and path.
7. The returned list is capped so the UI stays responsive.

The top pair coupling ratio is `conflictTouches / coTouches`. A high ratio means the pair is not merely changed together often, it is changed together in merges that often mention conflict.

### 5.10 Tree roll-up and preflight risk

The repository tree heatmap is built in two passes:

1. The backend scans repository paths and skips heavy/generated folders such as `.git`, `node_modules`, `target`, `dist`, `build`, and common IDE folders.
2. The frontend builds a tree skeleton from those paths.
3. File nodes receive their own hotspot score and collision index.
4. Directory nodes inherit the highest child risk score and sum child collision pressure.
5. Directories are sorted before files, then by risk, then by name.

Risk color is normalized inside the current result set. The list uses stronger bands than the tree because file rows show direct suspects, while folders summarize children:

| Surface | Critical | High | Moderate |
|---------|----------|------|----------|
| Hotspot list | 82% of max score | 64% of max score | 36% of max score |
| Tree heatmap | 62% of max score | 38% of max score | 16% of max score |

Merge preflight uses real branch overlap:

1. Resolve source and target branch commits.
2. Compute the merge base.
3. Diff merge base to source and merge base to target.
4. Intersect changed paths from both sides.
5. Keep shared paths that are also known hotspots.
6. Score risk as hotspot score sum, plus shared path count, plus overlap density bonus.
7. Convert the score into low, moderate, high, or critical.

Preflight risk levels are:

- 170 and above is critical
- 95 to 169 is high
- 45 to 94 is moderate
- below 45 is low

The conflict panel is a risk map, not a guarantee. It does not replay every historical textual conflict. It uses merge history, changed files, conflict mentions, and branch overlap to show where attention is most likely needed.

## 6. Burndown Analytics and Code Ownership

### 6.1 Purpose

Burndown Analytics complements Productivity Arena. Productivity focuses on repository rhythm and balance, while Burndown focuses on team focus, after-hours load, and hot-file ownership.

### 6.2 Data loading strategy

The panel loads two independent streams:

- full repository history, capped by the panel history limit
- hot-file scan from recent non-merge commits

The hot-file scan uses `git log --all --no-merges --name-only`, groups paths by touches, and keeps only files touched at least three times. The scan is capped so it stays fast and does not turn the panel into a full blame replacement.

### 6.3 Code ownership algorithm

Code ownership in Burndown is file-level and touch-based:

1. For every scanned non-merge commit, the parser stores current author and whether the subject looks fix-like.
2. Each changed file receives one touch.
3. If the subject contains words such as `fix`, `bug`, `hotfix`, `regression`, `revert`, `crash`, `broken`, `repair`, or `patch`, the file receives one fix touch.
4. The panel counts touches per author for each file.
5. The owner is the author with the most touches on that file.
6. Owner share is `ownerTouches / fileTouches * 100`.
7. Files are ranked by `touches + fixTouches * 2`, so repeatedly changed files with fix pressure rise first.

This is intentionally lightweight. It does not claim permanent ownership. It answers a practical question: who currently has the most recent working context for a hot file?

### 6.4 Burnout and focus algorithm

For each author, the panel calculates:

- commits
- active days
- first and last commit timestamps
- after-hours commits, before 07:00 or at/after 20:00
- late-night commits, before 05:00
- weekend commits
- Saturday-night commits
- recent weekly commit counts
- recent weekly after-hours counts
- peak after-hours hour and weekday
- hot files owned and hot-file touches

Author risk score is capped at 100 and combines:

| Signal | Weight |
|--------|--------|
| After-hours share | 30% of the percentage |
| Weekend share | 24% of the percentage |
| Late-night share | 28% of the percentage |
| Consecutive recent late-night weeks | 8 points per week, capped at 24 |
| Hot files owned | 3 points per file, capped at 18 |

Risk labels are:

- 65 and above is high
- 35 to 64 is watch
- below 35 is steady

The panel yields back to the UI while building large author stats, so scrolling and tab switching stay responsive during long scans.

### 6.5 How to read ownership safely

Ownership should be used as a coordination signal, not as blame. A high owner share can mean:

- one person has useful context for review
- a file needs pairing or documentation
- a hot file may need refactoring
- a team may need to spread knowledge before a release

It should not be used to judge quality by itself. Pair ownership with Productivity balance, Conflict hotspots, and Time Machine history before making process decisions.

## 7. How the Panels Fit Together

The panels are intentionally complementary:

- Productivity Arena answers “how is the repository moving?”
- Time Machine answers “what did the repository look like at a specific point in history?”
- Usual Conflict Suspects answers “where are merges likely to hurt us?”

Two newer modes extend this layer:

- Galaxy View answers “how do branches and commits visually relate?”
- Burndown Analytics answers “where are team focus, after-hours work, and workload pressure changing?”

Together they form the commit-intelligence layer of GitSwamp.

### 7.1 Useful reading recipes

Use these combinations when diagnosing a repository:

| Situation | Read these signals |
|-----------|--------------------|
| Preparing a risky merge | Conflict hotspots, conflict pairs, merge preflight, then Time Machine for suspect files |
| Planning a refactor | Conflict tree heatmap, hot-file ownership, top contributor share, and Time Machine snapshots |
| Investigating a regression | Productivity stability risk, Time Machine frame search, then file-level snapshot preview |
| Checking team sustainability | Burndown after-hours risk, Productivity bottleneck score, ownership concentration |
| Finding knowledge silos | Productivity balance score, Burndown owner share, hot files owned |

## 8. Implementation Notes

### 8.1 Vue state model

Each panel keeps its own reactive state and uses run tokens to prevent stale async responses from replacing current data. This is important because users can switch repositories or filters while a load is still in flight.

Galaxy View and Burndown Analytics follow the same repository workspace mode model. `RepositoryWorkspace.vue` lazy-loads their panel components, and `RepositoryTabs.vue` exposes their shortcuts from the View menu.

### 8.2 Cache model

The panel caches are keyed by repository and the relevant scope:

- Productivity Arena: repository + active commit window
- Time Machine: repository + commit SHA (+ file path for file preview)
- Conflict Suspects: repository + merge window
- Galaxy View: loaded commit/branch state and canvas-local layout state
- Burndown Analytics: repository + recent analytics window

This avoids repeating expensive scans when the user revisits the same data.

### 8.3 UX behavior

All commit-intelligence panels use loader overlays rather than full-screen blocking states. The UI keeps the structure visible so the user understands what is loading and what is already available.

### 8.4 Keyboard access

The panels are reachable through the View menu and the following shortcuts:

- Alt+2 - Galaxy View
- Alt+3 - Productivity Arena
- Alt+4 - Time Machine
- Alt+5 - Usual Conflict Suspects
- Alt+6 - Burndown Analytics

## 9. Related Files

- [RepositoryWorkspace.vue](../software/gitswamp/src/view/repository/RepositoryWorkspace.vue)
- [RepositoryTabs.vue](../software/gitswamp/src/view/repository/RepositoryTabs.vue)
- [CommitGalaxyPanel.vue](../software/gitswamp/src/view/commit/CommitGalaxyPanel.vue)
- [CommitProductivityPanel.vue](../software/gitswamp/src/view/commit/CommitProductivityPanel.vue)
- [CommitTimeMachinePanel.vue](../software/gitswamp/src/view/commit/CommitTimeMachinePanel.vue)
- [CommitConflictHeatmapPanel.vue](../software/gitswamp/src/view/commit/CommitConflictHeatmapPanel.vue)
- [CommitBurndownAnalyticsPanel.vue](../software/gitswamp/src/view/commit/CommitBurndownAnalyticsPanel.vue)
- [GitService](../software/gitswamp/src-tauri/src/services/git_service.rs)
- [Conflict commands](../software/gitswamp/src-tauri/src/commands/conflicts.rs)
- [Commit commands](../software/gitswamp/src-tauri/src/commands/commits.rs)
- [Conflict hotspot model](../software/gitswamp/src-tauri/src/models/conflict_hotspot.rs)
- [Backend overview](DOCUMENTATION_08_BACKEND_OVERVIEW.md)
- [Frontend overview](DOCUMENTATION_04_FRONTEND_OVERVIEW.md)
- [Core features](DOCUMENTATION_11_CORE_FEATURES.md)
