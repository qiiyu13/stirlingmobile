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
import uniffi.stirling_engine.pagesScale
import java.io.File
import java.util.UUID

data class ScaleUiState(
    val statusMessage: String = "Select a PDF to scale pages",
    val inputPath: String? = null,
    val busy: Boolean = false,
    val resultPath: String? = null,
)

class ScaleViewModel : ViewModel() {
    private val _state = MutableStateFlow(ScaleUiState())
    val state: StateFlow<ScaleUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = ScaleUiState(statusMessage = "Reading…")
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "scale_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = ScaleUiState(statusMessage = "Ready.", inputPath = path)
            } catch (e: Exception) {
                _state.value = ScaleUiState(statusMessage = "Failed: ${e.message}")
            }
        }
    }

    fun onScale(context: Context, scaleX: Float, scaleY: Float) {
        val inputPath = _state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, statusMessage = "Scaling…")
            try {
                val resultPath = withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val output = File(workingDir, "scaled_${UUID.randomUUID()}.pdf")
                    pagesScale(inputPath, scaleX, scaleY, output.absolutePath)
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
