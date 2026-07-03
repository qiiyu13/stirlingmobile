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
import uniffi.stirling_engine.contentAutoRedact
import java.io.File
import java.util.UUID

data class AutoRedactUiState(
    val statusMessage: String = "Select a PDF",
    val pdfPath: String? = null,
    val resultFilePath: String? = null,
)

/// Pattern-based redaction: finds text matching common PII shapes (email,
/// US phone, SSN, credit card) or a custom regex, and redacts every match
/// the same way `content_redact` does (true content-stream removal, not an
/// overlay).
class AutoRedactViewModel : ViewModel() {
    private val _state = MutableStateFlow(AutoRedactUiState())
    val state: StateFlow<AutoRedactUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = AutoRedactUiState(statusMessage = "Reading…")
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "auto_redact_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = AutoRedactUiState(statusMessage = "Choose what to redact, then Redact.", pdfPath = path)
            } catch (e: Exception) {
                _state.value = AutoRedactUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onRedactClicked(context: Context, patterns: List<String>) {
        val pdfPath = state.value.pdfPath ?: return
        if (patterns.isEmpty()) return

        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Scanning and redacting…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(pdfPath).parentFile!!
                    val output = File(workingDir, "auto_redact_result_${UUID.randomUUID()}.pdf")
                    contentAutoRedact(pdfPath, context.applicationInfo.nativeLibraryDir, patterns, output.absolutePath)
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Redaction failed: ${e.message}")
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
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = AutoRedactUiState(statusMessage = "Saved.")
        }
    }
}
