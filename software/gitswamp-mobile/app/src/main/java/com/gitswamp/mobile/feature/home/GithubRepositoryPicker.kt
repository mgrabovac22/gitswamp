package com.gitswamp.mobile.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material.icons.outlined.Source
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.gitswamp.mobile.core.git.GithubRepository

@Composable
fun GithubRepositoryPicker(
    state: HomeUiState,
    onDismiss: () -> Unit,
    onLoadMore: () -> Unit,
    onReconnect: () -> Unit,
    onClone: (GithubRepository, String) -> Unit,
) {
    var search by remember { mutableStateOf("") }
    var selected by remember { mutableStateOf<GithubRepository?>(null) }
    var destination by remember { mutableStateOf("") }
    val visible = remember(state.githubRepositories, search) {
        state.githubRepositories.filter { it.fullName.contains(search.trim(), ignoreCase = true) }
    }
    Dialog(onDismissRequest = onDismiss, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
            Column(Modifier.fillMaxSize().safeDrawingPadding().imePadding()) {
                Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("GitHub repositories", Modifier.weight(1f), style = MaterialTheme.typography.titleMedium)
                    TextButton(onClick = onReconnect) { Text("Reconnect") }
                    IconButton(onClick = onDismiss) { Icon(Icons.Outlined.Close, "Close repository picker") }
                }
                OutlinedTextField(search, { search = it }, label = { Text("Search repositories") },
                    singleLine = true, modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 8.dp))
                if (state.repositoriesLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                LazyColumn(Modifier.weight(1f).fillMaxWidth()) {
                    items(visible, key = { it.id }) { repository ->
                        ListItem(
                            headlineContent = { Text(repository.fullName, maxLines = 2, overflow = TextOverflow.Ellipsis) },
                            supportingContent = { Text(if (repository.private) "Private" else "Public") },
                            leadingContent = { Icon(if (repository.private) Icons.Outlined.Lock else Icons.Outlined.Source, null) },
                            colors = ListItemDefaults.colors(containerColor = if (selected?.id == repository.id)
                                MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.background),
                            modifier = Modifier.clickable {
                                selected = repository
                                destination = repository.name
                            },
                        )
                    }
                    if (visible.isEmpty() && !state.repositoriesLoading && state.repositoriesError == null) {
                        item { Text("No matching repositories", Modifier.padding(16.dp), color = MaterialTheme.colorScheme.onSurfaceVariant) }
                    }
                    state.repositoriesError?.let { error ->
                        item {
                            Text(error, Modifier.padding(16.dp), color = MaterialTheme.colorScheme.error)
                            TextButton(onClick = onLoadMore) { Text("Retry") }
                        }
                    }
                    if (state.repositoriesHasMore && state.repositoriesError == null) {
                        item { TextButton(onClick = onLoadMore, enabled = !state.repositoriesLoading) { Text("Load more") } }
                    }
                }
                HorizontalDivider()
                selected?.let { repository ->
                    Column(Modifier.fillMaxWidth().padding(12.dp)) {
                        Text(repository.fullName, style = MaterialTheme.typography.bodySmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        OutlinedTextField(destination, { destination = it }, label = { Text("Local folder name") }, singleLine = true, modifier = Modifier.fillMaxWidth())
                        Button(onClick = { onClone(repository, destination) }, enabled = destination.isNotBlank(), modifier = Modifier.align(Alignment.End).padding(top = 8.dp)) {
                            Icon(Icons.Outlined.Source, null, Modifier.size(18.dp))
                            Spacer(Modifier.width(8.dp))
                            Text("Clone")
                        }
                    }
                }
            }
        }
    }
}
