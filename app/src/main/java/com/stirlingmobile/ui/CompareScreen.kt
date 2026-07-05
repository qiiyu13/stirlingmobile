package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun CompareScreen(viewModel: CompareViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()

    val pickA = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPickA(context, it) }
    }
    val pickB = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        uri?.let { viewModel.onPickB(context, it) }
    }

    Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Compare PDFs")
        Button(enabled = !state.busy, onClick = { pickA.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pathA == null) "Select PDF A" else "PDF A selected")
        }
        Button(enabled = !state.busy, onClick = { pickB.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pathB == null) "Select PDF B" else "PDF B selected")
        }
        if (state.busy) CircularProgressIndicator()
        Text(state.statusMessage)

        if (state.pathA != null && state.pathB != null && !state.busy) {
            Button(onClick = { viewModel.onCompare(context) }) { Text("Compare") }
        }

        state.results.forEach { comparison ->
            Text(
                if (comparison.identical) "Page ${comparison.page}: identical"
                else "Page ${comparison.page}: differs${comparison.diffImagePath?.let { " ($it)" } ?: ""}"
            )
        }
    }
}
