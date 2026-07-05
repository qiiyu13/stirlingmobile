//! Overlay one PDF's pages onto another's (F-111): each `overlay_path` page
//! is drawn on top of the matching `base_path` page (repeating the overlay's
//! last page if the base has more pages), scaled to fit and centered.

use crate::content_util::{add_image_xobject, add_opacity_gs, page_size};
use crate::EngineError;
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, ObjectId, Stream};

/// Draws every page of `overlay_path` onto the corresponding page of
/// `base_path` (by index, repeating the overlay's last page if the base has
/// more pages) at `opacity` (0..=1), writing the result to `output_path`.
#[uniffi::export]
pub fn tool_overlay(
    base_path: String,
    overlay_path: String,
    opacity: f32,
    output_path: String,
) -> Result<(), EngineError> {
    let mut base_doc = Document::load(&base_path).map_err(|e| EngineError::ReadFailed {
        path: base_path.clone(),
        reason: e.to_string(),
    })?;
    let mut overlay_doc =
        Document::load(&overlay_path).map_err(|e| EngineError::ReadFailed {
            path: overlay_path.clone(),
            reason: e.to_string(),
        })?;

    let mut base_pages: Vec<(u32, ObjectId)> = base_doc.get_pages().into_iter().collect();
    base_pages.sort_by_key(|(n, _)| *n);
    if base_pages.is_empty() {
        return Err(EngineError::NoInput);
    }

    // Renumber the overlay doc's objects past the base doc's so merging them
    // into one object space can't collide ids (same technique as merge_pdfs).
    let overlay_start_id = base_doc.max_id + 1;
    overlay_doc.renumber_objects_with(overlay_start_id);
    let mut overlay_pages: Vec<(u32, ObjectId)> = overlay_doc.get_pages().into_iter().collect();
    overlay_pages.sort_by_key(|(n, _)| *n);
    if overlay_pages.is_empty() {
        return Err(EngineError::NoInput);
    }

    base_doc.objects.extend(overlay_doc.objects);
    base_doc.max_id = base_doc.max_id.max(overlay_doc.max_id);

    for (i, (_base_num, base_page_id)) in base_pages.iter().enumerate() {
        let ov_idx = i.min(overlay_pages.len() - 1);
        let (_ov_num, ov_page_id) = overlay_pages[ov_idx];

        let stream = build_form_xobject(&base_doc, ov_page_id);
        let name = format!("SMovl{i}");
        add_image_xobject(&mut base_doc, *base_page_id, &name, stream);

        let gs_name = format!("SMovlG{i}");
        add_opacity_gs(&mut base_doc, *base_page_id, &gs_name, opacity);

        let (ov_w, ov_h) = page_size(&base_doc, ov_page_id);
        let (base_w, base_h) = page_size(&base_doc, *base_page_id);
        let scale = (base_w / ov_w.max(1.0)).min(base_h / ov_h.max(1.0));
        let ox = (base_w - ov_w * scale) / 2.0;
        let oy = (base_h - ov_h * scale) / 2.0;

        let ops = vec![
            Operation::new("q", vec![]),
            Operation::new("gs", vec![gs_name.into()]),
            Operation::new(
                "cm",
                vec![
                    scale.into(),
                    0f32.into(),
                    0f32.into(),
                    scale.into(),
                    ox.into(),
                    oy.into(),
                ],
            ),
            Operation::new("Do", vec![name.into()]),
            Operation::new("Q", vec![]),
        ];
        base_doc
            .add_to_page_content(*base_page_id, Content { operations: ops })
            .map_err(|e| EngineError::WriteFailed {
                reason: e.to_string(),
            })?;
    }

    base_doc.compress();
    base_doc.save(&output_path).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

/// Builds a Form XObject wrapping `page_id`'s content and its own
/// `/Resources`, so the overlay page's fonts/images don't need renaming or
/// merging into the base page's resource dict.
fn build_form_xobject(doc: &Document, page_id: ObjectId) -> Stream {
    let (w, h) = page_size(doc, page_id);
    let content = doc.get_page_content(page_id).unwrap_or_default();
    let (resources, _) = doc
        .get_page_resources(page_id)
        .unwrap_or((None, Vec::new()));
    let resources_obj = resources
        .cloned()
        .map(Object::Dictionary)
        .unwrap_or_else(|| Object::Dictionary(lopdf::Dictionary::new()));

    let xobj_dict = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Form",
        "BBox" => vec![0f32.into(), 0f32.into(), w.into(), h.into()],
        "Resources" => resources_obj,
    };
    Stream::new(xobj_dict, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Content as LopdfContent;
    use std::env::temp_dir;

    fn one_page_pdf(path: &std::path::Path, w: f32, h: f32) {
        let mut doc = Document::with_version("1.7");
        let content_id = doc.add_object(lopdf::Stream::new(
            dictionary! {},
            LopdfContent { operations: vec![] }.encode().unwrap(),
        ));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), w.into(), h.into()],
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
        doc.save(path).unwrap();
    }

    #[test]
    fn overlay_adds_form_xobject_to_base_page() {
        let dir = temp_dir();
        let base = dir.join("overlay_base.pdf");
        let overlay = dir.join("overlay_stamp.pdf");
        let output = dir.join("overlay_out.pdf");
        one_page_pdf(&base, 612.0, 792.0);
        one_page_pdf(&overlay, 200.0, 100.0);

        tool_overlay(
            base.to_string_lossy().into_owned(),
            overlay.to_string_lossy().into_owned(),
            0.5,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let (resources, _) = doc.get_page_resources(page_id).unwrap();
        let xobjects = resources
            .and_then(|r| r.get(b"XObject").ok())
            .and_then(|o| o.as_dict().ok());
        assert!(xobjects.map(|d| !d.is_empty()).unwrap_or(false));
    }

    #[test]
    fn overlay_rejects_empty_overlay_doc() {
        let dir = temp_dir();
        let base = dir.join("overlay_base2.pdf");
        let overlay = dir.join("overlay_empty.pdf");
        let output = dir.join("overlay_out2.pdf");
        one_page_pdf(&base, 612.0, 792.0);

        let mut doc = Document::with_version("1.7");
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog" });
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(&overlay).unwrap();

        let result = tool_overlay(
            base.to_string_lossy().into_owned(),
            overlay.to_string_lossy().into_owned(),
            0.5,
            output.to_string_lossy().into_owned(),
        );
        assert!(result.is_err());
    }
}
