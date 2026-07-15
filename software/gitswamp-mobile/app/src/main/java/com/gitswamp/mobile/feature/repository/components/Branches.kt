package com.gitswamp.mobile.feature.repository.components

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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.CallSplit
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Cloud
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.gitswamp.mobile.core.designsystem.StatusPill
import com.gitswamp.mobile.core.designsystem.SwampColors
import com.gitswamp.mobile.core.designsystem.SwampPrimaryButton
import com.gitswamp.mobile.core.model.BranchInfo

@Composable
fun Branches(
    branches: List<BranchInfo>,
    busy: Boolean,
    onCheckout: (String) -> Unit,
    onCreate: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var query by rememberSaveable { mutableStateOf("") }
    var showCreate by remember { mutableStateOf(false) }
    val filtered = remember(branches, query) {
        val normalized = query.trim().lowercase()
        if (normalized.isBlank()) branches else branches.filter { it.name.lowercase().contains(normalized) }
    }
    val local = filtered.filterNot { it.isRemote }
    val remote = filtered.filter { it.isRemote }

    Column(modifier.fillMaxSize()) {
        Row(Modifier.fillMaxWidth().padding(10.dp), verticalAlignment = Alignment.CenterVertically) {
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                modifier = Modifier.weight(1f),
                placeholder = { Text("Filter branches") },
                leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                singleLine = true,
            )
            SwampPrimaryButton(
                text = "New",
                onClick = { showCreate = true },
                modifier = Modifier.padding(start = 8.dp),
                leading = { Icon(Icons.Outlined.Add, contentDescription = null, Modifier.size(17.dp)) },
            )
        }
        LazyColumn(Modifier.fillMaxSize()) {
            item { BranchSectionHeader("LOCAL", local.size, Icons.AutoMirrored.Outlined.CallSplit) }
            items(local, key = { "local:${it.name}" }) { branch ->
                BranchRow(branch, busy, onCheckout)
            }
            if (remote.isNotEmpty()) {
                item { BranchSectionHeader("REMOTE", remote.size, Icons.Outlined.Cloud) }
                items(remote, key = { "remote:${it.name}" }) { branch ->
                    BranchRow(branch, busy, onCheckout)
                }
            }
        }
    }

    if (showCreate) {
        CreateBranchDialog(
            onDismiss = { showCreate = false },
            onCreate = {
                showCreate = false
                onCreate(it)
            },
        )
    }
}

@Composable
private fun BranchSectionHeader(
    title: String,
    count: Int,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 9.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(icon, contentDescription = null, tint = SwampColors.Green, modifier = Modifier.size(17.dp))
        Text(title, Modifier.padding(start = 8.dp), fontWeight = FontWeight.SemiBold, style = MaterialTheme.typography.labelLarge)
        StatusPill(count.toString(), MaterialTheme.colorScheme.primary, Modifier.padding(start = 8.dp))
    }
}

@Composable
private fun BranchRow(branch: BranchInfo, busy: Boolean, onCheckout: (String) -> Unit) {
    Row(
        Modifier.fillMaxWidth().clickable(enabled = !busy && !branch.isCurrent) { onCheckout(branch.name) }.padding(horizontal = 18.dp, vertical = 11.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(8.dp), contentAlignment = Alignment.Center) {
            if (branch.isCurrent) {
                androidx.compose.foundation.Canvas(Modifier.fillMaxSize()) { drawCircle(SwampColors.Green) }
            }
        }
        Column(Modifier.weight(1f).padding(horizontal = 10.dp)) {
            Text(branch.name, maxLines = 1, overflow = TextOverflow.Ellipsis, fontWeight = if (branch.isCurrent) FontWeight.SemiBold else FontWeight.Normal)
            branch.upstream?.let {
                Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall, maxLines = 1)
            }
        }
        if (branch.ahead > 0) StatusPill("+${branch.ahead}", SwampColors.Green)
        if (branch.behind > 0) StatusPill("-${branch.behind}", SwampColors.Amber, Modifier.padding(start = 4.dp))
        if (branch.isCurrent) StatusPill("current", SwampColors.Green, Modifier.padding(start = 5.dp))
    }
}

@Composable
private fun CreateBranchDialog(onDismiss: () -> Unit, onCreate: (String) -> Unit) {
    var name by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Create branch") },
        text = {
            Column {
                Text("The new branch starts at the current HEAD.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                Spacer(Modifier.height(9.dp))
                OutlinedTextField(name, { name = it }, label = { Text("Branch name") }, singleLine = true, modifier = Modifier.fillMaxWidth())
            }
        },
        confirmButton = { TextButton(onClick = { onCreate(name.trim()) }, enabled = name.isNotBlank()) { Text("Create") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
