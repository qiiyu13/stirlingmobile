//! Extract embedded raster images from a PDF to files on disk.
//!
//! ponytail: DCTDecode (JPEG) streams are written out as-is - they're
//! already a valid image file. Everything else (raw/Flate-decoded samples)
//! is re-encoded to PNG for DeviceRGB/DeviceGray 8-bit images only; other
//! color spaces (indexed, CMYK, ICC-based) are skipped rather than guessing
//! at a conversion. Upgrade path: add a colorspace-aware decoder if a v1.1
//! testing pass finds real-world PDFs hitting the skip case often.

use crate::EngineError;
use image::{ImageBuffer, Luma, Rgb};
use lopdf::{Document, Object};

/// Extracts every embedded image from the PDF at `input_path` into
/// `output_dir` (must already exist), returning the written file paths in
/// document order.
#[uniffi::export]
pub fn extract_images(input_path: String, output_dir: String) -> Result<Vec<String>, EngineError> {
    let doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    let mut written = Vec::new();
    let mut index = 0usize;
    for (id, object) in doc.objects.iter() {
        let Object::Stream(stream) = object else {
            continue;
        };
        let is_image = stream
            .dict
            .get(b"Subtype")
            .and_then(Object::as_name)
            .map(|name| name == b"Image")
            .unwrap_or(false);
        if !is_image {
            continue;
        }

        let filters = stream.filters().unwrap_or_default();
        let is_jpeg = filters.iter().any(|f| f == "DCTDecode");

        let path = if is_jpeg {
            let path = format!("{output_dir}/image_{index}_{}_{}.jpg", id.0, id.1);
            std::fs::write(&path, &stream.content).map_err(|e| EngineError::WriteFailed {
                reason: e.to_string(),
            })?;
            Some(path)
        } else {
            write_raw_as_png(stream, &output_dir, index, *id)?
        };

        if let Some(path) = path {
            written.push(path);
            index += 1;
        }
    }
    Ok(written)
}

fn write_raw_as_png(
    stream: &lopdf::Stream,
    output_dir: &str,
    index: usize,
    id: lopdf::ObjectId,
) -> Result<Option<String>, EngineError> {
    let width = stream
        .dict
        .get(b"Width")
        .and_then(Object::as_i64)
        .unwrap_or(0) as u32;
    let height = stream
        .dict
        .get(b"Height")
        .and_then(Object::as_i64)
        .unwrap_or(0) as u32;
    let bpc = stream
        .dict
        .get(b"BitsPerComponent")
        .and_then(Object::as_i64)
        .unwrap_or(0);
    let color_space = stream
        .dict
        .get(b"ColorSpace")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|n| n.to_vec());

    if width == 0 || height == 0 || bpc != 8 {
        return Ok(None);
    }
    // `Stream::decompressed_content` refuses `/Subtype /Image` streams
    // outright, so decode the (at most one, for our supported cases) filter
    // by hand instead.
    let filters = stream.filters().unwrap_or_default();
    let bytes = match filters.first().map(String::as_str) {
        None => stream.content.clone(),
        Some("FlateDecode") => {
            use std::io::Read;
            let mut out = Vec::new();
            let Ok(_) =
                flate2::read::ZlibDecoder::new(stream.content.as_slice()).read_to_end(&mut out)
            else {
                return Ok(None);
            };
            out
        }
        Some(_) => return Ok(None),
    };

    let path = format!("{output_dir}/image_{index}_{}_{}.png", id.0, id.1);
    match color_space.as_deref() {
        Some(b"DeviceRGB") => {
            let expected = (width * height * 3) as usize;
            if bytes.len() < expected {
                return Ok(None);
            }
            let img: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_raw(width, height, bytes)
                .ok_or_else(|| EngineError::WriteFailed {
                    reason: "image buffer size mismatch".into(),
                })?;
            img.save(&path).map_err(|e| EngineError::WriteFailed {
                reason: e.to_string(),
            })?;
            Ok(Some(path))
        }
        Some(b"DeviceGray") => {
            let expected = (width * height) as usize;
            if bytes.len() < expected {
                return Ok(None);
            }
            let img: ImageBuffer<Luma<u8>, _> = ImageBuffer::from_raw(width, height, bytes)
                .ok_or_else(|| EngineError::WriteFailed {
                    reason: "image buffer size mismatch".into(),
                })?;
            img.save(&path).map_err(|e| EngineError::WriteFailed {
                reason: e.to_string(),
            })?;
            Ok(Some(path))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Stream};
    use std::env::temp_dir;

    #[test]
    fn extracts_jpeg_and_raw_rgb_images() {
        let dir = temp_dir();
        let out_dir = dir.join("extract_images_test_out");
        std::fs::create_dir_all(&out_dir).unwrap();
        let input = dir.join("extract_images_test_input.pdf");

        let mut doc = Document::with_version("1.7");
        let jpeg_bytes = vec![0xFFu8, 0xD8, 0xFF, 0xD9]; // minimal SOI/EOI marker pair
        let jpeg_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => 1,
                "Height" => 1,
                "Filter" => "DCTDecode",
            },
            jpeg_bytes,
        ));
        let rgb_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => 2,
                "Height" => 2,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
            },
            vec![255u8; 2 * 2 * 3],
        ));
        let resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! { "Im0" => Object::Reference(jpeg_id), "Im1" => Object::Reference(rgb_id) },
        });
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
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
        doc.save(&input).unwrap();

        let paths = extract_images(
            input.to_string_lossy().into_owned(),
            out_dir.to_string_lossy().into_owned(),
        )
        .unwrap();

        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|p| p.ends_with(".jpg")));
        assert!(paths.iter().any(|p| p.ends_with(".png")));
    }
}
