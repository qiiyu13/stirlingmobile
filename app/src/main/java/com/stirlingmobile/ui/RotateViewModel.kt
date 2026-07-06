package com.stirlingmobile.ui

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.stirlingmobile.R
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.stirling_engine.rotatePdf
import java.io.File
import java.util.UUID

data class RotateUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val rotatedFilePath: String? = null,
)

class RotateViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(RotateUiState())
    val state: StateFlow<RotateUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = RotateUiState(statusMessage = getApplication<Application>().getString(R.string.tool_rotate_choose_angle), inputPath = path)
    }


    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = RotateUiState(statusMessage = context.getString(R.string.tool_rotate_loading))
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "rotate_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = RotateUiState(statusMessage = context.getString(R.string.tool_rotate_choose_angle), inputPath = inputPath)
            } catch (e: Exception) {
                _state.value = RotateUiState(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
            }
        }
    }

    fun onAngleChosen(angleDegrees: Int) {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_rotate_rotating_status))
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val output = File(workingDir, "rotated_${UUID.randomUUID()}.pdf")
                    rotatePdf(inputPath, angleDegrees, output.absolutePath)
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_rotate_failed_status, e.message))
                return@launch
            }
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_rotate_done_status), rotatedFilePath = outputPath)
        }
    }

    fun onSaveDestinationChosen(context: Context, destination: Uri) {
        val path = state.value.rotatedFilePath ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                File(path).inputStream().use { input ->
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = RotateUiState(statusMessage = context.getString(R.string.status_saved))
        }
    }
}
