package com.gitswamp.mobile.feature.repository

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.gitswamp.mobile.core.git.GitRepository
import com.gitswamp.mobile.core.model.CommitDetails
import com.gitswamp.mobile.core.model.AuthorIdentity
import com.gitswamp.mobile.core.model.CommitInfo
import com.gitswamp.mobile.core.model.FileDiff
import com.gitswamp.mobile.core.model.RecentRepository
import com.gitswamp.mobile.core.model.RemoteAction
import com.gitswamp.mobile.core.model.RepositoryEntry
import com.gitswamp.mobile.core.model.RepositorySnapshot
import com.gitswamp.mobile.core.storage.AppPreferences
import com.gitswamp.mobile.core.storage.RecentRepositoriesStore
import com.gitswamp.mobile.core.storage.TokenVault
import kotlinx.coroutines.Job
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import java.io.File
import java.util.UUID

enum class RepositorySection {
    Graph,
    Changes,
    Files,
    Branches,
}

data class FilePreview(
    val path: String,
    val content: String,
)

data class DiffSelection(
    val diff: FileDiff,
    val staged: Boolean?,
    val commitSha: String?,
)

data class PendingDiscard(
    val id: String,
    val filePaths: List<String>,
    val label: String,
)

data class CredentialRequest(
    val action: RemoteAction?,
    val remoteUrl: String,
)

data class RepositoryUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val snapshot: RepositorySnapshot? = null,
    val section: RepositorySection = RepositorySection.Graph,
    val selectedCommit: CommitInfo? = null,
    val commitDetails: CommitDetails? = null,
    val detailsLoading: Boolean = false,
    val diffSelection: DiffSelection? = null,
    val diffLoading: Boolean = false,
    val directory: String = "",
    val entries: List<RepositoryEntry> = emptyList(),
    val treeLoading: Boolean = false,
    val filePreview: FilePreview? = null,
    val activeRemoteAction: RemoteAction? = null,
    val operationLabel: String? = null,
    val pendingDiscard: PendingDiscard? = null,
    val credentialRequest: CredentialRequest? = null,
    val identityRequest: Boolean = false,
    val pendingCommitMessage: String? = null,
    val message: String? = null,
)

class RepositoryViewModel(
    repositoryPath: String,
    private val git: GitRepository,
    private val recentStore: RecentRepositoriesStore,
    private val tokenVault: TokenVault,
    private val preferences: AppPreferences,
) : ViewModel() {
    private var path = File(repositoryPath)
    private val refreshMutex = Mutex()
    private var discardJob: Job? = null
    private var autoFetchStarted = false
    private val parentJob = requireNotNull(viewModelScope.coroutineContext[Job])
    private var sessionJob: Job = SupervisorJob(parentJob)
    private var sessionScope = CoroutineScope(viewModelScope.coroutineContext + sessionJob)

    private val _state = MutableStateFlow(RepositoryUiState())
    val state: StateFlow<RepositoryUiState> = _state.asStateFlow()

    init {
        loadInitial()
    }

    fun openRepository(repositoryPath: String) {
        val next = File(repositoryPath)
        val sameRepository = next.absolutePath == path.absolutePath
        if (sameRepository && sessionJob.isActive && _state.value.loading) return
        if (!sessionJob.isActive) resetSession()
        if (sameRepository && _state.value.snapshot != null) {
            if (_state.value.section == RepositorySection.Files && _state.value.entries.isEmpty()) {
                loadDirectory(_state.value.directory)
            }
            startAutoFetch(requireNotNull(_state.value.snapshot))
            return
        }
        resetSession()
        discardJob?.cancel()
        path = next
        autoFetchStarted = false
        loadInitial()
    }

    fun deactivate() {
        sessionJob.cancel()
        discardJob?.cancel()
        discardJob = null
        autoFetchStarted = false
        _state.value = _state.value.copy(
            selectedCommit = null,
            commitDetails = null,
            detailsLoading = false,
            diffSelection = null,
            diffLoading = false,
            entries = emptyList(),
            filePreview = null,
            treeLoading = false,
            activeRemoteAction = null,
            operationLabel = null,
            pendingDiscard = null,
            identityRequest = false,
            pendingCommitMessage = null,
        )
    }

    fun retry() = loadInitial()

    fun setSection(section: RepositorySection) {
        _state.value = _state.value.copy(section = section)
        if (section == RepositorySection.Files && _state.value.entries.isEmpty()) loadDirectory("")
    }

    fun refresh() {
        launchSession { refreshSnapshot(showSpinner = true) }
    }

    fun loadMoreCommits() {
        val snapshot = _state.value.snapshot ?: return
        if (!snapshot.hasMoreCommits || _state.value.refreshing) return
        launchSession {
            _state.value = _state.value.copy(refreshing = true, message = null)
            val requested = (snapshot.commits.size + preferences.commitPageSize).coerceAtMost(MAX_VISIBLE_COMMITS)
            runCatching { git.loadCommits(path, requested) }
                .onSuccess { (commits, hasMore) ->
                    _state.value = _state.value.copy(
                        refreshing = false,
                        snapshot = snapshot.copy(commits = commits, hasMoreCommits = hasMore && commits.size < MAX_VISIBLE_COMMITS),
                    )
                }
                .onFailure(::operationFailed)
        }
    }

    fun selectCommit(commit: CommitInfo) {
        if (_state.value.selectedCommit?.sha == commit.sha && _state.value.commitDetails != null) return
        _state.value = _state.value.copy(
            selectedCommit = commit,
            commitDetails = null,
            detailsLoading = true,
            diffSelection = null,
            message = null,
        )
        launchSession {
            runCatching { git.loadCommitDetails(path, commit.sha) }
                .onSuccess { details ->
                    if (_state.value.selectedCommit?.sha == commit.sha) {
                        _state.value = _state.value.copy(commitDetails = details, detailsLoading = false)
                    }
                }
                .onFailure { error ->
                    if (_state.value.selectedCommit?.sha == commit.sha) operationFailed(error, details = true)
                }
        }
    }

    fun closeCommitDetails() {
        _state.value = _state.value.copy(selectedCommit = null, commitDetails = null, detailsLoading = false)
    }

    fun openCommitDiff(filePath: String) {
        val commit = _state.value.selectedCommit ?: return
        loadDiff { git.loadCommitDiff(path, commit.sha, filePath) to DiffSelectionMeta(false, commit.sha) }
    }

    fun openWorkingDiff(filePath: String, staged: Boolean) {
        loadDiff { git.loadWorkingDiff(path, filePath, staged) to DiffSelectionMeta(staged, null) }
    }

    fun closeDiff() {
        _state.value = _state.value.copy(diffSelection = null, diffLoading = false)
    }

    fun stage(filePaths: List<String>) = mutateStatus("Staging files") { git.stage(path, filePaths) }

    fun unstage(filePaths: List<String>) = mutateStatus("Unstaging files") { git.unstage(path, filePaths) }

    fun scheduleDiscard(filePaths: List<String>) {
        val unstagedPaths = _state.value.snapshot?.status?.unstaged?.mapTo(hashSetOf()) { it.path }.orEmpty()
        val selected = filePaths.distinct().filter(unstagedPaths::contains)
        if (selected.isEmpty()) {
            _state.value = _state.value.copy(message = "Only unstaged changes can be discarded from this action.")
            return
        }

        discardJob?.cancel()
        val pending = PendingDiscard(
            id = UUID.randomUUID().toString(),
            filePaths = selected,
            label = if (selected.size == 1) "Discarding ${selected.first()} in 5 seconds" else "Discarding ${selected.size} unstaged files in 5 seconds",
        )
        _state.value = _state.value.copy(pendingDiscard = pending)
        discardJob = launchSession {
            delay(DISCARD_DELAY_MS)
            if (_state.value.pendingDiscard?.id == pending.id) {
                _state.value = _state.value.copy(pendingDiscard = null)
                mutateStatusNow("Discarding unstaged changes") { git.discardUnstaged(path, selected) }
            }
        }
    }

    fun undoDiscard() {
        discardJob?.cancel()
        discardJob = null
        _state.value = _state.value.copy(pendingDiscard = null, message = "Discard cancelled.")
    }

    fun commit(message: String) {
        if (_state.value.snapshot?.status?.staged.isNullOrEmpty()) {
            _state.value = _state.value.copy(message = "Stage at least one file before committing.")
            return
        }
        if (_state.value.snapshot?.identity == null) {
            _state.value = _state.value.copy(identityRequest = true, pendingCommitMessage = message)
            return
        }
        createCommit(message)
    }

    fun requestIdentity() {
        _state.value = _state.value.copy(identityRequest = true)
    }

    fun submitIdentity(name: String, email: String) {
        val identity = AuthorIdentity(name.trim(), email.trim())
        val pendingMessage = _state.value.pendingCommitMessage
        launchSession {
            setOperation("Saving Git identity")
            runCatching { git.saveIdentity(path, identity) }
                .onSuccess {
                    val snapshot = _state.value.snapshot
                    _state.value = _state.value.copy(
                        snapshot = snapshot?.copy(identity = identity),
                        identityRequest = false,
                        pendingCommitMessage = null,
                        operationLabel = null,
                    )
                    if (!pendingMessage.isNullOrBlank()) createCommit(pendingMessage)
                }
                .onFailure { operationFailed(it) }
        }
    }

    fun dismissIdentity() {
        _state.value = _state.value.copy(identityRequest = false, pendingCommitMessage = null)
    }

    private fun createCommit(message: String) {
        launchSession {
            setOperation("Creating commit")
            runCatching { git.commit(path, message) }
                .onSuccess { sha ->
                    _state.value = _state.value.copy(operationLabel = null, message = "Created commit ${sha.take(7)}.")
                    refreshSnapshot(showSpinner = false)
                }
                .onFailure(::operationFailed)
        }
    }

    fun checkoutBranch(name: String) {
        val status = _state.value.snapshot?.status ?: return
        if (!status.isClean) {
            _state.value = _state.value.copy(message = "Commit, stash, or discard working changes before switching branches.")
            return
        }
        launchSession {
            setOperation("Checking out $name")
            runCatching { git.checkoutBranch(path, name) }
                .onSuccess {
                    _state.value = _state.value.copy(operationLabel = null, message = "Checked out $name.")
                    refreshSnapshot(showSpinner = false)
                    loadDirectory("")
                }
                .onFailure(::operationFailed)
        }
    }

    fun createBranch(name: String) {
        launchSession {
            setOperation("Creating branch")
            runCatching { git.createBranch(path, name) }
                .onSuccess {
                    _state.value = _state.value.copy(operationLabel = null, message = "Created and checked out $name.")
                    refreshSnapshot(showSpinner = false)
                }
                .onFailure(::operationFailed)
        }
    }

    fun runRemoteAction(action: RemoteAction) {
        val snapshot = _state.value.snapshot ?: return
        val remoteUrl = snapshot.remoteUrl
        if (remoteUrl.isNullOrBlank()) {
            _state.value = _state.value.copy(message = "This repository has no origin remote.")
            return
        }
        val credential = tokenVault.read(remoteUrl)
        val requiresCredentials = remoteUrl.startsWith("https://", ignoreCase = true)
        if (action == RemoteAction.Push && requiresCredentials && credential == null) {
            _state.value = _state.value.copy(credentialRequest = CredentialRequest(action, remoteUrl))
            return
        }
        runRemote(action, credential?.username.orEmpty(), credential?.token.orEmpty())
    }

    fun requestCredentials() {
        val remote = _state.value.snapshot?.remoteUrl
        if (remote.isNullOrBlank()) {
            _state.value = _state.value.copy(message = "This repository has no origin remote.")
        } else {
            _state.value = _state.value.copy(credentialRequest = CredentialRequest(null, remote))
        }
    }

    fun submitCredentials(username: String, token: String, remember: Boolean) {
        val request = _state.value.credentialRequest ?: return
        if (remember) tokenVault.save(request.remoteUrl, username, token)
        _state.value = _state.value.copy(credentialRequest = null)
        request.action?.let { runRemote(it, username, token) }
    }

    fun dismissCredentials() {
        _state.value = _state.value.copy(credentialRequest = null)
    }

    fun loadDirectory(relativePath: String) {
        val normalized = relativePath.trim('/').replace('\\', '/')
        _state.value = _state.value.copy(directory = normalized, treeLoading = true, entries = emptyList(), filePreview = null)
        launchSession {
            runCatching { git.listTree(path, normalized) }
                .onSuccess { entries ->
                    if (_state.value.directory == normalized) {
                        _state.value = _state.value.copy(entries = entries, treeLoading = false)
                    }
                }
                .onFailure { operationFailed(it, tree = true) }
        }
    }

    fun openEntry(entry: RepositoryEntry) {
        if (entry.isDirectory) {
            loadDirectory(entry.relativePath)
        } else {
            launchSession {
                _state.value = _state.value.copy(treeLoading = true, filePreview = null)
                runCatching { git.readWorkingFile(path, entry.relativePath) }
                    .onSuccess { content ->
                        _state.value = _state.value.copy(treeLoading = false, filePreview = FilePreview(entry.relativePath, content))
                    }
                    .onFailure { operationFailed(it, tree = true) }
            }
        }
    }

    fun navigateUp() {
        val directory = _state.value.directory
        if (directory.isBlank()) return
        loadDirectory(directory.substringBeforeLast('/', ""))
    }

    fun closeFilePreview() {
        _state.value = _state.value.copy(filePreview = null)
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    private fun loadInitial() {
        launchSession {
            _state.value = RepositoryUiState(loading = true)
            runCatching { git.loadSnapshot(path, preferences.commitPageSize) }
                .onSuccess { snapshot ->
                    _state.value = _state.value.copy(loading = false, snapshot = snapshot)
                    recentStore.touch(
                        RecentRepository(snapshot.name, snapshot.path, snapshot.currentBranch, System.currentTimeMillis()),
                    )
                    startAutoFetch(snapshot)
                }
                .onFailure { error ->
                    _state.value = _state.value.copy(loading = false, message = error.message ?: "Repository could not be loaded.")
                }
        }
    }

    private suspend fun refreshSnapshot(showSpinner: Boolean) {
        refreshMutex.withLock {
            val currentLimit = _state.value.snapshot?.commits?.size?.coerceAtLeast(preferences.commitPageSize)
                ?: preferences.commitPageSize
            _state.value = _state.value.copy(refreshing = showSpinner)
            runCatching { git.loadSnapshot(path, currentLimit) }
                .onSuccess { snapshot ->
                    _state.value = _state.value.copy(refreshing = false, snapshot = snapshot)
                    recentStore.touch(
                        RecentRepository(snapshot.name, snapshot.path, snapshot.currentBranch, System.currentTimeMillis()),
                    )
                }
                .onFailure(::operationFailed)
        }
    }

    private fun startAutoFetch(snapshot: RepositorySnapshot) {
        if (autoFetchStarted || !preferences.autoFetchOnOpen || snapshot.remoteUrl.isNullOrBlank()) return
        autoFetchStarted = true
        launchSession {
            val credential = tokenVault.read(snapshot.remoteUrl)
            runCatching {
                git.runRemoteAction(path, RemoteAction.Fetch, credential?.username.orEmpty(), credential?.token.orEmpty())
            }.onSuccess {
                refreshSnapshot(showSpinner = false)
            }
        }
    }

    private fun runRemote(action: RemoteAction, username: String, token: String) {
        if (_state.value.activeRemoteAction != null) return
        launchSession {
            _state.value = _state.value.copy(activeRemoteAction = action, message = null)
            runCatching { git.runRemoteAction(path, action, username, token) }
                .onSuccess { message ->
                    _state.value = _state.value.copy(activeRemoteAction = null, message = message)
                    refreshSnapshot(showSpinner = false)
                }
                .onFailure { error ->
                    val remote = _state.value.snapshot?.remoteUrl.orEmpty()
                    val authRelated = error.message.orEmpty().contains("auth", true) ||
                        error.message.orEmpty().contains("not authorized", true) ||
                        error.message.orEmpty().contains("401")
                    _state.value = _state.value.copy(
                        activeRemoteAction = null,
                        message = error.message ?: "Remote operation failed.",
                        credentialRequest = if (authRelated && remote.isNotBlank()) CredentialRequest(action, remote) else null,
                    )
                }
        }
    }

    private fun loadDiff(loader: suspend () -> Pair<FileDiff, DiffSelectionMeta>) {
        _state.value = _state.value.copy(diffLoading = true, diffSelection = null, message = null)
        launchSession {
            runCatching { loader() }
                .onSuccess { (diff, meta) ->
                    _state.value = _state.value.copy(
                        diffLoading = false,
                        diffSelection = DiffSelection(diff, meta.staged, meta.commitSha),
                    )
                }
                .onFailure { operationFailed(it, diff = true) }
        }
    }

    private fun mutateStatus(label: String, operation: suspend () -> Unit) {
        launchSession { mutateStatusNow(label, operation) }
    }

    private suspend fun mutateStatusNow(label: String, operation: suspend () -> Unit) {
        setOperation(label)
        runCatching { operation() }
            .onSuccess {
                val snapshot = _state.value.snapshot
                val updatedStatus = git.loadStatus(path)
                _state.value = _state.value.copy(
                    operationLabel = null,
                    snapshot = snapshot?.copy(status = updatedStatus),
                    diffSelection = null,
                )
            }
            .onFailure(::operationFailed)
    }

    private fun setOperation(label: String) {
        _state.value = _state.value.copy(operationLabel = label, message = null)
    }

    private fun operationFailed(
        error: Throwable,
        details: Boolean = false,
        diff: Boolean = false,
        tree: Boolean = false,
    ) {
        _state.value = _state.value.copy(
            loading = false,
            refreshing = false,
            detailsLoading = if (details) false else _state.value.detailsLoading,
            diffLoading = if (diff) false else _state.value.diffLoading,
            treeLoading = if (tree) false else _state.value.treeLoading,
            activeRemoteAction = null,
            operationLabel = null,
            message = error.message ?: "Git operation failed.",
        )
    }

    private fun resetSession() {
        sessionJob.cancel()
        sessionJob = SupervisorJob(parentJob)
        sessionScope = CoroutineScope(viewModelScope.coroutineContext + sessionJob)
    }

    private fun launchSession(block: suspend CoroutineScope.() -> Unit): Job =
        sessionScope.launch(block = block)

    private data class DiffSelectionMeta(val staged: Boolean?, val commitSha: String?)

    companion object {
        private const val MAX_VISIBLE_COMMITS = 5_000
        private const val DISCARD_DELAY_MS = 5_000L

        fun factory(
            repositoryPath: String,
            git: GitRepository,
            recentStore: RecentRepositoriesStore,
            tokenVault: TokenVault,
            preferences: AppPreferences,
        ): ViewModelProvider.Factory = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T {
                return RepositoryViewModel(repositoryPath, git, recentStore, tokenVault, preferences) as T
            }
        }
    }
}
