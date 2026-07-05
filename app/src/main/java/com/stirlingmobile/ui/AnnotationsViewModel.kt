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
import uniffi.stirling_engine.contentAddAnnotation
import java.io.File
import java.util.UUID

data class AnnotationsUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val pageNumber: String = "1",
    val kind: String = "highlight",
    val x0: String = "10",
    val y0: String = "700",
    val x1: String = "200",
    val y1: String = "720",
    val noteText: String = "",
    val resultFilePath: String? = null,
)

class AnnotationsViewModel : ViewModel() {
    private val _state = MutableStateFlow(AnnotationsUiState())
    val state: StateFlow<AnnotationsUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = AnnotationsUiState(statusMessage = "From pipeline", inputPath = path)
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Loading…")
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "annotations_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = state.value.copy(statusMessage = "Ready", inputPath = inputPath, resultFilePath = null)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onPageNumberChanged(value: String) { _state.value = state.value.copy(pageNumber = value) }
    fun onKindSelected(kind: String) { _state.value = state.value.copy(kind = kind) }
    fun onX0Changed(value: String) { _state.value = state.value.copy(x0 = value) }
    fun onY0Changed(value: String) { _state.value = state.value.copy(y0 = value) }
    fun onX1Changed(value: String) { _state.value = state.value.copy(x1 = value) }
    fun onY1Changed(value: String) { _state.value = state.value.copy(y1 = value) }
    fun onNoteTextChanged(value: String) { _state.value = state.value.copy(noteText = value) }

    fun onAddClicked() {
        val inputPath = state.value.inputPath ?: return
        val pageNumber = state.value.pageNumber.toUIntOrNull() ?: return
        val x0 = state.value.x0.toFloatOrNull() ?: return
        val y0 = state.value.y0.toFloatOrNull() ?: return
        val x1 = state.value.x1.toFloatOrNull() ?: return
        val y1 = state.value.y1.toFloatOrNull() ?: return
        val kind = state.value.kind
        val noteText = state.value.noteText.ifBlank { null }
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Adding annotation…")
            val output = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "annotated_${UUID.randomUUID()}.pdf")
                    contentAddAnnotation(inputPath, pageNumber, kind, x0, y0, x1, y1, noteText, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(statusMessage = "Annotation added", resultFilePath = output.absolutePath)
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
            _state.value = state.value.copy(statusMessage = "Saved.")
        }
    }
}
