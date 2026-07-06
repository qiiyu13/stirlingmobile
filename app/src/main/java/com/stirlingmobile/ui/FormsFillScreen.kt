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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

@Composable
fun FormsFillScreen(pipeline: PipelineState? = null, viewModel: FormsFillViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.pdfPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultPath) {
        state.resultPath?.let { pipeline?.push(it, "Filled") }
    }

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
        Text(stringResource(R.string.tool_forms_fill_title))

        Button(enabled = !state.busy, onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(R.string.action_select_pdf))
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
                    label = { Text(stringResource(R.string.tool_forms_fill_field_label, field.name, field.fieldType)) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
            }
            Button(onClick = { viewModel.onFill(context) }) { Text(stringResource(R.string.tool_forms_fill_action)) }
        }

        if (state.resultPath != null) {
            Button(onClick = { saveResult.launch("filled.pdf") }) { Text(stringResource(R.string.tool_forms_fill_save)) }
        }
    }
}
