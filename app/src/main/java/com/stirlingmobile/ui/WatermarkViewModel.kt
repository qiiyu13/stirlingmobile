package com.stirlingmobile.ui

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.stirlingmobile.R
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
class WatermarkViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(WatermarkUiState())
    val state: StateFlow<WatermarkUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = WatermarkUiState(statusMessage = getApplication<Application>().getString(R.string.tool_watermark_set_options), pdfPath = path)
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = WatermarkUiState(statusMessage = context.getString(R.string.status_reading))
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "watermark_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = WatermarkUiState(statusMessage = context.getString(R.string.tool_watermark_set_options), pdfPath = path)
            } catch (e: Exception) {
                _state.value = WatermarkUiState(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
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
                _state.value = state.value.copy(statusMessage = context.getString(R.string.tool_watermark_image_chosen), imagePath = path)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = context.getString(R.string.error_failed_to_read_image, e.message))
            }
        }
    }

    fun applyText(text: String, fontSize: Float, rotation: Float, opacity: Float) {
        val pdfPath = state.value.pdfPath ?: return
        if (text.isBlank()) return
        run(getApplication<Application>().getString(R.string.tool_watermark_watermarking_status)) { output ->
            contentWatermarkText(pdfPath, text, fontSize, rotation, opacity, output)
        }
    }

    fun applyImage(widthFraction: Float, rotation: Float, opacity: Float) {
        val pdfPath = state.value.pdfPath ?: return
        val imagePath = state.value.imagePath ?: return
        run(getApplication<Application>().getString(R.string.tool_watermark_watermarking_status)) { output ->
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
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.error_failed, e.message))
                return@launch
            }
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.status_done_ready_to_save), resultFilePath = outputPath)
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
            _state.value = state.value.copy(statusMessage = context.getString(R.string.status_saved))
        }
    }
}
