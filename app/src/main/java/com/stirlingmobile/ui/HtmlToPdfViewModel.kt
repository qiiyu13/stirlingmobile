package com.stirlingmobile.ui

import android.app.Application
import android.content.Context
import android.net.Uri
import com.stirlingmobile.R
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class HtmlToPdfUiState(
    val statusMessage: String = "",
)

/// Renders HTML to PDF via Android's system print framework (PDFium under
/// the hood, same engine Chrome's "Print > Save as PDF" uses). No Rust
/// engine involved here.
///
/// PrintDocumentAdapter.LayoutResultCallback/WriteResultCallback have
/// package-private constructors in the real android.jar (verified via
/// javap) -- apps can't build them to drive WebView's adapter headlessly
/// without reflecting into non-SDK internals, which Android's hidden-API
/// enforcement can break at any OS update. Routing through the real
/// PrintManager dialog (user picks "Save as PDF") is the only path that's
/// actually part of the public API contract. See HtmlPrinter.
class HtmlToPdfViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(
        HtmlToPdfUiState(statusMessage = application.getString(R.string.tool_html_to_pdf_default_status))
    )
    val state: StateFlow<HtmlToPdfUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch(Dispatchers.Main) {
            _state.value = HtmlToPdfUiState(statusMessage = context.getString(R.string.tool_html_to_pdf_rendering))
            try {
                val html = withContext(Dispatchers.IO) {
                    context.contentResolver.openInputStream(uri)!!.bufferedReader().use { it.readText() }
                }
                HtmlPrinter.printToPdf(context, html, "html_to_pdf")
            } catch (e: Exception) {
                _state.value = HtmlToPdfUiState(statusMessage = context.getString(R.string.error_generic_message, e.message))
                return@launch
            }
            _state.value = HtmlToPdfUiState(statusMessage = context.getString(R.string.tool_html_to_pdf_choose_save))
        }
    }
}
