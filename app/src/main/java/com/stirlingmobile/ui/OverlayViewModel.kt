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
import uniffi.stirling_engine.toolOverlay
import java.io.File
import java.util.UUID

data class OverlayUiState(
    val statusMessage: String,
    val basePath: String? = null,
    val overlayPath: String? = null,
    val busy: Boolean = false,
    val resultPath: String? = null,
)

class OverlayViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(OverlayUiState(statusMessage = application.getString(R.string.tool_overlay_select_prompt)))
    val state: StateFlow<OverlayUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = _state.value.copy(basePath = path, statusMessage = getApplication<Application>().getString(R.string.tool_overlay_base_ready))
    }


    fun onPickBase(context: Context, uri: Uri) {
        val path = copyToWorking(context, uri, "overlay_base.pdf")
        _state.value = _state.value.copy(basePath = path, statusMessage = context.getString(R.string.tool_overlay_base_ready))
    }

    fun onPickOverlay(context: Context, uri: Uri) {
        val path = copyToWorking(context, uri, "overlay_layer.pdf")
        _state.value = _state.value.copy(overlayPath = path, statusMessage = context.getString(R.string.tool_overlay_overlay_ready))
    }

    private fun copyToWorking(context: Context, uri: Uri, name: String): String {
        val workingDir = File(context.filesDir, "working").apply { mkdirs() }
        val dest = File(workingDir, name)
        context.contentResolver.openInputStream(uri)!!.use { it.copyTo(dest.outputStream()) }
        return dest.absolutePath
    }

    fun onApply(context: Context, opacity: Float) {
        val basePath = _state.value.basePath ?: return
        val overlayPath = _state.value.overlayPath ?: return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, statusMessage = context.getString(R.string.tool_overlay_processing))
            try {
                val resultPath = withContext(Dispatchers.IO) {
                    val workingDir = File(basePath).parentFile!!
                    val output = File(workingDir, "overlay_${UUID.randomUUID()}.pdf")
                    toolOverlay(basePath, overlayPath, opacity, output.absolutePath)
                    output.absolutePath
                }
                _state.value = _state.value.copy(
                    busy = false, statusMessage = context.getString(R.string.status_done_ready_to_save), resultPath = resultPath
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, statusMessage = context.getString(R.string.error_failed, e.message))
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
