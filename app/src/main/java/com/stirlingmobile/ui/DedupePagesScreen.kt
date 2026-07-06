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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

@Composable
fun DedupePagesScreen(pipeline: PipelineState? = null, viewModel: DedupePagesViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current

    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, context.getString(R.string.tool_dedupe_pages_history_label)) }
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
        Text(stringResource(R.string.tool_dedupe_pages_title))

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(if (state.inputPath == null) stringResource(R.string.action_select_pdf) else stringResource(R.string.action_select_different_pdf))
        }

        Text(state.statusMessage)

        if (state.inputPath != null) {
            Button(onClick = { viewModel.onDetectClicked() }) {
                Text(stringResource(R.string.tool_dedupe_pages_action_detect))
            }
        }

        state.duplicatePages?.let { pages ->
            if (pages.isNotEmpty()) {
                Text(stringResource(R.string.tool_dedupe_pages_label_duplicate_list, pages.joinToString(", ")))
                Button(onClick = { viewModel.onRemoveClicked() }) {
                    Text(stringResource(R.string.tool_dedupe_pages_action_remove))
                }
            }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("deduped.pdf") }) {
                Text(stringResource(R.string.action_save_pdf))
            }
        }
    }
}
