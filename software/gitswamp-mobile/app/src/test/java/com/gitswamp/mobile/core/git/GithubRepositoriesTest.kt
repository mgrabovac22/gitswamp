package com.gitswamp.mobile.core.git

import org.junit.Assert.*
import org.junit.Test

class GithubRepositoriesTest {
    @Test
    fun `repository selection retains owner and privacy`() {
        val repository = GithubRepository.create(42, "team/project", "https://github.com/team/project.git", true)!!
        assertEquals("project", repository.name)
        assertEquals("team/project", repository.fullName)
        assertTrue(repository.private)
    }

    @Test
    fun `repository list cannot send a GitHub token to another host or insecure remote`() {
        for (url in listOf("http://github.com/team/project.git", "https://github.com.evil.test/team/project.git",
            "https://evil.test/team/project.git", "https://user:secret@github.com/team/project.git",
            "https://github.com:444/team/project.git", "https://github.com/team/other.git")) {
            assertNull(url, GithubRepository.create(42, "team/project", url, false))
        }
    }

    @Test
    fun `repositories are deduplicated by stable GitHub identity across pages`() {
        val first = GithubRepository.create(1, "team/one", "https://github.com/team/one.git", false)!!
        val second = GithubRepository.create(2, "team/two", "https://github.com/team/two.git", false)!!
        assertEquals(listOf(first, second), (listOf(first) + listOf(first, second)).distinctBy { it.id })
    }
}
