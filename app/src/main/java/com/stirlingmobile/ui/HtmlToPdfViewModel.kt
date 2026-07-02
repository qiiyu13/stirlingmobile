package com.stirlingmobile.ui

import android.content.Context
import android.net.Uri
import android.print.PrintAttributes
import android.print.PrintManager
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class HtmlToPdfUiState(
    val statusMessage: String = "Select an HTML file to convert",
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
/// actually part of the public API contract.
class HtmlToPdfViewModel : ViewModel() {
    private val _state = MutableStateFlow(HtmlToPdfUiState())
    val state: StateFlow<HtmlToPdfUiState> = _state

    fun onFilePicked(context: Context, uri: Uri) {
        viewModelScope.launch(Dispatchers.Main) {
            _state.value = HtmlToPdfUiState(statusMessage = "Rendering…")
            try {
                val html = withContext(Dispatchers.IO) {
                    context.contentResolver.openInputStream(uri)!!.bufferedReader().use { it.readText() }
                }
                printToPdf(context, html)
            } catch (e: Exception) {
                _state.value = HtmlToPdfUiState(statusMessage = "Failed: ${e.message}")
                return@launch
            }
            _state.value = HtmlToPdfUiState(statusMessage = "Choose \"Save as PDF\" in the print dialog to export.")
        }
    }

    private fun printToPdf(context: Context, html: String) {
        val webView = WebView(context)
        // ponytail: static HTML only in v1, no JS-driven content
        webView.settings.javaScriptEnabled = false
        webView.webViewClient = object : WebViewClient() {
            override fun onPageFinished(view: WebView?, url: String?) {
                val printManager = context.getSystemService(Context.PRINT_SERVICE) as PrintManager
                val printAdapter = webView.createPrintDocumentAdapter("html_to_pdf")
                printManager.print("html_to_pdf", printAdapter, PrintAttributes.Builder().build())
            }
        }
        webView.loadDataWithBaseURL(null, html, "text/html", "UTF-8", null)
    }
}
