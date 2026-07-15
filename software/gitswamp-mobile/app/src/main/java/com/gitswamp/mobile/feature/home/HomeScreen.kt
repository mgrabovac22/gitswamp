package com.gitswamp.mobile.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Code
import androidx.compose.material.icons.outlined.DeleteSweep
import androidx.compose.material.icons.outlined.FolderOpen
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material.icons.outlined.Source
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.VerticalDivider
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.gitswamp.mobile.core.designsystem.GitSwampBrand
import com.gitswamp.mobile.core.designsystem.InlineNotice
import com.gitswamp.mobile.core.designsystem.StatusPill
import com.gitswamp.mobile.core.designsystem.SwampColors
import com.gitswamp.mobile.core.designsystem.SwampPrimaryButton
import com.gitswamp.mobile.core.designsystem.SwampSecondaryButton
import com.gitswamp.mobile.core.model.CloneRequest
import com.gitswamp.mobile.core.model.RecentRepository
import java.time.Duration
import java.time.Instant

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    state: HomeUiState,
    onOpenRepository: (String) -> Unit,
    onBrowse: () -> Unit,
    onClone: (CloneRequest) -> Unit,
    onInit: (String) -> Unit,
    onRemoveRecent: (String) -> Unit,
    onClearRecent: () -> Unit,
    onOpenSettings: () -> Unit,
    onDismissMessage: () -> Unit,
) {
    var query by remember { mutableStateOf("") }
    var showClone by remember { mutableStateOf(false) }
    var showInit by remember { mutableStateOf(false) }
    val recent = remember(state.recentRepositories, query) {
        val normalized = query.trim().lowercase()
        if (normalized.isEmpty()) state.recentRepositories else state.recentRepositories.filter {
            it.name.lowercase().contains(normalized) || it.path.lowercase().contains(normalized)
        }
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { GitSwampBrand() },
                actions = {
                    IconButton(onClick = onOpenSettings) {
                        Icon(Icons.Outlined.Settings, contentDescription = "Settings")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
            )
        },
    ) { padding ->
        BoxWithConstraints(Modifier.fillMaxSize().padding(padding)) {
            val wide = maxWidth >= 720.dp
            if (wide) {
                Row(Modifier.fillMaxSize().padding(horizontal = 28.dp, vertical = 20.dp)) {
                    HomeActions(
                        modifier = Modifier.weight(0.42f).padding(end = 24.dp),
                        onBrowse = onBrowse,
                        onClone = { showClone = true },
                        onInit = { showInit = true },
                    )
                    VerticalDivider(Modifier.fillMaxHeight().width(1.dp), color = MaterialTheme.colorScheme.outline)
                    RecentRepositories(
                        repositories = recent,
                        totalCount = state.recentRepositories.size,
                        query = query,
                        onQueryChange = { query = it },
                        onOpen = onOpenRepository,
                        onRemove = onRemoveRecent,
                        onClear = onClearRecent,
                        modifier = Modifier.weight(0.58f).padding(start = 24.dp).verticalScroll(rememberScrollState()),
                    )
                }
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = androidx.compose.foundation.layout.PaddingValues(18.dp),
                    verticalArrangement = Arrangement.spacedBy(18.dp),
                ) {
                    item {
                        HomeActions(
                            onBrowse = onBrowse,
                            onClone = { showClone = true },
                            onInit = { showInit = true },
                        )
                    }
                    item { HorizontalDivider(color = MaterialTheme.colorScheme.outline) }
                    item {
                        RecentRepositories(
                            repositories = recent,
                            totalCount = state.recentRepositories.size,
                            query = query,
                            onQueryChange = { query = it },
                            onOpen = onOpenRepository,
                            onRemove = onRemoveRecent,
                            onClear = onClearRecent,
                        )
                    }
                }
            }
        }
    }

    if (showClone) {
        CloneRepositoryDialog(
            busy = state.busy,
            onDismiss = { if (!state.busy) showClone = false },
            onClone = {
                showClone = false
                onClone(it)
            },
        )
    }
    if (showInit) {
        InitRepositoryDialog(
            onDismiss = { showInit = false },
            onInit = {
                showInit = false
                onInit(it)
            },
        )
    }
    if (state.cloneProgress != null) {
        CloneProgressDialog(state.cloneProgress.task, state.cloneProgress.completed, state.cloneProgress.total)
    }
    if (state.message != null) {
        AlertDialog(
            onDismissRequest = onDismissMessage,
            confirmButton = { TextButton(onClick = onDismissMessage) { Text("Close") } },
            title = { Text("Git operation failed") },
            text = { Text(state.message) },
        )
    }
}

@Composable
private fun HomeActions(
    onBrowse: () -> Unit,
    onClone: () -> Unit,
    onInit: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier) {
        Text("Repository Management", style = MaterialTheme.typography.titleLarge)
        Spacer(Modifier.height(6.dp))
        Text(
            "Clone or import a repository stored on this device. Git commands run locally without an application server.",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            style = MaterialTheme.typography.bodyMedium,
        )
        Spacer(Modifier.height(18.dp))
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            SwampPrimaryButton("Clone", onClone, leading = {
                Icon(Icons.Outlined.Source, contentDescription = null, Modifier.size(17.dp))
            })
            SwampSecondaryButton("Browse", onBrowse, leading = {
                Icon(Icons.Outlined.FolderOpen, contentDescription = null, Modifier.size(17.dp))
            })
            SwampSecondaryButton("Init", onInit, leading = {
                Icon(Icons.Outlined.Add, contentDescription = null, Modifier.size(17.dp))
            })
        }
        Spacer(Modifier.height(22.dp))
        InlineNotice(
            title = "Local by design",
            detail = "Repository data never passes through a GitSwamp server. Browse imports a private editable copy when Android blocks direct filesystem access.",
            color = SwampColors.Green,
        )
        Spacer(Modifier.height(18.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            StatusPill("Native Android", SwampColors.Cyan)
            StatusPill("No WebView", SwampColors.Primary)
            StatusPill("Offline ready", SwampColors.Green)
        }
    }
}

@Composable
private fun RecentRepositories(
    repositories: List<RecentRepository>,
    totalCount: Int,
    query: String,
    onQueryChange: (String) -> Unit,
    onOpen: (String) -> Unit,
    onRemove: (String) -> Unit,
    onClear: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Outlined.History, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                Text("Recent repositories", Modifier.padding(start = 8.dp), fontWeight = FontWeight.SemiBold)
                StatusPill(totalCount.toString(), MaterialTheme.colorScheme.primary, Modifier.padding(start = 8.dp))
            }
            if (totalCount > 0) {
                IconButton(onClick = onClear) {
                    Icon(Icons.Outlined.DeleteSweep, contentDescription = "Remove all recent repositories")
                }
            }
        }
        Spacer(Modifier.height(10.dp))
        OutlinedTextField(
            value = query,
            onValueChange = onQueryChange,
            modifier = Modifier.fillMaxWidth(),
            placeholder = { Text("Search repositories") },
            leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
            trailingIcon = if (query.isNotEmpty()) {
                { IconButton(onClick = { onQueryChange("") }) { Icon(Icons.Outlined.Close, "Clear search") } }
            } else null,
            singleLine = true,
            shape = RoundedCornerShape(6.dp),
        )
        Spacer(Modifier.height(8.dp))
        if (repositories.isEmpty()) {
            Column(
                Modifier.fillMaxWidth().padding(vertical = 32.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Icon(Icons.Outlined.Code, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                Spacer(Modifier.height(8.dp))
                Text(
                    if (query.isBlank()) "No recent repositories yet" else "No repository matches this search",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else {
            repositories.forEach { repository ->
                RecentRepositoryRow(repository, onOpen, onRemove)
            }
        }
    }
}

@Composable
private fun RecentRepositoryRow(
    repository: RecentRepository,
    onOpen: (String) -> Unit,
    onRemove: (String) -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(6.dp))
            .clickable { onOpen(repository.path) }
            .padding(horizontal = 10.dp, vertical = 11.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(Icons.Outlined.FolderOpen, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
        Column(Modifier.weight(1f).padding(horizontal = 10.dp)) {
            Text(repository.name, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(
                repository.path,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                style = MaterialTheme.typography.bodySmall,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        StatusPill(repository.branch, SwampColors.Green)
        IconButton(onClick = { onRemove(repository.path) }, Modifier.size(36.dp)) {
            Icon(Icons.Outlined.Close, contentDescription = "Remove ${repository.name} from recent repositories", Modifier.size(17.dp))
        }
    }
}

@Composable
private fun CloneRepositoryDialog(
    busy: Boolean,
    onDismiss: () -> Unit,
    onClone: (CloneRequest) -> Unit,
) {
    var url by remember { mutableStateOf("") }
    var name by remember { mutableStateOf("") }
    var username by remember { mutableStateOf("git") }
    var token by remember { mutableStateOf("") }
    var rememberToken by remember { mutableStateOf(true) }
    val inferredName = url.trimEnd('/').substringAfterLast('/').substringAfterLast(':').removeSuffix(".git")
    val valid = url.isNotBlank() && (name.ifBlank { inferredName }).isNotBlank()

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Clone repository") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(9.dp)) {
                OutlinedTextField(
                    value = url,
                    onValueChange = { url = it },
                    label = { Text("HTTPS repository URL") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                )
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Local name") },
                    placeholder = { Text(inferredName.ifBlank { "repository" }) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                OutlinedTextField(
                    value = username,
                    onValueChange = { username = it },
                    label = { Text("Username") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                OutlinedTextField(
                    value = token,
                    onValueChange = { token = it },
                    label = { Text("Personal access token (optional)") },
                    leadingIcon = { Icon(Icons.Outlined.Lock, contentDescription = null) },
                    visualTransformation = PasswordVisualTransformation(),
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Checkbox(rememberToken, { rememberToken = it })
                    Text("Remember encrypted token on this device", style = MaterialTheme.typography.bodySmall)
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    onClone(CloneRequest(url.trim(), name.ifBlank { inferredName }, username.trim(), token, rememberToken))
                },
                enabled = valid && !busy,
            ) { Text("Clone") }
        },
        dismissButton = { TextButton(onClick = onDismiss, enabled = !busy) { Text("Cancel") } },
    )
}

@Composable
private fun InitRepositoryDialog(onDismiss: () -> Unit, onInit: (String) -> Unit) {
    var name by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Create local repository") },
        text = {
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                label = { Text("Repository name") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )
        },
        confirmButton = { TextButton(onClick = { onInit(name.trim()) }, enabled = name.isNotBlank()) { Text("Create") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun CloneProgressDialog(task: String, completed: Int, total: Int) {
    AlertDialog(
        onDismissRequest = {},
            title = { Text(if (task.startsWith("Import")) "Importing repository" else "Cloning repository") },
        text = {
            Column {
                Text(task, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Spacer(Modifier.height(12.dp))
                if (total > 0) {
                    LinearProgressIndicator(
                        progress = { (completed.toFloat() / total.toFloat()).coerceIn(0f, 1f) },
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(Modifier.height(6.dp))
                    Text("$completed / $total", style = MaterialTheme.typography.bodySmall)
                } else {
                    LinearProgressIndicator(Modifier.fillMaxWidth())
                }
            }
        },
        confirmButton = {},
    )
}
