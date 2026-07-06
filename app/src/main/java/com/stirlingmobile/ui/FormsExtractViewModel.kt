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
import org.json.JSONObject
import uniffi.stirling_engine.FormField
import uniffi.stirling_engine.formsGetFields
import java.io.File

data class FormsExtractUiState(
    val statusMessage: String = "",
    val fields: List<FormField> = emptyList(),
    val busy: Boolean = false,
    val jsonText: String? = null,
    val csvText: String? = null,
)

class FormsExtractViewModel(application: Application) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(
        FormsExtractUiState(statusMessage = application.getString(R.string.tool_forms_extract_default_status))
    )
    val state: StateFlow<FormsExtractUiState> = _state

    fun onPdfPicked(context: Context, uri: Uri) {
        viewModelScope.launch {
            _state.value = FormsExtractUiState(busy = true, statusMessage = context.getString(R.string.tool_forms_extract_extracting))
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
                    statusMessage = context.getString(R.string.tool_forms_extract_found, fields.size),
                    fields = fields,
                    jsonText = json,
                    csvText = csv,
                )
            } catch (e: Exception) {
                _state.value = FormsExtractUiState(statusMessage = context.getString(R.string.error_generic_message, e.message))
            }
        }
    }

    fun onSaveJson(context: Context, destination: Uri) {
        val json = _state.value.jsonText ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                context.contentResolver.openOutputStream(destination, "wt")!!.use { it.write(json.toByteArray()) }
            }
            _state.value = _state.value.copy(statusMessage = context.getString(R.string.tool_forms_extract_saved_json))
        }
    }

    fun onSaveCsv(context: Context, destination: Uri) {
        val csv = _state.value.csvText ?: return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                context.contentResolver.openOutputStream(destination, "wt")!!.use { it.write(csv.toByteArray()) }
            }
            _state.value = _state.value.copy(statusMessage = context.getString(R.string.tool_forms_extract_saved_csv))
        }
    }

    private fun fieldValue(f: FormField): String = f.value ?: ""

    private fun buildJson(fields: List<FormField>): String {
        val root = JSONObject()
        fields.forEach { f ->
            val entry = JSONObject()
            entry.put("type", f.fieldType)
            entry.put("value", fieldValue(f))
            entry.put("page", f.page)
            root.put(f.name, entry)
        }
        return root.toString(2)
    }

    private fun csvField(value: String): String =
        if (value.any { it == ',' || it == '"' || it == '\n' || it == '\r' }) {
            "\"${value.replace("\"", "\"\"")}\""
        } else {
            value
        }

    private fun buildCsv(fields: List<FormField>): String {
        val header = "Name,Type,Value,Page"
        val rows = fields.joinToString("\n") { f ->
            listOf(f.name, f.fieldType, fieldValue(f), f.page.toString())
                .joinToString(",") { csvField(it) }
        }
        return "$header\n$rows"
    }
}
