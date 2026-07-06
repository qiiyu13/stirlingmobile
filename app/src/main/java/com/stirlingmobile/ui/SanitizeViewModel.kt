package com.stirlingmobile.ui

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.stirlingmobile.R
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.stirling_engine.securitySanitize
import java.io.File
import java.util.UUID

data class SanitizeUiState(
    val statusMessage: String = "Select a PDF",
    val pdfPath: String? = null,
    val resultFilePath: String? = null,
)

/// Strip JavaScript, embedded files, metadata, and/or links (securitySanitize).
class SanitizeViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(SanitizeUiState())
    val state: StateFlow<SanitizeUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = SanitizeUiState(statusMessage = getApplication<Application>().getString(R.string.tool_sanitize_choose_options), pdfPath = path)
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = SanitizeUiState(statusMessage = context.getString(R.string.status_reading))
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "sanitize_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = SanitizeUiState(statusMessage = context.getString(R.string.tool_sanitize_choose_options), pdfPath = path)
            } catch (e: Exception) {
                _state.value = SanitizeUiState(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
            }
        }
    }

    fun onSanitize(js: Boolean, embedded: Boolean, metadata: Boolean, links: Boolean) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_sanitize_sanitizing_status))
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val output = File(File(pdfPath).parentFile!!, "sanitize_result_${UUID.randomUUID()}.pdf")
                    securitySanitize(pdfPath, js, embedded, metadata, links, output.absolutePath)
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.error_failed, e.message))
                return@launch
            }
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.status_done_ready_to_save), resultFilePath = outputPath)
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
            _state.value = state.value.copy(statusMessage = context.getString(R.string.status_saved))
        }
    }
}
