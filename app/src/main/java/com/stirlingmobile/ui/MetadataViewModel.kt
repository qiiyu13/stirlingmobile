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
import uniffi.stirling_engine.PdfMetadata
import uniffi.stirling_engine.metadataEdit
import uniffi.stirling_engine.metadataExtract
import java.io.File
import java.util.UUID

data class MetadataUiState(
    val statusMessage: String = "Select a PDF",
    val pdfPath: String? = null,
    val metadata: PdfMetadata? = null,
    val resultFilePath: String? = null,
)

/// Read (metadataExtract) then edit (metadataEdit) the PDF /Info fields.
class MetadataViewModel : ViewModel() {
    private val _state = MutableStateFlow(MetadataUiState())
    val state: StateFlow<MetadataUiState> = _state

    fun usePipelineFile(path: String) {
        viewModelScope.launch {
            _state.value = MetadataUiState(statusMessage = "Reading…")
            val meta = withContext(Dispatchers.IO) { metadataExtract(path) }
            _state.value = MetadataUiState(statusMessage = "Edit fields, then Save.", pdfPath = path, metadata = meta)
        }
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = MetadataUiState(statusMessage = "Reading…")
            try {
                val (path, meta) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "metadata_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to metadataExtract(input.absolutePath)
                }
                _state.value = MetadataUiState(statusMessage = "Edit fields, then Save.", pdfPath = path, metadata = meta)
            } catch (e: Exception) {
                _state.value = MetadataUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onSaveMetadata(title: String, author: String, subject: String, keywords: String, creator: String, producer: String) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Writing…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val output = File(File(pdfPath).parentFile!!, "metadata_result_${UUID.randomUUID()}.pdf")
                    metadataEdit(pdfPath, title, author, subject, keywords, creator, producer, output.absolutePath)
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
