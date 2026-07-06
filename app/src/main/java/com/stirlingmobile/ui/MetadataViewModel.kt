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
import uniffi.stirling_engine.PdfMetadata
import uniffi.stirling_engine.metadataEdit
import uniffi.stirling_engine.metadataExtract
import java.io.File
import java.util.UUID

data class MetadataUiState(
    val statusMessage: String,
    val pdfPath: String? = null,
    val metadata: PdfMetadata? = null,
    val resultFilePath: String? = null,
)

/// Read (metadataExtract) then edit (metadataEdit) the PDF /Info fields.
class MetadataViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(MetadataUiState(statusMessage = application.getString(R.string.tool_metadata_select_prompt)))
    val state: StateFlow<MetadataUiState> = _state

    fun usePipelineFile(path: String) {
        viewModelScope.launch {
            _state.value = MetadataUiState(statusMessage = getApplication<Application>().getString(R.string.status_reading))
            val meta = withContext(Dispatchers.IO) { metadataExtract(path) }
            _state.value = MetadataUiState(statusMessage = getApplication<Application>().getString(R.string.tool_metadata_edit_prompt), pdfPath = path, metadata = meta)
        }
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = MetadataUiState(statusMessage = context.getString(R.string.status_reading))
            try {
                val (path, meta) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "metadata_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to metadataExtract(input.absolutePath)
                }
                _state.value = MetadataUiState(statusMessage = context.getString(R.string.tool_metadata_ready), pdfPath = path, metadata = meta)
            } catch (e: Exception) {
                _state.value = MetadataUiState(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
            }
        }
    }

    fun onSaveMetadata(title: String, author: String, subject: String, keywords: String, creator: String, producer: String) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_metadata_writing))
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val output = File(File(pdfPath).parentFile!!, "metadata_result_${UUID.randomUUID()}.pdf")
                    metadataEdit(pdfPath, title, author, subject, keywords, creator, producer, output.absolutePath)
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
