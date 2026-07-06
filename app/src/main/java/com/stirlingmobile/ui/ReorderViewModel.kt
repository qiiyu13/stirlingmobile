package com.stirlingmobile.ui

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import com.stirlingmobile.R
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.stirling_engine.getPageCount
import uniffi.stirling_engine.pagesReorder
import java.io.File
import java.util.UUID

data class ReorderUiState(
    val statusMessage: String = "Select a PDF to reorder pages",
    val inputPath: String? = null,
    val pageCount: UInt? = null,
    val busy: Boolean = false,
    val resultPath: String? = null,
)

class ReorderViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(ReorderUiState())
    val state: StateFlow<ReorderUiState> = _state

    fun usePipelineFile(path: String) {
        viewModelScope.launch {
            _state.value = ReorderUiState(statusMessage = getApplication<Application>().getString(R.string.status_reading))
            val count = withContext(Dispatchers.IO) { getPageCount(path) }
            _state.value = ReorderUiState(
                statusMessage = getApplication<Application>().getString(R.string.tool_reorder_page_count, count.toString()),
                inputPath = path,
                pageCount = count,
            )
        }
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = ReorderUiState(statusMessage = context.getString(R.string.status_reading))
            try {
                val (path, count) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "reorder_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to getPageCount(input.absolutePath)
                }
                _state.value = ReorderUiState(
                    statusMessage = context.getString(R.string.tool_reorder_page_count, count.toString()),
                    inputPath = path,
                    pageCount = count,
                )
            } catch (e: Exception) {
                _state.value = ReorderUiState(statusMessage = context.getString(R.string.error_failed, e.message))
            }
        }
    }

    fun onReorder(context: Context, orderText: String) {
        val inputPath = _state.value.inputPath ?: return
        val order = orderText.split(",").mapNotNull { it.trim().toUIntOrNull() }
        if (order.isEmpty()) return

        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, statusMessage = context.getString(R.string.tool_reorder_reordering))
            try {
                val resultPath = withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val output = File(workingDir, "reordered_${UUID.randomUUID()}.pdf")
                    pagesReorder(inputPath, order, output.absolutePath)
                    output.absolutePath
                }
                _state.value = _state.value.copy(
                    busy = false, statusMessage = context.getString(R.string.status_done_ready_to_save), resultPath = resultPath
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, statusMessage = context.getString(R.string.error_failed, e.message))
            }
        }
    }

    fun onSave(context: Context, destination: Uri) {
        val path = _state.value.resultPath ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                File(path).inputStream().use { input ->
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = _state.value.copy(statusMessage = context.getString(R.string.status_saved))
        }
    }
}
