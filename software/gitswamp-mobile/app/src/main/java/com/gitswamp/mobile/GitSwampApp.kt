package com.gitswamp.mobile

import android.content.Intent
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.gitswamp.mobile.core.designsystem.GitSwampTheme
import com.gitswamp.mobile.di.AppContainer
import com.gitswamp.mobile.feature.home.HomeEvent
import com.gitswamp.mobile.feature.home.HomeScreen
import com.gitswamp.mobile.feature.home.HomeViewModel
import com.gitswamp.mobile.feature.repository.RepositoryRoute
import com.gitswamp.mobile.feature.repository.RepositoryViewModel
import com.gitswamp.mobile.feature.settings.SettingsSheet

@Composable
fun GitSwampApp(container: AppContainer) {
    val homeViewModel: HomeViewModel = viewModel(
        factory = HomeViewModel.factory(
            container.gitRepository,
            container.recentRepositories,
            container.tokenVault,
            container.pathResolver,
            container.preferences,
        ),
    )
    val homeState by homeViewModel.state.collectAsStateWithLifecycle()
    var repositoryPath by rememberSaveable { mutableStateOf<String?>(null) }
    var showSettings by rememberSaveable { mutableStateOf(false) }
    val context = LocalContext.current

    val folderPicker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        uri ?: return@rememberLauncherForActivityResult
        runCatching {
            context.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
            )
        }
        homeViewModel.openTreeUri(uri)
    }

    LaunchedEffect(homeViewModel) {
        homeViewModel.events.collect { event ->
            when (event) {
                is HomeEvent.OpenRepository -> repositoryPath = event.path
            }
        }
    }

    BackHandler(enabled = repositoryPath != null) {
        repositoryPath = null
        homeViewModel.refreshRecent()
    }

    GitSwampTheme(useSystemTheme = homeState.useSystemTheme) {
        val activePath = repositoryPath
        if (activePath == null) {
            HomeScreen(
                state = homeState,
                onOpenRepository = homeViewModel::openRecent,
                onBrowse = { folderPicker.launch(null) },
                onClone = homeViewModel::clone,
                onInit = homeViewModel::init,
                onRemoveRecent = homeViewModel::removeRecent,
                onClearRecent = homeViewModel::clearRecent,
                onOpenSettings = { showSettings = true },
                onDismissMessage = homeViewModel::clearMessage,
            )
        } else {
            ActiveRepository(
                path = activePath,
                container = container,
                onBack = {
                    repositoryPath = null
                    homeViewModel.refreshRecent()
                },
            )
        }

        if (showSettings) {
            SettingsSheet(
                state = homeState,
                onDismiss = { showSettings = false },
                onUseSystemTheme = homeViewModel::setUseSystemTheme,
                onAutoFetch = homeViewModel::setAutoFetch,
                onCommitPageSize = homeViewModel::setCommitPageSize,
            )
        }
    }
}

@Composable
private fun ActiveRepository(
    path: String,
    container: AppContainer,
    onBack: () -> Unit,
) {
    val repositoryViewModel: RepositoryViewModel = viewModel(
        key = "active-repository",
        factory = RepositoryViewModel.factory(
            repositoryPath = path,
            git = container.gitRepository,
            recentStore = container.recentRepositories,
            tokenVault = container.tokenVault,
            preferences = container.preferences,
        ),
    )
    LaunchedEffect(path) { repositoryViewModel.openRepository(path) }
    val closeRepository = {
        repositoryViewModel.deactivate()
        onBack()
    }
    BackHandler(onBack = closeRepository)
    RepositoryRoute(repositoryViewModel, closeRepository)
}
