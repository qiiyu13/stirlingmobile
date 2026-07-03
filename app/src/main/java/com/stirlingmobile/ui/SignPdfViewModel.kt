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
import uniffi.stirling_engine.certifyPdf
import uniffi.stirling_engine.signPdf
import java.io.File
import java.util.UUID

data class SignPdfUiState(
    val statusMessage: String = "Select a PDF",
    val pdfPath: String? = null,
    val pfxPath: String? = null,
    val resultFilePath: String? = null,
)

/// Cryptographic PKCS#12 signing (detached PKCS#7/CMS, RSA+SHA-256). See
/// docs/09-security.md - the PFX password never leaves this flow's scope
/// and is zeroed on the Rust side after use. `permission` non-null means
/// certify (sets /DocMDP) instead of a plain approval signature - must be
/// the document's first signature, per ISO 32000-1 §12.8.2.2.
class SignPdfViewModel : ViewModel() {
    private val _state = MutableStateFlow(SignPdfUiState())
    val state: StateFlow<SignPdfUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = SignPdfUiState(statusMessage = "Reading…")
            try {
                val path = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "sign_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath
                }
                _state.value = SignPdfUiState(statusMessage = "Now select your signing certificate (.pfx/.p12).", pdfPath = path)
            } catch (e: Exception) {
                _state.value = SignPdfUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onPfxPicked(context: Context, uri: Uri) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch(Dispatchers.IO) {
            val workingDir = File(pdfPath).parentFile!!
            val dest = File(workingDir, "sign_identity.pfx")
            context.contentResolver.openInputStream(uri)!!.use { it.copyTo(dest.outputStream()) }
            _state.value = state.value.copy(statusMessage = "Enter the certificate password, then Sign.", pfxPath = dest.absolutePath)
        }
    }

    fun onSignClicked(pfxPassword: String, certifyPermission: UByte?) {
        val pdfPath = state.value.pdfPath ?: return
        val pfxPath = state.value.pfxPath ?: return

        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = if (certifyPermission != null) "Certifying…" else "Signing…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(pdfPath).parentFile!!
                    val output = File(workingDir, "sign_result_${UUID.randomUUID()}.pdf")
                    if (certifyPermission != null) {
                        certifyPdf(pdfPath, pfxPath, pfxPassword, certifyPermission, output.absolutePath)
                    } else {
                        signPdf(pdfPath, pfxPath, pfxPassword, output.absolutePath)
                    }
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Signing failed: ${e.message}")
                return@launch
            }
            _state.value = state.value.copy(statusMessage = "Done. Ready to save.", resultFilePath = outputPath)
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
            _state.value = SignPdfUiState(statusMessage = "Saved.")
        }
    }
}
