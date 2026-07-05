//! Markup annotations: highlight, underline, strikeout, and text notes.
use crate::content_util::save_document;

use crate::EngineError;
use lopdf::{dictionary, Document, Object};

fn load(input_path: &str) -> Result<Document, EngineError> {
    Document::load(input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path.to_string(),
        reason: e.to_string(),
    })
}

fn save(doc: &mut Document, output_path: &str) -> Result<(), EngineError> {
    save_document(doc, output_path).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

fn subtype_for(kind: &str) -> Result<&'static str, EngineError> {
    match kind {
        "highlight" => Ok("Highlight"),
        "underline" => Ok("Underline"),
        "strikeout" => Ok("StrikeOut"),
        "note" => Ok("Text"),
        other => Err(EngineError::WriteFailed {
            reason: format!("unsupported annotation kind '{other}' (expected highlight, underline, strikeout, or note)"),
        }),
    }
}

/// Adds a markup annotation to 1-indexed `page_number` of the PDF at
/// `input_path`: `kind` is one of `highlight`/`underline`/`strikeout`/`note`,
/// anchored to the rectangle `(x0, y0, x1, y1)` (points, origin bottom-left).
/// `note_text` is the popup body for `note` annotations (ignored otherwise).
/// Writes the result to `output_path`.
#[uniffi::export]
pub fn content_add_annotation(
    input_path: String,
    page_number: u32,
    kind: String,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    note_text: Option<String>,
    output_path: String,
) -> Result<(), EngineError> {
    let subtype = subtype_for(&kind)?;
    let mut doc = load(&input_path)?;
    let page_id = *doc
        .get_pages()
        .get(&page_number)
        .ok_or_else(|| EngineError::WriteFailed {
            reason: format!("page {page_number} does not exist"),
        })?;

    let rect = vec![
        Object::Real(x0),
        Object::Real(y0),
        Object::Real(x1),
        Object::Real(y1),
    ];
    let mut annot_dict = dictionary! {
        "Type" => "Annot",
        "Subtype" => subtype,
        "Rect" => rect,
    };
    if subtype != "Text" {
        // Markup annotations anchor to text via QuadPoints; a single quad
        // covering the given rect is enough since we're not doing text-run
        // detection - just a positioned mark.
        annot_dict.set(
            "QuadPoints",
            vec![
                Object::Real(x0),
                Object::Real(y1),
                Object::Real(x1),
                Object::Real(y1),
                Object::Real(x0),
                Object::Real(y0),
                Object::Real(x1),
                Object::Real(y0),
            ],
        );
    }
    if let Some(text) = note_text {
        annot_dict.set("Contents", Object::string_literal(text));
    }
    let annot_id = doc.add_object(annot_dict);

    let existing = doc
        .get_dictionary(page_id)
        .ok()
        .and_then(|d| d.get(b"Annots").ok())
        .and_then(|o| o.as_array().ok())
        .cloned()
        .unwrap_or_default();
    let mut annots = existing;
    annots.push(Object::Reference(annot_id));

    if let Ok(page) = doc.get_object_mut(page_id).and_then(Object::as_dict_mut) {
        page.set("Annots", annots);
    }

    save(&mut doc, &output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, dictionary, Stream};
    use std::env::temp_dir;

    fn one_page_pdf(path: &std::path::Path) {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
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
    fn adds_highlight_annotation() {
        let dir = temp_dir();
        let input = dir.join("annot_test_input.pdf");
        let output = dir.join("annot_test_output.pdf");
        one_page_pdf(&input);

        content_add_annotation(
            input.to_string_lossy().into_owned(),
            1,
            "highlight".to_string(),
            10.0,
            10.0,
            100.0,
            30.0,
            None,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let page = doc.get_dictionary(page_id).unwrap();
        let annots = page.get(b"Annots").unwrap().as_array().unwrap();
        assert_eq!(annots.len(), 1);
    }

    #[test]
    fn adds_note_with_text() {
        let dir = temp_dir();
        let input = dir.join("annot_note_test_input.pdf");
        let output = dir.join("annot_note_test_output.pdf");
        one_page_pdf(&input);

        content_add_annotation(
            input.to_string_lossy().into_owned(),
            1,
            "note".to_string(),
            10.0,
            10.0,
            30.0,
            30.0,
            Some("a comment".to_string()),
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let page = doc.get_dictionary(page_id).unwrap();
        let annots = page.get(b"Annots").unwrap().as_array().unwrap();
        let annot_dict = doc
            .get_dictionary(annots[0].as_reference().unwrap())
            .unwrap();
        assert_eq!(
            annot_dict.get(b"Subtype").unwrap().as_name_str().unwrap(),
            "Text"
        );
    }

    #[test]
    fn rejects_unknown_kind() {
        let dir = temp_dir();
        let input = dir.join("annot_bad_test_input.pdf");
        let output = dir.join("annot_bad_test_output.pdf");
        one_page_pdf(&input);

        let result = content_add_annotation(
            input.to_string_lossy().into_owned(),
            1,
            "sparkle".to_string(),
            0.0,
            0.0,
            1.0,
            1.0,
            None,
            output.to_string_lossy().into_owned(),
        );
        assert!(result.is_err());
    }
}
