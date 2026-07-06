package com.stirlingmobile.ui

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import com.stirlingmobile.R
import uniffi.stirling_engine.convertPdfToPdfa
import uniffi.stirling_engine.pdfaValidate
import java.io.File
import java.util.UUID

data class PdfaUiState(
    val statusMessage: String,
    val inputPath: String? = null,
    val standard: String = "1b",
    val resultFilePath: String? = null,
    val validationErrors: List<String>? = null,
)

class PdfaViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(PdfaUiState(statusMessage = application.getString(R.string.tool_pdfa_select_prompt)))
    val state: StateFlow<PdfaUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = PdfaUiState(statusMessage = getApplication<Application>().getString(R.string.tool_pdfa_from_pipeline), inputPath = path)
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = context.getString(R.string.tool_pdfa_loading))
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "pdfa_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = PdfaUiState(statusMessage = context.getString(R.string.tool_pdfa_ready), inputPath = inputPath, standard = state.value.standard)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
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
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_pdfa_converting))
            val output = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "pdfa_${UUID.randomUUID()}.pdf")
                    convertPdfToPdfa(inputPath, standard, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_pdfa_conversion_failed, e.message))
                return@launch
            }
            _state.value = state.value.copy(
                statusMessage = getApplication<Application>().getString(R.string.tool_pdfa_converted, standard),
                resultFilePath = output.absolutePath,
            )
        }
    }

    fun onValidateClicked() {
        val path = state.value.resultFilePath ?: state.value.inputPath ?: return
        val standard = state.value.standard
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_pdfa_validating))
            val result = try {
                withContext(Dispatchers.IO) { pdfaValidate(path, standard) }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_pdfa_validation_failed, e.message))
                return@launch
            }
            _state.value = state.value.copy(
                statusMessage = getApplication<Application>().getString(
                    if (result.valid) R.string.tool_pdfa_valid else R.string.tool_pdfa_not_valid,
                    standard,
                ),
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
            _state.value = state.value.copy(statusMessage = context.getString(R.string.status_saved))
        }
    }
}
