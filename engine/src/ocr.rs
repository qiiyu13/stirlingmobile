//! OCR searchable-text layer (F-046). Kotlin runs Tesseract on rasterized page
//! images and hands back recognized words with pixel bounding boxes; this
//! overlays them as an *invisible* text layer (render mode 3) positioned over
//! the original page graphics, producing a searchable/selectable PDF — the
//! OCRmyPDF technique. Rust never runs Tesseract itself; it only owns the PDF.

use crate::content_util::{add_font, page_size, save_document};
use crate::EngineError;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object};

/// One recognized word. Coordinates are in image pixels with a top-left origin
/// (Tesseract / Android bitmap convention), relative to the page image whose
/// dimensions are given on the owning [`OcrPage`].
#[derive(Debug, Clone, uniffi::Record)]
pub struct OcrWord {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// OCR results for a single page. `page_index` is 0-based in document order.
#[derive(Debug, Clone, uniffi::Record)]
pub struct OcrPage {
    pub page_index: u32,
    pub image_width: f32,
    pub image_height: f32,
    pub words: Vec<OcrWord>,
}

/// Average Helvetica glyph advance as a fraction of the font size — used to
/// horizontally scale each word so its invisible text spans the same width as
/// the recognized image box (keeps selection/search aligned with the picture).
const HELVETICA_AVG_ADVANCE: f32 = 0.5;

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

/// Overlay an invisible OCR text layer onto `input_path` from `pages` and write
/// the searchable PDF to `output_path`. Pages/words with no id or no glyphs are
/// skipped; the original page graphics are untouched.
#[uniffi::export]
pub fn ocr_apply_text_layer(
    input_path: String,
    pages: Vec<OcrPage>,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    // get_pages(): 1-based page number -> object id, in document order.
    let page_map = doc.get_pages();

    for page in &pages {
        let Some(&page_id) = page_map.get(&(page.page_index + 1)) else {
            continue; // OCR page has no matching PDF page — skip.
        };
        if page.words.is_empty() || page.image_width <= 0.0 || page.image_height <= 0.0 {
            continue;
        }

        let (pw, ph) = page_size(&doc, page_id);
        let scale_x = pw / page.image_width;
        let scale_y = ph / page.image_height;

        add_font(&mut doc, page_id, "SMocr", "Helvetica");

        // Isolate our text block; render mode 3 = invisible (ISO 32000-1 §9.3.3).
        let mut ops = vec![
            Operation::new("q", vec![]),
            Operation::new("BT", vec![]),
            Operation::new("Tr", vec![3.into()]),
        ];

        for word in &page.words {
            let text = word.text.trim();
            if text.is_empty() {
                continue;
            }
            // Font size ~ glyph cap height ~ box height in points.
            let font_size = (word.height * scale_y).max(1.0);
            // Text baseline sits at the bottom of the image box; flip Y to the
            // PDF bottom-left origin.
            let x_pt = word.x * scale_x;
            let y_pt = ph - (word.y + word.height) * scale_y;
            // Horizontally squeeze/stretch so the word spans its box width.
            let box_w_pt = word.width * scale_x;
            let natural_w = text.chars().count() as f32 * font_size * HELVETICA_AVG_ADVANCE;
            let tz = if natural_w > 0.0 {
                (box_w_pt / natural_w * 100.0).clamp(1.0, 1000.0)
            } else {
                100.0
            };

            ops.push(Operation::new("Tf", vec!["SMocr".into(), font_size.into()]));
            ops.push(Operation::new("Tz", vec![tz.into()]));
            ops.push(Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    x_pt.into(),
                    y_pt.into(),
                ],
            ));
            ops.push(Operation::new("Tj", vec![Object::string_literal(text)]));
        }

        ops.push(Operation::new("ET", vec![]));
        ops.push(Operation::new("Q", vec![]));

        doc.add_to_page_content(page_id, Content { operations: ops })
            .map_err(|e| EngineError::WriteFailed {
                reason: e.to_string(),
            })?;
    }

    save(doc, &output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Stream};
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
    fn applies_invisible_text_layer() {
        let dir = temp_dir();
        let input = dir.join("ocr_in.pdf");
        let output = dir.join("ocr_out.pdf");
        one_page_pdf(&input);

        let pages = vec![OcrPage {
            page_index: 0,
            image_width: 1275.0, // 612pt @ 150dpi
            image_height: 1650.0,
            words: vec![OcrWord {
                text: "HELLO".to_string(),
                x: 100.0,
                y: 100.0,
                width: 200.0,
                height: 40.0,
            }],
        }];

        ocr_apply_text_layer(
            input.to_string_lossy().into_owned(),
            pages,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();

        // Font registered.
        let (resources, _) = doc.get_page_resources(page_id).unwrap();
        assert!(
            resources.unwrap().get(b"Font").is_ok(),
            "OCR font registered"
        );

        // Recognized text present in the content stream.
        assert!(doc.extract_text(&[1]).unwrap().contains("HELLO"));

        // Invisible render mode (`3 Tr`) is emitted.
        let content = doc.get_and_decode_page_content(page_id).unwrap();
        assert!(
            content.operations.iter().any(|op| op.operator == "Tr"
                && op.operands.first().and_then(|o| o.as_i64().ok()) == Some(3)),
            "invisible text render mode 3 present"
        );
    }

    #[test]
    fn skips_pages_without_match() {
        let dir = temp_dir();
        let input = dir.join("ocr_skip_in.pdf");
        let output = dir.join("ocr_skip_out.pdf");
        one_page_pdf(&input);

        // page_index 5 has no matching PDF page; empty-word page also skipped.
        let pages = vec![OcrPage {
            page_index: 5,
            image_width: 100.0,
            image_height: 100.0,
            words: vec![OcrWord {
                text: "GHOST".to_string(),
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            }],
        }];

        ocr_apply_text_layer(
            input.to_string_lossy().into_owned(),
            pages,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        assert!(!doc.extract_text(&[1]).unwrap().contains("GHOST"));
    }
}
