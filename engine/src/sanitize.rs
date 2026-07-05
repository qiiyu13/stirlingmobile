//! Strip active and hidden content from a PDF (F-055): JavaScript, embedded
//! files, document metadata, and link annotations. Each vector is opt-in via
//! a flag, mirroring Stirling's `sanitize-pdf`.
//!
//! ponytail: covers the common catalog/page-level vectors (Names-tree
//! JavaScript & embedded files, `/OpenAction`, `/AA`, `/AF`, Link annots). It
//! does not chase JavaScript buried in arbitrary indirect action chains; add a
//! full object-graph sweep if a threat model needs it.
use crate::content_util::save_document;

use crate::EngineError;
use lopdf::{Document, Object, ObjectId};

/// Sanitize the PDF at `input_path` into `output_path`. Each `remove_*` flag
/// independently removes that class of content.
#[uniffi::export]
pub fn security_sanitize(
    input_path: String,
    remove_javascript: bool,
    remove_embedded_files: bool,
    remove_metadata: bool,
    remove_links: bool,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    let root_id = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| EngineError::WriteFailed {
            reason: "PDF has no /Root catalog".into(),
        })?;

    // Batch the catalog key removals so we borrow it mutably just once.
    let mut catalog_keys: Vec<&[u8]> = Vec::new();
    if remove_javascript {
        catalog_keys.push(b"OpenAction");
        catalog_keys.push(b"AA");
    }
    if remove_embedded_files {
        catalog_keys.push(b"AF");
    }
    if remove_metadata {
        catalog_keys.push(b"Metadata");
    }
    let names_id = doc
        .get_dictionary(root_id)
        .ok()
        .and_then(|c| c.get(b"Names").ok())
        .and_then(|o| o.as_reference().ok());
    if let Ok(catalog) = doc.get_object_mut(root_id).and_then(Object::as_dict_mut) {
        for key in catalog_keys {
            catalog.remove(key);
        }
    }

    if remove_metadata {
        doc.trailer.remove(b"Info");
    }

    // JavaScript and embedded-file name trees hang off Catalog /Names.
    if let Some(names_id) = names_id {
        if let Ok(names) = doc.get_object_mut(names_id).and_then(Object::as_dict_mut) {
            if remove_javascript {
                names.remove(b"JavaScript");
            }
            if remove_embedded_files {
                names.remove(b"EmbeddedFiles");
            }
        }
    }

    if remove_links {
        strip_link_annotations(&mut doc);
    }

    save_document(&mut doc, &output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

/// Drop every `/Subtype /Link` annotation from each page's `/Annots` array.
fn strip_link_annotations(doc: &mut Document) {
    let page_ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    for page_id in page_ids {
        let Ok(page) = doc.get_dictionary(page_id) else {
            continue;
        };
        let Ok(annots) = page.get(b"Annots").and_then(Object::as_array) else {
            continue;
        };
        let kept: Vec<Object> = annots
            .iter()
            .filter(|obj| !is_link_annotation(doc, obj))
            .cloned()
            .collect();
        if let Ok(page) = doc.get_object_mut(page_id).and_then(Object::as_dict_mut) {
            page.set("Annots", kept);
        }
    }
}

fn is_link_annotation(doc: &Document, annot: &Object) -> bool {
    let dict = match annot {
        Object::Reference(id) => match doc.get_dictionary(*id) {
            Ok(d) => d,
            Err(_) => return false,
        },
        Object::Dictionary(d) => d,
        _ => return false,
    };
    dict.get(b"Subtype")
        .and_then(Object::as_name)
        .map(|n| n == b"Link")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;
    use std::env::temp_dir;

    fn build(path: &std::path::Path) {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let link = doc.add_object(dictionary! { "Type" => "Annot", "Subtype" => "Link" });
        let text = doc.add_object(dictionary! { "Type" => "Annot", "Subtype" => "Text" });
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Annots" => vec![Object::Reference(link), Object::Reference(text)],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Count" => 1, "Kids" => vec![Object::Reference(page_id)],
            }),
        );
        let js = doc.add_object(dictionary! {});
        let names_id = doc.add_object(dictionary! {
            "JavaScript" => Object::Reference(js),
            "EmbeddedFiles" => Object::Reference(js),
        });
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
            "Names" => Object::Reference(names_id),
            "OpenAction" => dictionary! { "S" => "JavaScript", "JS" => Object::string_literal("app.alert(1)") },
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));
        let info_id = doc.add_object(dictionary! { "Author" => Object::string_literal("Bob") });
        doc.trailer.set("Info", Object::Reference(info_id));
        save_document(&mut doc, path).unwrap();
    }

    #[test]
    fn removes_all_requested_vectors() {
        let dir = temp_dir();
        let input = dir.join("san_in.pdf");
        let output = dir.join("san_out.pdf");
        build(&input);

        security_sanitize(
            input.to_string_lossy().into_owned(),
            true,
            true,
            true,
            true,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let catalog = doc.catalog().unwrap();
        assert!(catalog.get(b"OpenAction").is_err(), "OpenAction JS removed");
        assert!(doc.trailer.get(b"Info").is_err(), "Info metadata removed");

        let names_id = catalog.get(b"Names").unwrap().as_reference().unwrap();
        let names = doc.get_dictionary(names_id).unwrap();
        assert!(names.get(b"JavaScript").is_err(), "JS name tree removed");
        assert!(
            names.get(b"EmbeddedFiles").is_err(),
            "embedded files removed"
        );

        let page_id = *doc.get_pages().get(&1).unwrap();
        let annots = doc
            .get_dictionary(page_id)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(annots.len(), 1, "only the non-link annotation survives");
    }

    #[test]
    fn flags_are_independent() {
        let dir = temp_dir();
        let input = dir.join("san_flag_in.pdf");
        let output = dir.join("san_flag_out.pdf");
        build(&input);

        // Only strip links; JS and metadata must remain.
        security_sanitize(
            input.to_string_lossy().into_owned(),
            false,
            false,
            false,
            true,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        assert!(doc.catalog().unwrap().get(b"OpenAction").is_ok(), "JS kept");
        assert!(doc.trailer.get(b"Info").is_ok(), "metadata kept");
        let page_id = *doc.get_pages().get(&1).unwrap();
        let annots = doc
            .get_dictionary(page_id)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(annots.len(), 1, "link stripped");
    }
}
