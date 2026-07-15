package com.gitswamp.mobile.core.storage

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.core.content.edit
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
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, getOrCreateKey())
        val encrypted = cipher.doFinal(token.toByteArray(Charsets.UTF_8))
        val encoded = Base64.encodeToString(cipher.iv + encrypted, Base64.NO_WRAP)
        preferences.edit {
            putString(tokenKey(remoteUrl), encoded)
            putString(userKey(remoteUrl), username)
        }
    }

    fun read(remoteUrl: String): StoredCredential? {
        val encoded = preferences.getString(tokenKey(remoteUrl), null) ?: return null
        return runCatching {
            val payload = Base64.decode(encoded, Base64.NO_WRAP)
            val iv = payload.copyOfRange(0, IV_SIZE)
            val encrypted = payload.copyOfRange(IV_SIZE, payload.size)
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, getOrCreateKey(), GCMParameterSpec(128, iv))
            StoredCredential(
                username = preferences.getString(userKey(remoteUrl), DEFAULT_USER) ?: DEFAULT_USER,
                token = cipher.doFinal(encrypted).toString(Charsets.UTF_8),
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
