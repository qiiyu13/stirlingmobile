use crate::EngineError;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object};
use std::collections::HashMap;

/// Per-glyph advance widths for one font, read from the PDF's own width
/// tables so redaction can locate text exactly instead of guessing with an
/// average glyph width - the latter drifts further off the further a
/// match is from the start of a text-show operator, letting a box under-
/// or over-shoot the real glyph position.
///
/// Two PDF font families need different code-to-glyph mapping: a "simple"
/// font (`/Widths`, `/FirstChar`) uses one byte per glyph, but a
/// `Type0`/CID composite font (near-universal for embedded Unicode
/// TrueType subsets - e.g. text produced via a browser/HTML-to-PDF
/// pipeline) uses two bytes per glyph (`Identity-H`/`-V` encoding, the
/// overwhelming common case) with widths from `/DescendantFonts[0]`'s
/// `/W` array and `/DW` default. Treating a CID font's bytes as one byte
/// per glyph (the old bug) advances the position cursor at roughly half
/// the real rate, drifting a redaction box further off the longer the
/// preceding text in that run.
enum FontWidths {
    Simple {
        first_char: i64,
        widths: Vec<f64>,
        missing_width: f64,
    },
    Cid {
        default_width: f64,
        widths: HashMap<u32, f64>,
    },
}

impl FontWidths {
    /// Returns `(advance width in em, bytes consumed)` for the glyph
    /// starting at `bytes[pos]`.
    fn glyph_at(&self, bytes: &[u8], pos: usize, fallback: f64) -> (f64, usize) {
        match self {
            FontWidths::Simple { first_char, widths, missing_width } => {
                let idx = bytes[pos] as i64 - first_char;
                let width = (idx >= 0).then(|| widths.get(idx as usize).copied()).flatten();
                let width = width.unwrap_or(if *missing_width > 0.0 { *missing_width } else { fallback });
                (width, 1)
            }
            FontWidths::Cid { default_width, widths } => {
                if pos + 1 < bytes.len() {
                    let cid = ((bytes[pos] as u32) << 8) | bytes[pos + 1] as u32;
                    (widths.get(&cid).copied().unwrap_or(*default_width), 2)
                } else {
                    (*default_width, 1)
                }
            }
        }
    }
}

fn resolve<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
    match obj {
        Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}

fn resolve_dict<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a lopdf::Dictionary> {
    resolve(doc, obj)?.as_dict().ok()
}

fn font_widths_from_dict(doc: &Document, font_obj: &Object) -> Option<FontWidths> {
    let font_dict = resolve_dict(doc, font_obj)?;
    if font_dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) == Some(b"Type0") {
        return Some(cid_font_widths_from_dict(doc, font_dict));
    }
    simple_font_widths_from_dict(doc, font_dict)
}

fn simple_font_widths_from_dict(doc: &Document, font_dict: &lopdf::Dictionary) -> Option<FontWidths> {
    let first_char = font_dict.get(b"FirstChar").ok().and_then(|o| o.as_i64().ok())?;
    let widths_arr = font_dict.get(b"Widths").ok().and_then(|o| resolve(doc, o))?.as_array().ok()?;
    let widths = widths_arr
        .iter()
        .map(|w| resolve(doc, w).and_then(|o| o.as_float().ok()).unwrap_or(0.0) as f64 / 1000.0)
        .collect();
    let missing_width = font_dict
        .get(b"FontDescriptor")
        .ok()
        .and_then(|o| resolve_dict(doc, o))
        .and_then(|fd| fd.get(b"MissingWidth").ok())
        .and_then(|o| o.as_float().ok())
        .map(|w| w as f64 / 1000.0)
        .unwrap_or(0.0);
    Some(FontWidths::Simple { first_char, widths, missing_width })
}

/// Parses `/DescendantFonts[0]`'s `/DW` and `/W` (CID width array, whose
/// entries can be either `c [w1 w2 ...]` - consecutive CIDs starting at
/// `c` - or `c_first c_last w` - a range sharing one width). Malformed or
/// absent entries just leave that CID on the `/DW` default rather than
/// aborting the whole font - getting the 2-bytes-per-glyph pairing right
/// matters more for alignment than getting every individual width exact.
fn cid_font_widths_from_dict(doc: &Document, font_dict: &lopdf::Dictionary) -> FontWidths {
    let descendant = font_dict
        .get(b"DescendantFonts")
        .ok()
        .and_then(|o| resolve(doc, o))
        .and_then(|o| o.as_array().ok())
        .and_then(|arr| arr.first())
        .and_then(|o| resolve_dict(doc, o));

    let mut default_width = 1.0; // PDF spec default DW is 1000/1000 em
    let mut widths = HashMap::new();

    if let Some(desc_dict) = descendant {
        if let Some(dw) = desc_dict.get(b"DW").ok().and_then(|o| o.as_float().ok()) {
            default_width = dw as f64 / 1000.0;
        }
        if let Some(w_arr) = desc_dict.get(b"W").ok().and_then(|o| resolve(doc, o)).and_then(|o| o.as_array().ok()) {
            let mut i = 0;
            while i + 1 < w_arr.len() {
                let Some(start_cid) = resolve(doc, &w_arr[i]).and_then(|o| o.as_i64().ok()) else { break };
                match resolve(doc, &w_arr[i + 1]) {
                    Some(Object::Array(list)) => {
                        for (offset, w) in list.iter().enumerate() {
                            if let Some(width) = resolve(doc, w).and_then(|o| o.as_float().ok()) {
                                widths.insert((start_cid + offset as i64) as u32, width as f64 / 1000.0);
                            }
                        }
                        i += 2;
                    }
                    Some(_) if i + 2 < w_arr.len() => {
                        let Some(end_cid) = resolve(doc, &w_arr[i + 1]).and_then(|o| o.as_i64().ok()) else { break };
                        let Some(width) = resolve(doc, &w_arr[i + 2]).and_then(|o| o.as_float().ok()) else { break };
                        for cid in start_cid..=end_cid {
                            widths.insert(cid as u32, width as f64 / 1000.0);
                        }
                        i += 3;
                    }
                    _ => break,
                }
            }
        }
    }

    FontWidths::Cid { default_width, widths }
}

/// Maps font resource name (e.g. `F1`, as used in `/F1 24 Tf`) to that
/// font's widths, for every font in the page's `/Resources /Font` dict.
/// Fonts without a `/Widths` array (e.g. an unembedded standard-14 font)
/// are simply absent from the map - callers fall back to an average-width
/// estimate for those.
///
/// Resolves `/Resources` itself rather than using
/// `Document::get_page_resources` - that method's `Option<&Dictionary>`
/// only handles a directly-embedded Resources dict and silently returns
/// `None` when (as is standard/common) it's an indirect reference.
fn load_font_widths(doc: &Document, page_id: lopdf::ObjectId) -> HashMap<Vec<u8>, FontWidths> {
    let mut map = HashMap::new();
    let Ok(page) = doc.get_dictionary(page_id) else {
        return map;
    };
    let Some(resources) = page.get(b"Resources").ok().and_then(|o| resolve_dict(doc, o)) else {
        return map;
    };
    let Some(fonts) = resources.get(b"Font").ok().and_then(|o| resolve_dict(doc, o)) else {
        return map;
    };
    for (name, font_obj) in fonts.iter() {
        if let Some(fw) = font_widths_from_dict(doc, font_obj) {
            map.insert(name.clone(), fw);
        }
    }
    map
}

/// A rectangle to redact, in PDF page-space points (origin bottom-left,
/// same coordinate system as the page's `/MediaBox`). `page` is 1-indexed.
#[derive(Debug, Clone, uniffi::Record)]
pub struct RedactionArea {
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// True content removal (not overlay): any text-show operator whose
/// rendered position falls inside a redaction rectangle is dropped from
/// the page's content stream entirely, so the text is gone from copy/paste
/// and extraction, not just hidden under a box. A solid black rectangle is
/// then painted over each area so the redaction is visible.
#[uniffi::export]
pub fn content_redact(
    input_path: String,
    redactions: Vec<RedactionArea>,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = load(&input_path)?;
    apply_redactions(&mut doc, &redactions)?;
    save(doc, &output_path)
}

pub(crate) fn load(input_path: &str) -> Result<Document, EngineError> {
    Document::load(input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path.to_string(),
        reason: e.to_string(),
    })
}

pub(crate) fn save(mut doc: Document, output_path: &str) -> Result<(), EngineError> {
    doc.prune_objects();
    doc.renumber_objects();
    doc.save(output_path).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

pub(crate) fn apply_redactions(doc: &mut Document, redactions: &[RedactionArea]) -> Result<(), EngineError> {
    let mut by_page: HashMap<u32, Vec<(f64, f64, f64, f64)>> = HashMap::new();
    for r in redactions {
        by_page
            .entry(r.page)
            .or_default()
            .push((r.x as f64, r.y as f64, r.width as f64, r.height as f64));
    }
    if by_page.is_empty() {
        return Ok(());
    }

    let pages = doc.get_pages();
    for (page_num, boxes) in &by_page {
        let Some(&page_id) = pages.get(page_num) else {
            return Err(EngineError::WriteFailed {
                reason: format!("redaction references page {page_num}, but the document has {} pages", pages.len()),
            });
        };
        redact_page(doc, page_id, boxes)?;
    }
    Ok(())
}

fn redact_page(doc: &mut Document, page_id: lopdf::ObjectId, boxes: &[(f64, f64, f64, f64)]) -> Result<(), EngineError> {
    let raw = doc.get_page_content(page_id).map_err(|e| EngineError::ReadFailed {
        path: String::new(),
        reason: format!("failed to read page content: {e}"),
    })?;
    let content = Content::decode(&raw).map_err(|e| EngineError::ReadFailed {
        path: String::new(),
        reason: format!("failed to decode page content stream: {e}"),
    })?;

    let font_widths = load_font_widths(doc, page_id);
    let kept_ops = strip_text_in_boxes(content.operations, boxes, &font_widths);

    let mut new_content = Content { operations: kept_ops };
    for &(x, y, w, h) in boxes {
        new_content.operations.push(Operation::new("q", vec![]));
        new_content
            .operations
            .push(Operation::new("rg", vec![0.0.into(), 0.0.into(), 0.0.into()]));
        new_content.operations.push(Operation::new(
            "re",
            vec![(x as f32).into(), (y as f32).into(), (w as f32).into(), (h as f32).into()],
        ));
        new_content.operations.push(Operation::new("f", vec![]));
        new_content.operations.push(Operation::new("Q", vec![]));
    }

    let encoded = new_content.encode().map_err(|e| EngineError::WriteFailed {
        reason: format!("failed to encode redacted content stream: {e}"),
    })?;

    let new_stream_id = doc.add_object(lopdf::Stream::new(lopdf::dictionary! {}, encoded));
    if let Ok(page_dict) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
        page_dict.set("Contents", Object::Reference(new_stream_id));
    }
    Ok(())
}

/// Blanks (replaces with spaces) the characters of any text-show operator
/// (`Tj`, `TJ`, `'`, `"`) whose position falls inside a redaction
/// rectangle, rather than dropping the whole operator - a rectangle
/// covering only part of a longer run (e.g. one word inside a paragraph
/// drawn as a single `Tj`) no longer wipes the rest of that run. Position
/// is computed from the text/graphics matrix chain (`cm`, `Tm`, `Td`/`TD`,
/// `T*`) and, per character, the font's real `/Widths` advance width when
/// available (see [`FontWidths`]) - falling back to an average-glyph-width
/// estimate only for fonts with no `/Widths` array.
fn strip_text_in_boxes(operations: Vec<Operation>, boxes: &[(f64, f64, f64, f64)], font_widths: &HashMap<Vec<u8>, FontWidths>) -> Vec<Operation> {
    const AVG_GLYPH_WIDTH_EM: f64 = 0.65;

    let mut ctm_stack: Vec<Matrix> = Vec::new();
    let mut ctm = Matrix::IDENTITY;
    let mut tm = Matrix::IDENTITY;
    let mut tlm = Matrix::IDENTITY;
    let mut font_size = 0.0f64;
    let mut leading = 0.0f64;
    let mut current_font: Option<&FontWidths> = None;

    let mut kept = Vec::with_capacity(operations.len());
    for mut op in operations {
        match op.operator.as_str() {
            "q" => {
                ctm_stack.push(ctm);
                kept.push(op);
            }
            "Q" => {
                if let Some(m) = ctm_stack.pop() {
                    ctm = m;
                }
                kept.push(op);
            }
            "cm" => {
                if let Some(m) = matrix_from_operands(&op.operands) {
                    ctm = m.then(&ctm);
                }
                kept.push(op);
            }
            "BT" => {
                tm = Matrix::IDENTITY;
                tlm = Matrix::IDENTITY;
                kept.push(op);
            }
            "Tf" => {
                if let Some(size) = op.operands.get(1).and_then(|o| o.as_float().ok()) {
                    font_size = size as f64;
                }
                current_font = op.operands.first().and_then(|o| o.as_name().ok()).and_then(|name| font_widths.get(name));
                kept.push(op);
            }
            "TL" => {
                if let Some(l) = op.operands.first().and_then(|o| o.as_float().ok()) {
                    leading = l as f64;
                }
                kept.push(op);
            }
            "Td" => {
                if let Some((tx, ty)) = xy_operands(&op.operands) {
                    tlm = Matrix::translation(tx, ty).then(&tlm);
                    tm = tlm;
                }
                kept.push(op);
            }
            "TD" => {
                if let Some((tx, ty)) = xy_operands(&op.operands) {
                    leading = -ty;
                    tlm = Matrix::translation(tx, ty).then(&tlm);
                    tm = tlm;
                }
                kept.push(op);
            }
            "T*" => {
                tlm = Matrix::translation(0.0, -leading).then(&tlm);
                tm = tlm;
                kept.push(op);
            }
            "Tm" => {
                if let Some(m) = matrix_from_operands(&op.operands) {
                    tlm = m;
                    tm = m;
                }
                kept.push(op);
            }
            "Tj" => {
                if let Some(Object::String(bytes, _)) = op.operands.last_mut() {
                    redact_string_bytes(bytes, 0.0, &tm, &ctm, font_size, current_font, AVG_GLYPH_WIDTH_EM, boxes);
                }
                kept.push(op);
            }
            "'" | "\"" => {
                tlm = Matrix::translation(0.0, -leading).then(&tlm);
                tm = tlm;
                if let Some(Object::String(bytes, _)) = op.operands.last_mut() {
                    redact_string_bytes(bytes, 0.0, &tm, &ctm, font_size, current_font, AVG_GLYPH_WIDTH_EM, boxes);
                }
                kept.push(op);
            }
            "TJ" => {
                if let Some(Object::Array(arr)) = op.operands.first_mut() {
                    // Advance an em-space cursor across the whole array:
                    // strings consume their own glyph widths, and bare
                    // numbers are inter-glyph kerning adjustments (in
                    // thousandths of text space, subtracted per spec).
                    let mut offset = 0.0f64;
                    for elem in arr.iter_mut() {
                        match elem {
                            Object::String(bytes, _) => {
                                offset = redact_string_bytes(bytes, offset, &tm, &ctm, font_size, current_font, AVG_GLYPH_WIDTH_EM, boxes);
                            }
                            other => {
                                if let Ok(num) = other.as_float() {
                                    offset -= num as f64 / 1000.0;
                                }
                            }
                        }
                    }
                }
                kept.push(op);
            }
            _ => kept.push(op),
        }
    }
    kept
}

fn xy_operands(operands: &[Object]) -> Option<(f64, f64)> {
    let tx = operands.first()?.as_float().ok()? as f64;
    let ty = operands.get(1)?.as_float().ok()? as f64;
    Some((tx, ty))
}

fn matrix_from_operands(operands: &[Object]) -> Option<Matrix> {
    if operands.len() < 6 {
        return None;
    }
    let v: Vec<f64> = operands[..6].iter().map(|o| o.as_float().ok().map(|f| f as f64)).collect::<Option<_>>()?;
    Some(Matrix::new(v[0], v[1], v[2], v[3], v[4], v[5]))
}

/// Replaces bytes in `bytes` with spaces wherever the glyph at that
/// position (an em-space cursor starting at `offset`, within the current
/// text-show operator's logical run) falls inside a redaction rectangle.
/// Glyph byte-width (1 for a simple font, 2 for a Type0/CID font - see
/// [`FontWidths`]) and advance come from `font` when known, else 1 byte /
/// `fallback_width_em`. Returns the cursor position after the last glyph,
/// so callers stringing multiple string operands together (`TJ` arrays)
/// can carry it over.
#[allow(clippy::too_many_arguments)]
fn redact_string_bytes(
    bytes: &mut [u8],
    mut offset: f64,
    tm: &Matrix,
    ctm: &Matrix,
    font_size: f64,
    font: Option<&FontWidths>,
    fallback_width_em: f64,
    boxes: &[(f64, f64, f64, f64)],
) -> f64 {
    let mut pos = 0;
    while pos < bytes.len() {
        let (width, consumed) = match font {
            Some(f) => f.glyph_at(bytes, pos, fallback_width_em),
            None => (fallback_width_em, 1),
        };
        if char_hits_box(tm, ctm, font_size, offset, offset + width, boxes) {
            let end = (pos + consumed).min(bytes.len());
            for b in &mut bytes[pos..end] {
                *b = b' ';
            }
        }
        offset += width;
        pos += consumed.max(1);
    }
    offset
}

fn char_hits_box(tm: &Matrix, ctm: &Matrix, font_size: f64, x0: f64, x1: f64, boxes: &[(f64, f64, f64, f64)]) -> bool {
    if font_size <= 0.0 {
        return false;
    }
    let scale = Matrix::new(font_size, 0.0, 0.0, font_size, 0.0, 0.0);
    let effective = scale.then(tm).then(ctm);

    let corners = [
        effective.apply(x0, 0.0),
        effective.apply(x1, 0.0),
        effective.apply(x0, 1.0),
        effective.apply(x1, 1.0),
    ];
    let min_x = corners.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let max_x = corners.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let min_y = corners.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let max_y = corners.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);

    boxes.iter().any(|&(bx, by, bw, bh)| {
        let (box_x0, box_x1) = (bx, bx + bw);
        let (box_y0, box_y1) = (by, by + bh);
        min_x < box_x1 && max_x > box_x0 && min_y < box_y1 && max_y > box_y0
    })
}

/// A PDF content-stream affine matrix `[a b c d e f]`, applied to row
/// vectors as `[x y 1] * M`.
#[derive(Clone, Copy)]
pub(crate) struct Matrix {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Matrix {
    pub(crate) const IDENTITY: Matrix = Matrix { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub(crate) fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Matrix { a, b, c, d, e, f }
    }

    pub(crate) fn translation(tx: f64, ty: f64) -> Self {
        Matrix { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: tx, f: ty }
    }

    pub(crate) fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (x * self.a + y * self.c + self.e, x * self.b + y * self.d + self.f)
    }

    /// Returns `self` concatenated with `other`, such that applying a point
    /// to the result equals applying it to `self` then to `other`.
    pub(crate) fn then(&self, other: &Matrix) -> Matrix {
        Matrix {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Object};
    use std::env::temp_dir;

    fn pdf_with_text(path: &std::path::Path, lines: &[(&str, f64, f64)]) {
        let mut doc = Document::with_version("1.7");
        let mut ops = vec![Operation::new("BT", vec![]), Operation::new("Tf", vec!["F1".into(), 24.0.into()])];
        for &(text, x, y) in lines {
            ops.push(Operation::new("Td", vec![x.into(), y.into()]));
            ops.push(Operation::new("Tj", vec![Object::string_literal(text)]));
        }
        ops.push(Operation::new("ET", vec![]));
        let content = Content { operations: ops }.encode().unwrap();
        let content_id = doc.add_object(lopdf::Stream::new(dictionary! {}, content));

        let font_id = doc.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => Object::Reference(font_id) },
        });

        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => Object::Reference(resources_id),
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
        doc.save(path).unwrap();
    }

    fn page_text(doc: &Document) -> String {
        let page_id = *doc.get_pages().values().next().unwrap();
        let bytes = doc.get_page_content(page_id).unwrap();
        let content = Content::decode(&bytes).unwrap();
        content
            .operations
            .iter()
            .filter(|op| op.operator == "Tj")
            .filter_map(|op| op.operands.last().and_then(|o| o.as_str().ok()))
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect::<Vec<_>>()
            .join("|")
    }

    #[test]
    fn redacts_text_inside_box_leaves_text_outside_alone() {
        let input = temp_dir().join("redact_test_input.pdf");
        let output = temp_dir().join("redact_test_output.pdf");
        pdf_with_text(&input, &[("secret ssn 123-45-6789", 50.0, 700.0), ("keep this line", 50.0, 500.0)]);

        content_redact(
            input.to_string_lossy().into_owned(),
            vec![RedactionArea { page: 1, x: 0.0, y: 690.0, width: 400.0, height: 40.0 }],
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let text = page_text(&doc);
        assert!(!text.contains("123-45-6789"), "redacted text should be gone from the content stream: {text}");
        assert!(text.contains("keep this line"), "text outside the redaction box should survive: {text}");
    }

    /// Same as `pdf_with_text` but the font declares an explicit
    /// `/Widths` array (monospace, 0.4em/glyph) instead of relying on the
    /// average-glyph-width fallback - lets a test box be sized from the
    /// font's *real* metrics rather than the 0.65em guess.
    fn pdf_with_text_and_widths(path: &std::path::Path, text: &str, x: f64, y: f64) {
        let mut doc = Document::with_version("1.7");
        let ops = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 24.0.into()]),
            Operation::new("Td", vec![x.into(), y.into()]),
            Operation::new("Tj", vec![Object::string_literal(text)]),
            Operation::new("ET", vec![]),
        ];
        let content = Content { operations: ops }.encode().unwrap();
        let content_id = doc.add_object(lopdf::Stream::new(dictionary! {}, content));

        let widths: Vec<Object> = (32..=126).map(|_| 400.into()).collect();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier",
            "FirstChar" => 32, "LastChar" => 126, "Widths" => widths,
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => Object::Reference(font_id) },
        });

        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => Object::Reference(resources_id),
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
        doc.save(path).unwrap();
    }

    /// Builds a page whose text is shown through a `Type0`/CID font with
    /// 2-byte (Identity-H-style) codes - the encoding used by virtually
    /// every embedded-Unicode TrueType subset (e.g. text from a
    /// browser/HTML-to-PDF pipeline). CID == the character's ASCII value,
    /// so the test can still reason about which characters got redacted.
    fn pdf_with_cid_text(path: &std::path::Path, text: &str, x: f64, y: f64) {
        let mut doc = Document::with_version("1.7");
        let mut encoded = Vec::with_capacity(text.len() * 2);
        for &b in text.as_bytes() {
            encoded.push(0u8);
            encoded.push(b);
        }
        let ops = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 24.0.into()]),
            Operation::new("Td", vec![x.into(), y.into()]),
            Operation::new("Tj", vec![Object::String(encoded, lopdf::StringFormat::Hexadecimal)]),
            Operation::new("ET", vec![]),
        ];
        let content = Content { operations: ops }.encode().unwrap();
        let content_id = doc.add_object(lopdf::Stream::new(dictionary! {}, content));

        // /W [32 126 400] - every CID in the printable-ASCII range advances
        // 0.4em, same as the simple-font Widths test, so the box math lines
        // up the same way.
        let descendant_id = doc.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "Courier",
            "DW" => 400, "W" => vec![32.into(), 126.into(), 400.into()],
        });
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "Courier",
            "Encoding" => "Identity-H", "DescendantFonts" => vec![Object::Reference(descendant_id)],
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => Object::Reference(font_id) },
        });

        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => Object::Reference(resources_id),
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
        doc.save(path).unwrap();
    }

    /// Decodes a 2-byte-per-glyph Tj string back to ASCII (CID == ASCII
    /// value in `pdf_with_cid_text`) for assertions.
    fn cid_page_text(doc: &Document) -> String {
        let page_id = *doc.get_pages().values().next().unwrap();
        let bytes = doc.get_page_content(page_id).unwrap();
        let content = Content::decode(&bytes).unwrap();
        content
            .operations
            .iter()
            .filter(|op| op.operator == "Tj")
            .filter_map(|op| op.operands.last().and_then(|o| o.as_str().ok()))
            .map(|s| s.chunks(2).map(|pair| pair[1] as char).collect::<String>())
            .collect::<Vec<_>>()
            .join("|")
    }

    #[test]
    fn redacts_using_real_font_widths_with_2byte_cid_font() {
        // Regression test: a Type0/CID font's glyphs are 2 content-stream
        // bytes each, not 1. Treating them as 1-byte-per-glyph (the old
        // bug) advances the position cursor at roughly half the real
        // rate, so a redaction box lands on the wrong characters - worse
        // the further into the line the match is. Same "John" target and
        // box as `redacts_using_real_font_widths_when_available`.
        let input = temp_dir().join("redact_test_cid_input.pdf");
        let output = temp_dir().join("redact_test_cid_output.pdf");
        pdf_with_cid_text(&input, "Contact John Doe here", 50.0, 700.0);

        content_redact(
            input.to_string_lossy().into_owned(),
            vec![RedactionArea { page: 1, x: 127.0, y: 690.0, width: 38.0, height: 40.0 }],
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let text = cid_page_text(&doc);
        assert!(!text.contains("John"), "targeted word should be redacted: {text}");
        assert!(text.contains("Contact"), "word before the redacted one should survive: {text}");
        assert!(text.contains("Doe"), "word after the redacted one should survive: {text}");
        assert!(text.contains("here"), "rest of the line should survive: {text}");
    }

    #[test]
    fn redacts_using_real_font_widths_when_available() {
        // "Contact John Doe here" at font size 24, 0.4em/glyph (from
        // /Widths) => 9.6pt/char. "John" is chars 8-11, so it spans
        // x=[50+8*9.6, 50+12*9.6] = [126.8, 165.2]. A box sized from that
        // exact math - not the 0.65em fallback used when /Widths is
        // absent - should redact exactly "John" and nothing else.
        let input = temp_dir().join("redact_test_widths_input.pdf");
        let output = temp_dir().join("redact_test_widths_output.pdf");
        pdf_with_text_and_widths(&input, "Contact John Doe here", 50.0, 700.0);

        content_redact(
            input.to_string_lossy().into_owned(),
            vec![RedactionArea { page: 1, x: 127.0, y: 690.0, width: 38.0, height: 40.0 }],
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let text = page_text(&doc);
        assert!(!text.contains("John"), "targeted word should be redacted: {text}");
        assert!(text.contains("Contact"), "word before the redacted one should survive: {text}");
        assert!(text.contains("Doe"), "word after the redacted one should survive: {text}");
        assert!(text.contains("here"), "rest of the line should survive: {text}");
    }

    #[test]
    fn redacting_one_word_in_a_line_leaves_rest_of_that_line_intact() {
        // Regression test: previously any box overlap dropped the WHOLE
        // Tj operator, so redacting one word out of a multi-word line
        // wiped every other word in that line too.
        let input = temp_dir().join("redact_test_partial_input.pdf");
        let output = temp_dir().join("redact_test_partial_output.pdf");
        pdf_with_text(&input, &[("Contact John Doe here", 50.0, 700.0)]);

        // Box tightly covers only "John" (chars 8-11 at font size 24,
        // AVG_GLYPH_WIDTH_EM 0.65 => ~15.6pt/char, starting at x=50).
        content_redact(
            input.to_string_lossy().into_owned(),
            vec![RedactionArea { page: 1, x: 175.0, y: 690.0, width: 60.0, height: 40.0 }],
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let text = page_text(&doc);
        assert!(!text.contains("John"), "targeted word should be redacted: {text}");
        assert!(text.contains("Contact"), "word before the redacted one should survive: {text}");
        assert!(text.contains("Doe"), "word after the redacted one should survive: {text}");
        assert!(text.contains("here"), "rest of the line should survive: {text}");
    }

    #[test]
    fn draws_black_box_over_redacted_area() {
        let input = temp_dir().join("redact_test_box_input.pdf");
        let output = temp_dir().join("redact_test_box_output.pdf");
        pdf_with_text(&input, &[("hide me", 50.0, 700.0)]);

        content_redact(
            input.to_string_lossy().into_owned(),
            vec![RedactionArea { page: 1, x: 0.0, y: 690.0, width: 400.0, height: 40.0 }],
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().values().next().unwrap();
        let bytes = doc.get_page_content(page_id).unwrap();
        let content = Content::decode(&bytes).unwrap();
        assert!(content.operations.iter().any(|op| op.operator == "re"), "expected a filled rectangle over the redaction area");
    }

    #[test]
    fn matrix_translation_composes() {
        let a = Matrix::translation(10.0, 0.0);
        let b = Matrix::translation(0.0, 5.0);
        let combined = a.then(&b);
        assert_eq!(combined.apply(0.0, 0.0), (10.0, 5.0));
    }
}
