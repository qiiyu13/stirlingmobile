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
import uniffi.stirling_engine.securitySanitize
import java.io.File
import java.util.UUID

data class SanitizeUiState(
    val statusMessage: String = "Select a PDF",
    val pdfPath: String? = null,
    val resultFilePath: String? = null,
)

/// Strip JavaScript, embedded files, metadata, and/or links (securitySanitize).
class SanitizeViewModel : ViewModel() {
    private val _state = MutableStateFlow(SanitizeUiState())
    val state: StateFlow<SanitizeUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = SanitizeUiState(statusMessage = "Reading…")
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "sanitize_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = SanitizeUiState(statusMessage = "Choose what to strip, then Sanitize.", pdfPath = path)
            } catch (e: Exception) {
                _state.value = SanitizeUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onSanitize(js: Boolean, embedded: Boolean, metadata: Boolean, links: Boolean) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Sanitizing…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val output = File(File(pdfPath).parentFile!!, "sanitize_result_${UUID.randomUUID()}.pdf")
                    securitySanitize(pdfPath, js, embedded, metadata, links, output.absolutePath)
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
