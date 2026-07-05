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
import uniffi.stirling_engine.pagesCrop
import java.io.File
import java.util.UUID

data class CropUiState(
    val statusMessage: String = "Select a PDF to crop",
    val inputPath: String? = null,
    val busy: Boolean = false,
    val resultPath: String? = null,
)

class CropViewModel : ViewModel() {
    private val _state = MutableStateFlow(CropUiState())
    val state: StateFlow<CropUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = CropUiState(statusMessage = "Ready.", inputPath = path)
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = CropUiState(statusMessage = "Reading…")
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "crop_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = CropUiState(statusMessage = "Ready.", inputPath = path)
            } catch (e: Exception) {
                _state.value = CropUiState(statusMessage = "Failed: ${e.message}")
            }
        }
    }

    fun onCrop(context: Context, x1: Float, y1: Float, x2: Float, y2: Float) {
        val inputPath = _state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, statusMessage = "Cropping…")
            try {
                val resultPath = withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val output = File(workingDir, "cropped_${UUID.randomUUID()}.pdf")
                    pagesCrop(inputPath, x1, y1, x2, y2, output.absolutePath)
                    output.absolutePath
                }
                _state.value = _state.value.copy(
                    busy = false, statusMessage = "Done. Ready to save.", resultPath = resultPath
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, statusMessage = "Failed: ${e.message}")
            }
        }
    }

    fun onSave(context: Context, destination: Uri) {
        val path = _state.value.resultPath ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                File(path).inputStream().use { input ->
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = _state.value.copy(statusMessage = "Saved.")
        }
    }
}
