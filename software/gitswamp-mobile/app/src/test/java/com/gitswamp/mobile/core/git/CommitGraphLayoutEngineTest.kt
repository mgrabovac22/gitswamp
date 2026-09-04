package com.gitswamp.mobile.core.git

import com.gitswamp.mobile.core.model.CommitInfo
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.Instant

class CommitGraphLayoutEngineTest {
    @Test
    fun `linear history stays in one lane`() {
        val layout = CommitGraphLayoutEngine.layout(
            listOf(commit("c", "b"), commit("b", "a"), commit("a")),
        )

        assertEquals(1, layout.laneCount)
        assertEquals(listOf(0, 0, 0), layout.rows.map { it.lane })
    }

    @Test
    fun `merge creates and rejoins a second lane`() {
        val layout = CommitGraphLayoutEngine.layout(
            listOf(
                commit("m", "left", "right"),
                commit("left", "base"),
                commit("right", "base"),
                commit("base"),
            ),
        )

        assertTrue(layout.laneCount >= 2)
        assertEquals(2, layout.rows.first().outgoingEdges.size)
        assertEquals(layout.rows[1].outgoingEdges.first().toLane, layout.rows[2].outgoingEdges.first().toLane)
    }

    private fun commit(sha: String, vararg parents: String) = CommitInfo(
        sha = sha,
        shortSha = sha,
        message = sha,
        authorName = "Test",
        authorEmail = "test@example.com",
        authoredAt = Instant.EPOCH,
        parentShas = parents.toList(),
        refs = emptyList(),
    )
}
