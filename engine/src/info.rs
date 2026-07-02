use crate::EngineError;
use lopdf::{Document, Object};

/// Returns the page count of the PDF at `path`.
#[uniffi::export]
pub fn get_page_count(path: String) -> Result<u32, EngineError> {
    let doc = Document::load(&path).map_err(|e| EngineError::ReadFailed {
        path,
        reason: e.to_string(),
    })?;
    Ok(doc.get_pages().len() as u32)
}

/// Diagnostic: one summary line per image XObject in the PDF (filter,
/// color space, bit depth, dimensions, stored byte size). Used to explain
/// why `compress_pdf_by_level` did or didn't shrink a given file.
#[uniffi::export]
pub fn describe_images(path: String) -> Result<Vec<String>, EngineError> {
    let doc = Document::load(&path).map_err(|e| EngineError::ReadFailed {
        path,
        reason: e.to_string(),
    })?;

    let mut lines = Vec::new();
    for object in doc.objects.values() {
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

        let width = stream.dict.get(b"Width").and_then(Object::as_i64).unwrap_or(-1);
        let height = stream.dict.get(b"Height").and_then(Object::as_i64).unwrap_or(-1);
        let bpc = stream
            .dict
            .get(b"BitsPerComponent")
            .and_then(Object::as_i64)
            .unwrap_or(-1);
        let color_space = stream
            .dict
            .get(b"ColorSpace")
            .and_then(Object::as_name)
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .unwrap_or_else(|_| "(none/indexed-array)".to_string());
        let filters = stream.filters().unwrap_or_default();
        let filter_desc = if filters.is_empty() {
            "(none, raw)".to_string()
        } else {
            filters.join(",")
        };
        let has_predictor = stream
            .dict
            .get(b"DecodeParms")
            .and_then(Object::as_dict)
            .and_then(|p| p.get(b"Predictor"))
            .and_then(Object::as_i64)
            .map(|p| p > 1)
            .unwrap_or(false);

        lines.push(format!(
            "{}x{} bpc={} colorspace={} filter={} predictor={} bytes={}",
            width, height, bpc, color_space, filter_desc, has_predictor, stream.content.len()
        ));
    }
    Ok(lines)
}
