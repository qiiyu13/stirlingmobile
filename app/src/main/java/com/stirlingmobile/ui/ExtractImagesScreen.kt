package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
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
import java.io.File

@Composable
fun ExtractImagesScreen(pipeline: PipelineState? = null, viewModel: ExtractImagesViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current

    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }

    val pickFile = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onFilePicked(context, it) } }

    val pickFolder = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocumentTree()
    ) { uri: Uri? -> uri?.let { viewModel.onSaveFolderChosen(context, it) } }

    Column(
        modifier = Modifier.padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Extract Images")

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(if (state.inputPath == null) "Select PDF" else "Select a different PDF")
        }

        Text(state.statusMessage)

        if (state.inputPath != null) {
            Button(onClick = { viewModel.onExtractClicked() }) {
                Text("Extract Images")
            }
        }

        state.extractedPaths.forEach { path ->
            Text("• ${File(path).name}")
        }

        if (state.extractedPaths.isNotEmpty()) {
            Button(onClick = { pickFolder.launch(null) }) {
                Text("Save to folder")
            }
        }
    }
}
