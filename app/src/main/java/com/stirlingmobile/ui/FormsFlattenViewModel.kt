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
import uniffi.stirling_engine.formsFlatten
import java.io.File
import java.util.UUID

data class FormsFlattenUiState(
    val statusMessage: String = "Select a PDF with form fields to flatten",
    val pdfPath: String? = null,
    val busy: Boolean = false,
    val resultPath: String? = null,
)

class FormsFlattenViewModel : ViewModel() {
    private val _state = MutableStateFlow(FormsFlattenUiState())
    val state: StateFlow<FormsFlattenUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = FormsFlattenUiState(statusMessage = "Reading…")
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "flatten_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = FormsFlattenUiState(statusMessage = "Ready. Tap Flatten.", pdfPath = path)
            } catch (e: Exception) {
                _state.value = FormsFlattenUiState(statusMessage = "Failed: ${e.message}")
            }
        }
    }

    fun onFlatten(context: Context) {
        val pdfPath = _state.value.pdfPath ?: return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, statusMessage = "Flattening…")
            try {
                val resultPath = withContext(Dispatchers.IO) {
                    val workingDir = File(pdfPath).parentFile!!
                    val output = File(workingDir, "flattened_${UUID.randomUUID()}.pdf")
                    formsFlatten(pdfPath, output.absolutePath)
                    output.absolutePath
                }
                _state.value = _state.value.copy(
                    busy = false,
                    statusMessage = "Flattened. Ready to save.",
                    resultPath = resultPath,
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
