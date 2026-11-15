# Current Workflows and Troubleshooting

This guide supplements the [user guide](DOCUMENTATION_31_USER_GUIDE.md). Android instructions are in the [mobile guide](../software/gitswamp-mobile/README.md).

## Amend the Last Commit

Stage the files that should be included in the last commit. Right-click the current branch's last commit or Working Changes in the graph, then choose **Amend commit**. Review the staged files, message and available options before confirming.

Amend can change commit content, not just its message. It creates a new commit SHA. Coordinate with collaborators before rewriting already published history.

For a repository initialized outside GitSwamp, stage your files and commit as usual. The first commit creates the branch configured by Git, including `main` or `master`, without requiring a separate README commit. Initialize a Repository uses the last clone location, or a folder under your home directory when no clone location is saved.

## Secret Review and Large Files

**Accidental Secret Detector** is off by default. Enable it in Options to scan locally and mark possible credentials in Working Changes without notifications. It checks text content regardless of extension, including UTF-16 text with a byte-order mark. Empty values, environment-variable references and common placeholder values are not treated as embedded credentials. An `.example` filename is not an exemption when it contains an actual value.

Before commit or amend, GitSwamp reviews the staged index, not the current working-file contents. A compact confirmation lists file paths, line numbers and finding types, never the matched secret values. Cancel preserves the commit message. **Add to .gitignore** only unstages newly added flagged files and appends literal paths to the root `.gitignore`; it keeps local files and never silently untracks existing files or stages `.gitignore`. Edit credentials out of tracked files instead. Confirmation is invalidated when the staged tree or HEAD changes.

Staged files larger than **100 MiB** block commits in GitSwamp even when secret detection is disabled. The check reads Git object sizes without loading large blobs. Open **Git LFS Manager** from the warning or **Menu > View**, then use **Convert & Stage** for the selected files. This uses installed Git LFS to convert the staged contents into pointers and stage the corresponding `.gitattributes` rules without replacing your working files. The manager also lists LFS files and tracking rules and can upload or download LFS objects. It does not commit, push Git refs or rewrite history automatically. Oversized blobs already in older commits may still need migration before a push can succeed.

Secret scanning is heuristic, not a guarantee that every credential will be found. It inspects at most 1 MiB per text file and 16 MiB per scan, up to 2048 files with a cooperative five-second budget. Unchecked content is reported before committing instead of being declared clean. Binary content is not decoded as text. Background checks are debounced and inspect only the staged and unstaged files in Working Changes. Hover or focus a warning triangle to see the type and line number; the matched value is never displayed. Very large index metadata can be refused for a bounded review.

## Branching Workflow

Open **Branch > Branching workflow**, right-click the commit graph, or press **Ctrl+Alt+B** (**Command+Option+B** on macOS). The same dialog opens whichever entry you use. The repository initializer has an optional strategy step. After cloning, GitSwamp can suggest a strategy based on the fetched refs; a shallow history alone is not treated as a new repository. Neither suggestion nor saving a strategy creates or rewrites branches.

- **GitSwamp Handoff:** Work on a short change or repair branch. The review shows whether the local source matches the fetched source, which target was compared, and paths flagged by the merge preview. It also shows recent commits, test paths touched and review areas such as migrations or CI. Use **Fetch & recheck** to refresh remote refs, or **Recheck** to review the refs already available locally. For supported GitHub remotes, the existing pull request form opens only after the reviewed refs are rechecked. Review notes can be copied or included in the form. Handoff does not push, merge, run tests or replace CI.
- **GitHub Flow:** Create feature and fix branches, hand them off for pull request review, and optionally clean up a fully integrated local branch.
- **Trunk-Based:** Use one trunk with optional short-lived feature and bugfix branches. Local integration is fast-forward only; release branches and tags are separate actions.
- **Gitflow:** Start features from development, releases from development, and hotfixes from production. Finish a release or hotfix into both configured branches before optionally tagging production. A maintenance branch can start from a prior release tag and remains separate.
- **Manual Git:** Disable the strategy without changing the repository.

Every local operation has a preview and requires confirmation. Mutations require a clean working tree and verify the relevant refs again before execution. GitSwamp does not automatically stash, push, overwrite a tag, or force-delete a branch. Cleanup requires that all target branches contain the source tip; fetched remote counterparts are retained. Squash and rebase merges may not establish ancestry, so the workflow does not delete those branches automatically.

If a multi-step merge stops, completed steps are reported and remaining steps are not run. Resolve or abort the pending merge with the normal GitSwamp controls, then reopen the workflow, select the source branch and preview again. Steps already integrated are skipped. Preferences use the existing app storage and do not write to Git configuration. If that storage is unavailable, the dialog reports that choices last only for the current session.

## Compose Commits

From **Working Changes**, open **Visual Commit Builder** (the hammer) and choose **Compose commits automatically** in its header. The experimental dialog proposes an ordered series from the current changes without an AI service. Edit each subject and description, reorder the commits, or move related change sets between commits. The current Commit Analyzer settings are used to validate the messages.

In the composer, the hammer beside the selected commit opens structured controls for its type, scope, summary and breaking-change marker. **Commit tag** adds a tag such as `#123` or `#release` to that message. Use **Tag all commits > Apply to all** to set or clear the same tag in every draft, then override individual drafts as needed. These are message labels, not Git tag references.

**Close issue** appears for a numeric tag such as `#123`, in both the composer and the Visual Commit Builder. When selected it puts `Closes #123` on a separate line at the end of the description, not in the subject. GitSwamp does not call the issue tracker; GitHub closes the referenced issue when the commit reaches the default branch.

The composer uses filenames, project layout and changed lines. It inspects one diff at a time, without sharing a single analysis budget across all files. It keeps exact renames, related source and tests, manifests and lockfiles, and other detected dependencies together. Fully unstaged files can be included; if a file has both staged and unstaged edits, only its staged version is committed. Stage files with a custom Git clean filter (including LFS) manually before composing. The normal secret-review preference and 100 MiB file limit still apply.

Drafts also group matching implementation layers and their tests, stories, styles, translations or feature documentation. The composer respects project boundaries and leaves ambiguous matches separate. It can propose a migration alongside the code that references its exact table when there is one clear match. In the preview, the newest commit is at the top and commits are created from bottom to top. Select a file to see its inline diff, or use its move menu to assign it to another commit. Long file lists are paged without excluding any changes. Only related hunks, overlapping paths and submodule metadata that must move together are kept linked. Unrelated files in the same directory can remain separate, up to 40 proposed commits.

Generated messages keep a short `Type(scope): Summary` subject. The description explains the main change, then lists what each file adds, updates, removes or renames with the available evidence. For very large drafts the description notes when the file list is shortened; the full change list stays in the preview.

When the diff provides enough evidence, drafts describe added, changed or removed declarations, dependency names, SQL objects, configuration keys, CSS selectors, CI jobs and documentation headings. They do not copy configuration values, dependency versions or route URLs. For unknown syntax, binary files or diffs outside the analysis budget, drafts fall back to paths and the Git changes the composer can verify. This is a local heuristic, not a language parser or a build check.

For supported source languages, the preview also compares changed lines within their enclosing function or method. It can describe changes to validation, caching, rate limiting, telemetry, background jobs, redaction and other implementation details before the file summary. If a function body changes without a stronger signal, a draft can still name the affected function. Separate rules recognize named GraphQL operations and fields, Protobuf endpoints, SQL columns and constraints, HDL signals, PowerShell and Tcl commands, CMake targets and Bicep resources. Signals come from changed code and declarations; the composer does not copy arguments or configuration values into the message. Independent changes in the same folder can be proposed as separate commits. The deeper analysis runs only while building the preview and uses the existing diff-size limits.

Preview uses a temporary index and Git objects and does not change the working files, live index or branch. Before creating commits, GitSwamp checks that the reviewed changes still match and verifies that the final tree matches the plan. The composer creates only local commits and does not run builds or tests by default. It supports up to 2,000 changed paths per plan. Text file versions above 2 MiB use path-based suggestions and retain their complete Git content.

In supported source files, separate Git hunks in identifiable functions can be assigned to different commits. The change list shows the function and line range. New or deleted files, binary files, changes that overlap, and hunks whose relationship cannot be separated safely stay together. The composer checks supported local imports and includes on intermediate Git trees, including simple JavaScript and TypeScript named exports. It groups dependent steps or refuses an unsafe manual order. When a provider is removed or renamed, the composer reviews relevant callers even when those callers did not change. Its bounded caches can discard previous analysis without stopping the review. Static checks cannot resolve every dynamic import, build flag or custom module resolver. If a source file exceeds the static analysis limits, the preview preserves separate groups and requires **Verify every commit** to create a series of multiple commits.

**Verify every commit** is optional. Enter a trusted build or compile command for the project (for example, `cargo check --locked`), then confirm the series. The composer runs that command against each staged state separately in a fresh temporary source directory. Any failure stops the series before updating the branch or index. The directory is not a security sandbox and does not include ignored files, submodules or Git metadata. Use a command that has access to its required dependencies. A successful command verifies only that command and configuration; it does not prove that every target or test will succeed. The composer bounds the input size and build time, and keeps only a short output tail.

With verification enabled, **Find buildable groups** tests the proposed prefixes in order. If a prefix fails, it adds the next group and retries rather than publishing a broken intermediate commit. Passing groups stay separate, and a failed final tree cannot be fixed by regrouping. Review the new subjects, descriptions and issue tags before confirming, or use **Undo regroup** to restore the previous draft. Only confirmation creates commits, and it verifies the final edited plan again.

**Branch Workspace** (`Alt+Shift+1`) and **Git LFS Manager** (`Alt+Shift+2`) are the final two entries under **Menu > View**.

## Visualise Git Operations

**Options > Advanced > Background Maintenance > Visualise operations** is off by default. When enabled, it previews newly detected conflicts over the current graph. The current and incoming code excerpts slide over each other, and lines inside Git's conflict markers turn red. Binary or delete conflicts may not have text ranges. Each file stays visible for three seconds, after which the preview advances or closes. **X** or Escape ends the preview immediately. It never resolves conflicts or changes the index.

Only the displayed file's excerpts are kept in memory. The preview reads at most 256 KiB of working-tree text and shows up to seven lines per side from the first conflict region. When text markers are missing, small staged text versions can still be shown without invented conflict lines. The app does not show the preview just because a repository with existing conflicts was opened.

With the same option enabled, a successful merge shows the incoming history joining the target branch and highlights the new merge commit. The preview lists up to six commits that were not already reachable from the target. A merge that changes nothing does not open it. Cherry-pick and revert show the selected commit and the new commit created on the target; the original commit remains in history.

Local branch deletion shows its reference dropping into the bin. Creating a branch highlights its starting commit, and renaming a branch changes its label without showing any change to commit history. Reset to Commit animates the move from the previous tip to the selected SHA and shows whether it was a soft, mixed or hard reset. A reset that does not move HEAD does not show a rewind. Existing confirmation and Undo steps are unchanged.

Push shows the branch moving toward the configured provider (or a generic remote icon). Pull and manual Fetch animate in the other direction; Fetch updates remote-tracking refs, not the working tree. Background fetch does not open a preview. Stash save, apply and pop show the movement between Working Changes and the local stash.

Success previews close within seven seconds, or immediately with **X** or Escape. The graph and toast actions remain available while a preview is open. If Undo is enabled, the preview for a pending local branch deletion or Reset to Commit also offers **Undo** during the same five-second window as the toast. Undo closes the preview and cancels the scheduled action before it runs; closing the preview alone leaves the toast and its countdown active. Undo does not reverse a Git operation that has already finished. The previews respect the system's reduced-motion setting. When the option is off, GitSwamp does not request commit summaries or load the animation panel.

## Commit Galaxy

Galaxy uses a Three.js spiral scene with a bright center and four arms. Older loaded commits are nearer the center and newer commits farther out; parent-first ordering preserves ancestry even when commit clocks disagree. Branch ancestry determines stable lanes and colors, parent lines show actual Git edges and merge commits can be highlighted. The stars behind the commit nodes are visual context, not additional commits.

The layout selector retains spiral, branch layers, constellation and tree views. Drag rotates the galaxy, the wheel zooms, and tree mode pans. Select a commit to inspect it; Reset and Focus restore framing or move to the selected commit. Circling smoothly orbits the camera, pauses during hover and dragging, and resumes after interaction. It stops for hidden tabs, reduced-motion preferences and tree view. Static modes render on demand. Geometry is bounded to 50,000 loaded commits and 100,000 parent edges, pixel ratio is capped at 1.5, and closing the view releases WebGL resources.

## Branches, Tags and Remote Checkout

Branches and tags with identical names are separate Git references. Use the branch entry to switch branches; selecting a tag is not equivalent to creating or tracking a branch.

Right-click a branch in the sidebar or its badge on the graph for branch actions. **Create branch here** starts from that branch's tip without first checking it out. **Rename branch** is available for local branches. Sidebar menus also open with **Shift+F10** on a focused branch row.

Checking out a remote-only branch creates or uses the corresponding local tracking branch. If a local branch contains commits not present in that remote reference, GitSwamp does not silently reset it to the older remote state. Review the divergence before choosing the next Git operation.

## Branch Workspace

Open **View > Branch Workspace** or right-click a branch and choose **Open Branch Workspace**. Add up to four branch columns with the picker on the left. The active branch is marked at the top; the tab strip focuses a column without checking anything out. Each column has its own history, Working Changes and **Commit to [branch]** action. Expand the compact changes panel when you need more room. The toolbar opens the active worktree's folder or its full repository tab.

Use **Open with** in a branch column header to open that worktree in the file manager or a detected IDE. **Choose application...** opens the system picker for other editors.

GitSwamp reuses an existing checkout for a branch. Otherwise it creates a linked worktree in `.gitswamp-worktrees` beside the main repository. A remote-only branch gets a local tracking branch; a same-named local branch with a different upstream is never reset or reassigned automatically. Worktrees share Git objects and branch references but have separate working directories, indexes and HEADs.

**Branch Workspace** stays inside the current repository tab. Use **Open worktree in repository tab** to open a linked worktree in a separate tab. Closing the Branch Workspace view returns to the original graph and leaves all worktree folders intact. Commits from linked worktrees appear under their branches in the refreshed original graph; working files stay in their own directories. History loads in pages, and workspace polling stops when the view closes.

Closing a linked branch column or its full worktree tab offers **Remove worktree** and **Close without removing**. Removal requires confirmation, protects the original worktree, and runs Git without force. Worktrees with staged, unstaged, untracked or ignored files, or an active Git operation, are not removed. Removing the worktree folder does not delete its branch or commits.

The active local branch cannot be deleted. Switch to another branch first. A refused local deletion must not become an accidental remote deletion.

## Pull with Local Changes

Review staged and unstaged changes before pulling. When the protected pull flow needs to move local changes aside, it uses a safety stash rather than intentionally discarding them. Conflicts during integration or restoration still require a decision.

If pull stops with a merge conflict, resolve and commit it or choose **Abort merge**. The abort flow attempts to restore the pre-pull safety stash when available. If restoration conflicts, preserve the stash and review the reported state. Do not remove a safety stash until the local work is verified.

## Complete or Abort an Operation

Git stores active-operation metadata separately from conflict markers. Resolving or discarding individual file content does not by itself finish a merge or cherry-pick.

- **Complete:** resolve the files, stage the result and commit.
- **Abort:** use **Abort merge** or **Abort cherry-pick** in the header or changes panel. Confirm and let the Undo countdown finish if enabled.
- **Resolve all:** choose a whole-file side for all conflicts. Manual resolution remains available per file, not as a bulk action.

Review the versions shown by Keep modified/Keep base before choosing. These side labels should not be assumed to mean local versus remote for every Git operation.

After a successful abort, the operation is cleared and the active-operation notice is removed. Startup reads the operation from Git rather than trusting a saved tab's old metadata.

## Undo and Safety Limits

When enabled, scheduled destructive UI actions wait five seconds. **Undo** cancels the pending action. It is not a recovery command for work already executed. Disabling the preference permits immediate execution after any applicable confirmation.

Pending actions are cancelled when a changed repository context or a conflicting operation is detected before execution. Closing the relevant view can cancel its pending actions too. Do not close the app during the countdown and assume an abort completed: wait for its completion message.

These checks do not replace backups or cover every external filesystem change. Keep important work committed or otherwise backed up.

## Interactive Rebase

Right-click a commit in the graph and select **Interactive Rebase from Here**. The selected commit stays unchanged; the dialog lists the commits after it up to HEAD, oldest first. Drag the grip or focus it and press Up or Down to reorder. Use the action dropdown to pick, reword, edit, squash, fix up or drop commits. Edit Reword messages in place and use the combined message field for a Squash group. Fixup does not add its own message.

To move local work onto newer upstream commits, fetch first, then right-click the `origin` branch badge or its commit and select **Interactive Rebase onto** that target. Only commits on your local side of the common ancestor are replayed. If you have no local commits to replay, the dialog offers **Fast-forward** instead. Neither action moves the remote branch or pushes changes.

Start from a clean working tree on a local branch. The visual editor supports linear ranges and does not flatten merges. **Edit** pauses after applying a commit; use the Changes panel to stage and amend, then **Continue rebase** in the toolbar. For a conflict, resolve and stage the files before continuing, or choose **Skip commit**. **Abort rebase** restores the original branch tip. Nothing is pushed automatically. The existing branch-to-branch rebase is unchanged.

## Edit Files and Inspect History

The commit **Info** tab lists valid `Co-authored-by` identities below the author and committer. Search by name or email as usual, or use `coauthor:NAME` to match only the co-author trailers.

In a text diff under **Working Changes**, hover an added or removed line and use **+** to stage it or **−** to unstage it. In split view, use the action on the side containing the change. You can also focus the line and press Enter. Stage, unstage and discard hunk controls stay visible when scrolling horizontally.

Open a working-file diff and select the pencil icon. Edit and choose **Save**. Selecting another file while the editor is open loads the newly selected file. Late responses for the previous file cannot overwrite the current editor.

Unsaved drafts are retained while the same diff viewer remains open. They are not an on-disk backup and are not guaranteed after closing the viewer or restarting. Save before leaving.

Editor and Time Machine syntax previews fall back to plain text above 60,000 characters, more than 1,500 lines, lines longer than 2,000 characters, binary-looking input or unsupported languages. Skipping coloring does not remove the text.

Time Machine reads a selected commit's snapshot without checking it out. Copying a rollback command does not run it; executing that command separately can modify the working tree.

## Compare Images and Binary Files

Open an image in a commit or Working Changes to see **Previous** and **Current** versions. For SVG files, choose the image icon in the diff toolbar. **Side by side** shows both images, **Slider** reveals either side with a draggable divider or keyboard-accessible range control, and **Difference** blends the images to highlight visual changes. Overlays use a common scale, including images with different dimensions.

Unstaged comparisons use the index as Previous and the working file as Current. Staged comparisons use HEAD and the index. Commit comparisons use the first parent and the selected commit. Added or deleted files show the available side only. Other binary files show sizes without loading their full contents into the frontend. **Reload preview** retries a failed preview or explicitly refreshes staged/commit metadata.

Preview reads are serialized and only the selected file's two sides are retained. Identical Git blobs share one URL. Changing files or closing the viewer releases preview URLs; previews are not kept in the diff cache. Working-file refresh checks metadata and skips image reads when unchanged.

Each image is limited to 10 MB, 8 million pixels and 8192 pixels per side. PNG, JPEG, static GIF/WebP, BMP, ICO and simple SVG previews are supported; browser decoding support still applies. Animated images, TIFF/AVIF, oversized images and complex SVG files show a message instead of starting an unrestricted decode. SVG has an additional 256 KB limit and rejects embedded/external content, filters, animation and styles. These bounds limit preview work; actual process RAM also depends on the WebView's decoder and graphics caches.

## Large Commits and Coloring

Selecting a large commit shows **Loading changed files...** while metadata is calculated in the background. File statistics are still computed, not replaced with guesses. The first read can take time due to disk speed, Git object storage and file size.

Large file and tree lists render only a window of rows with the normal scroll range and keyboard navigation. Smaller lists keep direct rendering. Recent commit metadata is cached within a memory bound. Duplicate requests for one repository/SHA share an in-flight request; multi-selection starts at most four concurrent metadata jobs from that selection flow.

Diff syntax coloring uses a Web Worker. Plain text appears first and colors arrive asynchronously in batches. Repeated lines reuse cached tokens. Addition/deletion markers and inline change markup remain separate from syntax coloring.

## Debian and Ubuntu Updates

1. Choose **Download .deb** in the update panel.
2. Choose **Open installer** and finish installation in the system package manager. If no installer is associated with Debian packages, open the saved package from Downloads with your package manager.
3. Return to GitSwamp and use **Check installation**, or close and reopen the app.
4. If the new package is installed but the old process is running, the panel offers **Restart** instead of another download.

The update panel shows the size of the Debian package for your architecture and reports progress from that package's download.

Debian updates do not use the AppImage installer. Other installation formats keep their native updater flow. Administrator authorization belongs to the system package manager; GitSwamp does not collect that password.

### Same Version Already Installed

This message is expected when the package's internal version matches the installed one. Changing the filename or rebuilding the same version does not make it an upgrade. It can also appear after successful installation if the old process has not been restarted.

Compare versions with the actual downloaded filename:

```bash
dpkg-deb --field ./GitSwamp_VERSION_amd64.deb Package Version Architecture
dpkg-query --show --showformat='${Version}\n' git-swamp
```

If versions match, restart instead of forcing a reinstall. If the offered version and the `.deb` metadata differ, report the release. The app rejects mismatched package metadata, and CI checks the package version before publishing.

The package includes a standard description, update text, AppStream metadata and a user guide. Package managers display different subsets of that information. GitHub's automatic release notes remain available through **Release notes** in the app.

## Implementation Map

| Area | Main files, relative to `software/gitswamp` |
| --- | --- |
| Pending destructive actions | `src/shared/notifications/destructiveScheduler.ts`, `src/domain/git/UseGit.ts` |
| Active-operation notices | `src/shared/notifications/useRepositoryOperationNotice.ts` |
| Editor guards | `src/shared/ui/FileDiffViewer.vue`, `CodeTextEditor.vue` |
| Image and binary comparison | `src/shared/ui/BinaryDiffViewer.vue`, `src/shared/config/binaryPreview.ts`, `src-tauri/src/services/diff_service.rs` |
| Syntax worker | `src/shared/config/code-view/useDiffSyntax.ts`, `syntaxQueue.ts`, `syntax.worker.ts` |
| Windowed file lists | `src/shared/ui/useWindowedRows.ts`, `src/view/commit/CommitDetails.vue` |
| Asynchronous native reads | `src-tauri/src/commands/commit_files.rs`, `repository.rs` |
| Debian updates | `src-tauri/src/commands/updates.rs`, `src/features/updater/useAppUpdate.ts` |
| Packaging checks | `scripts/prepare-linux-package.mjs`, `scripts/verify-deb.mjs` |

See the [updater guide](DOCUMENTATION_38_AUTO_UPDATER.md) for the repository-root release workflow, signing keys and publishing.
