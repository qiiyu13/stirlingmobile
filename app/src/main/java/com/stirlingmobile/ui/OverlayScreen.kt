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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

@Composable
fun OverlayScreen(pipeline: PipelineState? = null, viewModel: OverlayViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.basePath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultPath) {
        state.resultPath?.let { pipeline?.push(it, "Overlaid") }
    }
    var opacity by remember { mutableStateOf("1.0") }

    val pickBase = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPickBase(context, it) }
    }
    val pickOverlay = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPickOverlay(context, it) }
    }
    val saveResult = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/pdf")) { uri: Uri? ->
        if (uri != null) viewModel.onSave(context, uri)
    }

    Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(stringResource(R.string.tool_overlay_title))
        Button(enabled = !state.busy, onClick = { pickBase.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.basePath == null) R.string.tool_overlay_select_base else R.string.tool_overlay_base_selected))
        }
        Button(enabled = !state.busy, onClick = { pickOverlay.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.overlayPath == null) R.string.tool_overlay_select_overlay else R.string.tool_overlay_overlay_selected))
        }
        if (state.busy) CircularProgressIndicator()
        Text(state.statusMessage)

        if (state.basePath != null && state.overlayPath != null && !state.busy) {
            OutlinedTextField(opacity, { opacity = it }, label = { Text(stringResource(R.string.tool_overlay_opacity_label)) })
            Button(onClick = { viewModel.onApply(context, opacity.toFloatOrNull() ?: 1.0f) }) { Text(stringResource(R.string.tool_overlay_apply_button)) }
        }

        if (state.resultPath != null) {
            Button(onClick = { saveResult.launch("overlaid.pdf") }) { Text(stringResource(R.string.action_save)) }
        }
    }
}
