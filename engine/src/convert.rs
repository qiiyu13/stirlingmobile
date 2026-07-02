use crate::EngineError;
use image::{ImageEncoder, ImageFormat};
use lopdf::{dictionary, Document, Object, Stream};
use std::io::Cursor;

/// Builds a one-image-per-page PDF from `input_paths` (in order), writing
/// the result to `output_path`. Each page is sized to the image's pixel
/// dimensions (1px = 1pt). JPEG inputs are embedded byte-for-byte
/// (`DCTDecode`) to avoid a lossy re-encode; everything else the `image`
/// crate can decode (PNG, ...) is re-encoded to JPEG quality 92.
#[uniffi::export]
pub fn convert_images_to_pdf(input_paths: Vec<String>, output_path: String) -> Result<(), EngineError> {
    if input_paths.is_empty() {
        return Err(EngineError::NoInput);
    }

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::with_capacity(input_paths.len());

    for path in &input_paths {
        let bytes = std::fs::read(path).map_err(|e| EngineError::ReadFailed {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        let (jpeg_bytes, width, height) = prepare_jpeg(&bytes).ok_or_else(|| EngineError::ReadFailed {
            path: path.clone(),
            reason: "unrecognized or unsupported image format".to_string(),
        })?;

        let image_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => width as i64,
                "Height" => height as i64,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "DCTDecode",
            },
            jpeg_bytes,
        ));
        let resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! { "Im0" => image_id },
        });
        let content = format!("q {width} 0 0 {height} 0 0 cm /Im0 Do Q");
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), (width as i64).into(), (height as i64).into()],
            "Contents" => content_id,
        });
        kids.push(Object::Reference(page_id));
    }

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => kids.len() as i64,
            "Kids" => kids,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(&output_path).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

/// Decodes arbitrary image bytes and returns `(jpeg_bytes, width, height)`.
/// JPEG input passes through unchanged; everything else is re-encoded.
fn prepare_jpeg(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let format = image::guess_format(bytes).ok()?;
    let decoded = image::load_from_memory_with_format(bytes, format).ok()?;
    let (width, height) = (decoded.width(), decoded.height());

    if format == ImageFormat::Jpeg {
        return Some((bytes.to_vec(), width, height));
    }

    let mut encoded = Vec::new();
    let mut cursor = Cursor::new(&mut encoded);
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 92);
    encoder
        .write_image(decoded.to_rgb8().as_raw(), width, height, image::ExtendedColorType::Rgb8)
        .ok()?;
    Some((encoded, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use lopdf::Document as LopdfDocument;
    use std::env::temp_dir;

    fn write_png(path: &std::path::Path, width: u32, height: u32) {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        img.save(path).unwrap();
    }

    #[test]
    fn images_to_pdf_makes_one_page_per_image_sized_to_it() {
        let dir = temp_dir();
        let a = dir.join("convert_test_a.png");
        let b = dir.join("convert_test_b.png");
        let output = dir.join("convert_test_out.pdf");
        write_png(&a, 200, 100);
        write_png(&b, 50, 80);

        convert_images_to_pdf(
            vec![a.to_string_lossy().into_owned(), b.to_string_lossy().into_owned()],
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = LopdfDocument::load(&output).unwrap();
        let pages: Vec<_> = doc.get_pages().into_values().collect();
        assert_eq!(pages.len(), 2);

        let media_boxes: Vec<(i64, i64)> = pages
            .iter()
            .map(|id| {
                let page = doc.get_dictionary(*id).unwrap();
                let bbox = page.get(b"MediaBox").unwrap().as_array().unwrap();
                (
                    bbox[2].as_i64().unwrap(),
                    bbox[3].as_i64().unwrap(),
                )
            })
            .collect();
        assert!(media_boxes.contains(&(200, 100)));
        assert!(media_boxes.contains(&(50, 80)));
    }

    #[test]
    fn images_to_pdf_rejects_empty_input() {
        let result = convert_images_to_pdf(vec![], temp_dir().join("unused.pdf").to_string_lossy().into_owned());
        assert!(result.is_err());
    }
}
