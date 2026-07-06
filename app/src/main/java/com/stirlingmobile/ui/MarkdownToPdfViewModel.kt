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
import uniffi.stirling_engine.convertMarkdownToHtml

data class MarkdownToPdfUiState(
    val statusMessage: String,
)

/// Markdown -> HTML (Rust, pulldown-cmark) -> PDF (same print pipeline as
/// HtmlToPdfViewModel/HtmlPrinter). Mirrors Stirling's own approach of
/// going through HTML rather than a bespoke PDF text-layout engine.
class MarkdownToPdfViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(MarkdownToPdfUiState(statusMessage = application.getString(R.string.tool_markdown_to_pdf_select_prompt)))
    val state: StateFlow<MarkdownToPdfUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch(Dispatchers.Main) {
            _state.value = MarkdownToPdfUiState(statusMessage = context.getString(R.string.tool_markdown_to_pdf_rendering))
            try {
                val markdown = withContext(Dispatchers.IO) {
                    context.contentResolver.openInputStream(uri)!!.bufferedReader().use { it.readText() }
                }
                val html = convertMarkdownToHtml(markdown)
                HtmlPrinter.printToPdf(context, html, "markdown_to_pdf")
            } catch (e: Exception) {
                _state.value = MarkdownToPdfUiState(statusMessage = context.getString(R.string.error_failed, e.message))
                return@launch
            }
            _state.value = MarkdownToPdfUiState(statusMessage = context.getString(R.string.tool_markdown_to_pdf_print_hint))
        }
    }
}
