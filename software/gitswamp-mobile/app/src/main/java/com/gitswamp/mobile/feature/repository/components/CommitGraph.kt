package com.gitswamp.mobile.feature.repository.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Warning
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.gitswamp.mobile.core.designsystem.StatusPill
import com.gitswamp.mobile.core.designsystem.SwampColors
import com.gitswamp.mobile.core.git.CommitGraphLayoutEngine
import com.gitswamp.mobile.core.git.CommitGraphRow
import com.gitswamp.mobile.core.model.CommitInfo
import com.gitswamp.mobile.core.model.RepositorySnapshot
import java.time.Duration
import java.time.Instant
import kotlin.math.max

@Composable
fun CommitGraph(
    snapshot: RepositorySnapshot,
    selectedSha: String?,
    refreshing: Boolean,
    onSelectCommit: (CommitInfo) -> Unit,
    onOpenChanges: () -> Unit,
    onLoadMore: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var query by rememberSaveable { mutableStateOf("") }
    val filtered = remember(snapshot.commits, query) {
        val normalized = query.trim().lowercase()
        if (normalized.isEmpty()) snapshot.commits else snapshot.commits.filter { commit ->
            commit.message.lowercase().contains(normalized) ||
                commit.authorName.lowercase().contains(normalized) ||
                commit.authorEmail.lowercase().contains(normalized) ||
                commit.sha.lowercase().startsWith(normalized) ||
                commit.refs.any { it.lowercase().contains(normalized) }
        }
    }
    val graph = remember(filtered) { CommitGraphLayoutEngine.layout(filtered) }
    val listState = rememberLazyListState()

    Column(modifier.fillMaxSize()) {
        OutlinedTextField(
            value = query,
            onValueChange = { query = it },
            modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp, vertical = 8.dp),
            placeholder = { Text("Search commits, authors, refs, SHA") },
            leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
            trailingIcon = if (query.isNotEmpty()) {
                { IconButton(onClick = { query = "" }) { Icon(Icons.Outlined.Close, contentDescription = "Clear search") } }
            } else null,
            singleLine = true,
            shape = RoundedCornerShape(6.dp),
        )
        if (refreshing) LinearProgressIndicator(Modifier.fillMaxWidth().height(2.dp))

        BoxWithConstraints(Modifier.fillMaxSize()) {
            val wide = maxWidth >= 680.dp
            Column(Modifier.fillMaxSize()) {
                GraphHeader(wide)
                if (!snapshot.status.isClean) {
                    WorkingChangesGraphRow(snapshot, onOpenChanges)
                }
                if (graph.rows.isEmpty()) {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Text(
                            if (query.isBlank()) "No commits on this repository yet" else "No commits match this search",
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                } else {
                    LazyColumn(
                        state = listState,
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        items(graph.rows, key = { it.commit.sha }) { row ->
                            CommitRow(
                                row = row,
                                laneCount = graph.laneCount,
                                selected = row.commit.sha == selectedSha,
                                wide = wide,
                                onClick = { onSelectCommit(row.commit) },
                            )
                        }
                        if (snapshot.hasMoreCommits && query.isBlank()) {
                            item(key = "load-more") {
                                TextButton(
                                    onClick = onLoadMore,
                                    modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
                                ) { Text("Load more commits") }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun GraphHeader(wide: Boolean) {
    Row(
        Modifier.fillMaxWidth()
            .background(MaterialTheme.colorScheme.background)
            .padding(horizontal = 10.dp, vertical = 5.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("GRAPH", Modifier.width(82.dp), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text("COMMIT MESSAGE", Modifier.weight(1f), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (wide) {
            Text("AUTHOR", Modifier.width(150.dp), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text("DATE", Modifier.width(110.dp), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Text("SHA", Modifier.width(60.dp), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun WorkingChangesGraphRow(snapshot: RepositorySnapshot, onClick: () -> Unit) {
    val count = snapshot.status.totalCount
    val conflicts = snapshot.status.conflicts.size
    Row(
        modifier = Modifier.fillMaxWidth()
            .background(SwampColors.Green.copy(alpha = 0.07f))
            .clickable(onClick = onClick)
            .padding(horizontal = 10.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(82.dp), contentAlignment = Alignment.Center) {
            Box(Modifier.size(9.dp).background(if (conflicts > 0) SwampColors.Amber else SwampColors.Green, CircleShape))
        }
        StatusPill("Working Changes", if (conflicts > 0) SwampColors.Amber else SwampColors.Green)
        Text(
            "$count changed ${if (count == 1) "file" else "files"}",
            modifier = Modifier.padding(start = 10.dp).weight(1f),
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            style = MaterialTheme.typography.bodySmall,
        )
        if (conflicts > 0) {
            Icon(Icons.Outlined.Warning, contentDescription = "$conflicts conflicts", tint = SwampColors.Amber, modifier = Modifier.size(17.dp))
        }
    }
}

@Composable
private fun CommitRow(
    row: CommitGraphRow,
    laneCount: Int,
    selected: Boolean,
    wide: Boolean,
    onClick: () -> Unit,
) {
    val background = when {
        selected -> MaterialTheme.colorScheme.surfaceVariant
        row.commit.parentShas.size > 1 -> MaterialTheme.colorScheme.primary.copy(alpha = 0.035f)
        else -> Color.Transparent
    }
    Row(
        modifier = Modifier.fillMaxWidth()
            .height(68.dp)
            .background(background)
            .clickable(onClick = onClick)
            .padding(horizontal = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        GraphRail(row, laneCount, Modifier.width(82.dp).height(68.dp))
        Column(Modifier.weight(1f).padding(end = 8.dp)) {
            if (row.commit.refs.isNotEmpty()) {
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    row.commit.refs.take(2).forEach { ref ->
                        StatusPill(ref, if (ref.startsWith("origin/")) SwampColors.Cyan else SwampColors.Primary)
                    }
                    if (row.commit.refs.size > 2) {
                        StatusPill("+${row.commit.refs.size - 2}", MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                Spacer(Modifier.height(3.dp))
            }
            Text(
                row.commit.message,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal,
            )
            if (!wide) {
                Text(
                    "${row.commit.authorName}  ${relativeTime(row.commit.authoredAt)}",
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
        if (wide) {
            Text(
                row.commit.authorName,
                Modifier.width(150.dp),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                style = MaterialTheme.typography.bodySmall,
            )
            Text(
                relativeTime(row.commit.authoredAt),
                Modifier.width(110.dp),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                style = MaterialTheme.typography.bodySmall,
            )
        }
        Text(
            row.commit.shortSha,
            Modifier.width(60.dp),
            color = MaterialTheme.colorScheme.primary,
            style = MaterialTheme.typography.labelSmall,
        )
    }
}

@Composable
private fun GraphRail(row: CommitGraphRow, laneCount: Int, modifier: Modifier = Modifier) {
    val colors = listOf(
        SwampColors.Primary,
        SwampColors.Cyan,
        SwampColors.Amber,
        SwampColors.Green,
        Color(0xFFF472B6),
        Color(0xFFFB7185),
    )
    val background = MaterialTheme.colorScheme.background
    Canvas(modifier) {
        val centerY = size.height / 2f
        val usable = size.width - 12.dp.toPx()
        val spacing = if (laneCount <= 1) 0f else (usable / max(1, laneCount - 1)).coerceAtMost(15.dp.toPx())
        fun x(lane: Int) = 6.dp.toPx() + lane * spacing
        fun color(lane: Int) = colors[lane % colors.size]

        row.continuingLanes.forEach { lane ->
            drawLine(color(lane).copy(alpha = 0.72f), Offset(x(lane), 0f), Offset(x(lane), size.height), 1.5.dp.toPx())
        }
        if (row.hasIncomingLine) {
            drawLine(color(row.lane), Offset(x(row.lane), 0f), Offset(x(row.lane), centerY), 2.dp.toPx())
        }
        row.outgoingEdges.forEach { edge ->
            val path = Path().apply {
                moveTo(x(edge.fromLane), centerY)
                cubicTo(
                    x(edge.fromLane),
                    centerY + size.height * 0.2f,
                    x(edge.toLane),
                    size.height * 0.72f,
                    x(edge.toLane),
                    size.height,
                )
            }
            drawPath(path, color(edge.toLane), style = Stroke(2.dp.toPx(), cap = StrokeCap.Round))
        }
        val nodeRadius = if (row.commit.parentShas.size > 1) 7.dp.toPx() else 5.5.dp.toPx()
        drawCircle(background, nodeRadius + 2.dp.toPx(), Offset(x(row.lane), centerY))
        drawCircle(color(row.lane), nodeRadius, Offset(x(row.lane), centerY))
        drawCircle(background, nodeRadius * 0.45f, Offset(x(row.lane), centerY))
        if (row.commit.parentShas.size > 1) {
            drawLine(color(row.lane), Offset(x(row.lane) - 3.dp.toPx(), centerY), Offset(x(row.lane) + 3.dp.toPx(), centerY), 1.5.dp.toPx())
            drawLine(color(row.lane), Offset(x(row.lane), centerY - 3.dp.toPx()), Offset(x(row.lane), centerY + 3.dp.toPx()), 1.5.dp.toPx())
        }
    }
}

private fun relativeTime(instant: Instant): String {
    val duration = Duration.between(instant, Instant.now()).coerceAtLeast(Duration.ZERO)
    return when {
        duration.toMinutes() < 1 -> "now"
        duration.toHours() < 1 -> "${duration.toMinutes()}m ago"
        duration.toDays() < 1 -> "${duration.toHours()}h ago"
        duration.toDays() < 30 -> "${duration.toDays()}d ago"
        duration.toDays() < 365 -> "${duration.toDays() / 30}mo ago"
        else -> "${duration.toDays() / 365}y ago"
    }
}
