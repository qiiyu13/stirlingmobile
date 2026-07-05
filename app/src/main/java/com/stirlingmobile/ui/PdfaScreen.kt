package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun PdfaScreen(pipeline: PipelineState? = null, viewModel: PdfaViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current

    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Converted to PDF/A-${state.standard}") }
    }

    val pickFile = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onFilePicked(context, it) } }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? ->
        if (uri != null) viewModel.onSaveDestinationChosen(context, uri)
    }

    Column(
        modifier = Modifier.padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Convert to PDF/A")

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(if (state.inputPath == null) "Select PDF" else "Select a different PDF")
        }

        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf("1b", "2b", "3b").forEach { standard ->
                Button(onClick = { viewModel.onStandardSelected(standard) }) {
                    Text(if (state.standard == standard) "[$standard]" else standard)
                }
            }
        }

        Text(state.statusMessage)

        if (state.inputPath != null) {
            Button(onClick = { viewModel.onConvertClicked() }) {
                Text("Convert")
            }
            Button(onClick = { viewModel.onValidateClicked() }) {
                Text("Validate")
            }
        }

        state.validationErrors?.forEach { error ->
            Text("• $error")
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("converted_pdfa.pdf") }) {
                Text("Save PDF/A")
            }
        }
    }
}
