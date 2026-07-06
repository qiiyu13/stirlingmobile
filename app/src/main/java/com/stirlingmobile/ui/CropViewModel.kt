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
import uniffi.stirling_engine.pagesCrop
import java.io.File
import java.util.UUID

data class CropUiState(
    val statusMessage: String = "",
    val inputPath: String? = null,
    val busy: Boolean = false,
    val resultPath: String? = null,
)

class CropViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(
        CropUiState(statusMessage = application.getString(R.string.tool_crop_default_status))
    )
    val state: StateFlow<CropUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = CropUiState(statusMessage = getApplication<Application>().getString(R.string.status_ready_dot), inputPath = path)
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = CropUiState(statusMessage = context.getString(R.string.tool_crop_status_reading))
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "crop_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = CropUiState(statusMessage = context.getString(R.string.status_ready_dot), inputPath = path)
            } catch (e: Exception) {
                _state.value = CropUiState(statusMessage = context.getString(R.string.error_generic_reason, e.message))
            }
        }
    }

    fun onCrop(context: Context, x1: Float, y1: Float, x2: Float, y2: Float) {
        val inputPath = _state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, statusMessage = context.getString(R.string.tool_crop_status_cropping))
            try {
                val resultPath = withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val output = File(workingDir, "cropped_${UUID.randomUUID()}.pdf")
                    pagesCrop(inputPath, x1, y1, x2, y2, output.absolutePath)
                    output.absolutePath
                }
                _state.value = _state.value.copy(
                    busy = false, statusMessage = context.getString(R.string.status_done_ready_to_save), resultPath = resultPath
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, statusMessage = context.getString(R.string.error_generic_reason, e.message))
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
