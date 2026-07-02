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
import uniffi.stirling_engine.extractPages
import uniffi.stirling_engine.getPageCount
import uniffi.stirling_engine.removePages
import java.io.File
import java.util.UUID

enum class PagesToolMode(val label: String, val verb: String) {
    REMOVE("Remove Pages", "Remove"),
    EXTRACT("Extract Pages", "Extract"),
}

data class PagesToolUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val pageCount: UInt? = null,
    val resultFilePath: String? = null,
)

class PagesToolViewModel(private val mode: PagesToolMode) : ViewModel() {
    private val _state = MutableStateFlow(PagesToolUiState())
    val state: StateFlow<PagesToolUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = PagesToolUiState(statusMessage = "Reading…")
            try {
                val (inputPath, pageCount) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "pages_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to getPageCount(input.absolutePath)
                }
                _state.value = PagesToolUiState(
                    statusMessage = "$pageCount pages. Enter page numbers to ${mode.verb.lowercase()}.",
                    inputPath = inputPath,
                    pageCount = pageCount,
                )
            } catch (e: Exception) {
                _state.value = PagesToolUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onApplyClicked(pagesText: String) {
        val inputPath = state.value.inputPath ?: return
        val pages = pagesText.split(",").mapNotNull { it.trim().toUIntOrNull() }

        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "${mode.verb}ing…")
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
                _state.value = state.value.copy(statusMessage = "${mode.verb} failed: ${e.message}")
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
                    context.contentResolver.openOutputStream(destination)!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = PagesToolUiState(statusMessage = "Saved.")
        }
    }
}
