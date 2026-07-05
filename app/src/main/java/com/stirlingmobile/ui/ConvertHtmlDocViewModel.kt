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
import uniffi.stirling_engine.convertPdfToHtml
import java.io.File
import java.util.UUID

data class ConvertHtmlDocUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val resultFilePath: String? = null,
)

/// PDF -> single HTML file (not to be confused with HtmlToPdfViewModel,
/// which goes the other direction).
class ConvertHtmlDocViewModel : ViewModel() {
    private val _state = MutableStateFlow(ConvertHtmlDocUiState())
    val state: StateFlow<ConvertHtmlDocUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = ConvertHtmlDocUiState(statusMessage = "From pipeline", inputPath = path)
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Loading…")
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "convert_html_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = ConvertHtmlDocUiState(statusMessage = "Ready", inputPath = inputPath)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onConvertClicked() {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Converting…")
            val output = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "convert_${UUID.randomUUID()}.html")
                    convertPdfToHtml(inputPath, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Conversion failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(statusMessage = "Converted to HTML", resultFilePath = output.absolutePath)
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
