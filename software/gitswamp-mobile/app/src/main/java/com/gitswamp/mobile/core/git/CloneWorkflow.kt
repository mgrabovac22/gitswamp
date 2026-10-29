package com.gitswamp.mobile.core.git

import com.gitswamp.mobile.core.model.CloneProgress
import com.gitswamp.mobile.core.model.CloneRequest
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import java.net.UnknownHostException
import java.net.SocketTimeoutException
import java.net.ConnectException
import java.io.File
import java.net.URI
import org.eclipse.jgit.errors.TransportException

data class CloneCredential(val username: String, val token: String)

sealed interface CloneResult {
    data class Success(val directory: File) : CloneResult
    data class AuthenticationRequired(val request: CloneRequest) : CloneResult
}

class CloneWorkflow(
    private val readCredential: (String) -> CloneCredential?,
    private val saveCredential: (String, CloneCredential) -> Unit,
    private val retryDelay: suspend (Long) -> Unit = { delay(it) },
    private val cloneRepository: suspend (CloneRequest, (CloneProgress) -> Unit) -> File,
) {
    suspend fun clone(request: CloneRequest, onProgress: (CloneProgress) -> Unit): CloneResult {
        val saved = if (request.token.isBlank()) readCredential(request.url) else null
        val attempt = saved?.let { request.copy(username = it.username, token = it.token) } ?: request
        val directory = try {
            cloneWithNetworkRetry(attempt, onProgress)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (error: Exception) {
            if (!requiresAuthentication(request.url, error)) throw error
            // Retain the clone destination for retry, but never retain a rejected token in UI state.
            return CloneResult.AuthenticationRequired(request.copy(token = ""))
        }
        if (request.rememberToken && request.token.isNotBlank()) {
            saveCredential(request.url, CloneCredential(request.username, request.token))
        }
        return CloneResult.Success(directory)
    }

    private suspend fun cloneWithNetworkRetry(request: CloneRequest, progress: (CloneProgress) -> Unit): File {
        for (attempt in 0..2) {
            try {
                return cloneRepository(request, progress)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                if (!isNetworkFailure(error) || attempt == 2) throw error
                progress(CloneProgress("Waiting for network / DNS; retry ${attempt + 1}/2", 0, 0))
                retryDelay(1500L * (attempt + 1))
            }
        }
        error("Clone retry limit exceeded")
    }

    companion object {
        fun isNetworkFailure(error: Throwable): Boolean = generateSequence(error) { it.cause }.take(8).any {
            it is UnknownHostException || it is SocketTimeoutException || it is ConnectException ||
                it.message.orEmpty().let { message ->
                    message.contains("unable to resolve host", true) || message.contains("no address associated", true)
                }
        }
    }

    private fun requiresAuthentication(url: String, error: Throwable): Boolean {
        if (isNetworkFailure(error)) return false
        if (!runCatching { URI(url.trim()).scheme.equals("https", true) }.getOrDefault(false)) return false
        val causes = generateSequence(error) { it.cause }.take(8).toList()
        val messages = causes.joinToString(" ") { it.message.orEmpty() }.lowercase()
        return listOf("not authorized", "unauthorized", "authentication", "credentials", "authorization",
            "access denied", "forbidden", "permission denied", "401", "403", "404", "repository not found")
            .any(messages::contains) || (causes.any { it is TransportException } && messages.contains("not found"))
    }
}
