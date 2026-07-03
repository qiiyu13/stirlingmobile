package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun FormsFillScreen(viewModel: FormsFillViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()

    val pickPdf = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPdfPicked(context, it) }
    }
    val saveResult = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/pdf")) { uri: Uri? ->
        if (uri != null) viewModel.onSave(context, uri)
    }

    Column(
        modifier = Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Fill Form Fields")

        Button(enabled = !state.busy, onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text("Select PDF")
        }

        if (state.busy) {
            CircularProgressIndicator()
        }
        Text(state.statusMessage)

        if (state.fields.isNotEmpty()) {
            state.fields.forEach { field ->
                val currentValue = state.fieldValues[field.name] ?: ""
                OutlinedTextField(
                    value = currentValue,
                    onValueChange = { viewModel.onValueChanged(field.name, it) },
                    label = { Text("${field.name} (${field.fieldType})") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
            }
            Button(onClick = { viewModel.onFill(context) }) { Text("Fill & Save") }
        }

        if (state.resultPath != null) {
            Button(onClick = { saveResult.launch("filled.pdf") }) { Text("Save filled PDF") }
        }
    }
}
