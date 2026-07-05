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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun AnnotationsScreen(pipeline: PipelineState? = null, viewModel: AnnotationsViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current

    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Added annotation") }
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
        Text("Annotations")

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(if (state.inputPath == null) "Select PDF" else "Select a different PDF")
        }

        if (state.inputPath != null) {
            OutlinedTextField(value = state.pageNumber, onValueChange = viewModel::onPageNumberChanged, label = { Text("Page number") })

            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf("highlight", "underline", "strikeout", "note").forEach { kind ->
                    Button(onClick = { viewModel.onKindSelected(kind) }) {
                        Text(if (state.kind == kind) "[$kind]" else kind)
                    }
                }
            }

            OutlinedTextField(value = state.x0, onValueChange = viewModel::onX0Changed, label = { Text("X0") })
            OutlinedTextField(value = state.y0, onValueChange = viewModel::onY0Changed, label = { Text("Y0") })
            OutlinedTextField(value = state.x1, onValueChange = viewModel::onX1Changed, label = { Text("X1") })
            OutlinedTextField(value = state.y1, onValueChange = viewModel::onY1Changed, label = { Text("Y1") })
            if (state.kind == "note") {
                OutlinedTextField(value = state.noteText, onValueChange = viewModel::onNoteTextChanged, label = { Text("Note text") })
            }

            Button(onClick = { viewModel.onAddClicked() }) {
                Text("Add Annotation")
            }
        }

        Text(state.statusMessage)

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("annotated.pdf") }) {
                Text("Save PDF")
            }
        }
    }
}
