package com.gitswamp.mobile.feature.repository.components

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Description
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.gitswamp.mobile.core.designsystem.InlineNotice
import com.gitswamp.mobile.core.designsystem.MonoFontFamily
import com.gitswamp.mobile.core.designsystem.SwampColors
import com.gitswamp.mobile.core.model.DiffLine
import com.gitswamp.mobile.core.model.DiffLineKind
import com.gitswamp.mobile.core.model.FileDiff

@Composable
fun DiffViewer(
    diff: FileDiff,
    onClose: () -> Unit,
) {
    val horizontalScroll = rememberScrollState()
    Dialog(
        onDismissRequest = onClose,
        properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false),
    ) {
        Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
            Column(Modifier.fillMaxSize()) {
                Row(
                    Modifier.fillMaxWidth().padding(start = 12.dp, end = 4.dp, top = 30.dp, bottom = 7.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(Icons.Outlined.Description, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                    Column(Modifier.weight(1f).padding(horizontal = 9.dp)) {
                        Text(diff.path.substringAfterLast('/'), fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        Text(diff.path, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                    IconButton(onClick = onClose) { Icon(Icons.Outlined.Close, contentDescription = "Close diff") }
                }
                HorizontalDivider(color = MaterialTheme.colorScheme.outline)
                if (diff.binary) {
                    Box(Modifier.fillMaxSize().padding(18.dp), contentAlignment = Alignment.Center) {
                        InlineNotice("Binary file", "This file cannot be displayed as a text diff.", SwampColors.Amber)
                    }
                } else {
                    if (diff.truncated) {
                        InlineNotice(
                            title = "Preview limited",
                            detail = "The diff was capped to keep memory use predictable. Open it on desktop for the complete patch.",
                            color = SwampColors.Amber,
                            modifier = Modifier.padding(10.dp),
                        )
                    }
                    LazyColumn(Modifier.fillMaxSize().background(Color(0xFF0D1117))) {
                        itemsIndexed(diff.lines, key = { index, _ -> index }) { _, line ->
                            DiffLineRow(line, horizontalScroll)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun DiffLineRow(line: DiffLine, scroll: androidx.compose.foundation.ScrollState) {
    val background = when (line.kind) {
        DiffLineKind.Added -> SwampColors.DiffAddedBackground
        DiffLineKind.Deleted -> SwampColors.DiffDeletedBackground
        DiffLineKind.Header -> Color(0xFF161B22)
        DiffLineKind.Context -> Color.Transparent
    }
    val foreground = when (line.kind) {
        DiffLineKind.Added -> SwampColors.DiffAdded
        DiffLineKind.Deleted -> SwampColors.DiffDeleted
        DiffLineKind.Header -> Color(0xFF8B949E)
        DiffLineKind.Context -> Color(0xFFC9D1D9)
    }
    Row(
        Modifier.fillMaxWidth().background(background).horizontalScroll(scroll).padding(vertical = 2.dp),
    ) {
        Text(
            line.oldLine?.toString().orEmpty(),
            Modifier.width(42.dp).padding(end = 5.dp),
            color = Color(0xFF6E7681),
            fontFamily = MonoFontFamily,
            style = MaterialTheme.typography.bodySmall,
        )
        Text(
            line.newLine?.toString().orEmpty(),
            Modifier.width(42.dp).padding(end = 5.dp),
            color = Color(0xFF6E7681),
            fontFamily = MonoFontFamily,
            style = MaterialTheme.typography.bodySmall,
        )
        Text(
            when (line.kind) {
                DiffLineKind.Added -> "+"
                DiffLineKind.Deleted -> "-"
                else -> " "
            },
            Modifier.width(16.dp),
            color = foreground,
            fontFamily = MonoFontFamily,
            style = MaterialTheme.typography.bodySmall,
        )
        Text(
            line.text,
            color = foreground,
            fontFamily = MonoFontFamily,
            style = MaterialTheme.typography.bodySmall,
            softWrap = false,
        )
    }
}
