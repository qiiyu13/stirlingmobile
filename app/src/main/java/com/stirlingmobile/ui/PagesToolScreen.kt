package com.stirlingmobile.ui

import android.app.Application
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
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
import com.stirlingmobile.R
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewmodel.compose.viewModel

private class PagesToolViewModelFactory(private val application: Application, private val mode: PagesToolMode) : ViewModelProvider.Factory {
    @Suppress("UNCHECKED_CAST")
    override fun <T : ViewModel> create(modelClass: Class<T>): T = PagesToolViewModel(application, mode) as T
}

@Composable
fun PagesToolScreen(mode: PagesToolMode, pipeline: PipelineState? = null) {
    val context = LocalContext.current
    val viewModel: PagesToolViewModel = viewModel(
        factory = PagesToolViewModelFactory(context.applicationContext as Application, mode)
    )
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.inputPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Pages edited") }
    }

    var pagesText by remember { mutableStateOf("") }

    val pickFile = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onFilePicked(context, it) } }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? ->
        if (uri != null) viewModel.onSaveDestinationChosen(context, uri)
    }

    Column(
        modifier = Modifier.padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(mode.labelRes))

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.inputPath == null) R.string.action_select_pdf else R.string.action_select_different_pdf))
        }

        Text(state.statusMessage)

        if (state.pageCount != null) {
            OutlinedTextField(
                value = pagesText,
                onValueChange = { pagesText = it },
                label = { Text(stringResource(R.string.tool_pages_tool_pages_field_label, state.pageCount.toString())) }
            )

            Button(onClick = { viewModel.onApplyClicked(pagesText) }) {
                Text(stringResource(mode.verbRes))
            }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("result.pdf") }) {
                Text(stringResource(R.string.action_save_pdf))
            }
        }
    }
}
