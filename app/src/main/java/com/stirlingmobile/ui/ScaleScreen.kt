package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedTextField
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
fun ScaleScreen(pipeline: PipelineState? = null, viewModel: ScaleViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultPath) {
        state.resultPath?.let { pipeline?.push(it, "Scaled") }
    }
    var scaleText by remember { mutableStateOf("1.0") }

    val pickPdf = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPdfPicked(context, it) }
    }
    val saveResult = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/pdf")) { uri: Uri? ->
        if (uri != null) viewModel.onSave(context, uri)
    }

    Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Scale Pages")
        Button(enabled = !state.busy, onClick = { pickPdf.launch(arrayOf("application/pdf")) }) { Text("Select PDF") }
        if (state.busy) CircularProgressIndicator()
        Text(state.statusMessage)

        if (state.inputPath != null && !state.busy) {
            OutlinedTextField(
                value = scaleText,
                onValueChange = { scaleText = it },
                label = { Text("Scale factor (1.0 = original, 0.5 = half)") },
                singleLine = true,
            )
            Button(onClick = {
                val s = scaleText.toFloatOrNull() ?: 1.0f
                viewModel.onScale(context, s, s)
            }) { Text("Scale") }
        }

        if (state.resultPath != null) {
            Button(onClick = { saveResult.launch("scaled.pdf") }) { Text("Save") }
        }
    }
}
