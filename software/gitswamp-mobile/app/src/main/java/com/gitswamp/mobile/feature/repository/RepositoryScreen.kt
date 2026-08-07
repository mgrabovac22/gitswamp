package com.gitswamp.mobile.feature.repository

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.CallSplit
import androidx.compose.material.icons.automirrored.outlined.ListAlt
import androidx.compose.material.icons.outlined.AccountTree
import androidx.compose.material.icons.outlined.Download
import androidx.compose.material.icons.outlined.FolderOpen
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.MoreVert
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Upload
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.gitswamp.mobile.core.designsystem.GitSwampLogo
import com.gitswamp.mobile.core.designsystem.LoadingState
import com.gitswamp.mobile.core.designsystem.SwampColors
import com.gitswamp.mobile.core.model.RemoteAction
import com.gitswamp.mobile.feature.repository.components.Branches
import com.gitswamp.mobile.feature.repository.components.CommitDetailsPanel
import com.gitswamp.mobile.feature.repository.components.CommitGraph
import com.gitswamp.mobile.feature.repository.components.CredentialsDialog
import com.gitswamp.mobile.feature.repository.components.DiffViewer
import com.gitswamp.mobile.feature.repository.components.FileBrowser
import com.gitswamp.mobile.feature.repository.components.FilePreviewDialog
import com.gitswamp.mobile.feature.repository.components.IdentityDialog
import com.gitswamp.mobile.feature.repository.components.WorkingChanges

@Composable
fun RepositoryRoute(
    viewModel: RepositoryViewModel,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    RepositoryScreen(state, viewModel, onBack)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun RepositoryScreen(
    state: RepositoryUiState,
    actions: RepositoryViewModel,
    onBack: () -> Unit,
) {
    val snackbarHost = remember { SnackbarHostState() }

    LaunchedEffect(state.message) {
        state.message?.let {
            snackbarHost.showSnackbar(it, duration = SnackbarDuration.Long)
            actions.clearMessage()
        }
    }
    LaunchedEffect(state.pendingDiscard?.id) {
        state.pendingDiscard?.let { pending ->
            val result = snackbarHost.showSnackbar(
                message = pending.label,
                actionLabel = "Undo",
                duration = SnackbarDuration.Indefinite,
            )
            if (result == SnackbarResult.ActionPerformed) actions.undoDiscard()
        }
    }

    BoxWithConstraints(Modifier.fillMaxSize()) {
        val expanded = maxWidth >= 840.dp
        if (expanded) {
            ExpandedRepositoryLayout(state, actions, onBack, snackbarHost)
        } else {
            CompactRepositoryLayout(state, actions, onBack, snackbarHost)
        }
    }

    state.diffSelection?.let { DiffViewer(it.diff, actions::closeDiff) }
    state.filePreview?.let { FilePreviewDialog(it, actions::closeFilePreview) }
    state.credentialRequest?.let { request ->
        CredentialsDialog(request, actions::dismissCredentials, actions::submitCredentials)
    }
    if (state.identityRequest) {
        IdentityDialog(
            current = state.snapshot?.identity,
            onDismiss = actions::dismissIdentity,
            onSubmit = actions::submitIdentity,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CompactRepositoryLayout(
    state: RepositoryUiState,
    actions: RepositoryViewModel,
    onBack: () -> Unit,
    snackbarHost: SnackbarHostState,
) {
    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        snackbarHost = { SnackbarHost(snackbarHost) },
        topBar = { RepositoryTopBar(state, actions, onBack) },
        bottomBar = { RepositoryBottomBar(state, actions::setSection) },
    ) { padding ->
        RepositoryBody(state, actions, Modifier.fillMaxSize().padding(padding))
    }

    state.selectedCommit?.let { commit ->
        ModalBottomSheet(
            onDismissRequest = actions::closeCommitDetails,
            containerColor = MaterialTheme.colorScheme.surface,
        ) {
            CommitDetailsPanel(
                commit = commit,
                details = state.commitDetails,
                loading = state.detailsLoading,
                onClose = actions::closeCommitDetails,
                onOpenDiff = actions::openCommitDiff,
                modifier = Modifier.fillMaxWidth().fillMaxHeight(0.88f),
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ExpandedRepositoryLayout(
    state: RepositoryUiState,
    actions: RepositoryViewModel,
    onBack: () -> Unit,
    snackbarHost: SnackbarHostState,
) {
    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        snackbarHost = { SnackbarHost(snackbarHost) },
        topBar = { RepositoryTopBar(state, actions, onBack) },
    ) { padding ->
        Row(Modifier.fillMaxSize().padding(padding)) {
            RepositoryNavigationRail(state, actions::setSection)
            VerticalDivider(Modifier.fillMaxHeight().width(1.dp), color = MaterialTheme.colorScheme.outline)
            RepositoryBody(state, actions, Modifier.weight(1f).fillMaxHeight())
            state.selectedCommit?.let { commit ->
                Surface(
                    modifier = Modifier.width(360.dp).fillMaxHeight(),
                    color = MaterialTheme.colorScheme.surface,
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
                ) {
                    CommitDetailsPanel(
                        commit = commit,
                        details = state.commitDetails,
                        loading = state.detailsLoading,
                        onClose = actions::closeCommitDetails,
                        onOpenDiff = actions::openCommitDiff,
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun RepositoryTopBar(
    state: RepositoryUiState,
    actions: RepositoryViewModel,
    onBack: () -> Unit,
) {
    val snapshot = state.snapshot
    var showRemoteMenu by remember { mutableStateOf(false) }
    Column {
        TopAppBar(
            title = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    GitSwampLogo(Modifier.size(32.dp))
                    Column(Modifier.padding(start = 10.dp).weight(1f, fill = false)) {
                        Text(snapshot?.name ?: "Repository", maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.titleSmall)
                        snapshot?.currentBranch?.let {
                            Text(it, maxLines = 1, overflow = TextOverflow.Ellipsis, color = SwampColors.Green, style = MaterialTheme.typography.labelSmall)
                        }
                    }
                }
            },
            actions = {
                IconButton(onClick = onBack) {
                    Icon(Icons.AutoMirrored.Outlined.ArrowBack, contentDescription = "Back to repositories")
                }
                RemoteIconButton(RemoteAction.Fetch, state.activeRemoteAction, snapshot?.remoteUrl != null, actions::runRemoteAction)
                Box {
                    IconButton(onClick = { showRemoteMenu = true }) {
                        if (state.activeRemoteAction != null && state.activeRemoteAction != RemoteAction.Fetch) {
                            CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
                        } else {
                            Icon(Icons.Outlined.MoreVert, contentDescription = "Repository actions")
                        }
                    }
                    DropdownMenu(expanded = showRemoteMenu, onDismissRequest = { showRemoteMenu = false }) {
                        DropdownMenuItem(
                            text = { Text("Pull") },
                            leadingIcon = { Icon(Icons.Outlined.Download, contentDescription = null) },
                            enabled = snapshot?.remoteUrl != null && state.activeRemoteAction == null,
                            onClick = {
                                showRemoteMenu = false
                                actions.runRemoteAction(RemoteAction.Pull)
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Push") },
                            leadingIcon = { Icon(Icons.Outlined.Upload, contentDescription = null) },
                            enabled = snapshot?.remoteUrl != null && state.activeRemoteAction == null,
                            onClick = {
                                showRemoteMenu = false
                                actions.runRemoteAction(RemoteAction.Push)
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Remote credentials") },
                            leadingIcon = { Icon(Icons.Outlined.Key, contentDescription = null) },
                            enabled = snapshot?.remoteUrl != null,
                            onClick = {
                                showRemoteMenu = false
                                actions.requestCredentials()
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Git identity") },
                            leadingIcon = { Icon(Icons.Outlined.Person, contentDescription = null) },
                            onClick = {
                                showRemoteMenu = false
                                actions.requestIdentity()
                            },
                        )
                    }
                }
            },
            colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
        )
        if (state.operationLabel != null || state.refreshing) {
            LinearProgressIndicator(Modifier.fillMaxWidth().height(2.dp))
        }
    }
}

@Composable
private fun RemoteIconButton(
    action: RemoteAction,
    activeAction: RemoteAction?,
    enabled: Boolean,
    onClick: (RemoteAction) -> Unit,
) {
    val active = activeAction == action
    val icon = when (action) {
        RemoteAction.Pull -> Icons.Outlined.Download
        RemoteAction.Push -> Icons.Outlined.Upload
        RemoteAction.Fetch -> Icons.Outlined.Refresh
    }
    IconButton(onClick = { onClick(action) }, enabled = enabled && activeAction == null) {
        if (active) {
            CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
        } else {
            Icon(icon, contentDescription = action.name)
        }
    }
}

@Composable
private fun RepositoryBody(state: RepositoryUiState, actions: RepositoryViewModel, modifier: Modifier) {
    val snapshot = state.snapshot
    when {
        state.loading -> LoadingState("Opening repository", modifier)
        snapshot == null -> Box(modifier, contentAlignment = Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Text("Repository could not be loaded", color = MaterialTheme.colorScheme.error)
                androidx.compose.material3.TextButton(onClick = actions::retry) { Text("Try again") }
            }
        }
        state.section == RepositorySection.Graph -> CommitGraph(
            snapshot = snapshot,
            selectedSha = state.selectedCommit?.sha,
            refreshing = state.refreshing,
            onSelectCommit = actions::selectCommit,
            onOpenChanges = { actions.setSection(RepositorySection.Changes) },
            onLoadMore = actions::loadMoreCommits,
            onCheckoutCommit = actions::checkoutCommit,
            onCreateBranchAtCommit = actions::createBranchAt,
            modifier = modifier,
        )
        state.section == RepositorySection.Changes -> WorkingChanges(
            status = snapshot.status,
            busy = state.operationLabel != null,
            onOpenDiff = actions::openWorkingDiff,
            onStage = actions::stage,
            onUnstage = actions::unstage,
            onDiscard = actions::scheduleDiscard,
            onCommit = actions::commit,
            modifier = modifier,
        )
        state.section == RepositorySection.Files -> FileBrowser(
            directory = state.directory,
            entries = state.entries,
            loading = state.treeLoading,
            onNavigateUp = actions::navigateUp,
            onOpenEntry = actions::openEntry,
            modifier = modifier,
        )
        else -> Branches(
            branches = snapshot.branches,
            busy = state.operationLabel != null,
            onCheckout = actions::checkoutBranch,
            onCreate = actions::createBranch,
            modifier = modifier,
        )
    }
}

@Composable
private fun RepositoryBottomBar(state: RepositoryUiState, onSelect: (RepositorySection) -> Unit) {
    val changes = state.snapshot?.status?.totalCount ?: 0
    NavigationBar(containerColor = MaterialTheme.colorScheme.surface) {
        navigationItems.forEach { item ->
            NavigationBarItem(
                selected = state.section == item.section,
                onClick = { onSelect(item.section) },
                icon = {
                    if (item.section == RepositorySection.Changes && changes > 0) {
                        BadgedBox(badge = { Badge { Text(changes.coerceAtMost(99).toString()) } }) {
                            Icon(item.icon, contentDescription = item.label)
                        }
                    } else {
                        Icon(item.icon, contentDescription = item.label)
                    }
                },
                label = { Text(item.label) },
            )
        }
    }
}

@Composable
private fun RepositoryNavigationRail(state: RepositoryUiState, onSelect: (RepositorySection) -> Unit) {
    val changes = state.snapshot?.status?.totalCount ?: 0
    NavigationRail(containerColor = MaterialTheme.colorScheme.surface) {
        navigationItems.forEach { item ->
            NavigationRailItem(
                selected = state.section == item.section,
                onClick = { onSelect(item.section) },
                icon = {
                    if (item.section == RepositorySection.Changes && changes > 0) {
                        BadgedBox(badge = { Badge { Text(changes.coerceAtMost(99).toString()) } }) {
                            Icon(item.icon, contentDescription = item.label)
                        }
                    } else Icon(item.icon, contentDescription = item.label)
                },
                label = { Text(item.label) },
            )
        }
    }
}

private data class NavigationItem(
    val section: RepositorySection,
    val label: String,
    val icon: androidx.compose.ui.graphics.vector.ImageVector,
)

private val navigationItems = listOf(
    NavigationItem(RepositorySection.Graph, "Graph", Icons.Outlined.AccountTree),
    NavigationItem(RepositorySection.Changes, "Changes", Icons.AutoMirrored.Outlined.ListAlt),
    NavigationItem(RepositorySection.Files, "Files", Icons.Outlined.FolderOpen),
    NavigationItem(RepositorySection.Branches, "Branches", Icons.AutoMirrored.Outlined.CallSplit),
)
