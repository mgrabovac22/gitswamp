package com.gitswamp.mobile.feature.repository.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import com.gitswamp.mobile.feature.repository.CredentialRequest
import com.gitswamp.mobile.core.git.GithubDeviceAuth

@Composable
fun CredentialsDialog(
    request: CredentialRequest,
    onDismiss: () -> Unit,
    onSubmit: (username: String, token: String, remember: Boolean) -> Unit,
    title: String? = null,
    submitLabel: String? = null,
) {
    var username by remember(request.remoteUrl) { mutableStateOf("git") }
    var token by remember(request.remoteUrl) { mutableStateOf("") }
    var rememberToken by remember(request.remoteUrl) { mutableStateOf(true) }
    val github = GithubDeviceAuth.supports(request.remoteUrl)
    var useGithub by remember(request.remoteUrl) { mutableStateOf(github) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title ?: if (request.action == null) "Remote credentials" else "Credentials required") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                Text(request.remoteUrl, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
                if (github) {
                    SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth().padding(top = 12.dp)) {
                        SegmentedButton(selected = useGithub, onClick = { useGithub = true }, shape = SegmentedButtonDefaults.itemShape(0, 2)) { Text("GitHub") }
                        SegmentedButton(selected = !useGithub, onClick = { useGithub = false }, shape = SegmentedButtonDefaults.itemShape(1, 2)) { Text("Access token") }
                    }
                }
                if (useGithub) {
                    GithubSignIn(request.remoteUrl) { user, accessToken -> onSubmit(user, accessToken, true) }
                } else {
                    OutlinedTextField(
                        value = username,
                        onValueChange = { username = it },
                        modifier = Modifier.fillMaxWidth().padding(top = 10.dp),
                        label = { Text("Username") },
                        singleLine = true,
                    )
                    OutlinedTextField(
                        value = token,
                        onValueChange = { token = it },
                        modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                        label = { Text("Personal access token") },
                        leadingIcon = { Icon(Icons.Outlined.Lock, contentDescription = null) },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                        visualTransformation = PasswordVisualTransformation(),
                        singleLine = true,
                    )
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(rememberToken, { rememberToken = it })
                        Text("Save with Android Keystore", style = MaterialTheme.typography.bodySmall)
                    }
                }
            }
        },
        confirmButton = {
            if (!useGithub) {
                TextButton(onClick = { onSubmit(username.trim(), token, rememberToken) }, enabled = token.isNotBlank()) {
                    Text(submitLabel ?: if (request.action == null) "Save" else "Continue")
                }
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

