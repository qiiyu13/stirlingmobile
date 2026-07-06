package com.stirlingmobile.ui

import android.app.Application
import android.content.Context
import android.net.Uri
import com.stirlingmobile.R
import androidx.documentfile.provider.DocumentFile
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.stirling_engine.extractImages
import java.io.File
import java.util.UUID

data class ExtractImagesUiState(
    val statusMessage: String = "",
    val inputPath: String? = null,
    val extractedPaths: List<String> = emptyList(),
)

class ExtractImagesViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(
        ExtractImagesUiState(statusMessage = application.getString(R.string.tool_extract_images_default_status))
    )
    val state: StateFlow<ExtractImagesUiState> = _state

    fun usePipelineFile(path: String) {
        _state.value = ExtractImagesUiState(statusMessage = getApplication<Application>().getString(R.string.tool_extract_images_from_pipeline), inputPath = path)
    }

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = context.getString(R.string.status_loading))
            try {
                val inputPath = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "extract_images_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = ExtractImagesUiState(statusMessage = context.getString(R.string.status_ready), inputPath = inputPath)
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = context.getString(R.string.error_read_failed, e.message))
            }
        }
    }

    fun onExtractClicked() {
        val inputPath = state.value.inputPath ?: return
        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_extract_images_extracting))
            val paths = try {
                withContext(Dispatchers.IO) {
                    val outDir = File(File(inputPath).parentFile, "extracted_${UUID.randomUUID()}").apply { mkdirs() }
                    extractImages(inputPath, outDir.absolutePath)
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = getApplication<Application>().getString(R.string.tool_extract_images_error, e.message))
                return@launch
            }
            _state.value = state.value.copy(
                statusMessage = getApplication<Application>().getString(R.string.tool_extract_images_success, paths.size),
                extractedPaths = paths,
            )
        }
    }

    fun onSaveFolderChosen(context: Context, folderUri: Uri) {
        val paths = state.value.extractedPaths
        if (paths.isEmpty()) return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                val folder = DocumentFile.fromTreeUri(context, folderUri)!!
                for (path in paths) {
                    val file = File(path)
                    val mime = if (file.extension == "jpg") "image/jpeg" else "image/png"
                    val dest = folder.createFile(mime, file.name) ?: continue
                    context.contentResolver.openOutputStream(dest.uri)!!.use { output ->
                        file.inputStream().use { it.copyTo(output) }
                    }
                }
            }
            _state.value = state.value.copy(statusMessage = context.getString(R.string.tool_extract_images_saved, paths.size))
        }
    }
}
