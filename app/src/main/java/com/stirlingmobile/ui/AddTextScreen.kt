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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun AddTextScreen(pipeline: PipelineState? = null, viewModel: AddTextViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current

    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Added text") }
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
        Text("Add Text")

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(if (state.inputPath == null) "Select PDF" else "Select a different PDF")
        }

        if (state.inputPath != null) {
            OutlinedTextField(value = state.pageNumber, onValueChange = viewModel::onPageNumberChanged, label = { Text("Page number") })
            OutlinedTextField(value = state.text, onValueChange = viewModel::onTextChanged, label = { Text("Text") })
            OutlinedTextField(value = state.x, onValueChange = viewModel::onXChanged, label = { Text("X (points)") })
            OutlinedTextField(value = state.y, onValueChange = viewModel::onYChanged, label = { Text("Y (points)") })
            OutlinedTextField(value = state.fontSize, onValueChange = viewModel::onFontSizeChanged, label = { Text("Font size") })

            Button(onClick = { viewModel.onAddClicked() }) {
                Text("Add Text")
            }
        }

        Text(state.statusMessage)

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("with_text.pdf") }) {
                Text("Save PDF")
            }
        }
    }
}
