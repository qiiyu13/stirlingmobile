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
import uniffi.stirling_engine.compressPdfCustom
import uniffi.stirling_engine.compressPdfToTargetSize
import uniffi.stirling_engine.describeImages
import java.io.File
import java.util.UUID

data class CompressUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val originalSizeBytes: Long? = null,
    val resultFilePath: String? = null,
    val resultSizeBytes: Long? = null,
)

class CompressViewModel : ViewModel() {
    private val _state = MutableStateFlow(CompressUiState())
    val state: StateFlow<CompressUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = CompressUiState(statusMessage = "Loading…")
            try {
                val (inputPath, size) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "compress_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to input.length()
                }
                _state.value = CompressUiState(
                    statusMessage = "Original size: ${formatSize(size)}",
                    inputPath = inputPath,
                    originalSizeBytes = size,
                )
            } catch (e: Exception) {
                _state.value = CompressUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onCompressCustom(quality: Int, scalePercent: Int) {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Compressing…")
            runCompress(inputPath) { output ->
                compressPdfCustom(inputPath, quality.toUByte(), scalePercent.toUByte(), output)
            }
        }
    }

    fun onCompressToTargetSize(targetMb: Double) {
        val inputPath = state.value.inputPath ?: return
        val targetBytes = (targetMb * 1024 * 1024).toULong()
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Compressing to ~${targetMb}MB…")
            runCompress(inputPath) { output ->
                compressPdfToTargetSize(inputPath, targetBytes, output)
            }
        }
    }

    private suspend fun runCompress(inputPath: String, compress: (String) -> Unit) {
        val output = try {
            withContext(Dispatchers.IO) {
                val workingDir = File(inputPath).parentFile!!
                val outputFile = File(workingDir, "compressed_${UUID.randomUUID()}.pdf")
                compress(outputFile.absolutePath)
                outputFile
            }
        } catch (e: Exception) {
            _state.value = state.value.copy(statusMessage = "Compress failed: ${e.message}")
            return
        }
        _state.value = state.value.copy(
            statusMessage = "Compressed: ${formatSize(output.length())} (was ${formatSize(state.value.originalSizeBytes ?: 0)})",
            resultFilePath = output.absolutePath,
            resultSizeBytes = output.length(),
        )
    }

    fun onDiagnoseClicked() {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            val lines = withContext(Dispatchers.IO) { describeImages(inputPath) }
            _state.value = state.value.copy(
                statusMessage = if (lines.isEmpty()) "No image XObjects found." else lines.joinToString("\n")
            )
        }
    }

    fun onSaveDestinationChosen(context: Context, destination: Uri) {
        val path = state.value.resultFilePath ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                File(path).inputStream().use { input ->
                    context.contentResolver.openOutputStream(destination)!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = CompressUiState(statusMessage = "Saved.")
        }
    }
}

private fun formatSize(bytes: Long): String = "%.2f MB".format(bytes / 1024.0 / 1024.0)
