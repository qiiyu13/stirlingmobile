package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
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
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

private val POSITIONS = listOf("top-left", "top-right", "center", "bottom-left", "bottom-right")

@Composable
fun SignatureStampScreen(viewModel: SignatureStampViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()

    var pageText by remember { mutableStateOf("1") }
    var position by remember { mutableStateOf("bottom-right") }

    val pickPdf = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onPdfPicked(context, it) } }

    val pickSignature = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onSignaturePicked(context, it) } }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? -> if (uri != null) viewModel.onSaveDestinationChosen(context, uri) }

    Column(
        modifier = Modifier.padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Sign PDF (stamp)")

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pdfPath == null) "Select PDF" else "Select a different PDF")
        }

        Text(state.statusMessage)

        if (state.pageCount != null) {
            Button(onClick = { pickSignature.launch(arrayOf("image/png", "image/jpeg")) }) {
                Text(if (state.signaturePath == null) "Select signature image" else "Select a different signature")
            }
        }

        if (state.signaturePath != null) {
            OutlinedTextField(
                value = pageText,
                onValueChange = { pageText = it },
                label = { Text("Page, 1-${state.pageCount}") }
            )

            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                POSITIONS.forEach { candidate ->
                    Button(onClick = { position = candidate }) {
                        Text(if (position == candidate) "[${candidate}]" else candidate)
                    }
                }
            }

            Button(onClick = {
                pageText.trim().toUIntOrNull()?.let { viewModel.onStampClicked(it, position) }
            }) {
                Text("Stamp")
            }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("signed.pdf") }) {
                Text("Save PDF")
            }
        }
    }
}
