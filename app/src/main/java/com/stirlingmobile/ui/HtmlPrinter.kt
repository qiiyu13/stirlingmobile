package com.stirlingmobile.ui

import android.content.Context
import android.print.PrintAttributes
import android.print.PrintManager
import android.webkit.WebView
import android.webkit.WebViewClient

/// Shared by HtmlToPdfViewModel and MarkdownToPdfViewModel. See
/// HtmlToPdfViewModel's class doc for why this goes through the system
/// PrintManager dialog instead of driving WebView's PrintDocumentAdapter
/// headlessly.
object HtmlPrinter {
    fun printToPdf(context: Context, html: String, jobName: String) {
        val webView = WebView(context)
        // ponytail: static HTML only in v1, no JS-driven content
        webView.settings.javaScriptEnabled = false
        webView.webViewClient = object : WebViewClient() {
            override fun onPageFinished(view: WebView?, url: String?) {
                val printManager = context.getSystemService(Context.PRINT_SERVICE) as PrintManager
                val printAdapter = webView.createPrintDocumentAdapter(jobName)
                printManager.print(jobName, printAdapter, PrintAttributes.Builder().build())
            }
        }
        webView.loadDataWithBaseURL(null, html, "text/html", "UTF-8", null)
    }
}
