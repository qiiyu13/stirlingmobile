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
import uniffi.stirling_engine.rotatePdf
import java.io.File
import java.util.UUID

data class RotateUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val rotatedFilePath: String? = null,
)

class RotateViewModel : ViewModel() {
    private val _state = MutableStateFlow(RotateUiState())
    val state: StateFlow<RotateUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = RotateUiState(statusMessage = "Loading…")
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "rotate_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = RotateUiState(statusMessage = "Choose rotation angle", inputPath = inputPath)
            } catch (e: Exception) {
                _state.value = RotateUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onAngleChosen(angleDegrees: Int) {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Rotating…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val output = File(workingDir, "rotated_${UUID.randomUUID()}.pdf")
                    rotatePdf(inputPath, angleDegrees, output.absolutePath)
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Rotate failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(statusMessage = "Rotated. Ready to save.", rotatedFilePath = outputPath)
        }
    }

    fun onSaveDestinationChosen(context: Context, destination: Uri) {
        val path = state.value.rotatedFilePath ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                File(path).inputStream().use { input ->
                    context.contentResolver.openOutputStream(destination)!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = RotateUiState(statusMessage = "Saved.")
        }
    }
}
