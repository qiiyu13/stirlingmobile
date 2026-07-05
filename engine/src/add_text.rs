//! Add a single line of text at an explicit position on one page (F-043).

use crate::content_util::{add_font, save_document};
use crate::EngineError;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object};

fn load(input_path: &str) -> Result<Document, EngineError> {
    Document::load(input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path.to_string(),
        reason: e.to_string(),
    })
}

fn save(mut doc: Document, output_path: &str) -> Result<(), EngineError> {
    doc.compress();
    save_document(&mut doc, output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

/// Draws `text` at `(x, y)` (points, origin bottom-left) on 1-indexed
/// `page_number` of the PDF at `input_path`, at `font_size`, and writes the
/// result to `output_path`.
#[uniffi::export]
pub fn content_add_text(
    input_path: String,
    page_number: u32,
    text: String,
    x: f32,
    y: f32,
    font_size: f32,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    let page_id = *doc
        .get_pages()
        .get(&page_number)
        .ok_or_else(|| EngineError::WriteFailed {
            reason: format!("page {page_number} does not exist"),
        })?;

    add_font(&mut doc, page_id, "SMaddF", "Helvetica");

    let ops = vec![
        Operation::new("q", vec![]),
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["SMaddF".into(), font_size.into()]),
        Operation::new("Td", vec![x.into(), y.into()]),
        Operation::new("Tj", vec![Object::string_literal(text)]),
        Operation::new("ET", vec![]),
        Operation::new("Q", vec![]),
    ];
    doc.add_to_page_content(page_id, Content { operations: ops })
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;

    save(doc, &output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content as LContent, dictionary, Stream};
    use std::env::temp_dir;

    fn one_page_pdf(path: &std::path::Path) {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            LContent { operations: vec![] }.encode().unwrap(),
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Contents" => content_id,
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
    fn draws_text_at_position() {
        let dir = temp_dir();
        let input = dir.join("add_text_test_input.pdf");
        let output = dir.join("add_text_test_output.pdf");
        one_page_pdf(&input);

        content_add_text(
            input.to_string_lossy().into_owned(),
            1,
            "Hello".to_string(),
            100.0,
            700.0,
            14.0,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        assert!(doc.extract_text(&[1]).unwrap().contains("Hello"));
    }

    #[test]
    fn rejects_out_of_range_page() {
        let dir = temp_dir();
        let input = dir.join("add_text_test_range_input.pdf");
        let output = dir.join("add_text_test_range_output.pdf");
        one_page_pdf(&input);

        let result = content_add_text(
            input.to_string_lossy().into_owned(),
            2,
            "Hello".to_string(),
            0.0,
            0.0,
            12.0,
            output.to_string_lossy().into_owned(),
        );
        assert!(result.is_err());
    }
}
