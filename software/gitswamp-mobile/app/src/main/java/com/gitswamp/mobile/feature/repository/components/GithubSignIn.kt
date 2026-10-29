package com.gitswamp.mobile.feature.repository.components

import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.Button
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.AnnotatedString
import com.gitswamp.mobile.core.git.GithubDeviceAuth
import com.gitswamp.mobile.core.git.CloneWorkflow
import com.gitswamp.mobile.core.git.DeviceAuthorizationEnded
import com.gitswamp.mobile.core.storage.TokenVault
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

@Composable
fun GithubSignIn(remoteUrl: String, onAuthenticated: (String, String) -> Unit) {
    if (!GithubDeviceAuth.supports(remoteUrl)) return
    val context = LocalContext.current
    val vault = remember(context) { TokenVault(context) }
    val browser = LocalUriHandler.current
    val clipboard = LocalClipboardManager.current
    val scope = rememberCoroutineScope()
    var device by remember(remoteUrl) { mutableStateOf(vault.readGithubSession(remoteUrl)) }
    var starting by remember(remoteUrl) { mutableStateOf(false) }
    var polling by remember(remoteUrl) { mutableStateOf(false) }
    var retry by remember(remoteUrl) { mutableStateOf(0) }
    var message by remember(remoteUrl) { mutableStateOf<String?>(null) }
    val authenticated by rememberUpdatedState(onAuthenticated)

    LaunchedEffect(remoteUrl, device?.deviceCode, retry) {
        val pending = device ?: return@LaunchedEffect
        polling = true
        try {
            val token = GithubDeviceAuth.awaitToken(pending,
                onStatus = { if (device?.deviceCode == pending.deviceCode) message = it },
                onIntervalChanged = { interval ->
                    if (device?.deviceCode == pending.deviceCode) {
                        val updated = pending.copy(interval = interval)
                        vault.saveGithubSession(remoteUrl, updated)
                        device = updated
                    }
                },
            )
            if (device?.deviceCode != pending.deviceCode) return@LaunchedEffect
            vault.save(remoteUrl, "x-access-token", token)
            vault.clearGithubSession(remoteUrl)
            device = null
            message = "GitHub connected."
            authenticated("x-access-token", token)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (ended: DeviceAuthorizationEnded) {
            vault.clearGithubSession(remoteUrl)
            device = null
            message = ended.message
        } catch (error: Exception) {
            // Keep the encrypted device session so returning from the browser can resume it.
            message = "The connection was interrupted. Your GitHub code is retained; retry the connection, not authorization."
        } finally {
            polling = false
        }
    }

    Column {
        Button(enabled = !starting && !polling, onClick = {
            if (device != null) {
                retry++
            } else {
                starting = true
                scope.launch {
                    try {
                        message = null
                        val session = GithubDeviceAuth.start()
                        vault.saveGithubSession(remoteUrl, session)
                        device = session
                        clipboard.setText(AnnotatedString(session.userCode))
                        browser.openUri("https://github.com/login/device")
                    } catch (cancelled: CancellationException) {
                        throw cancelled
                    } catch (error: Exception) {
                        message = if (CloneWorkflow.isNetworkFailure(error)) "Cannot reach GitHub. Check the phone's connection or Private DNS and try again." else error.message ?: "GitHub sign-in failed."
                    } finally {
                        starting = false
                    }
                }
            }
        }) { Text(if (starting) "Connecting to GitHub..." else if (polling) "Waiting for GitHub..." else if (device != null) "Resume GitHub connection" else "Sign in with GitHub") }
        device?.let { pending ->
            Text("GitHub code: ${pending.userCode}")
            TextButton(onClick = {
                clipboard.setText(AnnotatedString(pending.userCode))
                browser.openUri("https://github.com/login/device")
            }) { Text("Copy code and open GitHub") }
            TextButton(onClick = { retry++ }) { Text("Retry connection") }
            TextButton(onClick = { vault.clearGithubSession(remoteUrl); device = null; message = null }) { Text("Cancel sign-in") }
        }
        message?.let { Text(it) }
    }
}
