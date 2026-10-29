package com.gitswamp.mobile.core.git

import com.gitswamp.mobile.core.model.CloneRequest
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.runBlocking
import org.eclipse.jgit.errors.TransportException
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.io.IOException
import java.net.UnknownHostException

class CloneWorkflowTest {
    private val request = CloneRequest("https://github.com/team/private.git", "my-project", "git", "", true)

    @Test
    fun `DNS failure after OAuth retries the same authorized request`() = runBlocking {
        val tokens = mutableListOf<String>()
        val waits = mutableListOf<Long>()
        val flow = CloneWorkflow({ CloneCredential("x-access-token", "authorized") }, { _, _ -> }, retryDelay = { waits += it }) { attempt, _ ->
            tokens += attempt.token
            if (tokens.size < 3) throw IOException("transport failed", UnknownHostException("github.com"))
            File("my-project")
        }
        assertTrue(flow.clone(request) {} is CloneResult.Success)
        assertEquals(listOf("authorized", "authorized", "authorized"), tokens)
        assertEquals(listOf(1500L, 3000L), waits)
    }

    @Test
    fun `persistent DNS failure is bounded and never asks for a new token`() = runBlocking {
        var attempts = 0
        val flow = CloneWorkflow({ null }, { _, _ -> }, retryDelay = {}) { _, _ ->
            attempts++
            throw UnknownHostException("Unable to resolve host github.com: No address associated with hostname")
        }
        val error = runCatching { flow.clone(request) {} }.exceptionOrNull()
        assertTrue(error is UnknownHostException)
        assertTrue(CloneWorkflow.isNetworkFailure(error!!))
        assertEquals(3, attempts)
    }

    @Test
    fun `an authentication wrapper around DNS failure must not request another OAuth`() = runBlocking {
        val flow = CloneWorkflow({ CloneCredential("git", "approved-token") }, { _, _ -> }, retryDelay = {}) { _, _ ->
            throw IOException("Authentication transport failed", UnknownHostException("github.com"))
        }
        val error = runCatching { flow.clone(request) {} }.exceptionOrNull()
        assertTrue(error is IOException)
        assertTrue(CloneWorkflow.isNetworkFailure(error!!))
    }

    @Test
    fun `public clone is attempted without requiring authentication`() = runBlocking {
        var attempts = 0
        val flow = CloneWorkflow({ null }, { _, _ -> fail("no credentials to save") }) { attempt, _ ->
            attempts++
            assertEquals("", attempt.token)
            File("my-project")
        }
        assertTrue(flow.clone(request) {} is CloneResult.Success)
        assertEquals(1, attempts)
    }

    @Test
    fun `saved token is reused and explicit replacement takes priority`() = runBlocking {
        val attempts = mutableListOf<CloneRequest>()
        val flow = CloneWorkflow({ CloneCredential("saved-user", "saved-token") }, { _, _ -> }) { attempt, _ ->
            attempts += attempt
            File("my-project")
        }
        flow.clone(request) {}
        flow.clone(request.copy(username = "new-user", token = "replacement")) {}
        assertEquals("saved-token", attempts[0].token)
        assertEquals("saved-user", attempts[0].username)
        assertEquals("replacement", attempts[1].token)
        assertEquals("new-user", attempts[1].username)
    }

    @Test
    fun `auth failure preserves destination and retry resumes clone with new credentials`() = runBlocking {
        val attempts = mutableListOf<CloneRequest>()
        val saved = mutableListOf<CloneCredential>()
        val flow = CloneWorkflow({ CloneCredential("old", "rejected") }, { _, credential -> saved += credential }) { attempt, _ ->
            attempts += attempt
            if (attempt.token != "authorized") throw TransportException("Authentication is required")
            File("my-project")
        }
        val pending = flow.clone(request) {} as CloneResult.AuthenticationRequired
        assertEquals(request.url, pending.request.url)
        assertEquals("my-project", pending.request.destinationName)
        assertEquals("", pending.request.token)
        assertTrue(saved.isEmpty())
        assertTrue(flow.clone(pending.request.copy(username = "x-access-token", token = "authorized")) {} is CloneResult.Success)
        assertEquals(2, attempts.size)
        assertEquals(listOf(CloneCredential("x-access-token", "authorized")), saved)
    }

    @Test
    fun `private repository not found opens authentication instead of ending clone`() = runBlocking {
        val flow = CloneWorkflow({ null }, { _, _ -> }) { _, _ ->
            throw TransportException("https://github.com/team/private.git: not found.")
        }
        assertTrue(flow.clone(request) {} is CloneResult.AuthenticationRequired)
    }

    @Test
    fun `network and local destination errors do not open authentication`() = runBlocking {
        for (message in listOf("Connection timed out", "A repository named my-project already exists", "Directory not found")) {
            val flow = CloneWorkflow({ null }, { _, _ -> }) { _, _ -> throw IOException(message) }
            val result = runCatching { flow.clone(request) {} }
            assertEquals(message, result.exceptionOrNull()?.message)
        }
    }

    @Test
    fun `unchecked remember option does not persist credentials`() = runBlocking {
        val flow = CloneWorkflow({ null }, { _, _ -> fail("must not save") }) { _, _ -> File("my-project") }
        assertTrue(flow.clone(request.copy(token = "one-off", rememberToken = false)) {} is CloneResult.Success)
    }

    @Test
    fun `cancelled clone does not become an auth prompt`() = runBlocking {
        val flow = CloneWorkflow({ null }, { _, _ -> }) { _, _ -> throw CancellationException("cancelled") }
        assertTrue(runCatching { flow.clone(request) {} }.exceptionOrNull() is CancellationException)
    }
}
