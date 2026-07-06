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
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

private val POSITIONS = listOf("top-left", "top-right", "center", "bottom-left", "bottom-right")

@Composable
fun SignatureStampScreen(pipeline: PipelineState? = null, viewModel: SignatureStampViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.pdfPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Stamped") }
    }

    var pageText by remember { mutableStateOf("1") }
    var position by remember { mutableStateOf("bottom-right") }

    val pickPdf = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onPdfPicked(context, it) } }

    val pickSignature = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onSignaturePicked(context, it) } }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? -> if (uri != null) viewModel.onSaveDestinationChosen(context, uri) }

    Column(
        modifier = Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(R.string.tool_signature_stamp_title))

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.pdfPath == null) R.string.action_select_pdf else R.string.action_select_different_pdf))
        }

        Text(state.statusMessage)

        if (state.pageCount != null) {
            Button(onClick = { pickSignature.launch(arrayOf("image/png", "image/jpeg")) }) {
                Text(stringResource(if (state.signaturePath == null) R.string.tool_signature_stamp_select_image else R.string.tool_signature_stamp_select_different_image))
            }
        }

        if (state.signaturePath != null) {
            OutlinedTextField(
                value = pageText,
                onValueChange = { pageText = it },
                label = { Text(stringResource(R.string.tool_signature_stamp_page_label, state.pageCount.toString())) }
            )

            POSITIONS.forEach { candidate ->
                Button(onClick = { position = candidate }) {
                    Text(if (position == candidate) "[${candidate}]" else candidate)
                }
            }

            Button(onClick = {
                pageText.trim().toUIntOrNull()?.let { viewModel.onStampClicked(it, position) }
            }) {
                Text(stringResource(R.string.tool_signature_stamp_action))
            }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("signed.pdf") }) {
                Text(stringResource(R.string.action_save_pdf))
            }
        }
    }
}
