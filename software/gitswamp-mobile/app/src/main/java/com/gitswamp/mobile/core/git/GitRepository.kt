package com.gitswamp.mobile.core.git

import com.gitswamp.mobile.core.model.BranchInfo
import com.gitswamp.mobile.core.model.AuthorIdentity
import com.gitswamp.mobile.core.model.CloneProgress
import com.gitswamp.mobile.core.model.CloneRequest
import com.gitswamp.mobile.core.model.CommitDetails
import com.gitswamp.mobile.core.model.CommitInfo
import com.gitswamp.mobile.core.model.FileDiff
import com.gitswamp.mobile.core.model.RemoteAction
import com.gitswamp.mobile.core.model.RepositoryEntry
import com.gitswamp.mobile.core.model.RepositorySnapshot
import com.gitswamp.mobile.core.model.RepositoryStatus
import java.io.File

interface GitRepository {
    suspend fun clone(
        request: CloneRequest,
        destinationRoot: File,
        onProgress: (CloneProgress) -> Unit,
    ): File

    suspend fun init(destinationRoot: File, name: String): File
    suspend fun validate(path: File): Result<Unit>
    suspend fun loadSnapshot(path: File, commitLimit: Int): RepositorySnapshot
    suspend fun loadCommits(path: File, limit: Int): Pair<List<CommitInfo>, Boolean>
    suspend fun loadStatus(path: File): RepositoryStatus
    suspend fun loadBranches(path: File): List<BranchInfo>
    suspend fun loadCommitDetails(path: File, sha: String): CommitDetails
    suspend fun loadCommitDiff(path: File, sha: String, filePath: String): FileDiff
    suspend fun loadWorkingDiff(path: File, filePath: String, staged: Boolean): FileDiff
    suspend fun listTree(path: File, relativePath: String): List<RepositoryEntry>
    suspend fun readWorkingFile(path: File, relativePath: String): String
    suspend fun stage(path: File, filePaths: List<String>)
    suspend fun unstage(path: File, filePaths: List<String>)
    suspend fun discardUnstaged(path: File, filePaths: List<String>)
    suspend fun commit(path: File, message: String): String
    suspend fun saveIdentity(path: File, identity: AuthorIdentity)
    suspend fun checkoutCommit(path: File, sha: String)
    suspend fun checkoutBranch(path: File, branchName: String)
    suspend fun createBranch(path: File, branchName: String)
    suspend fun createBranchAt(path: File, branchName: String, startPoint: String)
    suspend fun runRemoteAction(
        path: File,
        action: RemoteAction,
        username: String,
        token: String,
    ): String
}
