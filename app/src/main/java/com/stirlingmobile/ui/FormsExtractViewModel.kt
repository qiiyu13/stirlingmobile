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
import uniffi.stirling_engine.FormField
import uniffi.stirling_engine.formsGetFields
import java.io.File

data class FormsExtractUiState(
    val statusMessage: String = "Select a PDF to extract form data",
    val fields: List<FormField> = emptyList(),
    val busy: Boolean = false,
    val jsonText: String? = null,
    val csvText: String? = null,
)

class FormsExtractViewModel : ViewModel() {
    private val _state = MutableStateFlow(FormsExtractUiState())
    val state: StateFlow<FormsExtractUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = FormsExtractUiState(busy = true, statusMessage = "Extracting fields…")
            try {
                val fields = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "forms_extract_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    formsGetFields(input.absolutePath)
                }
                val json = buildJson(fields)
                val csv = buildCsv(fields)
                _state.value = FormsExtractUiState(
                    statusMessage = "${fields.size} field(s) found.",
                    fields = fields,
                    jsonText = json,
                    csvText = csv,
                )
            } catch (e: Exception) {
                _state.value = FormsExtractUiState(statusMessage = "Failed: ${e.message}")
            }
        }
    }

    fun onSaveJson(context: Context, destination: Uri) {
        val json = _state.value.jsonText ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                context.contentResolver.openOutputStream(destination, "wt")!!.use { it.write(json.toByteArray()) }
            }
            _state.value = _state.value.copy(statusMessage = "Saved JSON.")
        }
    }

    fun onSaveCsv(context: Context, destination: Uri) {
        val csv = _state.value.csvText ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                context.contentResolver.openOutputStream(destination, "wt")!!.use { it.write(csv.toByteArray()) }
            }
            _state.value = _state.value.copy(statusMessage = "Saved CSV.")
        }
    }

    private fun fieldValue(f: FormField): String = f.value ?: ""

    private fun buildJson(fields: List<FormField>): String {
        val entries = fields.joinToString(",\n  ") { f ->
            "\"${f.name}\": {\"type\": \"${f.fieldType}\", \"value\": \"${fieldValue(f)}\", \"page\": ${f.page}}"
        }
        return "{\n  $entries\n}"
    }

    private fun buildCsv(fields: List<FormField>): String {
        val header = "Name,Type,Value,Page"
        val rows = fields.joinToString("\n") { f ->
            "${f.name},${f.fieldType},${fieldValue(f)},${f.page}"
        }
        return "$header\n$rows"
    }
}
