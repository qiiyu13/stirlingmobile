package com.stirlingmobile.ui

import android.content.Context
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import uniffi.stirling_engine.convertImagesToPdf
import java.io.File
import java.util.UUID

data class ImagesToPdfUiState(
    val statusMessage: String = "Select images to combine into a PDF",
    val resultFilePath: String? = null,
)

class ImagesToPdfViewModel : ViewModel() {
    private val _state = MutableStateFlow(ImagesToPdfUiState())
    val state: StateFlow<ImagesToPdfUiState> = _state

    fun onFilesSelected(context: Context, uris: List<Uri>) {
        viewModelScope.launch {
            _state.value = ImagesToPdfUiState(statusMessage = "Converting ${uris.size} images…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val inputPaths = uris.mapIndexed { index, uri ->
                        val dest = File(workingDir, "image_$index")
                        context.contentResolver.openInputStream(uri)!!.use { input ->
                            dest.outputStream().use { output -> input.copyTo(output) }
                        }
                        dest.absolutePath
                    }
                    val output = File(workingDir, "images_to_pdf_${UUID.randomUUID()}.pdf")
                    convertImagesToPdf(inputPaths, output.absolutePath)
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = ImagesToPdfUiState(statusMessage = "Conversion failed: ${e.message}")
                return@launch
            }
            _state.value = ImagesToPdfUiState(statusMessage = "Done. Ready to save.", resultFilePath = outputPath)
        }
    }

    fun onSaveDestinationChosen(context: Context, destination: Uri) {
        val path = state.value.resultFilePath ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                File(path).inputStream().use { input ->
                    context.contentResolver.openOutputStream(destination)!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = ImagesToPdfUiState(statusMessage = "Saved.")
        }
    }
}
