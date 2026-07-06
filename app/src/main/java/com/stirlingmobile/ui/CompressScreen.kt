package com.stirlingmobile.ui

import android.content.ClipData
import android.content.ClipboardManager
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
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
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
fun CompressScreen(pipeline: PipelineState? = null, viewModel: CompressViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, context.getString(R.string.tool_compress_history_label)) }
    }

    var quality by remember { mutableFloatStateOf(54f) }
    var scalePercent by remember { mutableFloatStateOf(72f) }
    var targetMbText by remember { mutableStateOf("") }

    val pickFile = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onFilePicked(context, it) } }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? ->
        if (uri != null) viewModel.onSaveDestinationChosen(context, uri)
    }

    Column(
        modifier = Modifier
            .padding(24.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(R.string.tool_compress_title))

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(if (state.inputPath == null) stringResource(R.string.action_select_pdf) else stringResource(R.string.action_select_different_pdf))
        }

        Text(state.statusMessage)

        Button(onClick = {
            val clipboard = context.getSystemService(ClipboardManager::class.java)
            clipboard.setPrimaryClip(ClipData.newPlainText("log", state.statusMessage))
        }) {
            Text(stringResource(R.string.tool_compress_action_copy_log))
        }

        if (state.inputPath != null) {
            Button(onClick = { viewModel.onDiagnoseClicked() }) {
                Text(stringResource(R.string.tool_compress_action_diagnose))
            }

            Text(stringResource(R.string.tool_compress_label_jpeg_quality, quality.toInt()))
            Slider(value = quality, onValueChange = { quality = it }, valueRange = 1f..100f)

            Text(stringResource(R.string.tool_compress_label_image_scale, scalePercent.toInt()))
            Slider(value = scalePercent, onValueChange = { scalePercent = it }, valueRange = 10f..100f)

            Button(onClick = { viewModel.onCompressCustom(quality.toInt(), scalePercent.toInt()) }) {
                Text(stringResource(R.string.tool_compress_action_compress))
            }

            OutlinedTextField(
                value = targetMbText,
                onValueChange = { targetMbText = it },
                label = { Text(stringResource(R.string.tool_compress_label_target_size)) }
            )
            Button(onClick = {
                targetMbText.toDoubleOrNull()?.let { viewModel.onCompressToTargetSize(it) }
            }) {
                Text(stringResource(R.string.tool_compress_action_compress_target))
            }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("compressed.pdf") }) {
                Text(stringResource(R.string.tool_compress_action_save_compressed))
            }
        }
    }
}
