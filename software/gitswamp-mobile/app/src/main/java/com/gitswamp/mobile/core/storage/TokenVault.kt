package com.gitswamp.mobile.core.storage

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.core.content.edit
import com.gitswamp.mobile.core.git.GithubDeviceAuth
import java.net.URI
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class TokenVault(context: Context) {
    private val preferences = context.getSharedPreferences("remote-credentials", Context.MODE_PRIVATE)
    private val keyStore = KeyStore.getInstance(ANDROID_KEY_STORE).apply { load(null) }

    fun save(remoteUrl: String, username: String, token: String) {
        if (token.isBlank()) return
        preferences.edit {
            putString(tokenKey(remoteUrl), encrypt(token))
            putString(userKey(remoteUrl), username)
        }
    }

    private fun encrypt(value: String): String {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, getOrCreateKey())
        val encrypted = cipher.doFinal(value.toByteArray(Charsets.UTF_8))
        return Base64.encodeToString(cipher.iv + encrypted, Base64.NO_WRAP)
    }

    private fun decrypt(encoded: String): String {
        val payload = Base64.decode(encoded, Base64.NO_WRAP)
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, getOrCreateKey(), GCMParameterSpec(128, payload.copyOfRange(0, IV_SIZE)))
        return cipher.doFinal(payload.copyOfRange(IV_SIZE, payload.size)).toString(Charsets.UTF_8)
    }

    fun saveGithubSession(remoteUrl: String, code: GithubDeviceAuth.DeviceCode) {
        val encoded = encrypt(code.deviceCode)
        preferences.edit {
            putString("github-session-url", remoteUrl)
            putString("github-session-code", encoded)
            putString("github-session-user-code", code.userCode)
            putLong("github-session-expires", code.expiresAtMillis)
            putLong("github-session-interval", code.interval)
        }
    }

    fun readGithubSession(remoteUrl: String): GithubDeviceAuth.DeviceCode? {
        if (preferences.getString("github-session-url", null) != remoteUrl) return null
        val expiry = preferences.getLong("github-session-expires", 0)
        if (expiry <= System.currentTimeMillis()) {
            clearGithubSession(remoteUrl)
            return null
        }
        return runCatching {
            GithubDeviceAuth.DeviceCode(
                deviceCode = decrypt(preferences.getString("github-session-code", null) ?: return null),
                userCode = preferences.getString("github-session-user-code", null) ?: return null,
                expiresIn = (expiry - System.currentTimeMillis()) / 1000,
                interval = preferences.getLong("github-session-interval", 5), expiresAtMillis = expiry,
            )
        }.getOrNull()
    }

    fun clearGithubSession(remoteUrl: String) {
        if (preferences.getString("github-session-url", null) != remoteUrl) return
        preferences.edit {
            for (key in listOf("url", "code", "user-code", "expires", "interval")) remove("github-session-$key")
        }
    }

    fun read(remoteUrl: String): StoredCredential? {
        val encoded = preferences.getString(tokenKey(remoteUrl), null) ?: return null
        return runCatching {
            StoredCredential(
                username = preferences.getString(userKey(remoteUrl), DEFAULT_USER) ?: DEFAULT_USER,
                token = decrypt(encoded),
            )
        }.getOrNull()
    }

    fun clear(remoteUrl: String) {
        preferences.edit {
            remove(tokenKey(remoteUrl))
            remove(userKey(remoteUrl))
        }
    }

    private fun getOrCreateKey(): SecretKey {
        (keyStore.getKey(KEY_ALIAS, null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEY_STORE).run {
            init(
                KeyGenParameterSpec.Builder(
                    KEY_ALIAS,
                    KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
                )
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                    .build(),
            )
            generateKey()
        }
    }

    private fun tokenKey(remoteUrl: String) = "token:${host(remoteUrl)}"
    private fun userKey(remoteUrl: String) = "user:${host(remoteUrl)}"

    private fun host(remoteUrl: String): String {
        val normalized = remoteUrl.trim()
        return runCatching { URI(normalized).host }
            .getOrNull()
            ?.lowercase()
            ?: normalized.substringAfter('@', normalized).substringBefore(':').substringBefore('/').lowercase()
    }

    data class StoredCredential(val username: String, val token: String)

    private companion object {
        const val ANDROID_KEY_STORE = "AndroidKeyStore"
        const val KEY_ALIAS = "gitswamp-mobile-remote-token"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val IV_SIZE = 12
        const val DEFAULT_USER = "git"
    }
}
