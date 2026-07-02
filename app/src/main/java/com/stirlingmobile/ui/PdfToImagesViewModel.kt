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
import uniffi.stirling_engine.convertPdfToImages
import java.io.File
import java.util.UUID

data class PdfToImagesUiState(
    val statusMessage: String = "Select a PDF to export its pages as images",
    val resultPaths: List<String> = emptyList(),
)

class PdfToImagesViewModel : ViewModel() {
    private val _state = MutableStateFlow(PdfToImagesUiState())
    val state: StateFlow<PdfToImagesUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = PdfToImagesUiState(statusMessage = "Rendering pages…")
            val outputs = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "rasterize_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }

                    val outputDir = File(workingDir, "pages_${UUID.randomUUID()}").apply { mkdirs() }
                    convertPdfToImages(
                        input.absolutePath,
                        context.applicationInfo.nativeLibraryDir,
                        150u,
                        outputDir.absolutePath,
                    )
                }
            } catch (e: Exception) {
                _state.value = PdfToImagesUiState(statusMessage = "Failed: ${e.message}")
                return@launch
            }
            _state.value = PdfToImagesUiState(statusMessage = "Rendered ${outputs.size} pages. Ready to save.", resultPaths = outputs)
        }
    }

    /// Saves every rendered page into the user-chosen directory tree, named page_1.png, page_2.png, ...
    fun onSaveDestinationChosen(context: Context, destinationDir: Uri) {
        val paths = state.value.resultPaths
        if (paths.isEmpty()) return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                val tree = androidx.documentfile.provider.DocumentFile.fromTreeUri(context, destinationDir)!!
                paths.forEach { path ->
                    val file = File(path)
                    val doc = tree.createFile("image/png", file.nameWithoutExtension)!!
                    context.contentResolver.openOutputStream(doc.uri)!!.use { output ->
                        file.inputStream().use { it.copyTo(output) }
                    }
                }
            }
            _state.value = PdfToImagesUiState(statusMessage = "Saved ${paths.size} pages.")
        }
    }
}
