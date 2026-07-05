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

class ReorderViewModel : ViewModel() {
    private val _state = MutableStateFlow(ReorderUiState())
    val state: StateFlow<ReorderUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = ReorderUiState(statusMessage = "Reading…")
            try {
                val (path, count) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "reorder_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to getPageCount(input.absolutePath)
                }
                _state.value = ReorderUiState(
                    statusMessage = "$count pages. Enter new order.",
                    inputPath = path,
                    pageCount = count,
                )
            } catch (e: Exception) {
                _state.value = ReorderUiState(statusMessage = "Failed: ${e.message}")
            }
        }
    }

    fun onReorder(context: Context, orderText: String) {
        val inputPath = _state.value.inputPath ?: return
        val order = orderText.split(",").mapNotNull { it.trim().toUIntOrNull() }
        if (order.isEmpty()) return

        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, statusMessage = "Reordering…")
            try {
                val resultPath = withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val output = File(workingDir, "reordered_${UUID.randomUUID()}.pdf")
                    pagesReorder(inputPath, order, output.absolutePath)
                    output.absolutePath
                }
                _state.value = _state.value.copy(
                    busy = false, statusMessage = "Done. Ready to save.", resultPath = resultPath
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, statusMessage = "Failed: ${e.message}")
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
            _state.value = _state.value.copy(statusMessage = "Saved.")
        }
    }
}
