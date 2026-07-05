package com.stirlingmobile.ui

import android.content.Context
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.stirling_engine.contentWatermarkImage
import uniffi.stirling_engine.contentWatermarkText
import java.io.File
import java.util.UUID

data class WatermarkUiState(
    val statusMessage: String = "Select a PDF",
    val pdfPath: String? = null,
    val imagePath: String? = null,
    val resultFilePath: String? = null,
)

/// Text or image watermark, tiled across every page with opacity and rotation
/// (contentWatermarkText / contentWatermarkImage).
class WatermarkViewModel : ViewModel() {
    private val _state = MutableStateFlow(WatermarkUiState())
    val state: StateFlow<WatermarkUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = WatermarkUiState(statusMessage = "Set options, then Apply.", pdfPath = path)
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = WatermarkUiState(statusMessage = "Reading…")
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "watermark_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = WatermarkUiState(statusMessage = "Set options, then Apply.", pdfPath = path)
            } catch (e: Exception) {
                _state.value = WatermarkUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onImagePicked(context: Context, uri: Uri) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch {
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(pdfPath).parentFile!!
                    val img = File(workingDir, "watermark_image")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(img.outputStream()) }
                    img.absolutePath
                }
                _state.value = state.value.copy(statusMessage = "Image chosen.", imagePath = path)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Failed to read image: ${e.message}")
            }
        }
    }

    fun applyText(text: String, fontSize: Float, rotation: Float, opacity: Float) {
        val pdfPath = state.value.pdfPath ?: return
        if (text.isBlank()) return
        run("Watermarking…") { output ->
            contentWatermarkText(pdfPath, text, fontSize, rotation, opacity, output)
        }
    }

    fun applyImage(widthFraction: Float, rotation: Float, opacity: Float) {
        val pdfPath = state.value.pdfPath ?: return
        val imagePath = state.value.imagePath ?: return
        run("Watermarking…") { output ->
            contentWatermarkImage(pdfPath, imagePath, widthFraction, rotation, opacity, output)
        }
    }

    private fun run(status: String, op: (String) -> Unit) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = status)
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val output = File(File(pdfPath).parentFile!!, "watermark_result_${UUID.randomUUID()}.pdf")
                    op(output.absolutePath)
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Failed: ${e.message}")
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
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output -> input.copyTo(output) }
                }
            }
            _state.value = state.value.copy(statusMessage = "Saved.")
        }
    }
}
