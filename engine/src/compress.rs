use crate::content_util::save_document;
use crate::EngineError;
use image::imageops::FilterType;
use image::{DynamicImage, GrayImage, ImageEncoder, ImageFormat, RgbImage};
use lopdf::{Document, Object, Stream};
use std::io::Cursor;

/// Recompresses/downscales every image in the PDF at `input_path` at a
/// fixed quality `level` (1 = highest quality/least shrinkage, 9 = smallest
/// file/most shrinkage) and writes the result to `output_path`. Convenience
/// wrapper around [`compress_pdf_custom`] that couples quality and
/// downscale together; use `compress_pdf_custom` to set them independently.
#[uniffi::export]
pub fn compress_pdf_by_level(
    input_path: String,
    level: u8,
    output_path: String,
) -> Result<(), EngineError> {
    let (quality, scale_percent) = level_params(level.clamp(1, 9));
    compress_pdf_custom(input_path, quality, scale_percent, output_path)
}

/// Recompresses/downscales every image in the PDF at `input_path` with
/// independently chosen `quality` (1-100 JPEG quality) and `scale_percent`
/// (1-100, percent of each image's original width/height to keep), writing
/// the result to `output_path`.
#[uniffi::export]
pub fn compress_pdf_custom(
    input_path: String,
    quality: u8,
    scale_percent: u8,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    let quality = quality.clamp(1, 100);
    let scale = (scale_percent.clamp(1, 100) as f32) / 100.0;
    recompress_images(&mut doc, quality, scale);
    doc.compress();

    save_document(&mut doc, &output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

/// Like [`compress_pdf_by_level`], but tries levels 1 through 9 in order and
/// keeps the first result at or under `target_bytes`, falling back to the
/// smallest (level 9) result if none hit the target. Returns the level used.
#[uniffi::export]
pub fn compress_pdf_to_target_size(
    input_path: String,
    target_bytes: u64,
    output_path: String,
) -> Result<u8, EngineError> {
    let mut last_level = 1u8;
    for level in 1..=9u8 {
        compress_pdf_by_level(input_path.clone(), level, output_path.clone())?;
        last_level = level;

        let size = std::fs::metadata(&output_path)
            .map_err(|e| EngineError::WriteFailed {
                reason: e.to_string(),
            })?
            .len();
        if size <= target_bytes {
            break;
        }
    }
    Ok(last_level)
}

/// Quality (1-100) and downscale percent (1-100) for a 1-9 compression level.
fn level_params(level: u8) -> (u8, u8) {
    let step = (level.saturating_sub(1)) as f32 / 8.0; // 0.0 at level 1, 1.0 at level 9
    let quality = 90.0 - step * 72.0; // 90 -> 18
    let scale_percent = 100.0 - step * 55.0; // 100 -> 45
    (quality.round() as u8, scale_percent.round() as u8)
}

fn recompress_images(doc: &mut Document, quality: u8, scale: f32) {
    let image_ids: Vec<_> = doc
        .objects
        .iter()
        .filter(|(_, obj)| {
            matches!(obj, Object::Stream(s)
                if s.dict.get(b"Subtype").and_then(Object::as_name).map(|n| n == b"Image").unwrap_or(false))
        })
        .map(|(id, _)| *id)
        .collect();

    for id in image_ids {
        let Object::Stream(stream) = doc.objects.get(&id).unwrap() else {
            continue;
        };
        let Some(decoded) = decode_image(doc, stream) else {
            continue; // ponytail: CMYK/Indexed/16-bit images skipped in v1, common RGB/Gray/JPEG/ICCBased cases covered
        };
        let original_len = stream.content.len();

        let resized = if scale < 0.999 {
            let new_width = ((decoded.width() as f32 * scale).round() as u32).max(1);
            let new_height = ((decoded.height() as f32 * scale).round() as u32).max(1);
            decoded.resize(new_width, new_height, FilterType::Lanczos3)
        } else {
            decoded
        };

        let mut encoded = Vec::new();
        let mut cursor = Cursor::new(&mut encoded);
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
        if encoder
            .write_image(
                resized.to_rgb8().as_raw(),
                resized.width(),
                resized.height(),
                image::ExtendedColorType::Rgb8,
            )
            .is_err()
        {
            continue;
        }

        if encoded.len() < original_len {
            let Object::Stream(stream) = doc.objects.get_mut(&id).unwrap() else {
                continue;
            };
            stream.dict.set("Width", resized.width() as i64);
            stream.dict.set("Height", resized.height() as i64);
            stream
                .dict
                .set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
            stream.dict.set("BitsPerComponent", 8);
            stream
                .dict
                .set("Filter", Object::Name(b"DCTDecode".to_vec()));
            stream.dict.remove(b"DecodeParms");
            stream.set_content(encoded);
        }
    }
}

/// Decodes a PDF image XObject stream into a raster image, handling both
/// already-JPEG (`DCTDecode`) streams and raw 8-bit Gray/RGB bitmaps
/// (typically `FlateDecode`). Returns `None` for encodings not worth
/// covering in v1 (CMYK, Indexed, 16-bit, JPXDecode, ...).
fn decode_image(doc: &Document, stream: &Stream) -> Option<DynamicImage> {
    let is_jpeg = stream
        .filters()
        .unwrap_or_default()
        .iter()
        .any(|f| f == "DCTDecode");
    if is_jpeg {
        return image::load_from_memory_with_format(&stream.content, ImageFormat::Jpeg).ok();
    }

    let width = stream.dict.get(b"Width").and_then(Object::as_i64).ok()? as u32;
    let height = stream.dict.get(b"Height").and_then(Object::as_i64).ok()? as u32;
    let bits_per_component = stream
        .dict
        .get(b"BitsPerComponent")
        .and_then(Object::as_i64)
        .unwrap_or(8);
    if bits_per_component != 8 {
        return None;
    }
    let color_space_obj = stream.dict.get(b"ColorSpace").ok()?;
    let components = resolve_components(doc, color_space_obj)?;

    let has_predictor = stream
        .dict
        .get(b"DecodeParms")
        .and_then(Object::as_dict)
        .and_then(|p| p.get(b"Predictor"))
        .and_then(Object::as_i64)
        .map(|p| p > 1)
        .unwrap_or(false);
    if has_predictor {
        return None; // ponytail: PNG/TIFF predictor unfiltering not implemented in v1
    }

    let filters = stream.filters().unwrap_or_default();
    let raw = match filters.first().map(String::as_str) {
        None => stream.content.clone(),
        Some("FlateDecode") => inflate(&stream.content)?,
        _ => return None, // ponytail: LZW/ASCII85 raw images not implemented in v1
    };
    match components {
        3 => RgbImage::from_raw(width, height, raw).map(DynamicImage::ImageRgb8),
        1 => GrayImage::from_raw(width, height, raw).map(DynamicImage::ImageLuma8),
        _ => None, // ponytail: CMYK(4)/spot-color images not implemented in v1
    }
}

/// Resolves a `ColorSpace` value (direct name, indirect reference, or an
/// array like `[/ICCBased 5 0 R]` / `[/CalRGB ...]`) to its component
/// count. PDFs sourced from cameras/scanners overwhelmingly use ICCBased,
/// which is why this can't just match on `Object::Name`.
fn resolve_components(doc: &Document, obj: &Object) -> Option<u8> {
    match obj {
        Object::Name(name) => match name.as_slice() {
            b"DeviceGray" | b"CalGray" => Some(1),
            b"DeviceRGB" | b"CalRGB" | b"Lab" => Some(3),
            b"DeviceCMYK" => Some(4),
            _ => None,
        },
        Object::Reference(id) => resolve_components(doc, doc.get_object(*id).ok()?),
        Object::Array(items) => {
            let head = items.first()?.as_name().ok()?;
            match head {
                b"ICCBased" => {
                    let stream_obj = match items.get(1)? {
                        Object::Reference(id) => doc.get_object(*id).ok()?,
                        other => other,
                    };
                    stream_obj
                        .as_stream()
                        .ok()?
                        .dict
                        .get(b"N")
                        .and_then(Object::as_i64)
                        .ok()
                        .map(|n| n as u8)
                }
                b"CalRGB" | b"Lab" => Some(3),
                b"CalGray" => Some(1),
                b"DeviceN" | b"Indexed" | b"Separation" => None, // ponytail: not implemented in v1
                _ => None,
            }
        }
        _ => None,
    }
}

fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    let mut out = Vec::new();
    ZlibDecoder::new(data).read_to_end(&mut out).ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    #[test]
    fn level_params_shrink_monotonically() {
        let (q1, s1) = level_params(1);
        let (q9, s9) = level_params(9);
        assert!(q9 < q1);
        assert!(s9 < s1);
        assert_eq!(s1, 100);
    }

    #[test]
    fn iccbased_colorspace_resolves_to_component_count() {
        // Regression test: camera/photo-sourced PDFs almost universally use
        // `[/ICCBased n 0 R]` for ColorSpace rather than a plain /DeviceRGB
        // name. resolve_components must follow the indirect reference to
        // the ICC stream and read its /N entry, not just match on Name.
        let mut doc = Document::with_version("1.7");
        let icc_stream_id = doc.add_object(lopdf::Stream::new(
            dictionary! { "N" => 3i64 },
            vec![0u8; 16],
        ));
        let color_space = Object::Array(vec![
            Object::Name(b"ICCBased".to_vec()),
            Object::Reference(icc_stream_id),
        ]);
        assert_eq!(resolve_components(&doc, &color_space), Some(3));

        let indirect_color_space_id = doc.add_object(color_space);
        let indirect = Object::Reference(indirect_color_space_id);
        assert_eq!(resolve_components(&doc, &indirect), Some(3));
    }

    #[test]
    fn raw_rgb_bitmap_gets_recompressed_to_smaller_jpeg() {
        // Regression test: raw Flate-compressed RGB bitmaps (what most
        // scanner/print-to-PDF apps embed) must not be silently skipped
        // just because they aren't already DCTDecode/JPEG.
        let width = 256u32;
        let height = 256u32;
        // Genuine per-byte noise via xorshift32, not a flat color or a short
        // repeating pattern: those compress to near-nothing under Flate
        // already, which would make the "got smaller" assertion pass for
        // the wrong reason instead of proving JPEG recompression ran.
        let mut state = 0x9e3779b9u32;
        let raw_rgb: Vec<u8> = (0..(width * height * 3))
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                (state % 256) as u8
            })
            .collect();

        let dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => width as i64,
            "Height" => height as i64,
            "ColorSpace" => "DeviceRGB",
            "BitsPerComponent" => 8,
        };
        // Filter left unset so Stream::compress() both sets it to
        // FlateDecode *and* actually compresses the content; setting
        // Filter beforehand makes compress() skip compression entirely.
        let mut stream = Stream::new(dict, raw_rgb);
        stream.compress().unwrap();
        let original_len = stream.content.len();

        let mut doc = Document::with_version("1.7");
        let image_id = doc.add_object(Object::Stream(stream));

        if let Object::Stream(s) = doc.get_object(image_id).unwrap() {
            let decoded = decode_image(&doc, s).expect("raw RGB bitmap should decode");
            assert_eq!((decoded.width(), decoded.height()), (width, height));
        }

        let resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! { "Im0" => image_id },
        });
        let content_id = doc.add_object(lopdf::Stream::new(
            dictionary! {},
            lopdf::content::Content { operations: vec![] }
                .encode()
                .unwrap(),
        ));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
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

        recompress_images(&mut doc, 18, 0.45);

        let recompressed = doc.get_object(image_id).unwrap().as_stream().unwrap();
        assert!(recompressed.content.len() < original_len);
        assert_eq!(
            recompressed
                .dict
                .get(b"Filter")
                .and_then(Object::as_name)
                .unwrap(),
            b"DCTDecode"
        );
    }

    #[test]
    fn compress_by_level_leaves_page_count_unchanged() {
        use lopdf::{content::Content, dictionary};
        use std::env::temp_dir;

        let input = temp_dir().join("compress_test_input.pdf");
        let output = temp_dir().join("compress_test_output.pdf");

        let mut doc = Document::with_version("1.7");
        let content_id = doc.add_object(lopdf::Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
        ));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
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
        save_document(&mut doc, &input).unwrap();

        compress_pdf_by_level(
            input.to_string_lossy().into_owned(),
            5,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let result = Document::load(&output).unwrap();
        assert_eq!(result.get_pages().len(), 1);
    }
}
