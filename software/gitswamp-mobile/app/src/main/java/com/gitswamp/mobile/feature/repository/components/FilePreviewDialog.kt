package com.gitswamp.mobile.feature.repository.components

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.gitswamp.mobile.core.designsystem.MonoFontFamily
import com.gitswamp.mobile.feature.repository.FilePreview

@Composable
fun FilePreviewDialog(preview: FilePreview, onClose: () -> Unit) {
    val horizontal = rememberScrollState()
    val lines = preview.content.lines()
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
                        Text(preview.path.substringAfterLast('/'), fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        Text(preview.path, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                    IconButton(onClick = onClose) { Icon(Icons.Outlined.Close, contentDescription = "Close file preview") }
                }
                HorizontalDivider(color = MaterialTheme.colorScheme.outline)
                LazyColumn(Modifier.fillMaxSize().horizontalScroll(horizontal)) {
                    itemsIndexed(lines) { index, line ->
                        Row(Modifier.padding(vertical = 1.dp)) {
                            Text(
                                (index + 1).toString(),
                                Modifier.padding(horizontal = 8.dp),
                                color = Color(0xFF64748B),
                                fontFamily = MonoFontFamily,
                                fontSize = 11.sp,
                            )
                            Text(line, color = MaterialTheme.colorScheme.onBackground, fontFamily = MonoFontFamily, fontSize = 11.sp, softWrap = false)
                        }
                    }
                }
            }
        }
    }
}

