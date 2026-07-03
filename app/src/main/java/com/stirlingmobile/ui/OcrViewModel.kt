package com.stirlingmobile.ui

import android.content.Context
import android.graphics.BitmapFactory
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.paddle.ocr.PaddleOCR
import com.paddle.ocr.util.OpenCVUtils
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.stirling_engine.OcrPage
import uniffi.stirling_engine.OcrWord
import uniffi.stirling_engine.convertPdfToImages
import uniffi.stirling_engine.ocrApplyTextLayer
import java.io.File
import java.util.UUID

/// OCR renders each page to an image at this DPI before handing it to the OCR
/// engine. PP-OCRv5's detector downscales its input to ~960px on the long side
/// anyway, so higher DPI just wastes memory (a 300-DPI A4 page is a ~35MB ARGB
/// bitmap, plus an OpenCV Mat copy → OOM). 150 keeps small text legible while
/// staying ~9MB/page.
const val OCR_DPI = 150

data class OcrUiState(
    val statusMessage: String = "Select a scanned PDF to make it searchable",
    val pdfPath: String? = null,
    val busy: Boolean = false,
    val resultPdfPath: String? = null,
    val extractedText: String? = null,
)

/// Searchable-PDF OCR: Rust rasterizes the pages, PaddleOCR (PP-OCRv5 mobile,
/// via the vendored ppocr-sdk) recognizes text lines on each, then Rust
/// overlays them as an invisible text layer (ocrApplyTextLayer). Also exposes
/// the plain recognized text for a .txt export. Fully offline; models are
/// bundled in assets/models/.
class OcrViewModel : ViewModel() {
    private val _state = MutableStateFlow(OcrUiState())
    val state: StateFlow<OcrUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = OcrUiState(statusMessage = "Reading…")
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "ocr_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = OcrUiState(statusMessage = "Ready. Tap Run OCR.", pdfPath = path)
            } catch (e: Exception) {
                _state.value = OcrUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun runOcr(context: Context) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(busy = true, statusMessage = "Rendering pages…", resultPdfPath = null, extractedText = null)
            try {
                val (resultPath, text) = withContext(Dispatchers.IO) {
                    val workingDir = File(pdfPath).parentFile!!
                    val pageDir = File(workingDir, "ocr_pages_${UUID.randomUUID()}").apply { mkdirs() }
                    val pngPaths = convertPdfToImages(
                        pdfPath,
                        context.applicationInfo.nativeLibraryDir,
                        OCR_DPI.toUInt(),
                        pageDir.absolutePath,
                    )

                    OpenCVUtils.init(context)
                    val ocr = PaddleOCR.create(context)
                    val pages = ArrayList<OcrPage>(pngPaths.size)
                    val allText = StringBuilder()
                    try {
                        pngPaths.forEachIndexed { index, png ->
                            _state.value = state.value.copy(statusMessage = "OCR page ${index + 1}/${pngPaths.size}…")
                            val bitmap = BitmapFactory.decodeFile(png)
                                ?: throw IllegalStateException("Could not decode rendered page ${index + 1}")
                            val result = ocr.recognize(bitmap)

                            val words = ArrayList<OcrWord>(result.results.size)
                            result.results.forEach { line ->
                                if (line.text.isBlank()) return@forEach
                                // Bound the (possibly rotated) 4-point quad to an
                                // axis-aligned rect in image px (top-left origin) —
                                // enough for an invisible selectable layer.
                                val xs = line.box.points.map { it.x }
                                val ys = line.box.points.map { it.y }
                                val x = xs.min()
                                val y = ys.min()
                                val w = xs.max() - x
                                val h = ys.max() - y
                                if (w > 0f && h > 0f) {
                                    words.add(OcrWord(line.text, x, y, w, h))
                                }
                                allText.append(line.text).append("\n")
                            }
                            pages.add(OcrPage(index.toUInt(), bitmap.width.toFloat(), bitmap.height.toFloat(), words))
                            allText.append("\n")
                            bitmap.recycle()
                        }
                    } finally {
                        ocr.release()
                    }

                    val result = File(workingDir, "ocr_result_${UUID.randomUUID()}.pdf")
                    ocrApplyTextLayer(pdfPath, pages, result.absolutePath)
                    result.absolutePath to allText.toString().trim()
                }
                _state.value = state.value.copy(
                    busy = false,
                    statusMessage = "Done. Ready to save.",
                    resultPdfPath = resultPath,
                    extractedText = text,
                )
            } catch (e: Exception) {
                _state.value = state.value.copy(busy = false, statusMessage = "Failed: ${e.message}")
            }
        }
    }

    fun onSavePdf(context: Context, destination: Uri) {
        val path = state.value.resultPdfPath ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                File(path).inputStream().use { input ->
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output -> input.copyTo(output) }
                }
            }
            _state.value = state.value.copy(statusMessage = "Saved searchable PDF.")
        }
    }

    fun onSaveText(context: Context, destination: Uri) {
        val text = state.value.extractedText ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                context.contentResolver.openOutputStream(destination, "wt")!!.use { it.write(text.toByteArray()) }
            }
            _state.value = state.value.copy(statusMessage = "Saved text.")
        }
    }
}
