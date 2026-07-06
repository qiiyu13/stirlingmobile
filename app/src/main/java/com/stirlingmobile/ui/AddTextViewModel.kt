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
import uniffi.stirling_engine.contentAddText
import java.io.File
import java.util.UUID

data class AddTextUiState(
    val statusMessage: String = "",
    val inputPath: String? = null,
    val pageNumber: String = "1",
    val text: String = "",
    val x: String = "72",
    val y: String = "700",
    val fontSize: String = "14",
    val resultFilePath: String? = null,
)

class AddTextViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(
        AddTextUiState(statusMessage = application.getString(R.string.tool_add_text_default_status))
    )
    val state: StateFlow<AddTextUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = AddTextUiState(statusMessage = getApplication<Application>().getString(R.string.tool_add_text_from_pipeline), inputPath = path)
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = context.getString(R.string.status_loading))
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "add_text_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = state.value.copy(statusMessage = context.getString(R.string.status_ready), inputPath = inputPath, resultFilePath = null)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
            }
        }
    }

    fun onPageNumberChanged(value: String) { _state.value = state.value.copy(pageNumber = value) }
    fun onTextChanged(value: String) { _state.value = state.value.copy(text = value) }
    fun onXChanged(value: String) { _state.value = state.value.copy(x = value) }
    fun onYChanged(value: String) { _state.value = state.value.copy(y = value) }
    fun onFontSizeChanged(value: String) { _state.value = state.value.copy(fontSize = value) }

    fun onAddClicked() {
        val inputPath = state.value.inputPath ?: return
        val pageNumber = state.value.pageNumber.toUIntOrNull() ?: return
        val x = state.value.x.toFloatOrNull() ?: return
        val y = state.value.y.toFloatOrNull() ?: return
        val fontSize = state.value.fontSize.toFloatOrNull() ?: return
        val text = state.value.text
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_add_text_adding))
            val output = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "add_text_${UUID.randomUUID()}.pdf")
                    contentAddText(inputPath, pageNumber, text, x, y, fontSize, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_add_text_error, e.message))
                return@launch
            }
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_add_text_success), resultFilePath = output.absolutePath)
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
