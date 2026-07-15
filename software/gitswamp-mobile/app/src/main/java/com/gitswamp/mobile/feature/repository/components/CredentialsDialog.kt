package com.gitswamp.mobile.feature.repository.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
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

@Composable
fun CredentialsDialog(
    request: CredentialRequest,
    onDismiss: () -> Unit,
    onSubmit: (username: String, token: String, remember: Boolean) -> Unit,
) {
    var username by remember(request.remoteUrl) { mutableStateOf("git") }
    var token by remember(request.remoteUrl) { mutableStateOf("") }
    var rememberToken by remember(request.remoteUrl) { mutableStateOf(true) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (request.action == null) "Remote credentials" else "Credentials required") },
        text = {
            Column {
                Text(request.remoteUrl, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
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
        },
        confirmButton = {
            TextButton(onClick = { onSubmit(username.trim(), token, rememberToken) }, enabled = token.isNotBlank()) {
                Text(if (request.action == null) "Save" else "Continue")
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

