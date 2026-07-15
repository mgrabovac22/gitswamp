package com.gitswamp.mobile.feature.home

import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.gitswamp.mobile.core.git.GitRepository
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
    val message: String? = null,
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
            _state.value = _state.value.copy(busy = true, cloneProgress = CloneProgress("Preparing repository", 0, 0), message = null)
            runCatching {
                git.clone(request, pathResolver.workspaceRoot) { progress ->
                    _state.value = _state.value.copy(cloneProgress = progress)
                }
            }.onSuccess { directory ->
                if (request.rememberToken && request.token.isNotBlank()) {
                    tokenVault.save(request.url, request.username, request.token)
                }
                _state.value = _state.value.copy(busy = false, cloneProgress = null)
                openFile(directory)
            }.onFailure {
                _state.value = _state.value.copy(busy = false, cloneProgress = null, message = it.message ?: "Clone failed.")
            }
        }
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
        _state.value = _state.value.copy(message = null)
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
