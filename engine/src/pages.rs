use crate::EngineError;
use lopdf::Document;

fn load(input_path: &str) -> Result<Document, EngineError> {
    Document::load(input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path.to_string(),
        reason: e.to_string(),
    })
}

fn save(mut doc: Document, output_path: &str) -> Result<(), EngineError> {
    doc.prune_objects();
    doc.renumber_objects();
    doc.save(output_path).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

/// Removes the given 1-indexed `pages` from the PDF at `input_path` and
/// writes the rest to `output_path`.
#[uniffi::export]
pub fn remove_pages(
    input_path: String,
    pages: Vec<u32>,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    doc.delete_pages(&pages);
    save(doc, &output_path)
}

/// Keeps only the given 1-indexed `pages` from the PDF at `input_path`
/// (in document order, not `pages` order) and writes them to `output_path`.
#[uniffi::export]
pub fn extract_pages(
    input_path: String,
    pages: Vec<u32>,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    let total_pages = doc.get_pages().len() as u32;
    let to_delete: Vec<u32> = (1..=total_pages).filter(|p| !pages.contains(p)).collect();
    doc.delete_pages(&to_delete);
    save(doc, &output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, dictionary, Object};
    use std::env::temp_dir;

    fn n_page_pdf(path: &std::path::Path, n: u32) {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let mut kids = Vec::new();
        for _ in 0..n {
            let content_id = doc.add_object(lopdf::Stream::new(
                dictionary! {},
                Content { operations: vec![] }.encode().unwrap(),
            ));
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
                "Count" => n as i64,
                "Kids" => kids,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    #[test]
    fn remove_pages_drops_given_pages() {
        let input = temp_dir().join("pages_remove_input.pdf");
        let output = temp_dir().join("pages_remove_output.pdf");
        n_page_pdf(&input, 5);

        remove_pages(
            input.to_string_lossy().into_owned(),
            vec![2, 4],
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        assert_eq!(doc.get_pages().len(), 3);
    }

    #[test]
    fn extract_pages_keeps_only_given_pages() {
        let input = temp_dir().join("pages_extract_input.pdf");
        let output = temp_dir().join("pages_extract_output.pdf");
        n_page_pdf(&input, 5);

        extract_pages(
            input.to_string_lossy().into_owned(),
            vec![2, 4],
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
    }
}
