package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun GenerateCertificateScreen(viewModel: GenerateCertificateViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()

    var commonName by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/x-pkcs12")
    ) { uri: Uri? -> if (uri != null) viewModel.onSaveDestinationChosen(context, uri) }

    Column(
        modifier = Modifier.padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Generate signing certificate")

        OutlinedTextField(
            value = commonName,
            onValueChange = { commonName = it },
            label = { Text("Your name") }
        )

        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("Certificate password") },
            visualTransformation = PasswordVisualTransformation()
        )

        Button(onClick = { viewModel.onGenerateClicked(context, commonName, password) }) {
            Text("Generate")
        }

        Text(state.statusMessage)

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("signing_certificate.pfx") }) {
                Text("Save certificate (.pfx)")
            }
        }
    }
}
