# Current Workflows and Troubleshooting

This guide supplements the [user guide](DOCUMENTATION_31_USER_GUIDE.md). Android instructions are in the [mobile guide](../software/gitswamp-mobile/README.md).

## Amend the Last Commit

Stage the files that should be included in the last commit. Right-click the current branch's last commit or Working Changes in the graph, then choose **Amend commit**. Review the staged files, message and available options before confirming.

Amend can change commit content, not just its message. It creates a new commit SHA. Coordinate with collaborators before rewriting already published history.

## Branches, Tags and Remote Checkout

Branches and tags with identical names are separate Git references. Use the branch entry to switch branches; selecting a tag is not equivalent to creating or tracking a branch.

Checking out a remote-only branch creates or uses the corresponding local tracking branch. If a local branch contains commits not present in that remote reference, GitSwamp does not silently reset it to the older remote state. Review the divergence before choosing the next Git operation.

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

## Edit Files and Inspect History

Open a working-file diff and select the pencil icon. Edit and choose **Save**. Selecting another file while the editor is open loads the newly selected file. Late responses for the previous file cannot overwrite the current editor.

Unsaved drafts are retained while the same diff viewer remains open. They are not an on-disk backup and are not guaranteed after closing the viewer or restarting. Save before leaving.

Editor and Time Machine syntax previews fall back to plain text above 60,000 characters, more than 1,500 lines, lines longer than 2,000 characters, binary-looking input or unsupported languages. Skipping coloring does not remove the text.

Time Machine reads a selected commit's snapshot without checking it out. Copying a rollback command does not run it; executing that command separately can modify the working tree.

## Large Commits and Coloring

Selecting a large commit shows **Loading changed files...** while metadata is calculated in the background. File statistics are still computed, not replaced with guesses. The first read can take time due to disk speed, Git object storage and file size.

Large file and tree lists render only a window of rows with the normal scroll range and keyboard navigation. Smaller lists keep direct rendering. Recent commit metadata is cached within a memory bound. Duplicate requests for one repository/SHA share an in-flight request; multi-selection starts at most four concurrent metadata jobs from that selection flow.

Diff syntax coloring uses a Web Worker. Plain text appears first and colors arrive asynchronously in batches. Repeated lines reuse cached tokens. Addition/deletion markers and inline change markup remain separate from syntax coloring.

## Debian and Ubuntu Updates

1. Choose **Download .deb** in the update panel.
2. Choose **Open .deb** and finish installation in the system package manager.
3. Return to GitSwamp and use **Check installation**, or close and reopen the app.
4. If the new package is installed but the old process is running, the panel offers **Restart** instead of another download.

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
| Syntax worker | `src/shared/config/code-view/useDiffSyntax.ts`, `syntaxQueue.ts`, `syntax.worker.ts` |
| Windowed file lists | `src/shared/ui/useWindowedRows.ts`, `src/view/commit/CommitDetails.vue` |
| Asynchronous native reads | `src-tauri/src/commands/commit_files.rs`, `repository.rs` |
| Debian updates | `src-tauri/src/commands/updates.rs`, `src/features/updater/useAppUpdate.ts` |
| Packaging checks | `scripts/prepare-linux-package.mjs`, `scripts/verify-deb.mjs` |

See the [updater guide](DOCUMENTATION_38_AUTO_UPDATER.md) for the repository-root release workflow, signing keys and publishing.
