package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
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
fun OverlayScreen(viewModel: OverlayViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    var opacity by remember { mutableStateOf("1.0") }

    val pickBase = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPickBase(context, it) }
    }
    val pickOverlay = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPickOverlay(context, it) }
    }
    val saveResult = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/pdf")) { uri: Uri? ->
        if (uri != null) viewModel.onSave(context, uri)
    }

    Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Overlay PDFs")
        Button(enabled = !state.busy, onClick = { pickBase.launch(arrayOf("application/pdf")) }) {
            Text(if (state.basePath == null) "Select base PDF" else "Base PDF selected")
        }
        Button(enabled = !state.busy, onClick = { pickOverlay.launch(arrayOf("application/pdf")) }) {
            Text(if (state.overlayPath == null) "Select overlay PDF" else "Overlay PDF selected")
        }
        if (state.busy) CircularProgressIndicator()
        Text(state.statusMessage)

        if (state.basePath != null && state.overlayPath != null && !state.busy) {
            OutlinedTextField(opacity, { opacity = it }, label = { Text("Opacity (0-1)") })
            Button(onClick = { viewModel.onApply(context, opacity.toFloatOrNull() ?: 1.0f) }) { Text("Overlay") }
        }

        if (state.resultPath != null) {
            Button(onClick = { saveResult.launch("overlaid.pdf") }) { Text("Save") }
        }
    }
}
