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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

private val POSITIONS = listOf(
    "bottom-center", "bottom-left", "bottom-right", "top-center", "top-left", "top-right"
)

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun PageNumbersScreen(pipeline: PipelineState? = null, viewModel: PageNumbersViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.pdfPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Page numbers added") }
    }

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
        Text(stringResource(R.string.tool_page_numbers_title))

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.pdfPath == null) R.string.action_select_pdf else R.string.action_select_different_pdf))
        }
        Text(state.statusMessage)

        if (state.pdfPath != null) {
            Text(stringResource(R.string.tool_page_numbers_position_label))
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                POSITIONS.forEach { p ->
                    // ponytail: position id doubles as its own display label (bottom-center, top-left, …); not translated since it's also the value passed to the engine.
                    FilterChip(selected = position == p, onClick = { position = p }, label = { Text(p) })
                }
            }
            OutlinedTextField(format, { format = it }, label = { Text(stringResource(R.string.tool_page_numbers_format_label)) })
            OutlinedTextField(startNumber, { startNumber = it }, label = { Text(stringResource(R.string.tool_page_numbers_start_label)) })
            OutlinedTextField(fontSize, { fontSize = it }, label = { Text(stringResource(R.string.tool_page_numbers_font_size_label)) })

            Button(onClick = {
                viewModel.apply(
                    position,
                    format,
                    startNumber.toUIntOrNull() ?: 1u,
                    fontSize.toFloatOrNull() ?: 12f,
                )
            }) { Text(stringResource(R.string.action_apply)) }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("numbered.pdf") }) { Text(stringResource(R.string.action_save_pdf)) }
        }
    }
}
