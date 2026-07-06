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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

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
        state.resultFilePath?.let { pipeline?.push(it, context.getString(R.string.tool_annotations_history_label)) }
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
        Text(stringResource(R.string.tool_annotations_title))

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(if (state.inputPath == null) stringResource(R.string.action_select_pdf) else stringResource(R.string.action_select_different_pdf))
        }

        if (state.inputPath != null) {
            OutlinedTextField(value = state.pageNumber, onValueChange = viewModel::onPageNumberChanged, label = { Text(stringResource(R.string.label_page_number)) })

            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                // ponytail: "kind" doubles as the engine parameter and the on-screen label (highlight/underline/strikeout/note) — translating it needs a display-name map, not requested here.
                listOf("highlight", "underline", "strikeout", "note").forEach { kind ->
                    Button(onClick = { viewModel.onKindSelected(kind) }) {
                        Text(if (state.kind == kind) "[$kind]" else kind)
                    }
                }
            }

            OutlinedTextField(value = state.x0, onValueChange = viewModel::onX0Changed, label = { Text(stringResource(R.string.tool_annotations_label_x0)) })
            OutlinedTextField(value = state.y0, onValueChange = viewModel::onY0Changed, label = { Text(stringResource(R.string.tool_annotations_label_y0)) })
            OutlinedTextField(value = state.x1, onValueChange = viewModel::onX1Changed, label = { Text(stringResource(R.string.tool_annotations_label_x1)) })
            OutlinedTextField(value = state.y1, onValueChange = viewModel::onY1Changed, label = { Text(stringResource(R.string.tool_annotations_label_y1)) })
            if (state.kind == "note") {
                OutlinedTextField(value = state.noteText, onValueChange = viewModel::onNoteTextChanged, label = { Text(stringResource(R.string.tool_annotations_label_note_text)) })
            }

            Button(onClick = { viewModel.onAddClicked() }) {
                Text(stringResource(R.string.tool_annotations_action_add))
            }
        }

        Text(state.statusMessage)

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("annotated.pdf") }) {
                Text(stringResource(R.string.action_save_pdf))
            }
        }
    }
}
