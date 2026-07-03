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
import uniffi.stirling_engine.FieldFill
import uniffi.stirling_engine.FormField
import uniffi.stirling_engine.formsFill
import uniffi.stirling_engine.formsGetFields
import java.io.File
import java.util.UUID

data class FormsFillUiState(
    val statusMessage: String = "Select a PDF with form fields",
    val pdfPath: String? = null,
    val fields: List<FormField> = emptyList(),
    val fieldValues: Map<String, String> = emptyMap(),
    val busy: Boolean = false,
    val resultPath: String? = null,
)

class FormsFillViewModel : ViewModel() {
    private val _state = MutableStateFlow(FormsFillUiState())
    val state: StateFlow<FormsFillUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = FormsFillUiState(statusMessage = "Reading form fields…", busy = true)
            try {
                val (path, fields) = withContext(Dispatchers.IO) {
                    val workingDir = File(context.filesDir, "working").apply { mkdirs() }
                    val input = File(workingDir, "forms_fill_input.pdf")
                    context.contentResolver.openInputStream(uri)!!.use { it.copyTo(input.outputStream()) }
                    val fields = formsGetFields(input.absolutePath)
                    input.absolutePath to fields
                }
                val initialValues = fields.associate { it.name to (it.value ?: "") }
                _state.value = FormsFillUiState(
                    statusMessage = "${fields.size} field(s) found. Edit values below.",
                    pdfPath = path,
                    fields = fields,
                    fieldValues = initialValues,
                )
            } catch (e: Exception) {
                _state.value = FormsFillUiState(statusMessage = "Failed: ${e.message}")
            }
        }
    }

    fun onValueChanged(fieldName: String, value: String) {
        val updated = _state.value.fieldValues.toMutableMap()
        updated[fieldName] = value
        _state.value = _state.value.copy(fieldValues = updated)
    }

    fun onFill(context: Context) {
        val pdfPath = _state.value.pdfPath ?: return
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, statusMessage = "Filling fields…")
            try {
                val resultPath = withContext(Dispatchers.IO) {
                    val workingDir = File(pdfPath).parentFile!!
                    val output = File(workingDir, "forms_filled_${UUID.randomUUID()}.pdf")
                    val fills = _state.value.fieldValues.map { (name, value) ->
                        FieldFill(name, value)
                    }
                    formsFill(pdfPath, fills, output.absolutePath)
                    output.absolutePath
                }
                _state.value = _state.value.copy(
                    busy = false,
                    statusMessage = "Filled. Ready to save.",
                    resultPath = resultPath,
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, statusMessage = "Failed: ${e.message}")
            }
        }
    }

    fun onSave(context: Context, destination: Uri) {
        val path = _state.value.resultPath ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                File(path).inputStream().use { input ->
                    context.contentResolver.openOutputStream(destination, "wt")!!.use { output ->
                        input.copyTo(output)
                    }
                }
            }
            _state.value = _state.value.copy(statusMessage = "Saved.")
        }
    }
}
