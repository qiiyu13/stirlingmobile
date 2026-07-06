package com.stirlingmobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.stirlingmobile.R
import com.stirlingmobile.ui.AddTextScreen
import com.stirlingmobile.ui.AnnotationsScreen
import com.stirlingmobile.ui.AutoRedactScreen
import com.stirlingmobile.ui.CompareScreen
import com.stirlingmobile.ui.CompressScreen
import com.stirlingmobile.ui.ConvertHtmlDocScreen
import com.stirlingmobile.ui.ConvertXmlScreen
import com.stirlingmobile.ui.CropScreen
import com.stirlingmobile.ui.DedupePagesScreen
import com.stirlingmobile.ui.DrawScreen
import com.stirlingmobile.ui.ExtractImagesScreen
import com.stirlingmobile.ui.FormsExtractScreen
import com.stirlingmobile.ui.FormsFillScreen
import com.stirlingmobile.ui.FormsFlattenScreen
import com.stirlingmobile.ui.GenerateCertificateScreen
import com.stirlingmobile.ui.HtmlToPdfScreen
import com.stirlingmobile.ui.ImagesToPdfScreen
import com.stirlingmobile.ui.MarkdownToPdfScreen
import com.stirlingmobile.ui.MergeScreen
import com.stirlingmobile.ui.MetadataScreen
import com.stirlingmobile.ui.NUpScreen
import com.stirlingmobile.ui.OcrScreen
import com.stirlingmobile.ui.OptimizeScreen
import com.stirlingmobile.ui.PipelineState
import com.stirlingmobile.ui.OverlayScreen
import com.stirlingmobile.ui.PageNumbersScreen
import com.stirlingmobile.ui.PagesToolMode
import com.stirlingmobile.ui.PagesToolScreen
import com.stirlingmobile.ui.PasswordToolMode
import com.stirlingmobile.ui.PasswordToolScreen
import com.stirlingmobile.ui.PdfaScreen
import com.stirlingmobile.ui.PdfToImagesScreen
import com.stirlingmobile.ui.RedactScreen
import com.stirlingmobile.ui.ReorderScreen
import com.stirlingmobile.ui.RotateScreen
import com.stirlingmobile.ui.SanitizeScreen
import com.stirlingmobile.ui.ScaleScreen
import com.stirlingmobile.ui.SignPdfScreen
import com.stirlingmobile.ui.SignatureStampScreen
import com.stirlingmobile.ui.SplitScreen
import com.stirlingmobile.ui.WatermarkScreen

private enum class Tool { HOME, MERGE, SPLIT, ROTATE, REMOVE, EXTRACT, COMPRESS, OPTIMIZE, ADD_PASSWORD, REMOVE_PASSWORD, IMAGES_TO_PDF, PDF_TO_IMAGES, HTML_TO_PDF, MARKDOWN_TO_PDF, SIGNATURE_STAMP, SIGN_PDF, GENERATE_CERTIFICATE, REDACT, AUTO_REDACT, WATERMARK, PAGE_NUMBERS, SANITIZE, METADATA, OCR, FORMS_FILL, FORMS_FLATTEN, FORMS_EXTRACT, REORDER, N_UP, CROP, SCALE, COMPARE, OVERLAY, PDFA, CONVERT_XML, CONVERT_HTML_DOC, EXTRACT_IMAGES, DEDUPE_PAGES, ADD_TEXT, DRAW, ANNOTATIONS }

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface {
                    App()
                }
            }
        }
    }
}

@Composable
private fun App() {
    var tool by remember { mutableStateOf(Tool.HOME) }
    val pipeline: PipelineState = viewModel()
    Column {
    if (tool != Tool.HOME) {
        Button(onClick = { tool = Tool.HOME }) { Text(stringResource(R.string.nav_back_to_home)) }
    }
    when (tool) {
        Tool.HOME -> HomeScreen(pipeline = pipeline, onSelect = { tool = it })
        Tool.MERGE -> MergeScreen(pipeline = pipeline)
        Tool.SPLIT -> SplitScreen()
        Tool.ROTATE -> RotateScreen(pipeline = pipeline)
        Tool.REMOVE -> PagesToolScreen(PagesToolMode.REMOVE, pipeline = pipeline)
        Tool.EXTRACT -> PagesToolScreen(PagesToolMode.EXTRACT, pipeline = pipeline)
        Tool.COMPRESS -> CompressScreen(pipeline = pipeline)
        Tool.OPTIMIZE -> OptimizeScreen(pipeline = pipeline)
        Tool.ADD_PASSWORD -> PasswordToolScreen(PasswordToolMode.ADD, pipeline = pipeline)
        Tool.REMOVE_PASSWORD -> PasswordToolScreen(PasswordToolMode.REMOVE, pipeline = pipeline)
        Tool.IMAGES_TO_PDF -> ImagesToPdfScreen(pipeline = pipeline)
        Tool.PDF_TO_IMAGES -> PdfToImagesScreen()
        Tool.HTML_TO_PDF -> HtmlToPdfScreen()
        Tool.MARKDOWN_TO_PDF -> MarkdownToPdfScreen()
        Tool.SIGNATURE_STAMP -> SignatureStampScreen(pipeline = pipeline)
        Tool.SIGN_PDF -> SignPdfScreen(pipeline = pipeline)
        Tool.GENERATE_CERTIFICATE -> GenerateCertificateScreen()
        Tool.REDACT -> RedactScreen(pipeline = pipeline)
        Tool.AUTO_REDACT -> AutoRedactScreen(pipeline = pipeline)
        Tool.WATERMARK -> WatermarkScreen(pipeline = pipeline)
        Tool.PAGE_NUMBERS -> PageNumbersScreen(pipeline = pipeline)
        Tool.SANITIZE -> SanitizeScreen(pipeline = pipeline)
        Tool.METADATA -> MetadataScreen(pipeline = pipeline)
        Tool.OCR -> OcrScreen(pipeline = pipeline)
        Tool.FORMS_FILL -> FormsFillScreen(pipeline = pipeline)
        Tool.FORMS_FLATTEN -> FormsFlattenScreen(pipeline = pipeline)
        Tool.FORMS_EXTRACT -> FormsExtractScreen()
        Tool.REORDER -> ReorderScreen(pipeline = pipeline)
        Tool.N_UP -> NUpScreen(pipeline = pipeline)
        Tool.CROP -> CropScreen(pipeline = pipeline)
        Tool.SCALE -> ScaleScreen(pipeline = pipeline)
        Tool.COMPARE -> CompareScreen()
        Tool.OVERLAY -> OverlayScreen(pipeline = pipeline)
        Tool.PDFA -> PdfaScreen(pipeline = pipeline)
        Tool.CONVERT_XML -> ConvertXmlScreen(pipeline = pipeline)
        Tool.CONVERT_HTML_DOC -> ConvertHtmlDocScreen(pipeline = pipeline)
        Tool.EXTRACT_IMAGES -> ExtractImagesScreen(pipeline = pipeline)
        Tool.DEDUPE_PAGES -> DedupePagesScreen(pipeline = pipeline)
        Tool.ADD_TEXT -> AddTextScreen(pipeline = pipeline)
        Tool.DRAW -> DrawScreen(pipeline = pipeline)
        Tool.ANNOTATIONS -> AnnotationsScreen(pipeline = pipeline)
    }
    }
}

@Composable
private fun HomeScreen(pipeline: PipelineState, onSelect: (Tool) -> Unit) {
    val pipelineState by pipeline.state.collectAsState()
    Column(
        modifier = Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(stringResource(R.string.home_app_title))
        pipelineState.current?.let { entry ->
            Text(stringResource(R.string.home_pipeline_label, entry.label))
            if (pipelineState.canUndo) {
                Button(onClick = { pipeline.undo() }) { Text(stringResource(R.string.action_undo)) }
            }
        }
        Button(onClick = { onSelect(Tool.MERGE) }) { Text(stringResource(R.string.home_btn_merge)) }
        Button(onClick = { onSelect(Tool.SPLIT) }) { Text(stringResource(R.string.home_btn_split)) }
        Button(onClick = { onSelect(Tool.ROTATE) }) { Text(stringResource(R.string.home_btn_rotate)) }
        Button(onClick = { onSelect(Tool.REMOVE) }) { Text(stringResource(R.string.home_btn_remove_pages)) }
        Button(onClick = { onSelect(Tool.EXTRACT) }) { Text(stringResource(R.string.home_btn_extract_pages)) }
        Button(onClick = { onSelect(Tool.COMPRESS) }) { Text(stringResource(R.string.home_btn_compress)) }
        Button(onClick = { onSelect(Tool.OPTIMIZE) }) { Text(stringResource(R.string.home_btn_optimize)) }
        Button(onClick = { onSelect(Tool.ADD_PASSWORD) }) { Text(stringResource(R.string.home_btn_add_password)) }
        Button(onClick = { onSelect(Tool.REMOVE_PASSWORD) }) { Text(stringResource(R.string.home_btn_remove_password)) }
        Button(onClick = { onSelect(Tool.IMAGES_TO_PDF) }) { Text(stringResource(R.string.home_btn_images_to_pdf)) }
        Button(onClick = { onSelect(Tool.PDF_TO_IMAGES) }) { Text(stringResource(R.string.home_btn_pdf_to_images)) }
        Button(onClick = { onSelect(Tool.HTML_TO_PDF) }) { Text(stringResource(R.string.home_btn_html_to_pdf)) }
        Button(onClick = { onSelect(Tool.MARKDOWN_TO_PDF) }) { Text(stringResource(R.string.home_btn_markdown_to_pdf)) }
        Button(onClick = { onSelect(Tool.SIGNATURE_STAMP) }) { Text(stringResource(R.string.home_btn_signature_stamp)) }
        Button(onClick = { onSelect(Tool.SIGN_PDF) }) { Text(stringResource(R.string.home_btn_sign_pdf)) }
        Button(onClick = { onSelect(Tool.GENERATE_CERTIFICATE) }) { Text(stringResource(R.string.home_btn_generate_certificate)) }
        Button(onClick = { onSelect(Tool.REDACT) }) { Text(stringResource(R.string.home_btn_redact)) }
        Button(onClick = { onSelect(Tool.AUTO_REDACT) }) { Text(stringResource(R.string.home_btn_auto_redact)) }
        Button(onClick = { onSelect(Tool.WATERMARK) }) { Text(stringResource(R.string.home_btn_watermark)) }
        Button(onClick = { onSelect(Tool.PAGE_NUMBERS) }) { Text(stringResource(R.string.home_btn_page_numbers)) }
        Button(onClick = { onSelect(Tool.SANITIZE) }) { Text(stringResource(R.string.home_btn_sanitize)) }
        Button(onClick = { onSelect(Tool.METADATA) }) { Text(stringResource(R.string.home_btn_metadata)) }
        Button(onClick = { onSelect(Tool.OCR) }) { Text(stringResource(R.string.home_btn_ocr)) }
        Button(onClick = { onSelect(Tool.FORMS_FILL) }) { Text(stringResource(R.string.home_btn_forms_fill)) }
        Button(onClick = { onSelect(Tool.FORMS_FLATTEN) }) { Text(stringResource(R.string.home_btn_forms_flatten)) }
        Button(onClick = { onSelect(Tool.FORMS_EXTRACT) }) { Text(stringResource(R.string.home_btn_forms_extract)) }
        Button(onClick = { onSelect(Tool.REORDER) }) { Text(stringResource(R.string.home_btn_reorder)) }
        Button(onClick = { onSelect(Tool.N_UP) }) { Text(stringResource(R.string.home_btn_n_up)) }
        Button(onClick = { onSelect(Tool.CROP) }) { Text(stringResource(R.string.home_btn_crop)) }
        Button(onClick = { onSelect(Tool.SCALE) }) { Text(stringResource(R.string.home_btn_scale)) }
        Button(onClick = { onSelect(Tool.COMPARE) }) { Text(stringResource(R.string.home_btn_compare)) }
        Button(onClick = { onSelect(Tool.OVERLAY) }) { Text(stringResource(R.string.home_btn_overlay)) }
        Button(onClick = { onSelect(Tool.PDFA) }) { Text(stringResource(R.string.home_btn_pdfa)) }
        Button(onClick = { onSelect(Tool.CONVERT_XML) }) { Text(stringResource(R.string.home_btn_convert_xml)) }
        Button(onClick = { onSelect(Tool.CONVERT_HTML_DOC) }) { Text(stringResource(R.string.home_btn_convert_html_doc)) }
        Button(onClick = { onSelect(Tool.EXTRACT_IMAGES) }) { Text(stringResource(R.string.home_btn_extract_images)) }
        Button(onClick = { onSelect(Tool.DEDUPE_PAGES) }) { Text(stringResource(R.string.home_btn_dedupe_pages)) }
        Button(onClick = { onSelect(Tool.ADD_TEXT) }) { Text(stringResource(R.string.home_btn_add_text)) }
        Button(onClick = { onSelect(Tool.DRAW) }) { Text(stringResource(R.string.home_btn_draw)) }
        Button(onClick = { onSelect(Tool.ANNOTATIONS) }) { Text(stringResource(R.string.home_btn_annotations)) }
    }
}
