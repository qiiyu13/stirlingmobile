//! Convert a PDF to a simple structural XML dump (text per page).

use crate::EngineError;
use lopdf::Document;

/// Converts the PDF at `input_path` to a simple XML document (one `<page>`
/// element per page, holding its extracted text) written to `output_path`.
/// Not a fidelity-preserving conversion - layout/formatting/images are
/// dropped, only text content is kept.
#[uniffi::export]
pub fn convert_pdf_to_xml(input_path: String, output_path: String) -> Result<(), EngineError> {
    let doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<document>\n");
    let mut page_numbers: Vec<u32> = doc.get_pages().keys().copied().collect();
    page_numbers.sort_unstable();
    for page_number in page_numbers {
        // lopdf's text extraction can fail per-page (e.g. an embedded font
        // with a ToUnicode CMap it can't parse) - note that honestly rather
        // than silently emitting an empty page, which reads as "this page
        // has no text" instead of "extraction failed".
        let text = match doc.extract_text(&[page_number]) {
            Ok(text) => text,
            Err(e) => format!("[text extraction failed: {e}]"),
        };
        xml.push_str(&format!(
            "  <page number=\"{page_number}\">{}</page>\n",
            escape_xml(&text)
        ));
    }
    xml.push_str("</document>\n");

    std::fs::write(&output_path, xml).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

fn escape_xml(text: &str) -> String {
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
        doc.save(path).unwrap();
    }

    #[test]
    fn wraps_page_text_in_xml() {
        let dir = temp_dir();
        let input = dir.join("convert_xml_test_input.pdf");
        let output = dir.join("convert_xml_test_output.xml");
        text_pdf(&input, "Hello & <World>");

        convert_pdf_to_xml(
            input.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let xml = std::fs::read_to_string(&output).unwrap();
        assert!(xml.contains("<page number=\"1\">"));
        assert!(xml.contains("Hello &amp;"));
        assert!(!xml.contains("<World>"));
    }
}
