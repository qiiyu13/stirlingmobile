//! Read and edit the PDF document information dictionary (`/Info`): title,
//! author, subject, keywords, creator, producer, and dates (F-101).
//!
//! ponytail: edits the classic `/Info` dict only. A file may also carry XMP
//! metadata (Catalog `/Metadata` stream) that some viewers prefer; syncing or
//! stripping XMP is a `security_sanitize` / v1.1 concern, not handled here.

use crate::EngineError;
use lopdf::{Dictionary, Document, Object, StringFormat};

/// The document `/Info` fields. A field is `None` when the PDF doesn't set it.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct PdfMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub mod_date: Option<String>,
}

/// Read the `/Info` metadata from the PDF at `path`.
#[uniffi::export]
pub fn metadata_extract(path: String) -> Result<PdfMetadata, EngineError> {
    let doc = Document::load(&path).map_err(|e| EngineError::ReadFailed {
        path,
        reason: e.to_string(),
    })?;

    let info = match info_dict(&doc) {
        Some(d) => d,
        None => return Ok(PdfMetadata::default()),
    };
    let get = |key: &[u8]| {
        info.get(key)
            .ok()
            .and_then(|o| o.as_str().ok())
            .map(decode_pdf_string)
    };
    Ok(PdfMetadata {
        title: get(b"Title"),
        author: get(b"Author"),
        subject: get(b"Subject"),
        keywords: get(b"Keywords"),
        creator: get(b"Creator"),
        producer: get(b"Producer"),
        creation_date: get(b"CreationDate"),
        mod_date: get(b"ModDate"),
    })
}

/// Overwrite `/Info` fields on the PDF at `input_path`, writing the result to
/// `output_path`. Each argument is `Some(value)` to set the field (empty
/// string clears it) or `None` to leave it unchanged.
#[allow(clippy::too_many_arguments)]
#[uniffi::export]
pub fn metadata_edit(
    input_path: String,
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    // Ensure there is an /Info dictionary to write into.
    let info_id = match doc.trailer.get(b"Info").ok().and_then(|o| o.as_reference().ok()) {
        Some(id) => id,
        None => {
            let id = doc.add_object(Dictionary::new());
            doc.trailer.set("Info", Object::Reference(id));
            id
        }
    };
    let Ok(info) = doc.get_object_mut(info_id).and_then(Object::as_dict_mut) else {
        return Err(EngineError::WriteFailed {
            reason: "malformed /Info dictionary".into(),
        });
    };

    let fields: [(&[u8], Option<String>); 6] = [
        (b"Title", title),
        (b"Author", author),
        (b"Subject", subject),
        (b"Keywords", keywords),
        (b"Creator", creator),
        (b"Producer", producer),
    ];
    for (key, value) in fields {
        if let Some(v) = value {
            info.set(key.to_vec(), encode_pdf_string(&v));
        }
    }

    doc.save(&output_path).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

fn info_dict(doc: &Document) -> Option<&Dictionary> {
    let id = doc.trailer.get(b"Info").ok()?.as_reference().ok()?;
    doc.get_dictionary(id).ok()
}

/// Decode a PDF text string: UTF-16BE when it carries the `FE FF` BOM
/// (ISO 32000-1 §7.9.2.2), otherwise treat the bytes as PDFDocEncoding, which
/// coincides with Latin-1 across the printable range we care about.
fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let units: Vec<u16> = bytes[2..]
            .chunks(2)
            .map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

/// Encode a string as a PDF text string: a plain literal when it's ASCII,
/// otherwise UTF-16BE with a `FE FF` BOM so non-Latin text survives.
fn encode_pdf_string(s: &str) -> Object {
    if s.is_ascii() {
        Object::string_literal(s)
    } else {
        let mut bytes = vec![0xFE, 0xFF];
        for unit in s.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        Object::String(bytes, StringFormat::Literal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    fn pdf_with_info(path: &std::path::Path, set_info: bool) {
        use lopdf::dictionary;
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Count" => 1, "Kids" => vec![Object::Reference(page_id)],
            }),
        );
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", Object::Reference(catalog_id));
        if set_info {
            let info_id = doc.add_object(dictionary! {
                "Title" => Object::string_literal("Original Title"),
                "Author" => Object::string_literal("Alice"),
            });
            doc.trailer.set("Info", Object::Reference(info_id));
        }
        doc.save(path).unwrap();
    }

    #[test]
    fn extract_reads_existing_info() {
        let dir = temp_dir();
        let input = dir.join("md_extract.pdf");
        pdf_with_info(&input, true);
        let md = metadata_extract(input.to_string_lossy().into_owned()).unwrap();
        assert_eq!(md.title.as_deref(), Some("Original Title"));
        assert_eq!(md.author.as_deref(), Some("Alice"));
        assert_eq!(md.subject, None);
    }

    #[test]
    fn edit_sets_and_preserves_fields() {
        let dir = temp_dir();
        let input = dir.join("md_edit_in.pdf");
        let output = dir.join("md_edit_out.pdf");
        pdf_with_info(&input, true);

        metadata_edit(
            input.to_string_lossy().into_owned(),
            Some("New Title".into()),
            None, // leave Author unchanged
            Some("Café ☕".into()), // non-ASCII -> UTF-16BE
            None,
            None,
            None,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let md = metadata_extract(output.to_string_lossy().into_owned()).unwrap();
        assert_eq!(md.title.as_deref(), Some("New Title"));
        assert_eq!(md.author.as_deref(), Some("Alice"), "untouched field kept");
        assert_eq!(md.subject.as_deref(), Some("Café ☕"), "non-ASCII round-trips");
    }

    #[test]
    fn edit_creates_info_when_absent() {
        let dir = temp_dir();
        let input = dir.join("md_noinfo_in.pdf");
        let output = dir.join("md_noinfo_out.pdf");
        pdf_with_info(&input, false);

        metadata_edit(
            input.to_string_lossy().into_owned(),
            Some("Made Title".into()),
            None, None, None, None, None,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let md = metadata_extract(output.to_string_lossy().into_owned()).unwrap();
        assert_eq!(md.title.as_deref(), Some("Made Title"));
    }
}
