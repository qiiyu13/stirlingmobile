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
import uniffi.stirling_engine.getPageCount
import uniffi.stirling_engine.stampSignatureImage
import java.io.File
import java.util.UUID

data class SignatureStampUiState(
    val statusMessage: String = "Select a PDF",
    val pdfPath: String? = null,
    val pageCount: UInt? = null,
    val signaturePath: String? = null,
    val resultFilePath: String? = null,
)

/// Visual signature stamp: overlays a signature image onto one page. No
/// cryptographic signing - see docs/09-security.md for the planned
/// PKCS#12 security_sign/security_certify tools.
class SignatureStampViewModel : ViewModel() {
    private val _state = MutableStateFlow(SignatureStampUiState())
    val state: StateFlow<SignatureStampUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = SignatureStampUiState(statusMessage = "Reading…")
            try {
                val (path, pageCount) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "stamp_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    input.absolutePath to getPageCount(input.absolutePath)
                }
                _state.value = SignatureStampUiState(
                    statusMessage = "$pageCount pages. Now select a signature image.",
                    pdfPath = path,
                    pageCount = pageCount,
                )
            } catch (e: Exception) {
                _state.value = SignatureStampUiState(statusMessage = "Failed to read: ${e.message}")
            }
        }
    }

    fun onSignaturePicked(context: Context, uri: Uri) {
        val pdfPath = state.value.pdfPath ?: return
        viewModelScope.launch(Dispatchers.IO) {
            val workingDir = File(pdfPath).parentFile!!
            val dest = File(workingDir, "stamp_signature.png")
            context.contentResolver.openInputStream(uri)!!.use { it.copyTo(dest.outputStream()) }
            _state.value = state.value.copy(
                statusMessage = "Choose page and position, then Stamp.",
                signaturePath = dest.absolutePath,
            )
        }
    }

    fun onStampClicked(pageNumber: UInt, position: String) {
        val pdfPath = state.value.pdfPath ?: return
        val signaturePath = state.value.signaturePath ?: return

        viewModelScope.launch {
            _state.value = state.value.copy(statusMessage = "Stamping…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(pdfPath).parentFile!!
                    val output = File(workingDir, "stamp_result_${UUID.randomUUID()}.pdf")
                    stampSignatureImage(pdfPath, signaturePath, pageNumber, position, output.absolutePath)
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = state.value.copy(statusMessage = "Stamp failed: ${e.message}")
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
                    context.contentResolver.openOutputStream(destination)!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = SignatureStampUiState(statusMessage = "Saved.")
        }
    }
}
