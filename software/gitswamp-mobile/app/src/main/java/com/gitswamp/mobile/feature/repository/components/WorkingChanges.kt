package com.gitswamp.mobile.feature.repository.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.Description
import androidx.compose.material.icons.outlined.Remove
import androidx.compose.material.icons.outlined.Warning
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.gitswamp.mobile.core.designsystem.StatusPill
import com.gitswamp.mobile.core.designsystem.SwampColors
import com.gitswamp.mobile.core.designsystem.SwampPrimaryButton
import com.gitswamp.mobile.core.model.ChangeKind
import com.gitswamp.mobile.core.model.ChangedFile
import com.gitswamp.mobile.core.model.RepositoryStatus

@Composable
fun WorkingChanges(
    status: RepositoryStatus,
    busy: Boolean,
    onOpenDiff: (String, Boolean) -> Unit,
    onStage: (List<String>) -> Unit,
    onUnstage: (List<String>) -> Unit,
    onDiscard: (List<String>) -> Unit,
    onCommit: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var commitMessage by rememberSaveable { mutableStateOf("") }
    Column(modifier.fillMaxSize()) {
        if (status.isClean) {
            Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(Icons.Outlined.Check, contentDescription = null, tint = SwampColors.Green, modifier = Modifier.size(32.dp))
                    Spacer(Modifier.height(9.dp))
                    Text("Working tree is clean", fontWeight = FontWeight.SemiBold)
                    Text("There are no staged or unstaged changes.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 12.dp, vertical = 10.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                if (status.conflicts.isNotEmpty()) {
                    item { ChangeSectionHeader("Conflicts", status.conflicts.size, SwampColors.Amber, null, null) }
                    items(status.conflicts, key = { "conflict:${it.path}" }) { file ->
                        ChangedFileRow(file, onOpen = { onOpenDiff(file.path, false) })
                    }
                }
                if (status.unstaged.isNotEmpty()) {
                    item {
                        ChangeSectionHeader(
                            title = "Unstaged",
                            count = status.unstaged.size,
                            color = SwampColors.Amber,
                            primaryAction = "Stage all" to { onStage(status.unstaged.map { it.path }) },
                            destructiveAction = "Discard all" to { onDiscard(status.unstaged.map { it.path }) },
                        )
                    }
                    items(status.unstaged, key = { "unstaged:${it.path}" }) { file ->
                        ChangedFileRow(
                            file = file,
                            onOpen = { onOpenDiff(file.path, false) },
                            action = {
                                IconButton(onClick = { onStage(listOf(file.path)) }, enabled = !busy) {
                                    Icon(Icons.Outlined.Add, contentDescription = "Stage ${file.path}", tint = SwampColors.Green)
                                }
                                IconButton(onClick = { onDiscard(listOf(file.path)) }, enabled = !busy) {
                                    Icon(Icons.Outlined.DeleteOutline, contentDescription = "Discard unstaged changes in ${file.path}", tint = SwampColors.Red)
                                }
                            },
                        )
                    }
                }
                if (status.staged.isNotEmpty()) {
                    item {
                        ChangeSectionHeader(
                            title = "Staged",
                            count = status.staged.size,
                            color = SwampColors.Green,
                            primaryAction = "Unstage all" to { onUnstage(status.staged.map { it.path }) },
                            destructiveAction = null,
                        )
                    }
                    items(status.staged, key = { "staged:${it.path}" }) { file ->
                        ChangedFileRow(
                            file = file,
                            onOpen = { onOpenDiff(file.path, true) },
                            action = {
                                IconButton(onClick = { onUnstage(listOf(file.path)) }, enabled = !busy) {
                                    Icon(Icons.Outlined.Remove, contentDescription = "Unstage ${file.path}", tint = SwampColors.Amber)
                                }
                            },
                        )
                    }
                }
            }
        }

        Surface(
            color = MaterialTheme.colorScheme.surface,
            border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
        ) {
            Column(Modifier.fillMaxWidth().padding(12.dp)) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text("Commit", style = MaterialTheme.typography.titleSmall)
                    Text(
                        "${commitMessage.length}/72",
                        color = if (commitMessage.length > 72) SwampColors.Amber else MaterialTheme.colorScheme.onSurfaceVariant,
                        style = MaterialTheme.typography.labelSmall,
                    )
                }
                Spacer(Modifier.height(7.dp))
                OutlinedTextField(
                    value = commitMessage,
                    onValueChange = { commitMessage = it },
                    modifier = Modifier.fillMaxWidth(),
                    placeholder = { Text("Commit message") },
                    minLines = 2,
                    maxLines = 4,
                    shape = RoundedCornerShape(6.dp),
                )
                Spacer(Modifier.height(8.dp))
                SwampPrimaryButton(
                    text = "Commit staged changes",
                    onClick = {
                        onCommit(commitMessage)
                        commitMessage = ""
                    },
                    enabled = status.staged.isNotEmpty() && commitMessage.isNotBlank() && !busy,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}

@Composable
private fun ChangeSectionHeader(
    title: String,
    count: Int,
    color: Color,
    primaryAction: Pair<String, () -> Unit>?,
    destructiveAction: Pair<String, () -> Unit>?,
) {
    Row(
        Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(title, style = MaterialTheme.typography.titleSmall)
            StatusPill(count.toString(), color, Modifier.padding(start = 7.dp))
        }
        Row {
            destructiveAction?.let { action ->
                TextButton(onClick = action.second) { Text(action.first, color = SwampColors.Red) }
            }
            primaryAction?.let { action ->
                TextButton(onClick = action.second) { Text(action.first) }
            }
        }
    }
}

@Composable
private fun ChangedFileRow(
    file: ChangedFile,
    onOpen: () -> Unit,
    action: @Composable (() -> Unit)? = null,
) {
    Row(
        modifier = Modifier.fillMaxWidth()
            .clip(RoundedCornerShape(5.dp))
            .clickable(onClick = onOpen)
            .padding(start = 8.dp, top = 7.dp, bottom = 7.dp, end = 2.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        val color = changeColor(file.kind)
        Text(changeLetter(file.kind), color = color, fontWeight = FontWeight.Bold, style = MaterialTheme.typography.labelMedium)
        Icon(Icons.Outlined.Description, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(start = 8.dp).size(17.dp))
        Column(Modifier.weight(1f).padding(horizontal = 8.dp)) {
            Text(
                file.path.substringAfterLast('/'),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                style = MaterialTheme.typography.bodyMedium,
            )
            val folder = file.path.substringBeforeLast('/', "")
            if (folder.isNotEmpty()) {
                Text(folder, maxLines = 1, overflow = TextOverflow.Ellipsis, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
            }
        }
        action?.invoke()
    }
}

private fun changeLetter(kind: ChangeKind): String = when (kind) {
    ChangeKind.Added, ChangeKind.Untracked -> "A"
    ChangeKind.Modified -> "M"
    ChangeKind.Deleted -> "D"
    ChangeKind.Renamed -> "R"
    ChangeKind.Copied -> "C"
    ChangeKind.Conflicted -> "!"
}

private fun changeColor(kind: ChangeKind): Color = when (kind) {
    ChangeKind.Added, ChangeKind.Untracked, ChangeKind.Copied -> SwampColors.Green
    ChangeKind.Modified, ChangeKind.Renamed -> SwampColors.Amber
    ChangeKind.Deleted, ChangeKind.Conflicted -> SwampColors.Red
}
