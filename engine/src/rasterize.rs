use crate::EngineError;
use pdfium_render::prelude::*;
use std::path::Path;

/// Rasterizes every page of the PDF at `input_path` to a PNG at `dpi` and
/// writes them into `output_dir` as `page_1.png`, `page_2.png`, ... in
/// document order. Returns the written paths.
///
/// `pdfium_lib_dir` is the directory containing the platform's `libpdfium.so`
/// (Kotlin passes `context.applicationInfo.nativeLibraryDir`) — pdfium-render
/// dynamically loads the vendored PDFium binary from there rather than
/// statically linking it, since PDFium ships as a prebuilt binary per ABI
/// (see each ABI's jniLibs directory, e.g. jniLibs/arm64-v8a/libpdfium.so),
/// not something we compile ourselves.
#[uniffi::export]
pub fn convert_pdf_to_images(
    input_path: String,
    pdfium_lib_dir: String,
    dpi: u32,
    output_dir: String,
) -> Result<Vec<String>, EngineError> {
    let pdfium = bind_pdfium(&pdfium_lib_dir)?;

    let document = pdfium
        .load_pdf_from_file(&input_path, None)
        .map_err(|e| EngineError::ReadFailed {
            path: input_path.clone(),
            reason: e.to_string(),
        })?;

    let mut output_paths = Vec::new();
    for (index, page) in document.pages().iter().enumerate() {
        let target_width = (page.width().value / 72.0 * dpi as f32).round().max(1.0) as i32;
        let config = PdfRenderConfig::new().set_target_width(target_width);

        let bitmap = page.render_with_config(&config).map_err(|e| EngineError::WriteFailed {
            reason: format!("failed to render page {}: {e}", index + 1),
        })?;

        let output_path = Path::new(&output_dir)
            .join(format!("page_{}.png", index + 1))
            .to_string_lossy()
            .into_owned();
        bitmap
            .as_image()
            .save(&output_path)
            .map_err(|e| EngineError::WriteFailed {
                reason: format!("failed to save page {}: {e}", index + 1),
            })?;
        output_paths.push(output_path);
    }

    Ok(output_paths)
}

pub(crate) fn bind_pdfium(pdfium_lib_dir: &str) -> Result<Pdfium, EngineError> {
    let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(pdfium_lib_dir))
        .map_err(|e| EngineError::ReadFailed {
            path: pdfium_lib_dir.to_string(),
            reason: format!("failed to load libpdfium.so: {e}"),
        })?;
    Ok(Pdfium::new(bindings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, dictionary, Document, Object};
    use std::env::temp_dir;

    /// Points at the host-platform libpdfium built by `pdfium-render`'s
    /// build script isn't available in this workspace (only the Android
    /// .so's are vendored in jniLibs), so these tests only cover the parts
    /// that don't need to actually invoke Pdfium. Full rasterization is
    /// verified on-device.
    #[test]
    fn bind_pdfium_reports_missing_library_cleanly() {
        let result = bind_pdfium("/nonexistent/dir");
        assert!(result.is_err());
    }

    fn one_page_pdf(path: &std::path::Path) {
        let mut doc = Document::with_version("1.7");
        let content_id = doc.add_object(lopdf::Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
        ));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Count" => 1,
                "Kids" => vec![Object::Reference(page_id)],
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    #[test]
    fn convert_pdf_to_images_fails_cleanly_without_pdfium_lib() {
        let input = temp_dir().join("rasterize_test_input.pdf");
        one_page_pdf(&input);

        let result = convert_pdf_to_images(
            input.to_string_lossy().into_owned(),
            "/nonexistent/dir".to_string(),
            150,
            temp_dir().to_string_lossy().into_owned(),
        );
        assert!(result.is_err());
    }
}
