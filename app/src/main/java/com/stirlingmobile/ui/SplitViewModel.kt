package com.stirlingmobile.ui

import android.content.Context
import android.net.Uri
import androidx.documentfile.provider.DocumentFile
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.stirling_engine.getPageCount
import uniffi.stirling_engine.splitPdf
import java.io.File
import java.util.UUID

data class SplitUiState(
    val statusMessage: String = "Select a PDF",
    val inputPath: String? = null,
    val pageCount: UInt? = null,
    val outputPaths: List<String> = emptyList(),
)

class SplitViewModel : ViewModel() {
    private val _state = MutableStateFlow(SplitUiState())
    val state: StateFlow<SplitUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = SplitUiState(statusMessage = "Reading…")
            try {
                val (inputPath, pageCount) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "split_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to getPageCount(input.absolutePath)
                }
                _state.value = SplitUiState(
                    statusMessage = "$pageCount pages. Enter split points.",
                    inputPath = inputPath,
                    pageCount = pageCount,
                )
            } catch (e: Exception) {
                _state.value = SplitUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onSplitClicked(context: Context, splitPointsText: String) {
        val inputPath = state.value.inputPath ?: return
        val splitAfterPages = splitPointsText
            .split(",")
            .mapNotNull { it.trim().toUIntOrNull() }

        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Splitting…")
            val outputs = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val outputDir = File(workingDir, "split_${UUID.randomUUID()}").apply { mkdirs() }
                    splitPdf(inputPath, splitAfterPages, outputDir.absolutePath)
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Split failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(
                statusMessage = "Split into ${outputs.size} files. Ready to save.",
                outputPaths = outputs,
            )
        }
    }

    fun onSaveDestinationChosen(context: Context, treeUri: Uri) {
        val paths = state.value.outputPaths
        if (paths.isEmpty()) return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                val treeDir = DocumentFile.fromTreeUri(context, treeUri) ?: return@withContext
                for (path in paths) {
                    val file = File(path)
                    val dest = treeDir.createFile("application/pdf", file.nameWithoutExtension)
                        ?: continue
                    context.contentResolver.openOutputStream(dest.uri, "wt")!!.use { output ->
                        file.inputStream().use { input -> input.copyTo(output) }
                    }
                }
            }
            _state.value = SplitUiState(statusMessage = "Saved ${paths.size} files.")
        }
    }
}
