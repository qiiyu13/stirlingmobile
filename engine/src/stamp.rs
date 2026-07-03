use crate::content_util::page_size;
use crate::EngineError;
use image::GenericImageView;
use lopdf::{dictionary, Document, Object, Stream};

/// Margin between a preset-positioned stamp and the page edge, in points.
const MARGIN: f32 = 24.0;
/// Stamp width as a fraction of page width (height keeps the image's
/// aspect ratio). ponytail: fixed size in v1, add a size slider if users
/// need bigger/smaller signatures.
const WIDTH_FRACTION: f32 = 0.3;

/// Stamps `image_path` (any format the `image` crate decodes; alpha
/// transparency preserved via `/SMask`) onto 1-indexed `page_number` of the
/// PDF at `input_path`, anchored at `position`
/// (`top-left`/`top-right`/`bottom-left`/`bottom-right`/`center`), and
/// writes the result to `output_path`. Visual stamp only - no cryptographic
/// signature; see `security_sign`/`security_certify` for PKCS#12 signing.
#[uniffi::export]
pub fn stamp_signature_image(
    input_path: String,
    image_path: String,
    page_number: u32,
    position: String,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    let page_id = *doc
        .get_pages()
        .get(&page_number)
        .ok_or_else(|| EngineError::WriteFailed {
            reason: format!("page {page_number} does not exist"),
        })?;

    let img_bytes = std::fs::read(&image_path).map_err(|e| EngineError::ReadFailed {
        path: image_path.clone(),
        reason: e.to_string(),
    })?;
    let decoded = image::load_from_memory(&img_bytes).map_err(|e| EngineError::ReadFailed {
        path: image_path,
        reason: e.to_string(),
    })?;
    let (img_w, img_h) = decoded.dimensions();
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
    let img_stream = Stream::new(img_dict, rgb);

    let (page_w, page_h) = page_size(&doc, page_id);
    let stamp_w = page_w * WIDTH_FRACTION;
    let stamp_h = stamp_w * (img_h as f32 / img_w.max(1) as f32);
    let (x, y) = anchor(&position, page_w, page_h, stamp_w, stamp_h);

    doc.insert_image(page_id, img_stream, (x, y), (stamp_w, stamp_h))
        .map_err(|e| EngineError::WriteFailed { reason: e.to_string() })?;

    doc.compress();
    doc.save(&output_path).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

fn anchor(position: &str, page_w: f32, page_h: f32, stamp_w: f32, stamp_h: f32) -> (f32, f32) {
    match position {
        "top-left" => (MARGIN, page_h - MARGIN - stamp_h),
        "top-right" => (page_w - MARGIN - stamp_w, page_h - MARGIN - stamp_h),
        "bottom-left" => (MARGIN, MARGIN),
        "center" => ((page_w - stamp_w) / 2.0, (page_h - stamp_h) / 2.0),
        _ => (page_w - MARGIN - stamp_w, MARGIN), // bottom-right, the default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use lopdf::content::Content;
    use std::env::temp_dir;

    fn one_page_pdf(path: &std::path::Path, width: f32, height: f32) {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
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

    fn write_png_rgba(path: &std::path::Path, width: u32, height: u32) {
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(width, height, |_, _| Rgba([10, 20, 30, 128]));
        img.save(path).unwrap();
    }

    #[test]
    fn stamps_image_onto_requested_page() {
        let dir = temp_dir();
        let input = dir.join("stamp_test_input.pdf");
        let sig = dir.join("stamp_test_sig.png");
        let output = dir.join("stamp_test_output.pdf");
        one_page_pdf(&input, 612.0, 792.0);
        write_png_rgba(&sig, 100, 40);

        stamp_signature_image(
            input.to_string_lossy().into_owned(),
            sig.to_string_lossy().into_owned(),
            1,
            "bottom-right".to_string(),
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 1);
        let page_id = *pages.get(&1).unwrap();
        let (resources, _) = doc.get_page_resources(page_id).unwrap();
        let xobjects = resources.unwrap().get(b"XObject").unwrap().as_dict().unwrap();
        assert_eq!(xobjects.len(), 1);
    }

    #[test]
    fn rejects_out_of_range_page() {
        let dir = temp_dir();
        let input = dir.join("stamp_test_range_input.pdf");
        let sig = dir.join("stamp_test_range_sig.png");
        let output = dir.join("stamp_test_range_output.pdf");
        one_page_pdf(&input, 612.0, 792.0);
        write_png_rgba(&sig, 100, 40);

        let result = stamp_signature_image(
            input.to_string_lossy().into_owned(),
            sig.to_string_lossy().into_owned(),
            2,
            "bottom-right".to_string(),
            output.to_string_lossy().into_owned(),
        );
        assert!(result.is_err());
    }
}
