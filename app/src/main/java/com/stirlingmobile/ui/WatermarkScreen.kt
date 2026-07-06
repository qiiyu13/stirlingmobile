package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
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

@Composable
fun WatermarkScreen(pipeline: PipelineState? = null, viewModel: WatermarkViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.pdfPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Watermarked") }
    }

    var imageMode by remember { mutableStateOf(false) }
    var text by remember { mutableStateOf("CONFIDENTIAL") }
    var fontSize by remember { mutableStateOf("36") }
    var widthFraction by remember { mutableStateOf("0.3") }
    var rotation by remember { mutableStateOf("45") }
    var opacity by remember { mutableStateOf("0.3") }

    val pickPdf = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPdfPicked(context, it) }
    }
    val pickImage = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onImagePicked(context, it) }
    }
    val saveResult = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/pdf")) { uri: Uri? ->
        if (uri != null) viewModel.onSaveDestinationChosen(context, uri)
    }

    Column(
        modifier = Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(R.string.tool_watermark_title))

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.pdfPath == null) R.string.action_select_pdf else R.string.action_select_different_pdf))
        }
        Text(state.statusMessage)

        if (state.pdfPath != null) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                FilterChip(selected = !imageMode, onClick = { imageMode = false }, label = { Text(stringResource(R.string.tool_watermark_mode_text)) })
                FilterChip(selected = imageMode, onClick = { imageMode = true }, label = { Text(stringResource(R.string.tool_watermark_mode_image)) })
            }

            if (imageMode) {
                Button(onClick = { pickImage.launch(arrayOf("image/*")) }) {
                    Text(stringResource(if (state.imagePath == null) R.string.tool_watermark_select_image else R.string.tool_watermark_select_different_image))
                }
                OutlinedTextField(widthFraction, { widthFraction = it }, label = { Text(stringResource(R.string.tool_watermark_width_label)) })
            } else {
                OutlinedTextField(text, { text = it }, label = { Text(stringResource(R.string.tool_watermark_text_label)) })
                OutlinedTextField(fontSize, { fontSize = it }, label = { Text(stringResource(R.string.tool_watermark_font_size_label)) })
            }
            OutlinedTextField(rotation, { rotation = it }, label = { Text(stringResource(R.string.tool_watermark_rotation_label)) })
            OutlinedTextField(opacity, { opacity = it }, label = { Text(stringResource(R.string.tool_watermark_opacity_label)) })

            Button(onClick = {
                val rot = rotation.toFloatOrNull() ?: 0f
                val op = opacity.toFloatOrNull() ?: 0.3f
                if (imageMode) {
                    viewModel.applyImage(widthFraction.toFloatOrNull() ?: 0.3f, rot, op)
                } else {
                    viewModel.applyText(text, fontSize.toFloatOrNull() ?: 36f, rot, op)
                }
            }) { Text(stringResource(R.string.tool_watermark_apply_action)) }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("watermarked.pdf") }) { Text(stringResource(R.string.action_save_pdf)) }
        }
    }
}
