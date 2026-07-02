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
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

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
        Text("Split PDF")

        Button(onClick = { pickFile.launch(arrayOf("application/pdf")) }) {
            Text(if (state.inputPath == null) "Select PDF" else "Select a different PDF")
        }

        Text(state.statusMessage)

        if (state.pageCount != null) {
            OutlinedTextField(
                value = splitPointsText,
                onValueChange = { splitPointsText = it },
                label = { Text("Split after pages, 1-${state.pageCount!! - 1u} (e.g. 3,6)") }
            )

            Button(onClick = { viewModel.onSplitClicked(context, splitPointsText) }) {
                Text("Split")
            }
        }

        if (state.outputPaths.isNotEmpty()) {
            Button(onClick = { pickSaveFolder.launch(null) }) {
                Text("Save all to folder")
            }
        }
    }
}
