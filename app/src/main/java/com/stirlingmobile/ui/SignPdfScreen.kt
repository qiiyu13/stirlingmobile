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
fun SignPdfScreen(viewModel: SignPdfViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()

    var pfxPassword by remember { mutableStateOf("") }

    val pickPdf = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onPdfPicked(context, it) } }

    val pickPfx = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onPfxPicked(context, it) } }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? -> if (uri != null) viewModel.onSaveDestinationChosen(context, uri) }

    Column(
        modifier = Modifier.padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Sign PDF (digital signature)")

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pdfPath == null) "Select PDF" else "Select a different PDF")
        }

        Text(state.statusMessage)

        if (state.pdfPath != null) {
            Button(onClick = { pickPfx.launch(arrayOf("application/x-pkcs12", "*/*")) }) {
                Text(if (state.pfxPath == null) "Select certificate (.pfx/.p12)" else "Select a different certificate")
            }
        }

        if (state.pfxPath != null) {
            OutlinedTextField(
                value = pfxPassword,
                onValueChange = { pfxPassword = it },
                label = { Text("Certificate password") },
                visualTransformation = PasswordVisualTransformation()
            )

            Button(onClick = { viewModel.onSignClicked(pfxPassword) }) {
                Text("Sign")
            }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("signed.pdf") }) {
                Text("Save PDF")
            }
        }
    }
}
