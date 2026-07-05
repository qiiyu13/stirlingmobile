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
import uniffi.stirling_engine.toolOverlay
import java.io.File
import java.util.UUID

data class OverlayUiState(
    val statusMessage: String = "Select a base PDF and an overlay PDF",
    val basePath: String? = null,
    val overlayPath: String? = null,
    val busy: Boolean = false,
    val resultPath: String? = null,
)

class OverlayViewModel : ViewModel() {
    private val _state = MutableStateFlow(OverlayUiState())
    val state: StateFlow<OverlayUiState> = _state

    fun onPickBase(context: Context, uri: Uri) {
        val path = copyToWorking(context, uri, "overlay_base.pdf")
        _state.value = _state.value.copy(basePath = path, statusMessage = "Base PDF ready.")
    }

    fun onPickOverlay(context: Context, uri: Uri) {
        val path = copyToWorking(context, uri, "overlay_layer.pdf")
        _state.value = _state.value.copy(overlayPath = path, statusMessage = "Overlay PDF ready.")
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
            _state.value = _state.value.copy(busy = true, statusMessage = "Overlaying…")
            try {
                val resultPath = withContext(Dispatchers.IO) {
                    val workingDir = File(basePath).parentFile!!
                    val output = File(workingDir, "overlay_${UUID.randomUUID()}.pdf")
                    toolOverlay(basePath, overlayPath, opacity, output.absolutePath)
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
