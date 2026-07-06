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
import uniffi.stirling_engine.FieldFill
import uniffi.stirling_engine.FormField
import uniffi.stirling_engine.formsFill
import uniffi.stirling_engine.formsGetFields
import java.io.File
import java.util.UUID

data class FormsFillUiState(
    val statusMessage: String = "",
    val pdfPath: String? = null,
    val fields: List<FormField> = emptyList(),
    val fieldValues: Map<String, String> = emptyMap(),
    val busy: Boolean = false,
    val resultPath: String? = null,
)

class FormsFillViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(
        FormsFillUiState(statusMessage = application.getString(R.string.tool_forms_fill_default_status))
    )
    val state: StateFlow<FormsFillUiState> = _state

    fun usePipelineFile(path: String) {
        viewModelScope.launch {
            _state.value = FormsFillUiState(statusMessage = getApplication<Application>().getString(R.string.tool_forms_fill_reading), busy = true)
            val fields = withContext(Dispatchers.IO) { formsGetFields(path) }
            val initialValues = fields.associate { it.name to (it.value ?: "") }
            _state.value = FormsFillUiState(
                statusMessage = getApplication<Application>().getString(R.string.tool_forms_fill_found, fields.size),
                pdfPath = path,
                fields = fields,
                fieldValues = initialValues,
            )
        }
    }


    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = FormsFillUiState(statusMessage = context.getString(R.string.tool_forms_fill_reading), busy = true)
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
                    statusMessage = context.getString(R.string.tool_forms_fill_found, fields.size),
                    pdfPath = path,
                    fields = fields,
                    fieldValues = initialValues,
                )
            } catch (e: Exception) {
                _state.value = FormsFillUiState(statusMessage = context.getString(R.string.error_generic_message, e.message))
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
            _state.value = _state.value.copy(busy = true, statusMessage = context.getString(R.string.tool_forms_fill_filling))
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
                    statusMessage = context.getString(R.string.tool_forms_fill_ready_to_save),
                    resultPath = resultPath,
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, statusMessage = context.getString(R.string.error_generic_message, e.message))
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
            _state.value = _state.value.copy(statusMessage = context.getString(R.string.status_saved))
        }
    }
}
