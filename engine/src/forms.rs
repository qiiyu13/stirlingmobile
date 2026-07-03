use crate::EngineError;
use lopdf::{dictionary, Document, Object, ObjectId, Stream};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, uniffi::Record)]
pub struct FormField {
    pub name: String,
    pub field_type: String,
    pub value: Option<String>,
    pub page: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FieldFill {
    pub name: String,
    pub value: String,
}

fn resolve_page(pages: &BTreeMap<u32, ObjectId>, page_id: ObjectId) -> u32 {
    pages
        .iter()
        .find(|(_, pid)| **pid == page_id)
        .map(|(p, _)| *p)
        .unwrap_or(0)
}

fn field_value(dict: &lopdf::Dictionary) -> Option<String> {
    dict.get(b"V").ok().and_then(|o| match o {
        Object::String(s, _) => Some(String::from_utf8_lossy(s).to_string()),
        Object::Name(n) => Some(String::from_utf8_lossy(n).to_string()),
        _ => None,
    })
}

fn field_name(dict: &lopdf::Dictionary) -> Option<String> {
    dict.get(b"T")
        .ok()
        .and_then(|o| match o {
            Object::String(s, _) => Some(String::from_utf8_lossy(s).to_string()),
            _ => None,
        })
}

fn field_type(dict: &lopdf::Dictionary) -> Option<String> {
    dict.get(b"FT")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|n| String::from_utf8_lossy(n).to_string())
}

fn walk_fields(
    doc: &Document,
    fields: &[Object],
    pages: &BTreeMap<u32, ObjectId>,
) -> Vec<FormField> {
    let mut result = Vec::new();
    for field_obj in fields {
        let field_id = match field_obj.as_reference() {
            Ok(id) => id,
            _ => continue,
        };
        let dict = match doc.get_dictionary(field_id) {
            Ok(d) => d,
            _ => continue,
        };

        let kids = dict
            .get(b"Kids")
            .ok()
            .and_then(|o| o.as_array().ok());

        if let Some(kids_arr) = kids {
            let first_is_widget = kids_arr
                .first()
                .and_then(|o| o.as_reference().ok())
                .and_then(|id| doc.get_dictionary(id).ok())
                .and_then(|d| d.get(b"Subtype").ok())
                .and_then(|o| o.as_name().ok())
                .map(|n| n == b"Widget")
                .unwrap_or(false);

            if first_is_widget {
                if let (Some(ft), Some(name)) = (field_type(dict), field_name(dict)) {
                    let page = kids_arr
                        .first()
                        .and_then(|o| o.as_reference().ok())
                        .and_then(|id| doc.get_dictionary(id).ok())
                        .and_then(|d| d.get(b"P").ok())
                        .and_then(|o| o.as_reference().ok())
                        .map(|p_id| resolve_page(pages, p_id))
                        .unwrap_or(0);
                    result.push(FormField {
                        name,
                        field_type: ft,
                        value: field_value(dict),
                        page,
                    });
                }
            } else {
                result.extend(walk_fields(doc, kids_arr, pages));
            }
        } else if field_type(dict).is_some() {
            if let Some(name) = field_name(dict) {
                let page = dict
                    .get(b"P")
                    .ok()
                    .and_then(|o| o.as_reference().ok())
                    .map(|p_id| resolve_page(pages, p_id))
                    .unwrap_or(0);
                result.push(FormField {
                    name,
                    field_type: field_type(dict).unwrap(),
                    value: field_value(dict),
                    page,
                });
            }
        }
    }
    result
}

#[uniffi::export]
pub fn forms_get_fields(path: String) -> Result<Vec<FormField>, EngineError> {
    let doc = Document::load(&path).map_err(|e| EngineError::ReadFailed {
        path,
        reason: e.to_string(),
    })?;

    let pages = doc.get_pages();

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or(EngineError::NoAcroForm)?;

    let acroform_id = doc
        .get_dictionary(catalog_id)
        .ok()
        .and_then(|d| d.get(b"AcroForm").ok())
        .and_then(|o| o.as_reference().ok())
        .ok_or(EngineError::NoAcroForm)?;

    let fields = doc
        .get_dictionary(acroform_id)
        .ok()
        .and_then(|d| d.get(b"Fields").ok())
        .and_then(|o| o.as_array().ok())
        .cloned()
        .unwrap_or_default();

    Ok(walk_fields(&doc, &fields, &pages))
}

fn fill_recursive(doc: &mut Document, fields: &[Object], values: &HashMap<String, String>) {
    for field_obj in fields {
        let field_id = match field_obj.as_reference() {
            Ok(id) => id,
            _ => continue,
        };
        let name = doc
            .get_dictionary(field_id)
            .ok()
            .and_then(|d| d.get(b"T").ok())
            .and_then(|o| match o {
                Object::String(s, _) => Some(String::from_utf8_lossy(s).to_string()),
                _ => None,
            });

        let kids = doc
            .get_dictionary(field_id)
            .ok()
            .and_then(|d| d.get(b"Kids").ok())
            .and_then(|o| o.as_array().ok())
            .cloned();

        if let Some(kids_arr) = kids {
            let first_is_widget = kids_arr
                .first()
                .and_then(|o| o.as_reference().ok())
                .and_then(|id| doc.get_dictionary(id).ok())
                .and_then(|d| d.get(b"Subtype").ok())
                .and_then(|o| o.as_name().ok())
                .map(|n| n == b"Widget")
                .unwrap_or(false);

            if first_is_widget {
                if let Some(ref n) = name {
                    if let Some(val) = values.get(n) {
                        if let Ok(field) = doc.get_object_mut(field_id) {
                            if let Ok(d) = field.as_dict_mut() {
                                d.set("V", Object::string_literal(val.clone()));
                            }
                        }
                    }
                }
            } else {
                fill_recursive(doc, &kids_arr, values);
            }
        } else if let Some(ref n) = name {
            if let Some(val) = values.get(n) {
                if let Ok(field) = doc.get_object_mut(field_id) {
                    if let Ok(d) = field.as_dict_mut() {
                        d.set("V", Object::string_literal(val.clone()));
                    }
                }
            }
        }
    }
}

#[uniffi::export]
pub fn forms_fill(
    path: String,
    values: Vec<FieldFill>,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = Document::load(&path).map_err(|e| EngineError::ReadFailed {
        path,
        reason: e.to_string(),
    })?;

    let value_map: HashMap<String, String> =
        values.into_iter().map(|f| (f.name, f.value)).collect();

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or(EngineError::NoAcroForm)?;

    let acroform_id = doc
        .get_dictionary(catalog_id)
        .ok()
        .and_then(|d| d.get(b"AcroForm").ok())
        .and_then(|o| o.as_reference().ok())
        .ok_or(EngineError::NoAcroForm)?;

    let fields = doc
        .get_dictionary(acroform_id)
        .ok()
        .and_then(|d| d.get(b"Fields").ok())
        .and_then(|o| o.as_array().ok())
        .cloned()
        .unwrap_or_default();

    if let Ok(af) = doc.get_object_mut(acroform_id).and_then(|o| o.as_dict_mut()) {
        af.set("NeedAppearances", true);
    }

    fill_recursive(&mut doc, &fields, &value_map);

    doc.compress();
    doc.save(&output_path).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

fn rect_values(dict: &lopdf::Dictionary) -> Option<[f64; 4]> {
    let arr = dict.get(b"Rect").ok()?.as_array().ok()?;
    Some([
        arr.first()?.as_float().ok()? as f64,
        arr.get(1)?.as_float().ok()? as f64,
        arr.get(2)?.as_float().ok()? as f64,
        arr.get(3)?.as_float().ok()? as f64,
    ])
}

/// Register an XObject resource on a page, creating the /XObject sub-dict if absent.
fn register_xobject(doc: &mut Document, page_id: ObjectId, name: &str, obj_id: ObjectId) {
    let has_res = doc
        .get_dictionary(page_id)
        .map(|d| d.has(b"Resources"))
        .unwrap_or(false);
    if !has_res {
        if let Ok(page) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
            page.set("Resources", lopdf::Dictionary::new());
        }
    }
    let res_ref = doc
        .get_dictionary(page_id)
        .ok()
        .and_then(|d| d.get(b"Resources").ok())
        .and_then(|o| o.as_reference().ok());
    let resources = match res_ref {
        Some(rid) => doc.get_object_mut(rid).and_then(|o| o.as_dict_mut()),
        None => doc
            .get_object_mut(page_id)
            .and_then(|o| o.as_dict_mut())
            .and_then(|p| p.get_mut(b"Resources"))
            .and_then(|o| o.as_dict_mut()),
    };
    if let Ok(res) = resources {
        if !res.has(b"XObject") {
            res.set(b"XObject".to_vec(), lopdf::Dictionary::new());
        }
        if let Ok(xobjs) = res.get_mut(b"XObject").and_then(|o| o.as_dict_mut()) {
            xobjs.set(name.as_bytes().to_vec(), Object::Reference(obj_id));
        }
    }
}

/// Flatten widget annotations into page content. For each widget with an
/// `/AP/N` appearance stream, wraps the stream as a Form XObject and inserts
/// a `Do` command at the front of the page content, positioned by the widget's
/// `/Rect`. Widgets without appearance streams are silently left in place.
///
/// # ponytail: only handles /AP/N (normal appearance); /AP/R and /AP/D ignored.
#[uniffi::export]
pub fn forms_flatten(path: String, output_path: String) -> Result<(), EngineError> {
    let mut doc = Document::load(&path).map_err(|e| EngineError::ReadFailed {
        path,
        reason: e.to_string(),
    })?;

    let pages = doc.get_pages();

    for (_page_num, page_id) in &pages {
        let annots = doc
            .get_dictionary(*page_id)
            .ok()
            .and_then(|d| d.get(b"Annots").ok())
            .and_then(|o| o.as_array().ok())
            .cloned()
            .unwrap_or_default();

        let mut widget_ops: Vec<Vec<u8>> = Vec::new();
        let mut to_remove: Vec<usize> = Vec::new();

        for (idx, annot_obj) in annots.iter().enumerate() {
            let annot_id = match annot_obj.as_reference() {
                Ok(id) => id,
                _ => continue,
            };
            let annot_dict = match doc.get_dictionary(annot_id) {
                Ok(d) => d,
                _ => continue,
            };
            let subtype = annot_dict
                .get(b"Subtype")
                .ok()
                .and_then(|o| o.as_name().ok());
            if subtype != Some(b"Widget") {
                continue;
            }

            let rect = match rect_values(annot_dict) {
                Some(r) => r,
                None => continue,
            };

            let ap = annot_dict
                .get(b"AP")
                .ok()
                .and_then(|o| o.as_dict().ok());
            let ap_n = ap
                .and_then(|ap_dict| ap_dict.get(b"N").ok())
                .and_then(|o| o.as_reference().ok());
            let ap_n = match ap_n {
                Some(id) => id,
                None => continue,
            };

            let ap_bytes = match doc.get_object(ap_n).ok() {
                Some(Object::Stream(s)) => s.content.clone(),
                _ => continue,
            };

            let ap_dict = doc
                .get_dictionary(ap_n)
                .ok()
                .cloned()
                .unwrap_or_default();

            let xobj_id = doc.new_object_id();
            let bbox: Vec<Object> = vec![
                0.into(),
                0.into(),
                (rect[2] - rect[0]).into(),
                (rect[3] - rect[1]).into(),
            ];
            let mut xobj_dict = dictionary! {
                "Type" => "XObject",
                "Subtype" => "Form",
                "BBox" => bbox,
            };
            if ap_dict.has(b"Matrix") {
                if let Ok(m) = ap_dict.get(b"Matrix") {
                    xobj_dict.set("Matrix", m.clone());
                }
            }
            let xobj_stream = Stream::new(xobj_dict, ap_bytes);
            doc.objects
                .insert(xobj_id, Object::Stream(xobj_stream));

            let name = format!("FW{}", idx);
            register_xobject(&mut doc, *page_id, &name, xobj_id);

            let w = rect[2] - rect[0];
            let h = rect[3] - rect[1];
            let x = rect[0];
            let y = rect[1];
            widget_ops.push(
                format!("q {w:.3} 0 0 {h:.3} {x:.3} {y:.3} cm /{name} Do Q ")
                    .into_bytes(),
            );

            to_remove.push(idx);
        }

        if widget_ops.is_empty() {
            continue;
        }

        let page_content_id = doc
            .get_dictionary(*page_id)
            .ok()
            .and_then(|d| d.get(b"Contents").ok())
            .and_then(|o| match o {
                Object::Reference(id) => Some(*id),
                Object::Array(arr) => arr.first()?.as_reference().ok(),
                _ => None,
            });

        if let Some(content_id) = page_content_id {
            let original_bytes = doc
                .get_object(content_id)
                .ok()
                .and_then(|o| match o {
                    Object::Stream(s) => Some(s.content.clone()),
                    _ => None,
                })
                .unwrap_or_default();

            let mut combined = widget_ops.concat();
            combined.extend(original_bytes);

            if let Ok(obj) = doc.get_object_mut(content_id) {
                if let Object::Stream(s) = &mut *obj {
                    s.content = combined;
                    if s.dict.has(b"Filter") {
                        s.dict.remove(b"Filter");
                    }
                    if s.dict.has(b"DecodeParms") {
                        s.dict.remove(b"DecodeParms");
                    }
                }
            }
        }

        let mut page_annots = annots;
        for &idx in to_remove.iter().rev() {
            page_annots.remove(idx);
        }
        if let Ok(page_dict) = doc.get_dictionary_mut(*page_id) {
            if page_annots.is_empty() {
                page_dict.remove(b"Annots");
            } else {
                page_dict.set("Annots", page_annots);
            }
        }
    }

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok());
    if let Some(cat_id) = catalog_id {
        if let Ok(cat) = doc.get_dictionary_mut(cat_id) {
            cat.remove(b"AcroForm");
        }
    }

    doc.compress();
    doc.save(&output_path).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Content;

    fn form_pdf() -> Document {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
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
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Count" => 1,
                "Kids" => vec![Object::Reference(page_id)],
            }),
        );

        let widget_id = doc.new_object_id();
        doc.objects.insert(
            widget_id,
            Object::Dictionary(dictionary! {
                "Type" => "Annot",
                "Subtype" => "Widget",
                "FT" => "Tx",
                "T" => Object::string_literal("Name"),
                "V" => Object::string_literal("John"),
                "Rect" => vec![100.into(), 100.into(), 300.into(), 120.into()],
                "P" => Object::Reference(page_id),
                "F" => 4,
            }),
        );

        let page_annots = vec![Object::Reference(widget_id)];
        if let Ok(page) = doc.get_dictionary_mut(page_id) {
            page.set("Annots", page_annots);
        }

        let acroform_id = doc.new_object_id();
        doc.objects.insert(
            acroform_id,
            Object::Dictionary(dictionary! {
                "Fields" => vec![Object::Reference(widget_id)],
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
            "AcroForm" => Object::Reference(acroform_id),
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.max_id = doc.objects.len() as u32;
        doc
    }

    #[test]
    fn get_fields_returns_terminal_field() {
        let doc = form_pdf();
        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let acroform_id = doc
            .get_dictionary(catalog_id)
            .unwrap()
            .get(b"AcroForm")
            .unwrap()
            .as_reference()
            .unwrap();
        let fields_arr = doc
            .get_dictionary(acroform_id)
            .unwrap()
            .get(b"Fields")
            .unwrap()
            .as_array()
            .unwrap();
        let fields = walk_fields(&doc, fields_arr, &doc.get_pages());
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "Name");
        assert_eq!(fields[0].field_type, "Tx");
        assert_eq!(fields[0].value.as_deref(), Some("John"));
        assert_eq!(fields[0].page, 1);
    }

    #[test]
    fn fill_updates_field_value() {
        let mut doc = form_pdf();
        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let acroform_id = doc
            .get_dictionary(catalog_id)
            .unwrap()
            .get(b"AcroForm")
            .unwrap()
            .as_reference()
            .unwrap();
        let fields = doc
            .get_dictionary(acroform_id)
            .unwrap()
            .get(b"Fields")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();

        let mut values = HashMap::new();
        values.insert("Name".to_string(), "Jane".to_string());
        fill_recursive(&mut doc, &fields, &values);

        let field_id = fields[0].as_reference().unwrap();
        let new_val = doc
            .get_dictionary(field_id)
            .unwrap()
            .get(b"V")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(        new_val.to_vec(), b"Jane".to_vec());
    }

    #[test]
    fn flatten_removes_widgets_and_aroform() {
        use lopdf::content::Content;

        let dir = std::env::temp_dir();
        let input = dir.join("forms_flatten_test_input.pdf");
        let output = dir.join("forms_flatten_test_output.pdf");

        let mut doc = form_pdf();

        let ap_n_id = doc.new_object_id();
        let ap_content = Content { operations: vec![] }.encode().unwrap();
        doc.objects.insert(
            ap_n_id,
            Object::Stream(lopdf::Stream::new(dictionary! {}, ap_content)),
        );

        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let acroform_id = doc
            .get_dictionary(catalog_id)
            .unwrap()
            .get(b"AcroForm")
            .unwrap()
            .as_reference()
            .unwrap();
        let fields = doc
            .get_dictionary(acroform_id)
            .unwrap()
            .get(b"Fields")
            .unwrap()
            .as_array()
            .unwrap();
        let widget_id = fields[0].as_reference().unwrap();

        if let Ok(fd) = doc.get_dictionary_mut(widget_id) {
            fd.set(
                "AP",
                dictionary! {
                    "N" => Object::Reference(ap_n_id),
                },
            );
        }

        doc.save(&input).unwrap();

        forms_flatten(
            input.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let result = Document::load(&output).unwrap();
        let catalog_id = result.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let cat = result.get_dictionary(catalog_id).unwrap();
        assert!(!cat.has(b"AcroForm"), "AcroForm should be removed after flatten");

        let page_id = *result.get_pages().get(&1).unwrap();
        let page = result.get_dictionary(page_id).unwrap();
        assert!(
            !page.has(b"Annots"),
            "page annotations should be removed after flatten"
        );
    }
}

