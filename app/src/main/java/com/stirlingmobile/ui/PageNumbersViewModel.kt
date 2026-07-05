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
import uniffi.stirling_engine.contentPageNumbers
import java.io.File
import java.util.UUID

data class PageNumbersUiState(
    val statusMessage: String = "Select a PDF",
    val pdfPath: String? = null,
    val resultFilePath: String? = null,
)

/// Stamp page numbers on every page (contentPageNumbers). Format uses {n} for
/// the page number and {total} for the page count.
class PageNumbersViewModel : ViewModel() {
    private val _state = MutableStateFlow(PageNumbersUiState())
    val state: StateFlow<PageNumbersUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = PageNumbersUiState(statusMessage = "Set options, then Apply.", pdfPath = path)
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = PageNumbersUiState(statusMessage = "Reading…")
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "page_numbers_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = PageNumbersUiState(statusMessage = "Set options, then Apply.", pdfPath = path)
            } catch (e: Exception) {
                _state.value = PageNumbersUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun apply(position: String, format: String, startNumber: UInt, fontSize: Float) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Numbering…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val output = File(File(pdfPath).parentFile!!, "page_numbers_result_${UUID.randomUUID()}.pdf")
                    contentPageNumbers(pdfPath, position, format, startNumber, fontSize, output.absolutePath)
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
