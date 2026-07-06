package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

@Composable
fun DrawScreen(pipeline: PipelineState? = null, viewModel: DrawViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current

    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Drew on PDF") }
    }

    val pickFile = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onFilePicked(context, it) } }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? ->
        if (uri != null) viewModel.onSaveDestinationChosen(context, uri)
    }

    var canvasWidth by remember { mutableStateOf(0f) }
    var canvasHeight by remember { mutableStateOf(0f) }
    var currentPoints by remember { mutableStateOf(listOf<Offset>()) }

    Column(
        modifier = Modifier.padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(R.string.tool_draw_title))

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.inputPath == null) R.string.action_select_pdf else R.string.action_select_different_pdf))
        }

        if (state.inputPath != null) {
            OutlinedTextField(value = state.pageNumber, onValueChange = viewModel::onPageNumberChanged, label = { Text(stringResource(R.string.tool_draw_page_number_label)) })

            Canvas(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(400.dp)
                    .pointerInput(Unit) {
                        canvasWidth = size.width.toFloat()
                        canvasHeight = size.height.toFloat()
                        detectDragGestures(
                            onDragStart = { offset -> currentPoints = listOf(offset) },
                            onDragEnd = {
                                viewModel.onStrokeFinished(currentPoints)
                                currentPoints = emptyList()
                            },
                        ) { change, _ ->
                            currentPoints = currentPoints + change.position
                        }
                    }
            ) {
                canvasWidth = size.width
                canvasHeight = size.height
                (state.strokes.map { it.points } + listOf(currentPoints)).forEach { points ->
                    for (i in 0 until points.size - 1) {
                        drawLine(
                            color = Color.Black,
                            start = points[i],
                            end = points[i + 1],
                            strokeWidth = 4f,
                            cap = Stroke.DefaultCap,
                        )
                    }
                }
            }

            Button(onClick = { viewModel.onClearClicked() }) {
                Text(stringResource(R.string.action_clear))
            }
            Button(onClick = { viewModel.onApplyClicked(canvasWidth, canvasHeight) }) {
                Text(stringResource(R.string.tool_draw_apply))
            }
        }

        Text(state.statusMessage)

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("drawn.pdf") }) {
                Text(stringResource(R.string.action_save_pdf))
            }
        }
    }
}
