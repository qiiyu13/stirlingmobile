//! Freehand drawing: append vector strokes onto a page's content stream.

use crate::content_util::page_size;
use crate::EngineError;
use lopdf::content::{Content, Operation};
use lopdf::Document;

/// One freehand stroke: a polyline of points (points, origin bottom-left),
/// an RGB color (0.0..=1.0 each), and a line width in points.
#[derive(Debug, Clone, uniffi::Record)]
pub struct Stroke {
    pub points_x: Vec<f32>,
    pub points_y: Vec<f32>,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub width: f32,
}

/// A page's `/MediaBox` size in points (width, height).
#[derive(Debug, Clone, uniffi::Record)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
}

/// Returns the size (in points) of 1-indexed `page_number` of the PDF at
/// `input_path`. Callers that let a user draw/place content on a rendered
/// preview of the page need this to convert on-screen pixel coordinates into
/// PDF point coordinates - the two are not the same scale.
#[uniffi::export]
pub fn pdf_page_size(input_path: String, page_number: u32) -> Result<PageSize, EngineError> {
    let doc = load(&input_path)?;
    let page_id = *doc
        .get_pages()
        .get(&page_number)
        .ok_or_else(|| EngineError::WriteFailed {
            reason: format!("page {page_number} does not exist"),
        })?;
    let (width, height) = page_size(&doc, page_id);
    Ok(PageSize { width, height })
}

fn load(input_path: &str) -> Result<Document, EngineError> {
    Document::load(input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path.to_string(),
        reason: e.to_string(),
    })
}

fn save(mut doc: Document, output_path: &str) -> Result<(), EngineError> {
    doc.compress();
    doc.save(output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

/// Draws `strokes` onto 1-indexed `page_number` of the PDF at `input_path`
/// and writes the result to `output_path`. Each stroke's `points_x`/`points_y`
/// must have equal, non-empty length.
#[uniffi::export]
pub fn content_draw(
    input_path: String,
    page_number: u32,
    strokes: Vec<Stroke>,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    let page_id = *doc
        .get_pages()
        .get(&page_number)
        .ok_or_else(|| EngineError::WriteFailed {
            reason: format!("page {page_number} does not exist"),
        })?;

    let mut ops = vec![Operation::new("q", vec![])];
    for stroke in &strokes {
        if stroke.points_x.len() != stroke.points_y.len() || stroke.points_x.is_empty() {
            continue;
        }
        ops.push(Operation::new(
            "RG",
            vec![
                stroke.color_r.into(),
                stroke.color_g.into(),
                stroke.color_b.into(),
            ],
        ));
        ops.push(Operation::new("w", vec![stroke.width.into()]));
        ops.push(Operation::new(
            "m",
            vec![stroke.points_x[0].into(), stroke.points_y[0].into()],
        ));
        for i in 1..stroke.points_x.len() {
            ops.push(Operation::new(
                "l",
                vec![stroke.points_x[i].into(), stroke.points_y[i].into()],
            ));
        }
        ops.push(Operation::new("S", vec![]));
    }
    ops.push(Operation::new("Q", vec![]));

    doc.add_to_page_content(page_id, Content { operations: ops })
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;

    save(doc, &output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content as LContent, dictionary, Object, Stream};
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
        doc.save(path).unwrap();
    }

    #[test]
    fn draws_stroke_onto_page_content() {
        let dir = temp_dir();
        let input = dir.join("draw_test_input.pdf");
        let output = dir.join("draw_test_output.pdf");
        one_page_pdf(&input);

        let stroke = Stroke {
            points_x: vec![10.0, 20.0, 30.0],
            points_y: vec![10.0, 40.0, 10.0],
            color_r: 1.0,
            color_g: 0.0,
            color_b: 0.0,
            width: 2.0,
        };
        content_draw(
            input.to_string_lossy().into_owned(),
            1,
            vec![stroke],
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let content = doc.get_page_content(page_id).unwrap();
        let decoded = String::from_utf8_lossy(&content);
        assert!(decoded.lines().any(|l| l == "S"));
        assert!(decoded.contains(" m"));
    }

    #[test]
    fn page_size_matches_media_box() {
        let dir = temp_dir();
        let input = dir.join("page_size_test_input.pdf");
        one_page_pdf(&input);

        let size = pdf_page_size(input.to_string_lossy().into_owned(), 1).unwrap();
        assert_eq!(size.width, 612.0);
        assert_eq!(size.height, 792.0);
    }
}
