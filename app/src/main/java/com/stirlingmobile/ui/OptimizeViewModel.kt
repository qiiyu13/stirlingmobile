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
import uniffi.stirling_engine.optimizeLossless
import java.io.File
import java.util.UUID

data class OptimizeUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val originalSizeBytes: Long? = null,
    val resultFilePath: String? = null,
    val resultSizeBytes: Long? = null,
)

class OptimizeViewModel : ViewModel() {
    private val _state = MutableStateFlow(OptimizeUiState())
    val state: StateFlow<OptimizeUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = OptimizeUiState(statusMessage = "Loading…")
            try {
                val (inputPath, size) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "optimize_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to input.length()
                }
                _state.value = OptimizeUiState(
                    statusMessage = "Original size: ${formatSize(size)}",
                    inputPath = inputPath,
                    originalSizeBytes = size,
                )
            } catch (e: Exception) {
                _state.value = OptimizeUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onOptimizeClicked() {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Optimizing…")
            val output = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "optimized_${UUID.randomUUID()}.pdf")
                    optimizeLossless(inputPath, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Optimize failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(
                statusMessage = "Optimized: ${formatSize(output.length())} (was ${formatSize(state.value.originalSizeBytes ?: 0)})",
                resultFilePath = output.absolutePath,
                resultSizeBytes = output.length(),
            )
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
            _state.value = OptimizeUiState(statusMessage = "Saved.")
        }
    }
}

private fun formatSize(bytes: Long): String = "%.2f MB".format(bytes / 1024.0 / 1024.0)
