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
import uniffi.stirling_engine.PageComparison
import uniffi.stirling_engine.toolCompare
import java.io.File
import java.util.UUID

data class CompareUiState(
    val statusMessage: String = "Select two PDFs to compare",
    val pathA: String? = null,
    val pathB: String? = null,
    val busy: Boolean = false,
    val results: List<PageComparison> = emptyList(),
)

class CompareViewModel : ViewModel() {
    private val _state = MutableStateFlow(CompareUiState())
    val state: StateFlow<CompareUiState> = _state

    fun onPickA(context: Context, uri: Uri) {
        val path = copyToWorking(context, uri, "compare_a.pdf")
        _state.value = _state.value.copy(pathA = path, statusMessage = "PDF A ready.")
    }

    fun onPickB(context: Context, uri: Uri) {
        val path = copyToWorking(context, uri, "compare_b.pdf")
        _state.value = _state.value.copy(pathB = path, statusMessage = "PDF B ready.")
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
            _state.value = _state.value.copy(busy = true, statusMessage = "Comparing…")
            try {
                val results = withContext(Dispatchers.IO) {
                    val workingDir = File(pathA).parentFile!!
                    val outputDir = File(workingDir, "compare_${UUID.randomUUID()}").apply { mkdirs() }
                    toolCompare(pathA, pathB, context.applicationInfo.nativeLibraryDir, 150u, outputDir.absolutePath)
                }
                val diffCount = results.count { !it.identical }
                _state.value = _state.value.copy(
                    busy = false,
                    statusMessage = "Compared ${results.size} pages, $diffCount differ.",
                    results = results,
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, statusMessage = "Failed: ${e.message}")
            }
        }
    }
}
