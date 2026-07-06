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
import uniffi.stirling_engine.addPassword
import uniffi.stirling_engine.removePassword
import java.io.File
import java.util.UUID

enum class PasswordToolMode(@StringRes val labelRes: Int) {
    ADD(R.string.tool_password_tool_add_title),
    REMOVE(R.string.tool_password_tool_remove_title),
}

data class PasswordToolUiState(
    val statusMessage: String,
    val inputPath: String? = null,
    val resultFilePath: String? = null,
)

class PasswordToolViewModel(application: Application, private val mode: PasswordToolMode) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(PasswordToolUiState(statusMessage = application.getString(R.string.tool_password_tool_select_prompt)))
    val state: StateFlow<PasswordToolUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = PasswordToolUiState(
            statusMessage = getApplication<Application>().getString(
                if (mode == PasswordToolMode.ADD) R.string.tool_password_tool_enter_password
                else R.string.tool_password_tool_enter_document_password
            ),
            inputPath = path,
        )
    }


    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = PasswordToolUiState(statusMessage = context.getString(R.string.status_reading))
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "password_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = PasswordToolUiState(
                    statusMessage = context.getString(
                        if (mode == PasswordToolMode.ADD) R.string.tool_password_tool_enter_password
                        else R.string.tool_password_tool_enter_document_password
                    ),
                    inputPath = inputPath,
                )
            } catch (e: Exception) {
                _state.value = PasswordToolUiState(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
            }
        }
    }

    fun onApplyClicked(password: String, ownerPassword: String = "") {
        val inputPath = state.value.inputPath ?: return
        if (password.isEmpty()) {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_password_tool_empty_password))
            return
        }
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_password_tool_working))
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
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = PasswordToolUiState(statusMessage = context.getString(R.string.status_saved))
        }
    }
}
