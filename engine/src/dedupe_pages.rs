//! Detect and remove duplicate pages.
//!
//! ponytail: dedupe key is a hash of the page's raw content stream bytes.
//! Catches true duplicates (same page inserted twice, e.g. via a bad
//! merge/scan) but not visually-identical pages built from different content
//! streams. Upgrade path: hash a pdfium rasterization instead if that proves
//! too narrow in practice (see `compare.rs` for the rasterize primitive).

use crate::EngineError;
use lopdf::Document;
use sha2::{Digest, Sha256};

fn load(input_path: &str) -> Result<Document, EngineError> {
    Document::load(input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path.to_string(),
        reason: e.to_string(),
    })
}

fn save(mut doc: Document, output_path: &str) -> Result<(), EngineError> {
    doc.prune_objects();
    doc.renumber_objects();
    doc.save(output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

fn page_content_hash(doc: &Document, page_id: lopdf::ObjectId) -> Vec<u8> {
    let content = doc.get_page_content(page_id).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&content);
    hasher.finalize().to_vec()
}

/// Returns the 1-indexed page numbers that are exact duplicates of an
/// earlier page in the PDF at `input_path` (first occurrence of each page
/// is kept out of this list).
#[uniffi::export]
pub fn pages_detect_duplicates(input_path: String) -> Result<Vec<u32>, EngineError> {
    let doc = load(&input_path)?;
    let mut seen: Vec<Vec<u8>> = Vec::new();
    let mut duplicates = Vec::new();

    let mut pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().into_iter().collect();
    pages.sort_unstable_by_key(|(number, _)| *number);
    for (number, page_id) in pages {
        let hash = page_content_hash(&doc, page_id);
        if seen.contains(&hash) {
            duplicates.push(number);
        } else {
            seen.push(hash);
        }
    }
    Ok(duplicates)
}

/// Removes exact duplicate pages (keeping the first occurrence of each) from
/// the PDF at `input_path` and writes the result to `output_path`.
#[uniffi::export]
pub fn pages_remove_duplicates(input_path: String, output_path: String) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    let duplicates = pages_detect_duplicates(input_path)?;
    doc.delete_pages(&duplicates);
    save(doc, &output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, dictionary, Object, Stream};
    use std::env::temp_dir;

    fn doc_with_pages(contents: &[&str]) -> Document {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let mut kids = Vec::new();
        for text in contents {
            let content = Content {
                operations: vec![lopdf::content::Operation::new(
                    "Tj",
                    vec![Object::string_literal(text.as_bytes().to_vec())],
                )],
            };
            let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
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
        doc
    }

    #[test]
    fn detects_and_removes_duplicate_pages() {
        let dir = temp_dir();
        let input = dir.join("dedupe_test_input.pdf");
        let output = dir.join("dedupe_test_output.pdf");
        let mut doc = doc_with_pages(&["A", "B", "A", "C", "B"]);
        doc.save(&input).unwrap();

        let duplicates = pages_detect_duplicates(input.to_string_lossy().into_owned()).unwrap();
        assert_eq!(duplicates, vec![3, 5]);

        pages_remove_duplicates(
            input.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
        )
        .unwrap();
        let result = Document::load(&output).unwrap();
        assert_eq!(result.get_pages().len(), 3);
    }
}
