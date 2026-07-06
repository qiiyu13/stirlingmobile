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

@Composable
fun AutoRedactScreen(pipeline: PipelineState? = null, viewModel: AutoRedactViewModel = viewModel()) {
    val context = LocalContext.current
    val namedPatterns = listOf(
        "email" to stringResource(R.string.tool_auto_redact_pattern_email),
        "phone_us" to stringResource(R.string.tool_auto_redact_pattern_phone_us),
        "ssn" to stringResource(R.string.tool_auto_redact_pattern_ssn),
        "credit_card" to stringResource(R.string.tool_auto_redact_pattern_credit_card),
    )
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.pdfPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, context.getString(R.string.tool_auto_redact_history_label)) }
    }

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
        Text(stringResource(R.string.tool_auto_redact_title))

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pdfPath == null) stringResource(R.string.action_select_pdf) else stringResource(R.string.action_select_different_pdf))
        }

        Text(state.statusMessage)

        if (state.pdfPath != null) {
            namedPatterns.forEach { (key, label) ->
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
                label = { Text(stringResource(R.string.tool_auto_redact_label_search_text)) },
            )

            OutlinedTextField(
                value = customRegex,
                onValueChange = { customRegex = it },
                label = { Text(stringResource(R.string.tool_auto_redact_label_custom_regex)) },
            )

            Button(onClick = {
                val patterns = selected.value.toMutableList()
                if (searchText.isNotBlank()) patterns.add("text:${searchText.trim()}")
                if (customRegex.isNotBlank()) patterns.add("regex:${customRegex.trim()}")
                viewModel.onRedactClicked(context, patterns)
            }) {
                Text(stringResource(R.string.tool_auto_redact_action_redact))
            }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("redacted.pdf") }) { Text(stringResource(R.string.action_save_pdf)) }
        }
    }
}
