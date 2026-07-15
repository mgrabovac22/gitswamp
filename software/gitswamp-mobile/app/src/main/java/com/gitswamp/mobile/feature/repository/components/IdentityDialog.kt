package com.gitswamp.mobile.feature.repository.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.AlternateEmail
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material3.AlertDialog
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.unit.dp
import com.gitswamp.mobile.core.model.AuthorIdentity

@Composable
fun IdentityDialog(
    current: AuthorIdentity?,
    onDismiss: () -> Unit,
    onSubmit: (name: String, email: String) -> Unit,
) {
    var name by remember(current) { mutableStateOf(current?.name.orEmpty()) }
    var email by remember(current) { mutableStateOf(current?.email.orEmpty()) }
    val valid = name.isNotBlank() && email.substringAfterLast('@', "").contains('.')

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Git identity") },
        text = {
            Column {
                Text(
                    "Commits record this author in the current repository. It is stored in the local .git config only.",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodySmall,
                )
                Spacer(Modifier.height(10.dp))
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("Author name") },
                    leadingIcon = { Icon(Icons.Outlined.Person, contentDescription = null) },
                    singleLine = true,
                )
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(
                    value = email,
                    onValueChange = { email = it },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("Author email") },
                    leadingIcon = { Icon(Icons.Outlined.AlternateEmail, contentDescription = null) },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
                    singleLine = true,
                )
            }
        },
        confirmButton = { TextButton(onClick = { onSubmit(name, email) }, enabled = valid) { Text("Save") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
