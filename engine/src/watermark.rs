//! Text and image watermarks: tiled across every page, with adjustable
//! opacity and rotation (F-062). Draws on top of existing content via an
//! appended, self-contained `q ... Q` block so page graphics state is never
//! disturbed.

use crate::content_util::{add_font, add_image_xobject, add_opacity_gs, page_size};
use crate::EngineError;
use image::GenericImageView;
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};

/// Gap (points) added between tiled repetitions, on top of each tile's own
/// size. ponytail: fixed spacing in v1, expose if users want denser/sparser
/// tiling.
const TILE_GAP: f32 = 60.0;
/// Watermark fill colour: mid grey, so it reads as a background mark.
const GREY: f64 = 0.5;

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

/// Tile `text` diagonally across every page at `font_size`, rotated
/// `rotation_degrees` counter-clockwise, at `opacity` (0..=1), writing the
/// result to `output_path`.
#[uniffi::export]
pub fn content_watermark_text(
    input_path: String,
    text: String,
    font_size: f32,
    rotation_degrees: f32,
    opacity: f32,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    let page_ids: Vec<_> = doc.get_pages().values().copied().collect();

    let (cos, sin) = {
        let r = rotation_degrees.to_radians();
        (r.cos() as f64, r.sin() as f64)
    };
    // Rough Helvetica advance: ~0.5em average glyph width.
    let text_w = text.chars().count() as f32 * font_size * 0.5;
    let step_x = (text_w + TILE_GAP).max(1.0);
    let step_y = (font_size + TILE_GAP).max(1.0);

    for page_id in page_ids {
        add_font(&mut doc, page_id, "SMwmF", "Helvetica");
        add_opacity_gs(&mut doc, page_id, "SMwmG", opacity);
        let (pw, ph) = page_size(&doc, page_id);

        let mut ops = vec![
            Operation::new("q", vec![]),
            Operation::new("gs", vec!["SMwmG".into()]),
            Operation::new("rg", vec![GREY.into(), GREY.into(), GREY.into()]),
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["SMwmF".into(), font_size.into()]),
        ];
        let mut y = 0.0f32;
        while y < ph {
            let mut x = 0.0f32;
            while x < pw {
                ops.push(Operation::new(
                    "Tm",
                    vec![
                        cos.into(),
                        sin.into(),
                        (-sin).into(),
                        cos.into(),
                        x.into(),
                        y.into(),
                    ],
                ));
                ops.push(Operation::new(
                    "Tj",
                    vec![Object::string_literal(text.clone())],
                ));
                x += step_x;
            }
            y += step_y;
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

/// Tile the image at `image_path` across every page, each copy sized to
/// `width_fraction` of the page width (aspect ratio preserved), rotated
/// `rotation_degrees` counter-clockwise, at `opacity` (0..=1).
#[uniffi::export]
pub fn content_watermark_image(
    input_path: String,
    image_path: String,
    width_fraction: f32,
    rotation_degrees: f32,
    opacity: f32,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    let page_ids: Vec<_> = doc.get_pages().values().copied().collect();

    let img_bytes = std::fs::read(&image_path).map_err(|e| EngineError::ReadFailed {
        path: image_path.clone(),
        reason: e.to_string(),
    })?;
    let decoded = image::load_from_memory(&img_bytes).map_err(|e| EngineError::ReadFailed {
        path: image_path,
        reason: e.to_string(),
    })?;
    let (img_w, img_h) = decoded.dimensions();
    let aspect = img_h as f32 / img_w.max(1) as f32;

    let (cos, sin) = {
        let r = rotation_degrees.to_radians();
        (r.cos() as f64, r.sin() as f64)
    };

    for page_id in page_ids {
        let stream = build_image_xobject(&decoded, img_w, img_h, &mut doc);
        add_image_xobject(&mut doc, page_id, "SMwmI", stream);
        add_opacity_gs(&mut doc, page_id, "SMwmG", opacity);
        let (pw, ph) = page_size(&doc, page_id);

        let tile_w = pw * width_fraction;
        let tile_h = tile_w * aspect;
        let step_x = (tile_w + TILE_GAP).max(1.0);
        let step_y = (tile_h + TILE_GAP).max(1.0);
        // cm maps the image's unit square: rotate then scale to (tile_w,tile_h).
        let (a, b, c, d) = (
            tile_w as f64 * cos,
            tile_w as f64 * sin,
            -(tile_h as f64) * sin,
            tile_h as f64 * cos,
        );

        let mut ops = vec![
            Operation::new("q", vec![]),
            Operation::new("gs", vec!["SMwmG".into()]),
        ];
        let mut y = 0.0f32;
        while y < ph {
            let mut x = 0.0f32;
            while x < pw {
                ops.push(Operation::new("q", vec![]));
                ops.push(Operation::new(
                    "cm",
                    vec![a.into(), b.into(), c.into(), d.into(), x.into(), y.into()],
                ));
                ops.push(Operation::new("Do", vec!["SMwmI".into()]));
                ops.push(Operation::new("Q", vec![]));
                x += step_x;
            }
            y += step_y;
        }
        ops.push(Operation::new("Q", vec![]));

        doc.add_to_page_content(page_id, Content { operations: ops })
            .map_err(|e| EngineError::WriteFailed {
                reason: e.to_string(),
            })?;
    }

    save(doc, &output_path)
}

/// Build a DeviceRGB image XObject stream (with `/SMask` alpha channel when
/// the image is transparent) from an already-decoded image.
fn build_image_xobject(
    decoded: &image::DynamicImage,
    img_w: u32,
    img_h: u32,
    doc: &mut Document,
) -> Stream {
    let rgba = decoded.to_rgba8();
    let rgb: Vec<u8> = rgba.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
    let alpha: Vec<u8> = rgba.pixels().map(|p| p[3]).collect();
    let has_alpha = alpha.iter().any(|&a| a != 255);

    let mut img_dict = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => img_w as i64,
        "Height" => img_h as i64,
        "ColorSpace" => "DeviceRGB",
        "BitsPerComponent" => 8,
    };
    if has_alpha {
        let smask_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => img_w as i64,
                "Height" => img_h as i64,
                "ColorSpace" => "DeviceGray",
                "BitsPerComponent" => 8,
            },
            alpha,
        ));
        img_dict.set("SMask", Object::Reference(smask_id));
    }
    Stream::new(img_dict, rgb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use lopdf::content::Content;
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
        doc.save(path).unwrap();
    }

    fn count_xobjects(doc: &Document) -> usize {
        let page_id = *doc.get_pages().get(&1).unwrap();
        let (resources, _) = doc.get_page_resources(page_id).unwrap();
        resources
            .and_then(|r| r.get(b"XObject").ok())
            .and_then(|o| o.as_dict().ok())
            .map(|d| d.len())
            .unwrap_or(0)
    }

    #[test]
    fn text_watermark_adds_font_gs_and_content() {
        let dir = temp_dir();
        let input = dir.join("wm_text_in.pdf");
        let output = dir.join("wm_text_out.pdf");
        one_page_pdf(&input);

        content_watermark_text(
            input.to_string_lossy().into_owned(),
            "CONFIDENTIAL".to_string(),
            36.0,
            45.0,
            0.3,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let (resources, _) = doc.get_page_resources(page_id).unwrap();
        let res = resources.unwrap();
        assert!(res.get(b"Font").is_ok(), "font registered");
        assert!(res.get(b"ExtGState").is_ok(), "opacity gs registered");
        // Watermark text must be present in the page content stream.
        assert!(doc.extract_text(&[1]).unwrap().contains("CONFIDENTIAL"));
    }

    #[test]
    fn image_watermark_adds_xobject() {
        let dir = temp_dir();
        let input = dir.join("wm_img_in.pdf");
        let img = dir.join("wm_img.png");
        let output = dir.join("wm_img_out.pdf");
        one_page_pdf(&input);
        let buf: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(20, 20, |_, _| Rgba([200, 0, 0, 128]));
        buf.save(&img).unwrap();

        content_watermark_image(
            input.to_string_lossy().into_owned(),
            img.to_string_lossy().into_owned(),
            0.25,
            0.0,
            0.4,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        assert!(count_xobjects(&doc) >= 1, "image xobject present");
    }
}
