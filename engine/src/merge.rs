use lopdf::{Document, Object, ObjectId};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum EngineError {
    #[error("no input files provided")]
    NoInput,
    #[error("failed to read/parse pdf at {path}: {reason}")]
    ReadFailed { path: String, reason: String },
    #[error("failed to write output pdf: {reason}")]
    WriteFailed { reason: String },
}

/// Merges PDFs at `input_paths` (in order) into a single PDF written to `output_path`.
#[uniffi::export]
pub fn merge_pdfs(input_paths: Vec<String>, output_path: String) -> Result<(), EngineError> {
    if input_paths.is_empty() {
        return Err(EngineError::NoInput);
    }

    let mut documents = Vec::with_capacity(input_paths.len());
    for path in &input_paths {
        let doc = Document::load(path).map_err(|e| EngineError::ReadFailed {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        documents.push(doc);
    }

    let merged = merge_documents(documents);

    let mut merged = merged;
    merged
        .save(&output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

fn merge_documents(documents: Vec<Document>) -> Document {
    let mut max_id = 1;
    let mut all_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut all_page_ids: Vec<ObjectId> = Vec::new();

    for mut doc in documents {
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        all_page_ids.extend(doc.get_pages().values().copied());
        all_objects.extend(doc.objects);
    }

    let mut merged = Document::with_version("1.7");
    merged.objects = all_objects;
    // max_id must reflect the already-renumbered input objects before minting
    // new ids below, or new_object_id() hands out colliding ids that silently
    // overwrite real merged-in objects (e.g. the first doc's font dict).
    merged.max_id = max_id - 1;

    // Locate a Pages dict and a Catalog dict template to reuse, then rebuild
    // Kids/Count so the merged tree references every page from every input.
    let pages_id = merged.new_object_id();
    let mut kids = Vec::with_capacity(all_page_ids.len());
    for page_id in &all_page_ids {
        if let Ok(page_dict) = merged.get_object_mut(*page_id).and_then(|o| o.as_dict_mut()) {
            page_dict.set("Parent", Object::Reference(pages_id));
        }
        kids.push(Object::Reference(*page_id));
    }

    let mut pages_dict = lopdf::Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Count", Object::Integer(kids.len() as i64));
    pages_dict.set("Kids", Object::Array(kids));
    merged
        .objects
        .insert(pages_id, Object::Dictionary(pages_dict));

    let catalog_id = merged.new_object_id();
    let mut catalog_dict = lopdf::Dictionary::new();
    catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    merged
        .objects
        .insert(catalog_id, Object::Dictionary(catalog_dict));

    merged.trailer.set("Root", Object::Reference(catalog_id));
    merged.max_id = merged.objects.len() as u32;
    merged.renumber_objects();
    merged.compress();

    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, dictionary};

    fn one_page_doc() -> Document {
        let mut doc = Document::with_version("1.7");
        let content_id = doc.add_object(lopdf::Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
        ));
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });
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
        doc.max_id = doc.objects.len() as u32;
        doc
    }

    #[test]
    fn merge_combines_page_counts() {
        let merged = merge_documents(vec![one_page_doc(), one_page_doc(), one_page_doc()]);
        assert_eq!(merged.get_pages().len(), 3);
    }
}
