package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun OcrScreen(viewModel: OcrViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()

    val pickPdf = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPdfPicked(context, it) }
    }
    val savePdf = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/pdf")) { uri: Uri? ->
        if (uri != null) viewModel.onSavePdf(context, uri)
    }
    val saveText = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("text/plain")) { uri: Uri? ->
        if (uri != null) viewModel.onSaveText(context, uri)
    }

    Column(
        modifier = Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("OCR (make PDF searchable)")

        Button(enabled = !state.busy, onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pdfPath == null) "Select PDF" else "Select a different PDF")
        }

        if (state.busy) {
            CircularProgressIndicator()
        }
        Text(state.statusMessage)

        if (state.pdfPath != null && !state.busy) {
            Button(onClick = { viewModel.runOcr(context) }) { Text("Run OCR") }
        }

        if (state.resultPdfPath != null) {
            Button(onClick = { savePdf.launch("searchable.pdf") }) { Text("Save searchable PDF") }
        }
        if (state.extractedText != null) {
            Button(onClick = { saveText.launch("ocr.txt") }) { Text("Export text (.txt)") }
        }
    }
}
