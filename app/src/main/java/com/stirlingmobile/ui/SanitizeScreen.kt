package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun SanitizeScreen(pipeline: PipelineState? = null, viewModel: SanitizeViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.pdfPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Sanitized") }
    }

    var js by remember { mutableStateOf(true) }
    var embedded by remember { mutableStateOf(true) }
    var metadata by remember { mutableStateOf(false) }
    var links by remember { mutableStateOf(false) }

    val pickPdf = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPdfPicked(context, it) }
    }
    val saveResult = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/pdf")) { uri: Uri? ->
        if (uri != null) viewModel.onSaveDestinationChosen(context, uri)
    }

    Column(
        modifier = Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Sanitize PDF")

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pdfPath == null) "Select PDF" else "Select a different PDF")
        }
        Text(state.statusMessage)

        if (state.pdfPath != null) {
            CheckRow("Remove JavaScript", js) { js = it }
            CheckRow("Remove embedded files", embedded) { embedded = it }
            CheckRow("Remove metadata", metadata) { metadata = it }
            CheckRow("Remove links", links) { links = it }

            Button(
                onClick = { viewModel.onSanitize(js, embedded, metadata, links) },
                enabled = js || embedded || metadata || links,
            ) { Text("Sanitize") }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("sanitized.pdf") }) { Text("Save PDF") }
        }
    }
}

@Composable
private fun CheckRow(label: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Checkbox(checked = checked, onCheckedChange = onChange)
        Text(label)
    }
}
