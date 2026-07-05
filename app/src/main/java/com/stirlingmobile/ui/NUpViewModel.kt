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
import uniffi.stirling_engine.pagesNUp
import java.io.File
import java.util.UUID

data class NUpUiState(
    val statusMessage: String = "Select a PDF for multi-page layout",
    val inputPath: String? = null,
    val busy: Boolean = false,
    val resultPath: String? = null,
)

class NUpViewModel : ViewModel() {
    private val _state = MutableStateFlow(NUpUiState())
    val state: StateFlow<NUpUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = NUpUiState(statusMessage = "Reading…")
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "nup_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = NUpUiState(statusMessage = "Ready.", inputPath = path)
            } catch (e: Exception) {
                _state.value = NUpUiState(statusMessage = "Failed: ${e.message}")
            }
        }
    }

    fun onApply(context: Context, n: UInt) {
        val inputPath = _state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, statusMessage = "Making ${n}-up…")
            try {
                val resultPath = withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val output = File(workingDir, "nup_${UUID.randomUUID()}.pdf")
                    pagesNUp(inputPath, n, output.absolutePath)
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
