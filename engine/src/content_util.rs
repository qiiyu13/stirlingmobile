//! Shared helpers for tools that draw onto existing PDF pages
//! (`watermark`, `page_numbers`, `stamp`): page geometry and registering
//! font / graphics-state / xobject resources without clobbering resources a
//! page inherits from its `/Pages` ancestors.

use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use std::path::Path;

/// Save `doc` to `path`, working around a lopdf 0.34 bug: `Document::save`
/// writes a single flattened xref table but never clears a pre-existing
/// `trailer["Prev"]`/`trailer["XRefStm"]` carried over from the source file's
/// incremental-update chain (common in linearized PDFs, e.g. img2pdf output).
/// That stale offset no longer points at anything in the freshly written,
/// differently-sized file, corrupting it for any reader that follows `Prev`
/// (lopdf itself included). Every tool must save through this helper instead
/// of calling `doc.save()` directly.
pub(crate) fn save_document(doc: &mut Document, path: impl AsRef<Path>) -> lopdf::Result<()> {
    doc.trailer.remove(b"Prev");
    doc.trailer.remove(b"XRefStm");
    doc.save(path)?;
    Ok(())
}

/// Used when a page (or its inherited ancestors) has no `/MediaBox` -
/// US Letter in points, the PDF spec default.
const DEFAULT_MEDIA_BOX: (f32, f32, f32, f32) = (0.0, 0.0, 612.0, 792.0);

/// Page `/MediaBox` is inheritable (ISO 32000-1 §7.7.3.4) - walk `/Parent`
/// until found, falling back to US Letter if the tree has none at all.
pub(crate) fn page_size(doc: &Document, page_id: ObjectId) -> (f32, f32) {
    let mut current = Some(page_id);
    while let Some(id) = current {
        let Ok(dict) = doc.get_dictionary(id) else {
            break;
        };
        if let Some(bbox) = media_box_of(dict) {
            return bbox;
        }
        current = dict.get(b"Parent").ok().and_then(|o| o.as_reference().ok());
    }
    let (_, _, w, h) = DEFAULT_MEDIA_BOX;
    (w, h)
}

fn media_box_of(dict: &Dictionary) -> Option<(f32, f32)> {
    let arr = dict.get(b"MediaBox").ok()?.as_array().ok()?;
    let x0 = arr.first()?.as_float().ok()?;
    let y0 = arr.get(1)?.as_float().ok()?;
    let x1 = arr.get(2)?.as_float().ok()?;
    let y1 = arr.get(3)?.as_float().ok()?;
    Some((x1 - x0, y1 - y0))
}

/// Ensure `page_id` has its *own* `/Resources` dict. If it currently inherits
/// one from a `/Pages` ancestor, clone the inherited dict onto the page first
/// so adding our resources doesn't shadow (and break) the fonts/xobjects the
/// existing page content already references.
fn ensure_own_resources(doc: &mut Document, page_id: ObjectId) {
    let page_has_own = doc
        .get_dictionary(page_id)
        .map(|d| d.has(b"Resources"))
        .unwrap_or(false);
    if page_has_own {
        return;
    }
    let inherited = inherited_resources(doc, page_id);
    if let Ok(page) = doc.get_object_mut(page_id).and_then(Object::as_dict_mut) {
        page.set("Resources", inherited.unwrap_or_default());
    }
}

/// Resolve the nearest ancestor `/Resources` (following `/Parent`) to a plain
/// inline `Dictionary` we can clone onto the page.
fn inherited_resources(doc: &Document, page_id: ObjectId) -> Option<Dictionary> {
    let mut current = doc
        .get_dictionary(page_id)
        .ok()?
        .get(b"Parent")
        .ok()
        .and_then(|o| o.as_reference().ok());
    while let Some(id) = current {
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(res) = dict.get(b"Resources") {
            let resolved = match res {
                Object::Reference(rid) => doc.get_dictionary(*rid).ok()?,
                Object::Dictionary(d) => d,
                _ => return None,
            };
            return Some(resolved.clone());
        }
        current = dict.get(b"Parent").ok().and_then(|o| o.as_reference().ok());
    }
    None
}

/// Register `obj_id` under `name` in `Resources -> category` (e.g. `Font`,
/// `ExtGState`, `XObject`), creating the sub-dict if absent. Handles a
/// `/Resources` that is stored as an indirect reference.
fn register_resource(
    doc: &mut Document,
    page_id: ObjectId,
    category: &[u8],
    name: &str,
    obj_id: ObjectId,
) {
    ensure_own_resources(doc, page_id);
    // /Resources may itself be an indirect reference - resolve to the id we
    // can mutate in place.
    let res_ref = doc
        .get_dictionary(page_id)
        .ok()
        .and_then(|d| d.get(b"Resources").ok())
        .and_then(|o| o.as_reference().ok());
    let resources = match res_ref {
        Some(rid) => doc.get_object_mut(rid).and_then(Object::as_dict_mut),
        None => doc
            .get_object_mut(page_id)
            .and_then(Object::as_dict_mut)
            .and_then(|p| p.get_mut(b"Resources"))
            .and_then(Object::as_dict_mut),
    };
    let Ok(resources) = resources else { return };
    if !resources.has(category) {
        resources.set(category.to_vec(), Dictionary::new());
    }
    if let Ok(sub) = resources.get_mut(category).and_then(Object::as_dict_mut) {
        sub.set(name.as_bytes().to_vec(), Object::Reference(obj_id));
    }
}

/// Add a standard-14 Type1 font (e.g. `Helvetica`) to the page and register it
/// under `name`. Standard-14 fonts are always available, need no embedding,
/// and cover WinAnsi/ASCII text - enough for watermarks and page numbers.
pub(crate) fn add_font(doc: &mut Document, page_id: ObjectId, name: &str, base_font: &str) {
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => base_font,
        "Encoding" => "WinAnsiEncoding",
    });
    register_resource(doc, page_id, b"Font", name, font_id);
}

/// Add an `/ExtGState` setting fill and stroke alpha to `opacity` (clamped to
/// 0..=1) and register it under `name`, so a `<name> gs` operator turns on
/// transparency for subsequent drawing.
pub(crate) fn add_opacity_gs(doc: &mut Document, page_id: ObjectId, name: &str, opacity: f32) {
    let a = opacity.clamp(0.0, 1.0) as f64;
    let gs_id = doc.add_object(dictionary! {
        "Type" => "ExtGState",
        "ca" => a,
        "CA" => a,
    });
    register_resource(doc, page_id, b"ExtGState", name, gs_id);
}

/// Add an image `/XObject` (already-built stream) to the page under `name`.
pub(crate) fn add_image_xobject(
    doc: &mut Document,
    page_id: ObjectId,
    name: &str,
    stream: Stream,
) -> ObjectId {
    let id = doc.add_object(stream);
    register_resource(doc, page_id, b"XObject", name, id);
    id
}
