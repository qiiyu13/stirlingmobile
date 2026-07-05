package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

private val N_OPTIONS = listOf(2u, 4u, 6u, 9u)

@Composable
fun NUpScreen(pipeline: PipelineState? = null, viewModel: NUpViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultPath) {
        state.resultPath?.let { pipeline?.push(it, "N-up") }
    }
    var selectedN by remember { mutableIntStateOf(0) }

    val pickPdf = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPdfPicked(context, it) }
    }
    val saveResult = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/pdf")) { uri: Uri? ->
        if (uri != null) viewModel.onSave(context, uri)
    }

    Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Multi-Page Layout (N-up)")
        Button(enabled = !state.busy, onClick = { pickPdf.launch(arrayOf("application/pdf")) }) { Text("Select PDF") }
        if (state.busy) CircularProgressIndicator()
        Text(state.statusMessage)

        if (state.inputPath != null && !state.busy) {
            N_OPTIONS.forEach { n ->
                Button(onClick = { selectedN = n.toInt(); viewModel.onApply(context, n) }) {
                    Text("${n}-up")
                }
            }
        }

        if (state.resultPath != null) {
            Button(onClick = { saveResult.launch("n-up.pdf") }) { Text("Save") }
        }
    }
}
