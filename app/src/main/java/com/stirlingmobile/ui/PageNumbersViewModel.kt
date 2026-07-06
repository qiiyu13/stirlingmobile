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
import uniffi.stirling_engine.contentPageNumbers
import java.io.File
import java.util.UUID

data class PageNumbersUiState(
    val statusMessage: String,
    val pdfPath: String? = null,
    val resultFilePath: String? = null,
)

/// Stamp page numbers on every page (contentPageNumbers). Format uses {n} for
/// the page number and {total} for the page count.
class PageNumbersViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(PageNumbersUiState(statusMessage = application.getString(R.string.tool_page_numbers_select_prompt)))
    val state: StateFlow<PageNumbersUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = PageNumbersUiState(statusMessage = getApplication<Application>().getString(R.string.tool_page_numbers_ready), pdfPath = path)
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = PageNumbersUiState(statusMessage = context.getString(R.string.status_reading))
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "page_numbers_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = PageNumbersUiState(statusMessage = context.getString(R.string.tool_page_numbers_ready), pdfPath = path)
            } catch (e: Exception) {
                _state.value = PageNumbersUiState(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
            }
        }
    }

    fun apply(position: String, format: String, startNumber: UInt, fontSize: Float) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_page_numbers_numbering))
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val output = File(File(pdfPath).parentFile!!, "page_numbers_result_${UUID.randomUUID()}.pdf")
                    contentPageNumbers(pdfPath, position, format, startNumber, fontSize, output.absolutePath)
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
