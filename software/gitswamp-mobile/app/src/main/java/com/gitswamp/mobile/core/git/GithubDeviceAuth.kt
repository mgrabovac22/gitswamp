package com.gitswamp.mobile.core.git

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URI
import java.net.URL
import java.net.URLEncoder

object GithubDeviceAuth {
    // Public OAuth app identifier, shared with the desktop device flow (not a secret).
    private const val CLIENT_ID = "178c6fc778ccc68e1d6a"

    data class DeviceCode(
        val deviceCode: String, val userCode: String, val expiresIn: Long, val interval: Long,
        val expiresAtMillis: Long = System.currentTimeMillis() + expiresIn * 1000,
    )

    fun supports(remoteUrl: String): Boolean = runCatching {
        val uri = URI(remoteUrl.trim())
        uri.scheme.equals("https", true) && uri.host.equals("github.com", true)
    }.getOrDefault(false)

    suspend fun start(): DeviceCode {
        val response = post("device/code", mapOf("client_id" to CLIENT_ID, "scope" to "repo read:org"))
        return DeviceCode(response.getString("device_code"), response.getString("user_code"),
            response.getLong("expires_in"), response.optLong("interval", 5).coerceAtLeast(5))
    }

    suspend fun awaitToken(code: DeviceCode, onStatus: (String) -> Unit = {}, onIntervalChanged: (Long) -> Unit = {}): String =
        DeviceTokenPoller().awaitToken(code.expiresAtMillis, code.interval, onStatus, onIntervalChanged) {
            val response = post("oauth/access_token", mapOf(
                "client_id" to CLIENT_ID, "device_code" to code.deviceCode,
                "grant_type" to "urn:ietf:params:oauth:grant-type:device_code",
            ))
            DeviceTokenReply(response.optString("access_token"), response.optString("error"),
                if (response.has("interval")) response.getLong("interval") else null)
        }

    private suspend fun post(endpoint: String, parameters: Map<String, String>): JSONObject = withContext(Dispatchers.IO) {
        val connection = URL("https://github.com/login/$endpoint").openConnection() as HttpURLConnection
        try {
            connection.requestMethod = "POST"
            connection.instanceFollowRedirects = false
            connection.connectTimeout = 15000
            connection.readTimeout = 15000
            connection.setRequestProperty("Accept", "application/json")
            connection.setRequestProperty("Content-Type", "application/x-www-form-urlencoded")
            connection.setRequestProperty("User-Agent", "GitSwamp-Mobile")
            connection.doOutput = true
            val body = parameters.entries.joinToString("&") {
                "${URLEncoder.encode(it.key, "UTF-8")}=${URLEncoder.encode(it.value, "UTF-8")}"
            }
            connection.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
            val status = connection.responseCode
            if (status == 429 || status in 500..599) throw TemporaryGithubFailure()
            check(status in 200..299) { "GitHub sign-in is unavailable. Please try again." }
            connection.inputStream.bufferedReader().use { JSONObject(it.readText()) }
        } finally {
            connection.disconnect()
        }
    }
}
