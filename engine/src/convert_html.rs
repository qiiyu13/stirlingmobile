//! Convert a PDF to a single HTML file (text per page, no layout fidelity).
use crate::content_util::save_document;

use crate::EngineError;
use lopdf::Document;

/// Converts the PDF at `input_path` to a single HTML file (one `<section>`
/// per page holding its extracted text) written to `output_path`. Not a
/// fidelity-preserving conversion - layout/formatting/images are dropped,
/// only text content is kept.
#[uniffi::export]
pub fn convert_pdf_to_html(input_path: String, output_path: String) -> Result<(), EngineError> {
    let doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    let mut html =
        String::from("<!doctype html>\n<html><head><meta charset=\"utf-8\"></head><body>\n");
    let mut page_numbers: Vec<u32> = doc.get_pages().keys().copied().collect();
    page_numbers.sort_unstable();
    for page_number in page_numbers {
        // See convert_xml.rs: lopdf's text extraction can fail per-page
        // (e.g. an embedded font with an unparseable ToUnicode CMap) - surface
        // that instead of silently emitting an empty page.
        let text = match doc.extract_text(&[page_number]) {
            Ok(text) => text,
            Err(e) => format!("[text extraction failed: {e}]"),
        };
        html.push_str(&format!(
            "<section id=\"page-{page_number}\"><pre>{}</pre></section>\n",
            escape_html(&text)
        ));
    }
    html.push_str("</body></html>\n");

    std::fs::write(&output_path, html).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, dictionary, Object, Stream};
    use std::env::temp_dir;

    fn text_pdf(path: &std::path::Path, text: &str) {
        let mut doc = Document::with_version("1.7");
        let content = Content {
            operations: vec![
                lopdf::content::Operation::new("BT", vec![]),
                lopdf::content::Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
                lopdf::content::Operation::new(
                    "Tj",
                    vec![Object::string_literal(text.as_bytes().to_vec())],
                ),
                lopdf::content::Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => Object::Reference(font_id) },
        });
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => Object::Reference(resources_id),
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
        save_document(&mut doc, path).unwrap();
    }

    #[test]
    fn wraps_page_text_in_html() {
        let dir = temp_dir();
        let input = dir.join("convert_html_test_input.pdf");
        let output = dir.join("convert_html_test_output.html");
        text_pdf(&input, "Hi & <there>");

        convert_pdf_to_html(
            input.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("id=\"page-1\""));
        assert!(html.contains("Hi &amp;"));
        assert!(!html.contains("<there>"));
    }
}
