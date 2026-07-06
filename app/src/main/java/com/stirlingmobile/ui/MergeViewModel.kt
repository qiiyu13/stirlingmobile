package com.stirlingmobile.ui

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import com.stirlingmobile.R
import uniffi.stirling_engine.mergePdfs
import java.io.File
import java.util.UUID

data class MergeUiState(
    val statusMessage: String,
    val mergedFilePath: String? = null,
)

class MergeViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(MergeUiState(statusMessage = application.getString(R.string.tool_merge_select_prompt)))
    val state: StateFlow<MergeUiState> = _state

    fun onFilesSelected(context: Context, uris: List<Uri>) {
        viewModelScope.launch {
            _state.value = MergeUiState(statusMessage = context.getString(R.string.tool_merge_processing, uris.size))
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val inputPaths = uris.mapIndexed { index, uri ->
                        val dest = File(workingDir, "input_$index.pdf")
                        context.contentResolver.openInputStream(uri)!!.use { input ->
                            dest.outputStream().use { output -> input.copyTo(output) }
                        }
                        dest.absolutePath
                    }
                    val output = File(workingDir, "merged_${UUID.randomUUID()}.pdf")
                    mergePdfs(inputPaths, output.absolutePath)
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = MergeUiState(statusMessage = context.getString(R.string.tool_merge_error_failed, e.message))
                return@launch
            }
            _state.value = MergeUiState(statusMessage = context.getString(R.string.tool_merge_success), mergedFilePath = outputPath)
        }
    }

    fun onSaveDestinationChosen(context: Context, destination: Uri) {
        val path = state.value.mergedFilePath ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                File(path).inputStream().use { input ->
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = MergeUiState(statusMessage = context.getString(R.string.status_saved))
        }
    }
}
