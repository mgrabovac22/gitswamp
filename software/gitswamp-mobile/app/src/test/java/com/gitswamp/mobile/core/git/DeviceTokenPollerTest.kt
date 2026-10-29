package com.gitswamp.mobile.core.git

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import java.net.UnknownHostException

class DeviceTokenPollerTest {
    private class Clock {
        var time = 0L
        val poller = DeviceTokenPoller(now = { time }, sleep = { time += it })
    }

    @Test
    fun `approved OAuth survives DNS failures while exchanging the same code`() = runBlocking {
        val clock = Clock()
        val statuses = mutableListOf<String>()
        var calls = 0
        val token = clock.poller.awaitToken(900_000, 5, statuses::add) {
            calls++
            if (calls < 3) throw UnknownHostException("Unable to resolve host github.com")
            DeviceTokenReply(token = "approved-token")
        }
        assertEquals("approved-token", token)
        assertEquals(3, calls)
        assertEquals(2, statuses.size)
        assertTrue(statuses.all { it.contains("authorization is retained") })
    }

    @Test
    fun `persistent network outage ends only at original code expiry`() = runBlocking {
        val clock = Clock()
        var calls = 0
        val error = runCatching {
            clock.poller.awaitToken(60_000, 5, {}) {
                calls++
                throw UnknownHostException("github.com")
            }
        }.exceptionOrNull()
        assertTrue(error is DeviceAuthorizationEnded)
        assertTrue(error!!.message!!.contains("expired"))
        assertEquals(60_000L, clock.time)
        assertTrue(calls in 1..10)
    }

    @Test
    fun `GitHub slow down is respected and saved for session resumption`() = runBlocking {
        val clock = Clock()
        val calls = mutableListOf<Long>()
        val intervals = mutableListOf<Long>()
        clock.poller.awaitToken(900_000, 5, {}, intervals::add) {
            calls += clock.time
            when (calls.size) {
                1 -> DeviceTokenReply(error = "authorization_pending")
                2 -> DeviceTokenReply(error = "slow_down", interval = 12)
                3 -> DeviceTokenReply(error = "authorization_pending")
                else -> DeviceTokenReply(token = "token")
            }
        }
        assertEquals(listOf(5000L, 10000L, 22000L, 34000L), calls)
        assertEquals(listOf(12L), intervals)
    }

    @Test
    fun `resuming a stored session never extends its expiry`() = runBlocking {
        val clock = Clock().apply { time = 55_000 }
        var calls = 0
        val error = runCatching {
            clock.poller.awaitToken(60_000, 5, {}) { calls++; DeviceTokenReply(token = "too-late") }
        }.exceptionOrNull()
        assertTrue(error is DeviceAuthorizationEnded)
        assertEquals(0, calls)
    }

    @Test
    fun `temporary GitHub outage retains authorization`() = runBlocking {
        val clock = Clock()
        var calls = 0
        val token = clock.poller.awaitToken(900_000, 5, {}) {
            if (++calls == 1) throw TemporaryGithubFailure()
            DeviceTokenReply(token = "token")
        }
        assertEquals("token", token)
        assertEquals(2, calls)
    }

    @Test
    fun `denied authorization terminates rather than retrying`() = runBlocking {
        val clock = Clock()
        val error = runCatching {
            clock.poller.awaitToken(900_000, 5, {}) { DeviceTokenReply(error = "access_denied") }
        }.exceptionOrNull()
        assertTrue(error is DeviceAuthorizationEnded)
        assertTrue(error!!.message!!.contains("declined"))
        assertEquals(5000L, clock.time)
    }

    @Test
    fun `leaving the screen cancels polling without rejecting the authorization`() = runBlocking {
        val clock = Clock()
        val error = runCatching {
            clock.poller.awaitToken(900_000, 5, {}) { throw CancellationException("screen disposed") }
        }.exceptionOrNull()
        assertTrue(error is CancellationException)
    }
}
