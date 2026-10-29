package com.gitswamp.mobile.core.git

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import java.io.IOException

data class DeviceTokenReply(val token: String? = null, val error: String? = null, val interval: Long? = null)
class TemporaryGithubFailure : IOException("GitHub is temporarily unavailable.")
class DeviceAuthorizationEnded(message: String) : IllegalStateException(message)

class DeviceTokenPoller(
    private val now: () -> Long = System::currentTimeMillis,
    private val sleep: suspend (Long) -> Unit = { delay(it) },
) {
    suspend fun awaitToken(
        expiresAtMillis: Long,
        initialInterval: Long,
        onStatus: (String) -> Unit,
        onIntervalChanged: (Long) -> Unit = {},
        poll: suspend () -> DeviceTokenReply,
    ): String {
        var interval = initialInterval.coerceAtLeast(5)
        var networkBackoff = 0L
        while (now() < expiresAtMillis) {
            sleep(minOf(maxOf(interval, networkBackoff) * 1000, expiresAtMillis - now()).coerceAtLeast(0))
            if (now() >= expiresAtMillis) break
            val response = try {
                poll()
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                if (!CloneWorkflow.isNetworkFailure(error) && error !is TemporaryGithubFailure) throw error
                networkBackoff = if (networkBackoff == 0L) 5 else (networkBackoff * 2).coerceAtMost(30)
                onStatus("Waiting for network / DNS. Your GitHub authorization is retained; no new code is needed.")
                continue
            }
            networkBackoff = 0
            response.token?.takeIf(String::isNotBlank)?.let { return it }
            when (response.error) {
                "authorization_pending" -> onStatus("Waiting for GitHub approval...")
                "slow_down" -> {
                    interval = maxOf(interval + 5, response.interval ?: 0)
                    onIntervalChanged(interval)
                }
                "access_denied" -> throw DeviceAuthorizationEnded("GitHub sign-in was declined.")
                "expired_token" -> break
                else -> throw DeviceAuthorizationEnded("GitHub could not complete this sign-in. Please start again.")
            }
        }
        throw DeviceAuthorizationEnded("The GitHub code expired. Start sign-in again to get a new code.")
    }
}
