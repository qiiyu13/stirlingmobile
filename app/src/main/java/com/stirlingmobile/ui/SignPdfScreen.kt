package com.stirlingmobile.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R

@Composable
fun SignPdfScreen(pipeline: PipelineState? = null, viewModel: SignPdfViewModel = viewModel()) {
    val context = LocalContext.current
    val state by viewModel.state.collectAsState()
    val pipelineCurrent = pipeline?.state?.collectAsState()?.value?.current
    LaunchedEffect(pipelineCurrent) {
        if (state.pdfPath == null && pipelineCurrent != null) {
            viewModel.usePipelineFile(pipelineCurrent.path)
        }
    }
    LaunchedEffect(state.resultFilePath) {
        state.resultFilePath?.let { pipeline?.push(it, "Signed") }
    }

    var pfxPassword by remember { mutableStateOf("") }
    var certify by remember { mutableStateOf(false) }
    var permission by remember { mutableStateOf<UByte>(1u) }

    val pickPdf = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onPdfPicked(context, it) } }

    val pickPfx = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? -> uri?.let { viewModel.onPfxPicked(context, it) } }

    val saveResult = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? -> if (uri != null) viewModel.onSaveDestinationChosen(context, uri) }

    Column(
        modifier = Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(if (certify) R.string.tool_sign_pdf_title_certify else R.string.tool_sign_pdf_title_sign))

        Button(onClick = { certify = !certify }) {
            Text(stringResource(if (certify) R.string.tool_sign_pdf_switch_to_plain else R.string.tool_sign_pdf_switch_to_certifying))
        }

        Button(onClick = { pickPdf.launch(arrayOf("application/pdf")) }) {
            Text(stringResource(if (state.pdfPath == null) R.string.action_select_pdf else R.string.action_select_different_pdf))
        }

        Text(state.statusMessage)

        if (state.pdfPath != null) {
            Button(onClick = { pickPfx.launch(arrayOf("application/x-pkcs12", "*/*")) }) {
                Text(stringResource(if (state.pfxPath == null) R.string.tool_sign_pdf_select_certificate else R.string.tool_sign_pdf_select_different_certificate))
            }
        }

        if (state.pfxPath != null) {
            OutlinedTextField(
                value = pfxPassword,
                onValueChange = { pfxPassword = it },
                label = { Text(stringResource(R.string.tool_sign_pdf_certificate_password_label)) },
                visualTransformation = PasswordVisualTransformation()
            )

            if (certify) {
                Text(stringResource(R.string.tool_sign_pdf_permission_label))
                val options = listOf(
                    1.toUByte() to stringResource(R.string.tool_sign_pdf_permission_no_changes),
                    2.toUByte() to stringResource(R.string.tool_sign_pdf_permission_form_fill_only),
                    3.toUByte() to stringResource(R.string.tool_sign_pdf_permission_form_fill_comments),
                )
                options.forEach { (value, label) ->
                    Button(onClick = { permission = value }) {
                        Text(if (permission == value) "[$label]" else label)
                    }
                }
            }

            Button(onClick = { viewModel.onSignClicked(pfxPassword, if (certify) permission else null) }) {
                Text(stringResource(if (certify) R.string.tool_sign_pdf_action_certify else R.string.tool_sign_pdf_action_sign))
            }
        }

        if (state.resultFilePath != null) {
            Button(onClick = { saveResult.launch("signed.pdf") }) {
                Text(stringResource(R.string.action_save_pdf))
            }
        }
    }
}
