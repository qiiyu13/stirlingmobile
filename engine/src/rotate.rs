use crate::content_util::save_document;
use crate::EngineError;
use lopdf::{Document, Object};

/// Rotates every page of the PDF at `input_path` by `angle_degrees`
/// (added to each page's existing rotation, normalized to 0/90/180/270)
/// and writes the result to `output_path`.
#[uniffi::export]
pub fn rotate_pdf(
    input_path: String,
    angle_degrees: i32,
    output_path: String,
) -> Result<(), EngineError> {
    if angle_degrees % 90 != 0 {
        return Err(EngineError::WriteFailed {
            reason: format!("angle must be a multiple of 90, got {angle_degrees}"),
        });
    }

    let mut doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    let page_ids: Vec<_> = doc.get_pages().into_values().collect();
    for page_id in page_ids {
        if let Ok(page_dict) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
            let existing = page_dict
                .get(b"Rotate")
                .and_then(Object::as_i64)
                .unwrap_or(0);
            let new_rotation = (existing + angle_degrees as i64).rem_euclid(360);
            page_dict.set("Rotate", Object::Integer(new_rotation));
        }
    }

    save_document(&mut doc, &output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, dictionary};
    use std::env::temp_dir;

    fn one_page_pdf(path: &std::path::Path) {
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
        save_document(&mut doc, path).unwrap();
    }

    #[test]
    fn rotate_sets_normalized_angle() {
        let input = temp_dir().join("rotate_test_input.pdf");
        let output = temp_dir().join("rotate_test_output.pdf");
        one_page_pdf(&input);

        rotate_pdf(
            input.to_string_lossy().into_owned(),
            450, // 450 % 360 == 90
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().values().next().unwrap();
        let rotate = doc
            .get_object(page_id)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Rotate")
            .and_then(Object::as_i64)
            .unwrap();
        assert_eq!(rotate, 90);
    }

    #[test]
    fn rejects_non_multiple_of_90() {
        let input = temp_dir().join("rotate_test_input2.pdf");
        one_page_pdf(&input);
        let result = rotate_pdf(
            input.to_string_lossy().into_owned(),
            45,
            temp_dir()
                .join("rotate_test_output2.pdf")
                .to_string_lossy()
                .into_owned(),
        );
        assert!(result.is_err());
    }
}
