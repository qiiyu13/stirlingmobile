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
import uniffi.stirling_engine.pagesDetectDuplicates
import uniffi.stirling_engine.pagesRemoveDuplicates
import java.io.File
import java.util.UUID

data class DedupePagesUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val duplicatePages: List<UInt>? = null,
    val resultFilePath: String? = null,
)

class DedupePagesViewModel : ViewModel() {
    private val _state = MutableStateFlow(DedupePagesUiState())
    val state: StateFlow<DedupePagesUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = DedupePagesUiState(statusMessage = "From pipeline", inputPath = path)
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Loading…")
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "dedupe_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = DedupePagesUiState(statusMessage = "Ready", inputPath = inputPath)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onDetectClicked() {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Scanning…")
            val duplicates = try {
                withContext(Dispatchers.IO) { pagesDetectDuplicates(inputPath) }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Scan failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(
                statusMessage = if (duplicates.isEmpty()) "No duplicate pages found" else "${duplicates.size} duplicate page(s) found",
                duplicatePages = duplicates,
            )
        }
    }

    fun onRemoveClicked() {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Removing…")
            val output = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "dedupe_${UUID.randomUUID()}.pdf")
                    pagesRemoveDuplicates(inputPath, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Removal failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(statusMessage = "Duplicates removed", resultFilePath = output.absolutePath)
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
