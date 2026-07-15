package com.gitswamp.mobile.feature.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.CloudOff
import androidx.compose.material.icons.outlined.Memory
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.gitswamp.mobile.core.designsystem.InlineNotice
import com.gitswamp.mobile.core.designsystem.SwampColors
import com.gitswamp.mobile.feature.home.HomeUiState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsSheet(
    state: HomeUiState,
    onDismiss: () -> Unit,
    onUseSystemTheme: (Boolean) -> Unit,
    onAutoFetch: (Boolean) -> Unit,
    onCommitPageSize: (Int) -> Unit,
) {
    ModalBottomSheet(onDismissRequest = onDismiss, containerColor = MaterialTheme.colorScheme.surface) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 18.dp).padding(bottom = 28.dp)) {
            Text("Mobile settings", style = MaterialTheme.typography.titleLarge)
            Spacer(Modifier.height(4.dp))
            Text(
                "Small defaults keep repository switching responsive on phones.",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                style = MaterialTheme.typography.bodyMedium,
            )
            Spacer(Modifier.height(16.dp))
            HorizontalDivider(color = MaterialTheme.colorScheme.outline)
            ListItem(
                headlineContent = { Text("Follow system theme") },
                supportingContent = { Text("Use the light palette only when Android is in light mode.") },
                trailingContent = {
                    Switch(checked = state.useSystemTheme, onCheckedChange = onUseSystemTheme)
                },
            )
            ListItem(
                headlineContent = { Text("Fetch after opening") },
                supportingContent = { Text("Runs after cached local data is already visible.") },
                trailingContent = {
                    Switch(checked = state.autoFetchOnOpen, onCheckedChange = onAutoFetch)
                },
            )
            Spacer(Modifier.height(8.dp))
            Text("Initial graph size", style = MaterialTheme.typography.titleSmall)
            Spacer(Modifier.height(8.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf(100, 200, 500).forEach { size ->
                    FilterChip(
                        selected = state.commitPageSize == size,
                        onClick = { onCommitPageSize(size) },
                        label = { Text(size.toString()) },
                    )
                }
            }
            Spacer(Modifier.height(16.dp))
            InlineNotice(
                title = "No background account service",
                detail = "GitSwamp Mobile does not upload repository metadata, run analytics, or poll a GitSwamp server.",
                color = SwampColors.Green,
            )
            Spacer(Modifier.height(10.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Outlined.Memory, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(
                    "Only the visible repository page, selected diff, and current folder are retained in memory.",
                    modifier = Modifier.padding(start = 9.dp),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
    }
}
