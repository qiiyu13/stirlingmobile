package com.stirlingmobile.ui

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.annotation.StringRes
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import com.stirlingmobile.R
import uniffi.stirling_engine.extractPages
import uniffi.stirling_engine.getPageCount
import uniffi.stirling_engine.removePages
import java.io.File
import java.util.UUID

enum class PagesToolMode(
    @StringRes val labelRes: Int,
    @StringRes val verbRes: Int,
    @StringRes val promptRes: Int,
    @StringRes val progressRes: Int,
    @StringRes val failedRes: Int,
) {
    REMOVE(
        R.string.tool_pages_tool_remove_title,
        R.string.tool_pages_tool_remove_verb,
        R.string.tool_pages_tool_prompt_remove,
        R.string.tool_pages_tool_removing,
        R.string.tool_pages_tool_remove_failed,
    ),
    EXTRACT(
        R.string.tool_pages_tool_extract_title,
        R.string.tool_pages_tool_extract_verb,
        R.string.tool_pages_tool_prompt_extract,
        R.string.tool_pages_tool_extracting,
        R.string.tool_pages_tool_extract_failed,
    ),
}

data class PagesToolUiState(
    val statusMessage: String,
    val inputPath: String? = null,
    val pageCount: UInt? = null,
    val resultFilePath: String? = null,
)

class PagesToolViewModel(application: Application, private val mode: PagesToolMode) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(PagesToolUiState(statusMessage = application.getString(R.string.tool_pages_tool_select_prompt)))
    val state: StateFlow<PagesToolUiState> = _state

    fun usePipelineFile(path: String) {
        viewModelScope.launch {
            _state.value = PagesToolUiState(statusMessage = getApplication<Application>().getString(R.string.status_reading))
            val pageCount = withContext(Dispatchers.IO) { getPageCount(path) }
            _state.value = PagesToolUiState(
                statusMessage = getApplication<Application>().getString(mode.promptRes, pageCount.toInt()),
                inputPath = path,
                pageCount = pageCount,
            )
        }
    }


    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = PagesToolUiState(statusMessage = context.getString(R.string.status_reading))
            try {
                val (inputPath, pageCount) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "pages_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to getPageCount(input.absolutePath)
                }
                _state.value = PagesToolUiState(
                    statusMessage = context.getString(mode.promptRes, pageCount.toInt()),
                    inputPath = inputPath,
                    pageCount = pageCount,
                )
            } catch (e: Exception) {
                _state.value = PagesToolUiState(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
            }
        }
    }

    fun onApplyClicked(pagesText: String) {
        val inputPath = state.value.inputPath ?: return
        val pages = pagesText.split(",").mapNotNull { it.trim().toUIntOrNull() }

        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(mode.progressRes))
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val output = File(workingDir, "pages_result_${UUID.randomUUID()}.pdf")
                    when (mode) {
                        PagesToolMode.REMOVE -> removePages(inputPath, pages, output.absolutePath)
                        PagesToolMode.EXTRACT -> extractPages(inputPath, pages, output.absolutePath)
                    }
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(mode.failedRes, e.message))
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
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = PagesToolUiState(statusMessage = context.getString(R.string.status_saved))
        }
    }
}
