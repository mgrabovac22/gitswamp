package com.gitswamp.mobile.feature.repository.components

import android.content.ClipData
import android.content.ClipboardManager
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.CalendarMonth
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Description
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Tab
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.gitswamp.mobile.core.designsystem.LoadingState
import com.gitswamp.mobile.core.designsystem.StatusPill
import com.gitswamp.mobile.core.designsystem.SwampColors
import com.gitswamp.mobile.core.model.ChangeKind
import com.gitswamp.mobile.core.model.CommitDetails
import com.gitswamp.mobile.core.model.CommitInfo
import java.time.ZoneId
import java.time.format.DateTimeFormatter

@Composable
fun CommitDetailsPanel(
    commit: CommitInfo,
    details: CommitDetails?,
    loading: Boolean,
    onClose: () -> Unit,
    onOpenDiff: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var selectedTab by remember(commit.sha) { mutableIntStateOf(0) }
    Column(modifier.fillMaxSize()) {
        Row(
            Modifier.fillMaxWidth().padding(start = 14.dp, end = 5.dp, top = 8.dp, bottom = 7.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text("Commit details", style = MaterialTheme.typography.titleSmall)
                Text(commit.shortSha, color = MaterialTheme.colorScheme.primary, style = MaterialTheme.typography.labelSmall)
            }
            IconButton(onClick = onClose) {
                Icon(Icons.Outlined.Close, contentDescription = "Close commit details")
            }
        }
        PrimaryTabRow(selectedTabIndex = selectedTab, containerColor = MaterialTheme.colorScheme.surface) {
            Tab(selected = selectedTab == 0, onClick = { selectedTab = 0 }, text = { Text("Info") })
            Tab(selected = selectedTab == 1, onClick = { selectedTab = 1 }, text = { Text("Changes ${details?.files?.size ?: ""}") })
        }
        when {
            loading -> LoadingState("Loading commit details", Modifier.fillMaxSize())
            selectedTab == 0 -> CommitInfoContent(commit)
            else -> CommitFilesContent(details, onOpenDiff)
        }
    }
}

@Composable
private fun CommitInfoContent(commit: CommitInfo) {
    val context = LocalContext.current
    val clipboard = remember(context) { context.getSystemService(ClipboardManager::class.java) }
    LazyColumn(
        Modifier.fillMaxSize(),
        contentPadding = androidx.compose.foundation.layout.PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(13.dp),
    ) {
        item {
            Text(commit.message, style = MaterialTheme.typography.titleMedium)
        }
        if (commit.refs.isNotEmpty()) {
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                    commit.refs.take(4).forEach { ref -> StatusPill(ref, SwampColors.Primary) }
                }
            }
        }
        item { HorizontalDivider(color = MaterialTheme.colorScheme.outline) }
        item {
            MetadataRow(Icons.Outlined.Person, "Author", commit.authorName, commit.authorEmail)
        }
        item {
            val formatted = DateTimeFormatter.ofPattern("MMM d, yyyy  HH:mm")
                .withZone(ZoneId.systemDefault())
                .format(commit.authoredAt)
            MetadataRow(Icons.Outlined.CalendarMonth, "Authored", formatted, null)
        }
        item {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("Commit SHA", color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.labelSmall)
                    Text(commit.sha, color = MaterialTheme.colorScheme.primary, style = MaterialTheme.typography.bodySmall)
                }
                IconButton(onClick = {
                    clipboard.setPrimaryClip(ClipData.newPlainText("Commit SHA", commit.sha))
                }) {
                    Icon(Icons.Outlined.ContentCopy, contentDescription = "Copy commit SHA", Modifier.size(18.dp))
                }
            }
        }
        if (commit.parentShas.isNotEmpty()) {
            item {
                Column {
                    Text("Parents", color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.labelSmall)
                    commit.parentShas.forEach { parent -> Text(parent.take(12), style = MaterialTheme.typography.bodySmall) }
                }
            }
        }
    }
}

@Composable
private fun MetadataRow(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    label: String,
    value: String,
    detail: String?,
) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.size(18.dp))
        Column(Modifier.padding(start = 10.dp)) {
            Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.labelSmall)
            Text(value, style = MaterialTheme.typography.bodyMedium)
            detail?.takeIf { it.isNotBlank() }?.let {
                Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}

@Composable
private fun CommitFilesContent(details: CommitDetails?, onOpenDiff: (String) -> Unit) {
    val files = details?.files.orEmpty()
    if (files.isEmpty()) {
        Column(Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
            Text("No file changes in this commit", color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        return
    }
    LazyColumn(Modifier.fillMaxSize()) {
        items(files, key = { it.path }) { file ->
            Row(
                Modifier.fillMaxWidth().clickable { onOpenDiff(file.path) }.padding(horizontal = 14.dp, vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(changeLetter(file.kind), color = changeColor(file.kind), fontWeight = FontWeight.Bold)
                Icon(Icons.Outlined.Description, contentDescription = null, modifier = Modifier.padding(start = 9.dp).size(17.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                Column(Modifier.weight(1f).padding(horizontal = 8.dp)) {
                    Text(file.path.substringAfterLast('/'), maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(
                        file.path.substringBeforeLast('/', ""),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        style = MaterialTheme.typography.bodySmall,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                if (file.additions > 0) Text("+${file.additions}", color = SwampColors.Green, style = MaterialTheme.typography.labelSmall)
                if (file.deletions > 0) Text(" -${file.deletions}", color = SwampColors.Red, style = MaterialTheme.typography.labelSmall)
            }
        }
    }
}

private fun changeLetter(kind: ChangeKind) = when (kind) {
    ChangeKind.Added, ChangeKind.Untracked -> "A"
    ChangeKind.Modified -> "M"
    ChangeKind.Deleted -> "D"
    ChangeKind.Renamed -> "R"
    ChangeKind.Copied -> "C"
    ChangeKind.Conflicted -> "!"
}

private fun changeColor(kind: ChangeKind) = when (kind) {
    ChangeKind.Added, ChangeKind.Untracked, ChangeKind.Copied -> SwampColors.Green
    ChangeKind.Modified, ChangeKind.Renamed -> SwampColors.Amber
    ChangeKind.Deleted, ChangeKind.Conflicted -> SwampColors.Red
}
