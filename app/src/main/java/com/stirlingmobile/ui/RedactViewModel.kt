package com.stirlingmobile.ui

import android.content.Context
import android.graphics.BitmapFactory
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.stirling_engine.RedactionArea
import uniffi.stirling_engine.contentRedact
import uniffi.stirling_engine.convertPdfToImages
import java.io.File
import java.util.UUID

/// Preview pages are rasterized at this DPI - the picker draws boxes
/// against the bitmap, then this converts pixel coordinates back to PDF
/// points (72 points/inch) before calling into the engine.
const val REDACT_PREVIEW_DPI = 100

data class PendingRedaction(val page: UInt, val x: Float, val y: Float, val width: Float, val height: Float)

data class RedactUiState(
    val statusMessage: String = "Select a PDF",
    val pdfPath: String? = null,
    val pageImagePaths: List<String> = emptyList(),
    val currentPage: Int = 0,
    val pending: List<PendingRedaction> = emptyList(),
    val resultFilePath: String? = null,
)

/// Manual redaction: the user draws rectangles on a rendered page preview;
/// `content_redact` then genuinely removes any text under those rectangles
/// from the PDF's content stream (not just an overlay) and paints a black
/// box over the area. See docs/09-security.md for why "true removal" matters
/// here, not just visual hiding.
class RedactViewModel : ViewModel() {
    private val _state = MutableStateFlow(RedactUiState())
    val state: StateFlow<RedactUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = RedactUiState(statusMessage = "Rendering preview…")
            try {
                val (path, pages) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "redact_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }

                    val outputDir = File(workingDir, "redact_preview_${UUID.randomUUID()}").apply { mkdirs() }
                    val pages = convertPdfToImages(
                        input.absolutePath,
                        context.applicationInfo.nativeLibraryDir,
                        REDACT_PREVIEW_DPI.toUInt(),
                        outputDir.absolutePath,
                    )
                    input.absolutePath to pages
                }
                _state.value = RedactUiState(
                    statusMessage = "Draw a box over anything to redact, then pick another page or Redact.",
                    pdfPath = path,
                    pageImagePaths = pages,
                )
            } catch (e: Exception) {
                _state.value = RedactUiState(statusMessage = "Failed to render preview: ${e.message}")
            }
        }
    }

    fun onPageSelected(page: Int) {
        _state.value = state.value.copy(currentPage = page)
    }

    /// `x0,y0,x1,y1` are pixel coordinates within the rendered page bitmap
    /// (any order/direction) - converted here to PDF points with a
    /// bottom-left origin.
    fun onBoxDrawn(bitmapWidthPx: Int, bitmapHeightPx: Int, x0: Float, y0: Float, x1: Float, y1: Float) {
        val pointsPerPixel = 72f / REDACT_PREVIEW_DPI
        val pageHeightPts = bitmapHeightPx * pointsPerPixel

        val left = minOf(x0, x1) * pointsPerPixel
        val right = maxOf(x0, x1) * pointsPerPixel
        val topPx = minOf(y0, y1)
        val bottomPx = maxOf(y0, y1)

        val redaction = PendingRedaction(
            page = (state.value.currentPage + 1).toUInt(),
            x = left,
            y = pageHeightPts - bottomPx * pointsPerPixel,
            width = right - left,
            height = (bottomPx - topPx) * pointsPerPixel,
        )
        if (redaction.width < 1f || redaction.height < 1f) return
        _state.value = state.value.copy(pending = state.value.pending + redaction)
    }

    fun onRemovePending(index: Int) {
        _state.value = state.value.copy(pending = state.value.pending.filterIndexed { i, _ -> i != index })
    }

    fun onRedactClicked() {
        val pdfPath = state.value.pdfPath ?: return
        if (state.value.pending.isEmpty()) return

        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Redacting…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(pdfPath).parentFile!!
                    val output = File(workingDir, "redact_result_${UUID.randomUUID()}.pdf")
                    val areas = state.value.pending.map { RedactionArea(it.page, it.x, it.y, it.width, it.height) }
                    contentRedact(pdfPath, areas, output.absolutePath)
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Redaction failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(statusMessage = "Done. Ready to save.", resultFilePath = outputPath)
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
            _state.value = RedactUiState(statusMessage = "Saved.")
        }
    }
}

fun loadPreviewBitmap(path: String) = BitmapFactory.decodeFile(path)
