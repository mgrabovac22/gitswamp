package com.gitswamp.mobile.feature.repository.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ArrowUpward
import androidx.compose.material.icons.outlined.Code
import androidx.compose.material.icons.outlined.Description
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.gitswamp.mobile.core.designsystem.LoadingState
import com.gitswamp.mobile.core.designsystem.SwampColors
import com.gitswamp.mobile.core.model.RepositoryEntry

@Composable
fun FileBrowser(
    directory: String,
    entries: List<RepositoryEntry>,
    loading: Boolean,
    onNavigateUp: () -> Unit,
    onOpenEntry: (RepositoryEntry) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier.fillMaxSize()) {
        Breadcrumbs(directory, onNavigateUp)
        when {
            loading -> LoadingState("Reading repository files", Modifier.fillMaxSize())
            entries.isEmpty() -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("This folder is empty", color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            else -> LazyColumn(Modifier.fillMaxSize()) {
                items(entries, key = { it.relativePath }) { entry ->
                    Row(
                        Modifier.fillMaxWidth().clickable { onOpenEntry(entry) }.padding(horizontal = 14.dp, vertical = 11.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(
                            if (entry.isDirectory) Icons.Outlined.Folder else Icons.Outlined.Description,
                            contentDescription = null,
                            tint = if (entry.isDirectory) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.size(19.dp),
                        )
                        Text(
                            entry.name,
                            modifier = Modifier.weight(1f).padding(start = 10.dp),
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            fontWeight = if (entry.isDirectory) FontWeight.SemiBold else FontWeight.Normal,
                        )
                        if (!entry.isDirectory) {
                            Text(formatBytes(entry.sizeBytes), color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun Breadcrumbs(directory: String, onNavigateUp: () -> Unit) {
    val scroll = rememberScrollState()
    Row(
        Modifier.fillMaxWidth().horizontalScroll(scroll).padding(horizontal = 8.dp, vertical = 5.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        IconButton(onClick = onNavigateUp, enabled = directory.isNotBlank()) {
            Icon(Icons.Outlined.ArrowUpward, contentDescription = "Parent folder")
        }
        Icon(Icons.Outlined.Code, contentDescription = null, tint = SwampColors.Green, modifier = Modifier.size(17.dp))
        Text("root", modifier = Modifier.padding(horizontal = 6.dp), fontWeight = FontWeight.SemiBold)
        if (directory.isNotBlank()) {
            directory.split('/').forEach { segment ->
                Text("/", color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(segment, modifier = Modifier.padding(horizontal = 6.dp), style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}

private fun formatBytes(bytes: Long): String = when {
    bytes < 1_024 -> "$bytes B"
    bytes < 1_048_576 -> "${bytes / 1_024} KB"
    else -> "${"%.1f".format(bytes / 1_048_576.0)} MB"
}

