package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

private val NAMED_PATTERNS = listOf("email" to "Email addresses", "phone_us" to "US phone numbers", "ssn" to "SSNs", "credit_card" to "Credit card numbers")

@Composable
fun AutoRedactScreen(viewModel: AutoRedactViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()

    val selected = remember { mutableStateOf(setOf<String>()) }
    var searchText by remember { mutableStateOf("") }
    var customRegex by remember { mutableStateOf("") }

    val pickPdf = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onPdfPicked(context, it) } }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? -> if (uri != null) viewModel.onSaveDestinationChosen(context, uri) }

    Column(
        modifier = Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Auto-Redact PDF")

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pdfPath == null) "Select PDF" else "Select a different PDF")
        }

        Text(state.statusMessage)

        if (state.pdfPath != null) {
            NAMED_PATTERNS.forEach { (key, label) ->
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Checkbox(
                        checked = selected.value.contains(key),
                        onCheckedChange = { checked ->
                            selected.value = if (checked) selected.value + key else selected.value - key
                        },
                    )
                    Text(label)
                }
            }

            OutlinedTextField(
                value = searchText,
                onValueChange = { searchText = it },
                label = { Text("Search for text (optional)") },
            )

            OutlinedTextField(
                value = customRegex,
                onValueChange = { customRegex = it },
                label = { Text("Advanced: custom regex (optional)") },
            )

            Button(onClick = {
                val patterns = selected.value.toMutableList()
                if (searchText.isNotBlank()) patterns.add("text:${searchText.trim()}")
                if (customRegex.isNotBlank()) patterns.add("regex:${customRegex.trim()}")
                viewModel.onRedactClicked(context, patterns)
            }) {
                Text("Redact")
            }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("redacted.pdf") }) { Text("Save PDF") }
        }
    }
}
