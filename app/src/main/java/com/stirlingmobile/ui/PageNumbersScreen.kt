package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
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

private val POSITIONS = listOf(
    "bottom-center", "bottom-left", "bottom-right", "top-center", "top-left", "top-right"
)

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun PageNumbersScreen(viewModel: PageNumbersViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()

    var position by remember { mutableStateOf("bottom-center") }
    var format by remember { mutableStateOf("{n}") }
    var startNumber by remember { mutableStateOf("1") }
    var fontSize by remember { mutableStateOf("12") }

    val pickPdf = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPdfPicked(context, it) }
    }
    val saveResult = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/pdf")) { uri: Uri? ->
        if (uri != null) viewModel.onSaveDestinationChosen(context, uri)
    }

    Column(
        modifier = Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Add Page Numbers")

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pdfPath == null) "Select PDF" else "Select a different PDF")
        }
        Text(state.statusMessage)

        if (state.pdfPath != null) {
            Text("Position")
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                POSITIONS.forEach { p ->
                    FilterChip(selected = position == p, onClick = { position = p }, label = { Text(p) })
                }
            }
            OutlinedTextField(format, { format = it }, label = { Text("Format ({n}, {total})") })
            OutlinedTextField(startNumber, { startNumber = it }, label = { Text("Start number") })
            OutlinedTextField(fontSize, { fontSize = it }, label = { Text("Font size") })

            Button(onClick = {
                viewModel.apply(
                    position,
                    format,
                    startNumber.toUIntOrNull() ?: 1u,
                    fontSize.toFloatOrNull() ?: 12f,
                )
            }) { Text("Apply") }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("numbered.pdf") }) { Text("Save PDF") }
        }
    }
}
