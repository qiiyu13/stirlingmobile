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
fun CropScreen(pipeline: PipelineState? = null, viewModel: CropViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultPath) {
        state.resultPath?.let { pipeline?.push(it, context.getString(R.string.tool_crop_history_label)) }
    }
    var x1Text by remember { mutableStateOf("") }
    var y1Text by remember { mutableStateOf("") }
    var x2Text by remember { mutableStateOf("612") }
    var y2Text by remember { mutableStateOf("792") }

    val pickPdf = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPdfPicked(context, it) }
    }
    val saveResult = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/pdf")) { uri: Uri? ->
        if (uri != null) viewModel.onSave(context, uri)
    }

    Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(stringResource(R.string.tool_crop_title))
        Button(enabled = !state.busy, onClick = { pickPdf.launch(arrayOf("application/pdf")) }) { Text(stringResource(R.string.action_select_pdf)) }
        if (state.busy) CircularProgressIndicator()
        Text(state.statusMessage)

        if (state.inputPath != null && !state.busy) {
            OutlinedTextField(value = x1Text, onValueChange = { x1Text = it }, label = { Text(stringResource(R.string.tool_crop_label_x1)) }, singleLine = true)
            OutlinedTextField(value = y1Text, onValueChange = { y1Text = it }, label = { Text(stringResource(R.string.tool_crop_label_y1)) }, singleLine = true)
            OutlinedTextField(value = x2Text, onValueChange = { x2Text = it }, label = { Text(stringResource(R.string.tool_crop_label_x2)) }, singleLine = true)
            OutlinedTextField(value = y2Text, onValueChange = { y2Text = it }, label = { Text(stringResource(R.string.tool_crop_label_y2)) }, singleLine = true)
            Button(onClick = {
                val x1 = x1Text.toFloatOrNull() ?: 0f
                val y1 = y1Text.toFloatOrNull() ?: 0f
                val x2 = x2Text.toFloatOrNull() ?: 612f
                val y2 = y2Text.toFloatOrNull() ?: 792f
                viewModel.onCrop(context, x1, y1, x2, y2)
            }) { Text(stringResource(R.string.tool_crop_action_crop)) }
        }

        if (state.resultPath != null) {
            Button(onClick = { saveResult.launch("cropped.pdf") }) { Text(stringResource(R.string.action_save)) }
        }
    }
}
