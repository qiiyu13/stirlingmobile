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
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

@Composable
fun MetadataScreen(pipeline: PipelineState? = null, viewModel: MetadataViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.pdfPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Metadata edited") }
    }

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
        Text(stringResource(R.string.tool_metadata_title))

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.pdfPath == null) R.string.action_select_pdf else R.string.action_select_different_pdf))
        }
        Text(state.statusMessage)

        val meta = state.metadata
        if (meta != null) {
            // key() so fields re-init from freshly extracted metadata per file.
            key(state.pdfPath) {
                var title by remember { mutableStateOf(meta.title ?: "") }
                var author by remember { mutableStateOf(meta.author ?: "") }
                var subject by remember { mutableStateOf(meta.subject ?: "") }
                var keywords by remember { mutableStateOf(meta.keywords ?: "") }
                var creator by remember { mutableStateOf(meta.creator ?: "") }
                var producer by remember { mutableStateOf(meta.producer ?: "") }

                OutlinedTextField(title, { title = it }, label = { Text(stringResource(R.string.tool_metadata_field_title)) })
                OutlinedTextField(author, { author = it }, label = { Text(stringResource(R.string.tool_metadata_field_author)) })
                OutlinedTextField(subject, { subject = it }, label = { Text(stringResource(R.string.tool_metadata_field_subject)) })
                OutlinedTextField(keywords, { keywords = it }, label = { Text(stringResource(R.string.tool_metadata_field_keywords)) })
                OutlinedTextField(creator, { creator = it }, label = { Text(stringResource(R.string.tool_metadata_field_creator)) })
                OutlinedTextField(producer, { producer = it }, label = { Text(stringResource(R.string.tool_metadata_field_producer)) })

                Button(onClick = {
                    viewModel.onSaveMetadata(title, author, subject, keywords, creator, producer)
                }) { Text(stringResource(R.string.action_apply)) }
            }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("metadata.pdf") }) { Text(stringResource(R.string.action_save_pdf)) }
        }
    }
}
