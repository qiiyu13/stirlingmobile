use crate::EngineError;
use lopdf::{dictionary, Document, Object, ObjectId, Stream};

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

/// Reorder pages by the given 1-indexed page `order`. Must include every page
/// exactly once (a permutation). Output has pages in the specified order.
#[uniffi::export]
pub fn pages_reorder(
    input_path: String,
    order: Vec<u32>,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    let pages = doc.get_pages();
    let total = pages.len() as u32;

    let mut seen = vec![false; total as usize];
    for &p in &order {
        if p < 1 || p > total {
            return Err(EngineError::WriteFailed {
                reason: format!("page {p} out of range (1..{total})"),
            });
        }
        if seen[p as usize - 1] {
            return Err(EngineError::WriteFailed {
                reason: format!("duplicate page {p} in order"),
            });
        }
        seen[p as usize - 1] = true;
    }
    if order.len() != total as usize {
        return Err(EngineError::WriteFailed {
            reason: format!("order must contain all {total} pages"),
        });
    }

    let mut page_ids = Vec::new();
    for (_num, id) in &pages {
        page_ids.push(*id);
    }
    page_ids.sort_by_key(|id| {
        pages.iter().find(|(_n, i)| *i == id).map(|(n, _)| *n).unwrap_or(0)
    });

    let new_page_ids: Vec<ObjectId> = order
        .iter()
        .map(|p| page_ids[*p as usize - 1])
        .collect();

    let mut kids = Vec::new();
    for pid in &new_page_ids {
        if let Ok(page) = doc.get_dictionary_mut(*pid) {
            page.remove(b"Parent");
        }
        kids.push(Object::Reference(*pid));
    }

    let pages_dict_id = *doc.get_pages().values().next().unwrap();
    let parent_id = doc
        .get_dictionary(pages_dict_id)
        .ok()
        .and_then(|d| d.get(b"Parent").ok())
        .and_then(|o| o.as_reference().ok());
    let root_pages_id = parent_id.unwrap_or(pages_dict_id);

    if let Ok(root) = doc.get_dictionary_mut(root_pages_id) {
        root.set("Count", Object::Integer(kids.len() as i64));
        root.set("Kids", Object::Array(kids.clone()));
    }

    for pid in &new_page_ids {
        if let Ok(page) = doc.get_dictionary_mut(*pid) {
            page.set("Parent", Object::Reference(root_pages_id));
        }
    }

    save(doc, &output_path)
}

/// Scale all pages by `scale_x` and `scale_y` (1.0 = unchanged). Updates
/// /MediaBox and wraps page content with a scaling transform.
/// # ponytail: scales content via cm transform, not actual raster resize.
#[uniffi::export]
pub fn pages_scale(
    input_path: String,
    scale_x: f32,
    scale_y: f32,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    let pages = doc.get_pages();

    for (_page_num, page_id) in &pages {
        let media = doc
            .get_dictionary(*page_id)
            .ok()
            .and_then(|d| d.get(b"MediaBox").ok())
            .and_then(|o| o.as_array().ok())
            .cloned();

        if let Some(ref arr) = media {
            let x1 = arr.first().and_then(|o| o.as_float().ok()).unwrap_or(0.0);
            let y1 = arr.get(1).and_then(|o| o.as_float().ok()).unwrap_or(0.0);
            let x2 = arr.get(2).and_then(|o| o.as_float().ok()).unwrap_or(612.0);
            let y2 = arr.get(3).and_then(|o| o.as_float().ok()).unwrap_or(792.0);
            let new_box: Vec<Object> = vec![
                (x1 * scale_x).into(),
                (y1 * scale_y).into(),
                (x2 * scale_x).into(),
                (y2 * scale_y).into(),
            ];
            if let Ok(page) = doc.get_dictionary_mut(*page_id) {
                page.set("MediaBox", Object::Array(new_box));
            }
        }

        let content_id = doc
            .get_dictionary(*page_id)
            .ok()
            .and_then(|d| d.get(b"Contents").ok())
            .and_then(|o| o.as_reference().ok());

        if let Some(cid) = content_id {
            if let Ok(obj) = doc.get_object_mut(cid) {
                if let Object::Stream(s) = obj {
                    let scale_cm = format!("{scale_x:.3} 0 0 {scale_y:.3} 0 0 cm ")
                        .into_bytes();
                    let mut new_content = scale_cm;
                    new_content.extend(s.content.clone());
                    s.content = new_content;
                    if s.dict.has(b"Filter") {
                        s.dict.remove(b"Filter");
                    }
                    if s.dict.has(b"DecodeParms") {
                        s.dict.remove(b"DecodeParms");
                    }
                }
            }
        }
    }

    save(doc, &output_path)
}

/// Apply a crop box (x1, y1, x2, y2 in PDF user-space coordinates) to every
/// page in the PDF. Pages outside the crop box are hidden but not deleted.
#[uniffi::export]
pub fn pages_crop(
    input_path: String,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    let pages = doc.get_pages();
    let crop_arr: Vec<Object> = vec![x1.into(), y1.into(), x2.into(), y2.into()];

    for (_page_num, page_id) in &pages {
        if let Ok(page) = doc.get_dictionary_mut(*page_id) {
            page.set("CropBox", Object::Array(crop_arr.clone()));
        }
    }

    save(doc, &output_path)
}

/// Arrange `n` pages per sheet (2, 4, 6, or 9-up). Original pages are scaled
/// down and positioned in a grid on new output pages. The output sheet keeps
/// the original page size of the first page.
/// # ponytail: only supports n in {2, 4, 6, 9}; creates Form XObjects from
/// page content — doesn't preserve annotations or form fields.
#[uniffi::export]
pub fn pages_n_up(
    input_path: String,
    n: u32,
    output_path: String,
) -> Result<(), EngineError> {
    if ![2, 4, 6, 9].contains(&n) {
        return Err(EngineError::WriteFailed {
            reason: "n must be 2, 4, 6, or 9".to_string(),
        });
    }

    let doc = load(&input_path)?;
    let pages = doc.get_pages();
    let total = pages.len() as u32;
    let mut page_ids: Vec<ObjectId> = Vec::new();
    for i in 1..=total {
        if let Some(id) = pages.get(&i) {
            page_ids.push(*id);
        }
    }

    let cols = if n == 2 { 2 } else if n <= 4 { 2 } else { 3 };
    let rows = n / cols;

    let first_media = doc
        .get_dictionary(page_ids[0])
        .ok()
        .and_then(|d| d.get(b"MediaBox").ok())
        .and_then(|o| o.as_array().ok())
        .cloned();
    let (sheet_w, sheet_h) = if let Some(ref arr) = first_media {
        (
            arr.get(2).and_then(|o| o.as_float().ok()).unwrap_or(612.0),
            arr.get(3).and_then(|o| o.as_float().ok()).unwrap_or(792.0),
        )
    } else {
        (612.0, 792.0)
    };
    let cell_w = sheet_w / cols as f32;
    let cell_h = sheet_h / rows as f32;

    let mut out_doc = Document::with_version("1.7");
    let pages_id = out_doc.new_object_id();
    let mut kids = Vec::new();

    let mut idx = 0usize;
    while idx < page_ids.len() {
        let out_page_id = out_doc.new_object_id();
        let mut xobj_refs: Vec<(String, ObjectId)> = Vec::new();
        let mut content_ops = Vec::new();

        for row in (0..rows).rev() {
            for col in 0..cols {
                if idx >= page_ids.len() {
                    break;
                }
                let page_id = page_ids[idx];
                let (page_w, page_h) = doc
                    .get_dictionary(page_id)
                    .ok()
                    .and_then(|d| d.get(b"MediaBox").ok())
                    .and_then(|o| o.as_array().ok())
                    .and_then(|a| Some((
                        a.get(2).and_then(|o| o.as_float().ok()).unwrap_or(612.0)
                            - a.get(0).and_then(|o| o.as_float().ok()).unwrap_or(0.0),
                        a.get(3).and_then(|o| o.as_float().ok()).unwrap_or(792.0)
                            - a.get(1).and_then(|o| o.as_float().ok()).unwrap_or(0.0),
                    )))
                    .unwrap_or((cell_w, cell_h));

                let content_id = doc
                    .get_dictionary(page_id)
                    .ok()
                    .and_then(|d| d.get(b"Contents").ok())
                    .and_then(|o| o.as_reference().ok());

                let (cp_bytes, cp_dict) = if let Some(cid) = content_id {
                    match doc.get_object(cid).ok() {
                        Some(Object::Stream(s)) => (s.content.clone(), s.dict.clone()),
                        _ => (Vec::new(), lopdf::Dictionary::new()),
                    }
                } else {
                    (Vec::new(), lopdf::Dictionary::new())
                };

                let xobj_id = out_doc.new_object_id();
                let mut xobj_dict = dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Form",
                    "BBox" => vec![
                        0f32.into(),
                        0f32.into(),
                        page_w.into(),
                        page_h.into(),
                    ],
                };
                if cp_dict.has(b"Matrix") {
                    if let Ok(m) = cp_dict.get(b"Matrix") {
                        xobj_dict.set("Matrix", m.clone());
                    }
                }
                out_doc.objects.insert(
                    xobj_id,
                    Object::Stream(Stream::new(xobj_dict, cp_bytes)),
                );

                let name = format!("P{}", idx);
                xobj_refs.push((name.clone(), xobj_id));

                let x = col as f32 * cell_w;
                let y = row as f32 * cell_h;
                let sx = cell_w / page_w;
                let sy = cell_h / page_h;
                let s = sx.min(sy);
                let ox = (cell_w - page_w * s) / 2.0;
                let oy = (cell_h - page_h * s) / 2.0;
                content_ops.push(format!(
                    "q {s:.3} 0 0 {s:.3} {:.3} {:.3} cm /{name} Do Q ",
                    x + ox, y + oy
                ));

                idx += 1;
            }
        }

        if xobj_refs.is_empty() {
            break;
        }

        let mut resources_dict = lopdf::Dictionary::new();
        let mut xobjs_dict = lopdf::Dictionary::new();
        for (name, xobj_id) in &xobj_refs {
            xobjs_dict.set(name.as_bytes().to_vec(), Object::Reference(*xobj_id));
        }
        resources_dict.set(b"XObject".to_vec(), xobjs_dict);

        let content_bytes: Vec<u8> =
            content_ops.iter().flat_map(|s| s.as_bytes().to_vec()).collect();
        let content_id = out_doc.add_object(Stream::new(
            dictionary! {},
            content_bytes,
        ));

        out_doc.objects.insert(
            out_page_id,
            Object::Dictionary(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![
                    0f32.into(),
                    0f32.into(),
                    sheet_w.into(),
                    sheet_h.into(),
                ],
                "Contents" => content_id,
                "Resources" => resources_dict,
            }),
        );
        kids.push(Object::Reference(out_page_id));
    }

    out_doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => kids.len() as i64,
            "Kids" => kids,
        }),
    );
    let catalog_id = out_doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    out_doc.trailer.set("Root", Object::Reference(catalog_id));
    out_doc.max_id = out_doc.objects.len() as u32;

    save(out_doc, &output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, Object};
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
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
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

    fn doc_page_count(path: &std::path::Path) -> usize {
        Document::load(path).unwrap().get_pages().len()
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
        assert_eq!(doc_page_count(&output), 3);
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
        assert_eq!(doc_page_count(&output), 2);
    }

    #[test]
    fn reorder_swaps_pages() {
        let input = temp_dir().join("pages_reorder_input.pdf");
        let output = temp_dir().join("pages_reorder_output.pdf");
        n_page_pdf(&input, 3);
        pages_reorder(
            input.to_string_lossy().into_owned(),
            vec![3, 1, 2],
            output.to_string_lossy().into_owned(),
        )
        .unwrap();
        assert_eq!(doc_page_count(&output), 3);
    }

    #[test]
    fn reorder_rejects_invalid_order() {
        let input = temp_dir().join("pages_reorder_bad_input.pdf");
        let output = temp_dir().join("pages_reorder_bad_output.pdf");
        n_page_pdf(&input, 3);
        let r = pages_reorder(
            input.to_string_lossy().into_owned(),
            vec![1, 2],
            output.to_string_lossy().into_owned(),
        );
        assert!(r.is_err());
    }

    #[test]
    fn scale_pages_by_factor() {
        let input = temp_dir().join("pages_scale_input.pdf");
        let output = temp_dir().join("pages_scale_output.pdf");
        n_page_pdf(&input, 1);
        pages_scale(
            input.to_string_lossy().into_owned(),
            0.5,
            0.5,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();
        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let media = doc.get_dictionary(page_id).unwrap().get(b"MediaBox").unwrap().as_array().unwrap();
        let w = media[2].as_float().unwrap();
        assert!((w - 306.0).abs() < 1.0, "scaled width should be ~306");
    }

    #[test]
    fn crop_sets_cropbox() {
        let input = temp_dir().join("pages_crop_input.pdf");
        let output = temp_dir().join("pages_crop_output.pdf");
        n_page_pdf(&input, 1);
        pages_crop(
            input.to_string_lossy().into_owned(),
            10.0,
            10.0,
            500.0,
            700.0,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();
        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        assert!(doc.get_dictionary(page_id).unwrap().has(b"CropBox"));
    }

    #[test]
    fn n_up_2_reduces_pages() {
        let input = temp_dir().join("pages_nup_input.pdf");
        let output = temp_dir().join("pages_nup_output.pdf");
        n_page_pdf(&input, 6);
        pages_n_up(
            input.to_string_lossy().into_owned(),
            2,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();
        assert_eq!(doc_page_count(&output), 3);
    }

    #[test]
    fn n_up_4_reduces_pages() {
        let input = temp_dir().join("pages_nup4_input.pdf");
        let output = temp_dir().join("pages_nup4_output.pdf");
        n_page_pdf(&input, 8);
        pages_n_up(
            input.to_string_lossy().into_owned(),
            4,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();
        assert_eq!(doc_page_count(&output), 2);
    }

    #[test]
    fn n_up_9_reduces_pages() {
        let input = temp_dir().join("pages_nup9_input.pdf");
        let output = temp_dir().join("pages_nup9_output.pdf");
        n_page_pdf(&input, 9);
        pages_n_up(
            input.to_string_lossy().into_owned(),
            9,
            output.to_string_lossy().into_owned(),
        )
        .unwrap();
        assert_eq!(doc_page_count(&output), 1);
    }
}

