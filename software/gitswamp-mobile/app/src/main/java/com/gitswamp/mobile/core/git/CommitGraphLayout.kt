package com.gitswamp.mobile.core.git

import com.gitswamp.mobile.core.model.CommitInfo

data class GraphEdgeLayout(
    val fromLane: Int,
    val toLane: Int,
)

data class CommitGraphRow(
    val commit: CommitInfo,
    val lane: Int,
    val hasIncomingLine: Boolean,
    val continuingLanes: List<Int>,
    val outgoingEdges: List<GraphEdgeLayout>,
)

data class CommitGraphLayout(
    val rows: List<CommitGraphRow>,
    val laneCount: Int,
)

object CommitGraphLayoutEngine {
    fun layout(commits: List<CommitInfo>): CommitGraphLayout {
        val active = mutableListOf<String?>()
        val rows = ArrayList<CommitGraphRow>(commits.size)
        var maxLaneCount = 1

        commits.forEach { commit ->
            val existingLane = active.indexOf(commit.sha)
            val lane = if (existingLane >= 0) existingLane else firstFreeLane(active)
            ensureLane(active, lane)
            val hasIncoming = existingLane >= 0
            val continuing = active.indices.filter { index -> index != lane && active[index] != null }
            active[lane] = null

            val edges = buildList {
                commit.parentShas.forEachIndexed { parentIndex, parentSha ->
                    val existingParentLane = active.indexOf(parentSha)
                    val targetLane = when {
                        existingParentLane >= 0 -> existingParentLane
                        parentIndex == 0 && active[lane] == null -> lane
                        else -> firstFreeLane(active)
                    }
                    ensureLane(active, targetLane)
                    if (active[targetLane] == null) active[targetLane] = parentSha
                    add(GraphEdgeLayout(lane, targetLane))
                }
            }

            while (active.size > 1 && active.last() == null) active.removeAt(active.lastIndex)
            maxLaneCount = maxOf(maxLaneCount, active.size, lane + 1, edges.maxOfOrNull { it.toLane + 1 } ?: 1)
            rows += CommitGraphRow(commit, lane, hasIncoming, continuing, edges)
        }

        return CommitGraphLayout(rows, maxLaneCount)
    }

    private fun firstFreeLane(active: List<String?>): Int {
        val free = active.indexOfFirst { it == null }
        return if (free >= 0) free else active.size
    }

    private fun ensureLane(active: MutableList<String?>, lane: Int) {
        while (active.size <= lane) active += null
    }
}

