package com.gitswamp.mobile.core.model

import java.time.Instant

data class RecentRepository(
    val name: String,
    val path: String,
    val branch: String,
    val lastOpenedAt: Long,
)

data class RepositorySnapshot(
    val name: String,
    val path: String,
    val currentBranch: String,
    val remoteUrl: String?,
    val identity: AuthorIdentity?,
    val commits: List<CommitInfo>,
    val branches: List<BranchInfo>,
    val status: RepositoryStatus,
    val hasMoreCommits: Boolean,
)

data class AuthorIdentity(
    val name: String,
    val email: String,
)

data class CommitInfo(
    val sha: String,
    val shortSha: String,
    val message: String,
    val authorName: String,
    val authorEmail: String,
    val authoredAt: Instant,
    val parentShas: List<String>,
    val refs: List<String>,
)

data class BranchInfo(
    val name: String,
    val isCurrent: Boolean,
    val isRemote: Boolean,
    val upstream: String?,
    val ahead: Int,
    val behind: Int,
)

enum class ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Conflicted,
}

data class ChangedFile(
    val path: String,
    val kind: ChangeKind,
    val staged: Boolean,
    val oldPath: String? = null,
)

data class RepositoryStatus(
    val staged: List<ChangedFile> = emptyList(),
    val unstaged: List<ChangedFile> = emptyList(),
    val conflicts: List<ChangedFile> = emptyList(),
) {
    val isClean: Boolean get() = staged.isEmpty() && unstaged.isEmpty() && conflicts.isEmpty()
    val totalCount: Int get() = (staged + unstaged + conflicts).map { it.path }.distinct().size
}

data class CommitFile(
    val path: String,
    val oldPath: String?,
    val kind: ChangeKind,
    val additions: Int,
    val deletions: Int,
)

enum class DiffLineKind {
    Context,
    Added,
    Deleted,
    Header,
}

data class DiffLine(
    val kind: DiffLineKind,
    val text: String,
    val oldLine: Int?,
    val newLine: Int?,
)

data class FileDiff(
    val path: String,
    val oldPath: String?,
    val lines: List<DiffLine>,
    val truncated: Boolean,
    val binary: Boolean,
)

data class CommitDetails(
    val commit: CommitInfo,
    val files: List<CommitFile>,
)

data class RepositoryEntry(
    val name: String,
    val relativePath: String,
    val isDirectory: Boolean,
    val sizeBytes: Long,
)

enum class RemoteAction {
    Fetch,
    Pull,
    Push,
}

data class CloneRequest(
    val url: String,
    val destinationName: String,
    val username: String,
    val token: String,
    val rememberToken: Boolean,
)

data class CloneProgress(
    val task: String,
    val completed: Int,
    val total: Int,
)
