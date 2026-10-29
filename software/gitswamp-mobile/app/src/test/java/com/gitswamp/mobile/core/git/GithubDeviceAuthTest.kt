package com.gitswamp.mobile.core.git

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GithubDeviceAuthTest {
    @Test
    fun `OAuth is offered only for HTTPS remotes on the exact GitHub host`() {
        assertTrue(GithubDeviceAuth.supports("https://github.com/team/repo.git"))
        assertTrue(GithubDeviceAuth.supports(" https://GITHUB.COM/team/repo "))
        assertFalse(GithubDeviceAuth.supports("http://github.com/team/repo.git"))
        assertFalse(GithubDeviceAuth.supports("https://github.com.evil.example/team/repo.git"))
        assertFalse(GithubDeviceAuth.supports("https://gitlab.com/team/repo.git"))
        assertFalse(GithubDeviceAuth.supports("git@github.com:team/repo.git"))
        assertFalse(GithubDeviceAuth.supports("not a URL"))
    }
}
