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
import uniffi.stirling_engine.PageComparison
import uniffi.stirling_engine.toolCompare
import java.io.File
import java.util.UUID

data class CompareUiState(
    val statusMessage: String = "",
    val pathA: String? = null,
    val pathB: String? = null,
    val busy: Boolean = false,
    val results: List<PageComparison> = emptyList(),
)

class CompareViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(
        CompareUiState(statusMessage = application.getString(R.string.tool_compare_default_status))
    )
    val state: StateFlow<CompareUiState> = _state

    fun onPickA(context: Context, uri: Uri) {
        val path = copyToWorking(context, uri, "compare_a.pdf")
        _state.value = _state.value.copy(pathA = path, statusMessage = context.getString(R.string.tool_compare_status_a_ready))
    }

    fun onPickB(context: Context, uri: Uri) {
        val path = copyToWorking(context, uri, "compare_b.pdf")
        _state.value = _state.value.copy(pathB = path, statusMessage = context.getString(R.string.tool_compare_status_b_ready))
    }

    private fun copyToWorking(context: Context, uri: Uri, name: String): String {
        val workingDir = File(context.filesDir, "working").apply { mkdirs() }
        val dest = File(workingDir, name)
        context.contentResolver.openInputStream(uri)!!.use { it.copyTo(dest.outputStream()) }
        return dest.absolutePath
    }

    fun onCompare(context: Context) {
        val pathA = _state.value.pathA ?: return
        val pathB = _state.value.pathB ?: return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, statusMessage = context.getString(R.string.tool_compare_status_comparing))
            try {
                val results = withContext(Dispatchers.IO) {
                    val workingDir = File(pathA).parentFile!!
                    val outputDir = File(workingDir, "compare_${UUID.randomUUID()}").apply { mkdirs() }
                    toolCompare(pathA, pathB, context.applicationInfo.nativeLibraryDir, 150u, outputDir.absolutePath)
                }
                val diffCount = results.count { !it.identical }
                _state.value = _state.value.copy(
                    busy = false,
                    statusMessage = context.getString(R.string.tool_compare_status_result, results.size, diffCount),
                    results = results,
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, statusMessage = context.getString(R.string.error_generic_reason, e.message))
            }
        }
    }
}
