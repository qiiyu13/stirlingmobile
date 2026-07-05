//! Stamp page numbers onto every page (F-063). `format` is a template where
//! `{n}` is the page's number (offset by `start_number`) and `{total}` is the
//! page count, e.g. `"Page {n} of {total}"` or just `"{n}"`.

use crate::content_util::{add_font, page_size, save_document};
use crate::EngineError;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object};

/// Distance from the page edge to the number, in points.
const MARGIN: f32 = 24.0;

/// Draw a page number on each page. `position` is one of
/// `bottom-center` (default), `bottom-left`, `bottom-right`, `top-left`,
/// `top-center`, `top-right`. `start_number` is the value shown on the first
/// page (usually 1). Numbers are drawn in black Helvetica at `font_size`.
#[uniffi::export]
pub fn content_page_numbers(
    input_path: String,
    position: String,
    format: String,
    start_number: u32,
    font_size: f32,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    // get_pages() is 1-indexed and ordered by page number.
    let pages: Vec<_> = {
        let mut v: Vec<_> = doc.get_pages().into_iter().collect();
        v.sort_by_key(|(n, _)| *n);
        v
    };
    let total = pages.len() as u32;

    for (idx, (_, page_id)) in pages.into_iter().enumerate() {
        let number = start_number + idx as u32;
        let label = format
            .replace("{n}", &number.to_string())
            .replace("{total}", &total.to_string());

        add_font(&mut doc, page_id, "SMpnF", "Helvetica");
        let (pw, ph) = page_size(&doc, page_id);
        // Rough Helvetica advance: ~0.5em average glyph width.
        let text_w = label.chars().count() as f32 * font_size * 0.5;
        let (x, y) = place(&position, pw, ph, text_w, font_size);

        let ops = vec![
            Operation::new("q", vec![]),
            Operation::new("rg", vec![0.into(), 0.into(), 0.into()]),
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["SMpnF".into(), font_size.into()]),
            Operation::new("Td", vec![x.into(), y.into()]),
            Operation::new("Tj", vec![Object::string_literal(label)]),
            Operation::new("ET", vec![]),
            Operation::new("Q", vec![]),
        ];
        doc.add_to_page_content(page_id, Content { operations: ops })
            .map_err(|e| EngineError::WriteFailed {
                reason: e.to_string(),
            })?;
    }

    doc.compress();
    save_document(&mut doc, &output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

/// Baseline origin (bottom-left of the text) for `position`. Top rows sit one
/// `font_size` below the top margin so the glyphs stay inside the page.
fn place(position: &str, pw: f32, ph: f32, text_w: f32, font_size: f32) -> (f32, f32) {
    let left = MARGIN;
    let center = (pw - text_w) / 2.0;
    let right = pw - MARGIN - text_w;
    let bottom = MARGIN;
    let top = ph - MARGIN - font_size;
    match position {
        "bottom-left" => (left, bottom),
        "bottom-right" => (right, bottom),
        "top-left" => (left, top),
        "top-center" => (center, top),
        "top-right" => (right, top),
        _ => (center, bottom), // bottom-center, the default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Content;
    use lopdf::{dictionary, Stream};
    use std::env::temp_dir;

    fn two_page_pdf(path: &std::path::Path) {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let mut kids = vec![];
        for _ in 0..2 {
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
            kids.push(Object::Reference(page_id));
        }
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Count" => 2,
                "Kids" => kids,
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
    fn numbers_each_page_with_format() {
        let dir = temp_dir();
        let input = dir.join("pn_in.pdf");
        let output = dir.join("pn_out.pdf");
        two_page_pdf(&input);

        content_page_numbers(
            input.to_string_lossy().into_owned(),
            "bottom-center".to_string(),
            "Page {n} of {total}".to_string(),
            1,
            12.0,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        assert!(doc.extract_text(&[1]).unwrap().contains("Page 1 of 2"));
        assert!(doc.extract_text(&[2]).unwrap().contains("Page 2 of 2"));
    }
}
