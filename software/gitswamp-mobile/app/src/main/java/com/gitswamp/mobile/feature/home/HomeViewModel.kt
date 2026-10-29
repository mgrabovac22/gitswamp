package com.gitswamp.mobile.feature.home

import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.gitswamp.mobile.core.git.GitRepository
import com.gitswamp.mobile.core.git.CloneCredential
import com.gitswamp.mobile.core.git.CloneResult
import com.gitswamp.mobile.core.git.CloneWorkflow
import com.gitswamp.mobile.core.git.GithubDeviceAuth
import com.gitswamp.mobile.core.git.GithubRepositories
import com.gitswamp.mobile.core.git.GithubRepository
import kotlinx.coroutines.Job
import kotlinx.coroutines.CancellationException
import com.gitswamp.mobile.core.model.CloneProgress
import com.gitswamp.mobile.core.model.CloneRequest
import com.gitswamp.mobile.core.model.RecentRepository
import com.gitswamp.mobile.core.storage.AppPreferences
import com.gitswamp.mobile.core.storage.RecentRepositoriesStore
import com.gitswamp.mobile.core.storage.RepositoryPathResolver
import com.gitswamp.mobile.core.storage.TokenVault
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch
import java.io.File

data class HomeUiState(
    val recentRepositories: List<RecentRepository> = emptyList(),
    val busy: Boolean = false,
    val cloneProgress: CloneProgress? = null,
    val cloneAuthentication: CloneResult.AuthenticationRequired? = null,
    val message: String? = null,
    val canRetryClone: Boolean = false,
    val repositoryPickerOpen: Boolean = false,
    val githubRepositories: List<GithubRepository> = emptyList(),
    val repositoriesLoading: Boolean = false,
    val repositoriesHasMore: Boolean = true,
    val repositoriesError: String? = null,
    val useSystemTheme: Boolean = false,
    val autoFetchOnOpen: Boolean = false,
    val commitPageSize: Int = 200,
)

sealed interface HomeEvent {
    data class OpenRepository(val path: String) : HomeEvent
}

class HomeViewModel(
    private val git: GitRepository,
    private val recentStore: RecentRepositoriesStore,
    private val tokenVault: TokenVault,
    private val pathResolver: RepositoryPathResolver,
    private val preferences: AppPreferences,
) : ViewModel() {
    private val _state = MutableStateFlow(readState())
    val state: StateFlow<HomeUiState> = _state.asStateFlow()

    private val eventChannel = Channel<HomeEvent>(Channel.BUFFERED)
    val events = eventChannel.receiveAsFlow()
    private var failedClone: CloneRequest? = null
    private var catalogCredential: CloneCredential? = null
    private var catalogRequest: CloneRequest? = null
    private var catalogPage = 1
    private var catalogJob: Job? = null
    private val cloneWorkflow = CloneWorkflow(
        readCredential = { url -> tokenVault.read(url)?.let { CloneCredential(it.username, it.token) } },
        saveCredential = { url, credential -> tokenVault.save(url, credential.username, credential.token) },
        cloneRepository = { request, progress -> git.clone(request, pathResolver.workspaceRoot, progress) },
    )

    fun openRecent(path: String) = openFile(File(path))

    fun openTreeUri(uri: Uri) {
        if (_state.value.busy) return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, message = null)
            runCatching {
                val direct = pathResolver.resolveTreeUri(uri).getOrNull()
                if (direct != null && git.validate(direct).isSuccess) {
                    direct
                } else {
                    pathResolver.importTree(uri) { progress ->
                        _state.value = _state.value.copy(cloneProgress = progress)
                    }
                }
            }.onSuccess { file ->
                val validation = git.validate(file)
                validation.onSuccess {
                    _state.value = _state.value.copy(busy = false, cloneProgress = null)
                    eventChannel.send(HomeEvent.OpenRepository(file.canonicalPath))
                }.onFailure {
                    _state.value = _state.value.copy(busy = false, cloneProgress = null, message = it.message ?: "This folder is not a Git repository.")
                }
            }.onFailure {
                _state.value = _state.value.copy(busy = false, cloneProgress = null, message = it.message ?: "Repository import failed.")
            }
        }
    }

    fun clone(request: CloneRequest) {
        if (_state.value.busy) return
        viewModelScope.launch {
            failedClone = null
            _state.value = _state.value.copy(busy = true, cloneProgress = CloneProgress("Preparing repository", 0, 0), cloneAuthentication = null, message = null, canRetryClone = false)
            runCatching {
                cloneWorkflow.clone(request) { progress ->
                    _state.value = _state.value.copy(cloneProgress = progress)
                }
            }.onSuccess { result ->
                when (result) {
                    is CloneResult.Success -> {
                        _state.value = _state.value.copy(busy = false, cloneProgress = null)
                        openFile(result.directory)
                    }
                    is CloneResult.AuthenticationRequired -> {
                        _state.value = _state.value.copy(busy = false, cloneProgress = null, cloneAuthentication = result)
                    }
                }
            }.onFailure {
                if (it is CancellationException) throw it
                val network = CloneWorkflow.isNetworkFailure(it)
                failedClone = if (network) request else null
                _state.value = _state.value.copy(busy = false, cloneProgress = null, canRetryClone = network,
                    message = if (network) "The phone cannot reach the repository host. Check Wi-Fi, mobile data, VPN or Private DNS, then retry. Your authorization is retained." else it.message ?: "Clone failed.")
            }
        }
    }

    fun retryClone() { failedClone?.let(::clone) }

    fun authorizeClone(username: String, token: String, remember: Boolean) {
        val pending = _state.value.cloneAuthentication ?: return
        if (_state.value.busy || token.isBlank()) return
        val request = pending.request.copy(username = username.trim(), token = token, rememberToken = remember)
        if (GithubDeviceAuth.supports(request.url)) {
            if (remember) tokenVault.save(request.url, request.username, token)
            openGithubRepositories(request)
        } else clone(request)
    }

    fun browseGithubRepositories() {
        if (_state.value.busy) return
        val url = "https://github.com"
        val credential = tokenVault.read(url)
        val request = CloneRequest(url, "", credential?.username ?: "git", credential?.token.orEmpty(), true)
        if (credential == null) {
            _state.value = _state.value.copy(cloneAuthentication = CloneResult.AuthenticationRequired(request))
        } else openGithubRepositories(request)
    }

    private fun openGithubRepositories(request: CloneRequest) {
        catalogJob?.cancel()
        failedClone = null
        catalogCredential = CloneCredential(request.username, request.token)
        catalogRequest = request.copy(token = "")
        catalogPage = 1
        _state.value = _state.value.copy(cloneAuthentication = null, repositoryPickerOpen = true,
            githubRepositories = emptyList(), repositoriesLoading = false, repositoriesHasMore = true, repositoriesError = null, message = null, canRetryClone = false)
        loadGithubRepositories()
    }

    fun loadGithubRepositories() {
        val credential = catalogCredential ?: return
        if (!_state.value.repositoryPickerOpen || _state.value.repositoriesLoading || !_state.value.repositoriesHasMore) return
        _state.value = _state.value.copy(repositoriesLoading = true, repositoriesError = null)
        catalogJob = viewModelScope.launch {
            try {
                val page = GithubRepositories.fetch(credential.token, catalogPage)
                _state.value = _state.value.copy(githubRepositories = (_state.value.githubRepositories + page.repositories).distinctBy { it.id },
                    repositoriesLoading = false, repositoriesHasMore = page.hasMore)
                catalogPage++
            } catch (cancelled: CancellationException) { throw cancelled
            } catch (error: Exception) {
                _state.value = _state.value.copy(repositoriesLoading = false, repositoriesError =
                    if (CloneWorkflow.isNetworkFailure(error)) "Cannot reach GitHub. Your connection is saved; retry loading repositories."
                    else error.message ?: "Could not load repositories.")
            }
        }
    }

    fun cloneGithubRepository(repository: GithubRepository, destination: String) {
        val credential = catalogCredential ?: return
        val request = catalogRequest ?: return
        if (_state.value.busy || _state.value.githubRepositories.none { it.id == repository.id && it.cloneUrl == repository.cloneUrl }) return
        dismissGithubRepositories()
        clone(CloneRequest(repository.cloneUrl, destination.trim().ifBlank { repository.name }, credential.username, credential.token, request.rememberToken))
    }

    fun reconnectGithub() {
        val request = catalogRequest ?: return
        dismissGithubRepositories()
        _state.value = _state.value.copy(cloneAuthentication = CloneResult.AuthenticationRequired(request))
    }

    fun dismissGithubRepositories() {
        catalogJob?.cancel()
        catalogCredential = null
        catalogRequest = null
        _state.value = _state.value.copy(repositoryPickerOpen = false, githubRepositories = emptyList(), repositoriesLoading = false)
    }

    fun dismissCloneAuthentication() {
        _state.value = _state.value.copy(cloneAuthentication = null)
    }

    fun init(name: String) {
        if (_state.value.busy) return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, message = null)
            runCatching { git.init(pathResolver.workspaceRoot, name) }
                .onSuccess { directory ->
                    _state.value = _state.value.copy(busy = false)
                    openFile(directory)
                }
                .onFailure { _state.value = _state.value.copy(busy = false, message = it.message ?: "Init failed.") }
        }
    }

    fun removeRecent(path: String) {
        recentStore.remove(path)
        _state.value = _state.value.copy(recentRepositories = recentStore.read())
    }

    fun clearRecent() {
        recentStore.clear()
        _state.value = _state.value.copy(recentRepositories = emptyList())
    }

    fun refreshRecent() {
        _state.value = _state.value.copy(recentRepositories = recentStore.read())
    }

    fun setUseSystemTheme(value: Boolean) {
        preferences.useSystemTheme = value
        _state.value = _state.value.copy(useSystemTheme = value)
    }

    fun setAutoFetch(value: Boolean) {
        preferences.autoFetchOnOpen = value
        _state.value = _state.value.copy(autoFetchOnOpen = value)
    }

    fun setCommitPageSize(value: Int) {
        preferences.commitPageSize = value
        _state.value = _state.value.copy(commitPageSize = preferences.commitPageSize)
    }

    fun clearMessage() {
        failedClone = null
        _state.value = _state.value.copy(message = null, canRetryClone = false)
    }

    private fun openFile(file: File) {
        if (_state.value.busy) return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, message = null)
            git.validate(file)
                .onSuccess {
                    _state.value = _state.value.copy(busy = false)
                    eventChannel.send(HomeEvent.OpenRepository(file.canonicalPath))
                }
                .onFailure {
                    _state.value = _state.value.copy(busy = false, message = it.message ?: "This folder is not a Git repository.")
                }
        }
    }

    private fun readState() = HomeUiState(
        recentRepositories = recentStore.read(),
        useSystemTheme = preferences.useSystemTheme,
        autoFetchOnOpen = preferences.autoFetchOnOpen,
        commitPageSize = preferences.commitPageSize,
    )

    companion object {
        fun factory(
            git: GitRepository,
            recentStore: RecentRepositoriesStore,
            tokenVault: TokenVault,
            pathResolver: RepositoryPathResolver,
            preferences: AppPreferences,
        ): ViewModelProvider.Factory = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T {
                return HomeViewModel(git, recentStore, tokenVault, pathResolver, preferences) as T
            }
        }
    }
}
