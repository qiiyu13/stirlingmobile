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
import uniffi.stirling_engine.addPassword
import uniffi.stirling_engine.removePassword
import java.io.File
import java.util.UUID

enum class PasswordToolMode(val label: String) {
    ADD("Add Password"),
    REMOVE("Remove Password"),
}

data class PasswordToolUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val resultFilePath: String? = null,
)

class PasswordToolViewModel(private val mode: PasswordToolMode) : ViewModel() {
    private val _state = MutableStateFlow(PasswordToolUiState())
    val state: StateFlow<PasswordToolUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = PasswordToolUiState(statusMessage = "Reading…")
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "password_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = PasswordToolUiState(
                    statusMessage = if (mode == PasswordToolMode.ADD) "Enter a password" else "Enter the document's password",
                    inputPath = inputPath,
                )
            } catch (e: Exception) {
                _state.value = PasswordToolUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onApplyClicked(password: String, ownerPassword: String = "") {
        val inputPath = state.value.inputPath ?: return
        if (password.isEmpty()) {
            _state.value = state.value.copy(statusMessage = "Password can't be empty.")
            return
        }
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Working…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val output = File(workingDir, "password_result_${UUID.randomUUID()}.pdf")
                    when (mode) {
                        PasswordToolMode.ADD -> addPassword(inputPath, password, ownerPassword, output.absolutePath)
                        PasswordToolMode.REMOVE -> removePassword(inputPath, password, output.absolutePath)
                    }
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Failed: ${e.message}")
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
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = PasswordToolUiState(statusMessage = "Saved.")
        }
    }
}
