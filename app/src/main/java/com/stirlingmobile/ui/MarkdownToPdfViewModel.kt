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
import uniffi.stirling_engine.convertMarkdownToHtml

data class MarkdownToPdfUiState(
    val statusMessage: String = "Select a Markdown file to convert",
)

/// Markdown -> HTML (Rust, pulldown-cmark) -> PDF (same print pipeline as
/// HtmlToPdfViewModel/HtmlPrinter). Mirrors Stirling's own approach of
/// going through HTML rather than a bespoke PDF text-layout engine.
class MarkdownToPdfViewModel : ViewModel() {
    private val _state = MutableStateFlow(MarkdownToPdfUiState())
    val state: StateFlow<MarkdownToPdfUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch(Dispatchers.Main) {
            _state.value = MarkdownToPdfUiState(statusMessage = "Rendering…")
            try {
                val markdown = withContext(Dispatchers.IO) {
                    context.contentResolver.openInputStream(uri)!!.bufferedReader().use { it.readText() }
                }
                val html = convertMarkdownToHtml(markdown)
                HtmlPrinter.printToPdf(context, html, "markdown_to_pdf")
            } catch (e: Exception) {
                _state.value = MarkdownToPdfUiState(statusMessage = "Failed: ${e.message}")
                return@launch
            }
            _state.value = MarkdownToPdfUiState(statusMessage = "Choose \"Save as PDF\" in the print dialog to export.")
        }
    }
}
