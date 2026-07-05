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
import uniffi.stirling_engine.convertPdfToPdfa
import uniffi.stirling_engine.pdfaValidate
import java.io.File
import java.util.UUID

data class PdfaUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val standard: String = "1b",
    val resultFilePath: String? = null,
    val validationErrors: List<String>? = null,
)

class PdfaViewModel : ViewModel() {
    private val _state = MutableStateFlow(PdfaUiState())
    val state: StateFlow<PdfaUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = PdfaUiState(statusMessage = "From pipeline", inputPath = path)
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Loading…")
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "pdfa_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = PdfaUiState(statusMessage = "Ready", inputPath = inputPath, standard = state.value.standard)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onStandardSelected(standard: String) {
        _state.value = state.value.copy(standard = standard, resultFilePath = null, validationErrors = null)
    }

    fun onConvertClicked() {
        val inputPath = state.value.inputPath ?: return
        val standard = state.value.standard
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Converting…")
            val output = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "pdfa_${UUID.randomUUID()}.pdf")
                    convertPdfToPdfa(inputPath, standard, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Conversion failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(
                statusMessage = "Converted to PDF/A-$standard",
                resultFilePath = output.absolutePath,
            )
        }
    }

    fun onValidateClicked() {
        val path = state.value.resultFilePath ?: state.value.inputPath ?: return
        val standard = state.value.standard
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Validating…")
            val result = try {
                withContext(Dispatchers.IO) { pdfaValidate(path, standard) }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Validation failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(
                statusMessage = if (result.valid) "Valid PDF/A-$standard" else "Not valid PDF/A-$standard",
                validationErrors = result.errors,
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
            _state.value = state.value.copy(statusMessage = "Saved.")
        }
    }
}
