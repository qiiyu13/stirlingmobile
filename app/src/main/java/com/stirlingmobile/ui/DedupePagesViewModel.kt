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
import uniffi.stirling_engine.pagesDetectDuplicates
import uniffi.stirling_engine.pagesRemoveDuplicates
import java.io.File
import java.util.UUID

data class DedupePagesUiState(
    val statusMessage: String = "",
    val inputPath: String? = null,
    val duplicatePages: List<UInt>? = null,
    val resultFilePath: String? = null,
)

class DedupePagesViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(
        DedupePagesUiState(statusMessage = application.getString(R.string.tool_dedupe_default_status))
    )
    val state: StateFlow<DedupePagesUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = DedupePagesUiState(statusMessage = getApplication<Application>().getString(R.string.tool_dedupe_from_pipeline), inputPath = path)
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = context.getString(R.string.status_loading))
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "dedupe_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = DedupePagesUiState(statusMessage = context.getString(R.string.status_ready), inputPath = inputPath)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
            }
        }
    }

    fun onDetectClicked() {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_dedupe_scanning))
            val duplicates = try {
                withContext(Dispatchers.IO) { pagesDetectDuplicates(inputPath) }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_dedupe_scan_failed, e.message))
                return@launch
            }
            _state.value = state.value.copy(
                statusMessage = if (duplicates.isEmpty()) getApplication<Application>().getString(R.string.tool_dedupe_none_found) else getApplication<Application>().getString(R.string.tool_dedupe_found_count, duplicates.size),
                duplicatePages = duplicates,
            )
        }
    }

    fun onRemoveClicked() {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_dedupe_removing))
            val output = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "dedupe_${UUID.randomUUID()}.pdf")
                    pagesRemoveDuplicates(inputPath, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_dedupe_removal_failed, e.message))
                return@launch
            }
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_dedupe_removed), resultFilePath = output.absolutePath)
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
