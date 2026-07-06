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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

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
        Text(stringResource(R.string.tool_compare_title))
        Button(enabled = !state.busy, onClick = { pickA.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pathA == null) stringResource(R.string.tool_compare_action_select_a) else stringResource(R.string.tool_compare_selected_a))
        }
        Button(enabled = !state.busy, onClick = { pickB.launch(arrayOf("application/pdf")) }) {
            Text(if (state.pathB == null) stringResource(R.string.tool_compare_action_select_b) else stringResource(R.string.tool_compare_selected_b))
        }
        if (state.busy) CircularProgressIndicator()
        Text(state.statusMessage)

        if (state.pathA != null && state.pathB != null && !state.busy) {
            Button(onClick = { viewModel.onCompare(context) }) { Text(stringResource(R.string.tool_compare_action_compare)) }
        }

        state.results.forEach { comparison ->
            Text(
                if (comparison.identical) stringResource(R.string.tool_compare_result_identical, comparison.page.toInt())
                else stringResource(R.string.tool_compare_result_differs, comparison.page.toInt()) + (comparison.diffImagePath?.let { " ($it)" } ?: "")
            )
        }
    }
}
