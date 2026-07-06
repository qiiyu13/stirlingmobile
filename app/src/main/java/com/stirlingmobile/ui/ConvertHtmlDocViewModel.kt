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
import uniffi.stirling_engine.convertPdfToHtml
import java.io.File
import java.util.UUID

data class ConvertHtmlDocUiState(
    val statusMessage: String = "",
    val inputPath: String? = null,
    val resultFilePath: String? = null,
)

/// PDF -> single HTML file (not to be confused with HtmlToPdfViewModel,
/// which goes the other direction).
class ConvertHtmlDocViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(
        ConvertHtmlDocUiState(statusMessage = application.getString(R.string.tool_convert_html_default_status))
    )
    val state: StateFlow<ConvertHtmlDocUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = ConvertHtmlDocUiState(statusMessage = getApplication<Application>().getString(R.string.tool_convert_html_from_pipeline), inputPath = path)
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = context.getString(R.string.status_loading))
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "convert_html_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = ConvertHtmlDocUiState(statusMessage = context.getString(R.string.status_ready), inputPath = inputPath)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
            }
        }
    }

    fun onConvertClicked() {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_convert_html_converting))
            val output = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "convert_${UUID.randomUUID()}.html")
                    convertPdfToHtml(inputPath, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_convert_html_error, e.message))
                return@launch
            }
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_convert_html_success), resultFilePath = output.absolutePath)
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
