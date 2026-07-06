package com.stirlingmobile.ui

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
fun SplitScreen(viewModel: SplitViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()

    var splitPointsText by remember { mutableStateOf("") }

    val pickFile = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onFilePicked(context, it) } }

    val pickSaveFolder = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocumentTree()
    ) { uri: Uri? ->
        if (uri != null) viewModel.onSaveDestinationChosen(context, uri)
    }

    Column(
        modifier = Modifier.padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(R.string.tool_split_title))

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.inputPath == null) R.string.action_select_pdf else R.string.action_select_different_pdf))
        }

        Text(state.statusMessage)

        if (state.pageCount != null) {
            OutlinedTextField(
                value = splitPointsText,
                onValueChange = { splitPointsText = it },
                label = { Text(stringResource(R.string.tool_split_input_label, (state.pageCount!! - 1u).toString())) }
            )

            Button(onClick = { viewModel.onSplitClicked(context, splitPointsText) }) {
                Text(stringResource(R.string.tool_split_action))
            }
        }

        if (state.outputPaths.isNotEmpty()) {
            Button(onClick = { pickSaveFolder.launch(null) }) {
                Text(stringResource(R.string.tool_split_save_all_action))
            }
        }
    }
}
