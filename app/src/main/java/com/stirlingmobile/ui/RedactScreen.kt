package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
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
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

@Composable
fun RedactScreen(pipeline: PipelineState? = null, viewModel: RedactViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.pdfPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(context, pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Redacted") }
    }

    val pickPdf = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onPdfPicked(context, it) } }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? -> if (uri != null) viewModel.onSaveDestinationChosen(context, uri) }

    Column(
        modifier = Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(R.string.tool_redact_title))

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.pdfPath == null) R.string.action_select_pdf else R.string.action_select_different_pdf))
        }

        Text(state.statusMessage)

        if (state.pageImagePaths.size > 1) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = { viewModel.onPageSelected((state.currentPage - 1).coerceAtLeast(0)) }) { Text(stringResource(R.string.action_prev)) }
                Text(stringResource(R.string.tool_redact_page_indicator, state.currentPage + 1, state.pageImagePaths.size))
                Button(onClick = { viewModel.onPageSelected((state.currentPage + 1).coerceAtMost(state.pageImagePaths.size - 1)) }) { Text(stringResource(R.string.action_next)) }
            }
        }

        if (state.pageImagePaths.isNotEmpty()) {
            PagePicker(
                imagePath = state.pageImagePaths[state.currentPage],
                boxesOnThisPage = state.pending.filter { it.page.toInt() == state.currentPage + 1 },
                onBoxDrawn = { w, h, x0, y0, x1, y1 -> viewModel.onBoxDrawn(w, h, x0, y0, x1, y1) },
            )
        }

        state.pending.forEachIndexed { index, r ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(stringResource(R.string.tool_redact_pending_area, r.page.toInt(), r.width.toInt(), r.height.toInt()))
                Button(onClick = { viewModel.onRemovePending(index) }) { Text(stringResource(R.string.action_remove)) }
            }
        }

        if (state.pending.isNotEmpty()) {
            Button(onClick = { viewModel.onRedactClicked() }) { Text(stringResource(R.string.tool_redact_action)) }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("redacted.pdf") }) { Text(stringResource(R.string.action_save_pdf)) }
        }
    }
}

@Composable
private fun PagePicker(
    imagePath: String,
    boxesOnThisPage: List<PendingRedaction>,
    onBoxDrawn: (bitmapWidthPx: Int, bitmapHeightPx: Int, x0: Float, y0: Float, x1: Float, y1: Float) -> Unit,
) {
    val bitmap = remember(imagePath) { loadPreviewBitmap(imagePath) }
    val imageBitmap = remember(bitmap) { bitmap.asImageBitmap() }
    var dragStart by remember(imagePath) { mutableStateOf<Offset?>(null) }
    var dragCurrent by remember(imagePath) { mutableStateOf<Offset?>(null) }

    // The canvas is drawn at whatever width Compose lays it out to (up to
    // the screen width), scaled by aspect ratio - drag coordinates come in
    // canvas-pixel space, so they're scaled back to bitmap-pixel space
    // before being reported to onBoxDrawn.
    Canvas(
        modifier = Modifier
            .fillMaxWidth()
            .aspectRatio(bitmap.width.toFloat() / bitmap.height.toFloat())
            .pointerInput(imagePath) {
                detectDragGestures(
                    onDragStart = { offset -> dragStart = offset; dragCurrent = offset },
                    onDrag = { change, _ -> dragCurrent = change.position },
                    onDragEnd = {
                        val start = dragStart
                        val end = dragCurrent
                        if (start != null && end != null) {
                            val scale = bitmap.width.toFloat() / size.width
                            onBoxDrawn(
                                bitmap.width, bitmap.height,
                                start.x * scale, start.y * scale,
                                end.x * scale, end.y * scale,
                            )
                        }
                        dragStart = null
                        dragCurrent = null
                    },
                )
            }
    ) {
        val scale = size.width / bitmap.width.toFloat()
        drawImage(imageBitmap, dstSize = IntSize(size.width.toInt(), size.height.toInt()))
        for (box in boxesOnThisPage) {
            val pointsPerPixel = 72f / REDACT_PREVIEW_DPI
            val leftPx = box.x / pointsPerPixel * scale
            val widthPx = box.width / pointsPerPixel * scale
            val heightPx = box.height / pointsPerPixel * scale
            val topPx = size.height - (box.y / pointsPerPixel * scale) - heightPx
            drawRect(color = Color.Red.copy(alpha = 0.4f), topLeft = Offset(leftPx, topPx), size = Size(widthPx, heightPx))
        }
        val start = dragStart
        val current = dragCurrent
        if (start != null && current != null) {
            val rect = Rect(start, current)
            drawRect(color = Color.Blue.copy(alpha = 0.3f), topLeft = rect.topLeft, size = rect.size)
        }
    }
}
