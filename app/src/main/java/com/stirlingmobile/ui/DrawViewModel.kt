package com.stirlingmobile.ui

import android.content.Context
import android.net.Uri
import androidx.compose.ui.geometry.Offset
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.stirling_engine.Stroke
import uniffi.stirling_engine.contentDraw
import uniffi.stirling_engine.pdfPageSize
import java.io.File
import java.util.UUID

/// Canvas-space stroke (pixels, origin top-left) before it's translated into
/// PDF-space points (origin bottom-left) at export time.
data class CanvasStroke(val points: List<Offset>)

data class DrawUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val pageNumber: String = "1",
    val strokes: List<CanvasStroke> = emptyList(),
    val resultFilePath: String? = null,
)

class DrawViewModel : ViewModel() {
    private val _state = MutableStateFlow(DrawUiState())
    val state: StateFlow<DrawUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = DrawUiState(statusMessage = "From pipeline", inputPath = path)
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Loading…")
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "draw_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = state.value.copy(statusMessage = "Draw on the canvas below", inputPath = inputPath, resultFilePath = null, strokes = emptyList())
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onPageNumberChanged(value: String) { _state.value = state.value.copy(pageNumber = value) }

    fun onStrokeFinished(points: List<Offset>) {
        if (points.size < 2) return
        _state.value = state.value.copy(strokes = state.value.strokes + CanvasStroke(points))
    }

    fun onClearClicked() {
        _state.value = state.value.copy(strokes = emptyList())
    }

    /// Canvas pixels aren't the same scale as PDF points, and the canvas's
    /// top-left origin is flipped vs. PDF's bottom-left - both need
    /// correcting using the real page size before strokes are sent to the
    /// engine, or they land outside the page (or squashed/stretched).
    fun onApplyClicked(canvasWidth: Float, canvasHeight: Float) {
        val inputPath = state.value.inputPath ?: return
        val pageNumber = state.value.pageNumber.toUIntOrNull() ?: return
        val strokes = state.value.strokes
        if (strokes.isEmpty() || canvasWidth <= 0f || canvasHeight <= 0f) return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Drawing…")
            val output = try {
                withContext(Dispatchers.IO) {
                    val pageSize = pdfPageSize(inputPath, pageNumber)
                    val scaleX = pageSize.width / canvasWidth
                    val scaleY = pageSize.height / canvasHeight
                    val pdfStrokes = strokes.map { stroke ->
                        Stroke(
                            pointsX = stroke.points.map { it.x * scaleX },
                            pointsY = stroke.points.map { (canvasHeight - it.y) * scaleY },
                            colorR = 0f,
                            colorG = 0f,
                            colorB = 0f,
                            width = 3f,
                        )
                    }
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "draw_${UUID.randomUUID()}.pdf")
                    contentDraw(inputPath, pageNumber, pdfStrokes, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(statusMessage = "Drawing applied", resultFilePath = output.absolutePath)
        }
    }

    fun onSaveDestinationChosen(context: Context, destination: Uri) {
        val path = state.value.resultFilePath ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                File(path).inputStream().use { input ->
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = state.value.copy(statusMessage = "Saved.")
        }
    }
}
