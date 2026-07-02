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
import uniffi.stirling_engine.generateSelfSignedPfx
import java.io.File
import java.util.UUID

data class GenerateCertificateUiState(
    val statusMessage: String = "Enter a name and password to generate a signing certificate",
    val resultFilePath: String? = null,
)

/// Self-signed RSA-2048 identity for use with the "Sign PDF (digital
/// signature)" tool. Always self-signed - readers will show "issuer
/// unknown" since this app can't act as a trusted CA. Fine for
/// personal/internal signing; use a CA-issued PFX for anything that needs
/// cross-organization trust.
class GenerateCertificateViewModel : ViewModel() {
    private val _state = MutableStateFlow(GenerateCertificateUiState())
    val state: StateFlow<GenerateCertificateUiState> = _state

    fun onGenerateClicked(context: Context, commonName: String, password: String) {
        viewModelScope.launch {
            _state.value = GenerateCertificateUiState(statusMessage = "Generating…")
            val outputPath = try {
                withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val output = File(workingDir, "generated_${UUID.randomUUID()}.pfx")
                    generateSelfSignedPfx(commonName, password, output.absolutePath)
                    output.absolutePath
                }
            } catch (e: Exception) {
                _state.value = GenerateCertificateUiState(statusMessage = "Generation failed: ${e.message}")
                return@launch
            }
            _state.value = GenerateCertificateUiState(statusMessage = "Done. Save it, then remember the password - it's not stored.", resultFilePath = outputPath)
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
            _state.value = GenerateCertificateUiState(statusMessage = "Saved.")
        }
    }
}
