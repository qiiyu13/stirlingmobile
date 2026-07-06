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
import uniffi.stirling_engine.optimizeLossless
import java.io.File
import java.util.UUID

data class OptimizeUiState(
    val statusMessage: String,
    val inputPath: String? = null,
    val originalSizeBytes: Long? = null,
    val resultFilePath: String? = null,
    val resultSizeBytes: Long? = null,
)

class OptimizeViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(OptimizeUiState(statusMessage = application.getString(R.string.tool_optimize_select_prompt)))
    val state: StateFlow<OptimizeUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = OptimizeUiState(
            statusMessage = getApplication<Application>().getString(R.string.tool_optimize_from_pipeline, formatSize(File(path).length())),
            inputPath = path,
            originalSizeBytes = File(path).length(),
        )
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = OptimizeUiState(statusMessage = context.getString(R.string.tool_optimize_loading))
            try {
                val (inputPath, size) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "optimize_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to input.length()
                }
                _state.value = OptimizeUiState(
                    statusMessage = context.getString(R.string.tool_optimize_original_size, formatSize(size)),
                    inputPath = inputPath,
                    originalSizeBytes = size,
                )
            } catch (e: Exception) {
                _state.value = OptimizeUiState(statusMessage = context.getString(R.string.error_failed_to_read, e.message))
            }
        }
    }

    fun onOptimizeClicked() {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_optimize_optimizing))
            val output = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(inputPath).parentFile!!
                    val outputFile = File(workingDir, "optimized_${UUID.randomUUID()}.pdf")
                    optimizeLossless(inputPath, outputFile.absolutePath)
                    outputFile
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_optimize_failed, e.message))
                return@launch
            }
            _state.value = state.value.copy(
                statusMessage = getApplication<Application>().getString(
                    R.string.tool_optimize_result,
                    formatSize(output.length()),
                    formatSize(state.value.originalSizeBytes ?: 0),
                ),
                resultFilePath = output.absolutePath,
                resultSizeBytes = output.length(),
            )
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
            _state.value = OptimizeUiState(statusMessage = context.getString(R.string.status_saved))
        }
    }
}

private fun formatSize(bytes: Long): String = "%.2f MB".format(bytes / 1024.0 / 1024.0)
