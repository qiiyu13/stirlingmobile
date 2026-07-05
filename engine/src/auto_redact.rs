use crate::content_util::save_document;
use crate::rasterize::bind_pdfium;
use crate::redact::{apply_redactions, load, save, RedactionArea};
use crate::EngineError;
use regex::Regex;

/// Finds text matching any of `patterns` and redacts it (see
/// [`crate::redact::content_redact`] for what "redact" means here - true
/// removal from the content stream, not just an overlay).
///
/// Each pattern is either a named detector (`"email"`, `"phone_us"`,
/// `"ssn"`, `"credit_card"`), an exact-text literal (`"text:<literal>"`,
/// for users who don't know regex - matched verbatim, special chars
/// escaped), or a caller-supplied `"regex:<pattern>"`.
/// `pdfium_lib_dir` is `context.applicationInfo.nativeLibraryDir`, same as
/// [`crate::convert_pdf_to_images`] - used to locate matched text on the
/// page, since content-stream parsing alone can't do fuzzy text search.
#[uniffi::export]
pub fn content_auto_redact(
    input_path: String,
    pdfium_lib_dir: String,
    patterns: Vec<String>,
    output_path: String,
) -> Result<(), EngineError> {
    let regexes = compile_patterns(&patterns)?;
    let redactions = find_matches(&input_path, &pdfium_lib_dir, &regexes)?;

    let mut doc = load(&input_path)?;
    apply_redactions(&mut doc, &redactions)?;
    save(doc, &output_path)
}

fn compile_patterns(patterns: &[String]) -> Result<Vec<Regex>, EngineError> {
    patterns
        .iter()
        .map(|p| {
            let source = match p.as_str() {
                "email" => r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}".to_string(),
                "phone_us" => r"(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]\d{3}[-.\s]\d{4}".to_string(),
                "ssn" => r"\d{3}-\d{2}-\d{4}".to_string(),
                "credit_card" => r"\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}".to_string(),
                other => match other.strip_prefix("regex:") {
                    Some(pattern) => pattern.to_string(),
                    None => match other.strip_prefix("text:") {
                        Some(literal) => regex::escape(literal),
                        None => {
                            return Err(EngineError::WriteFailed {
                                reason: format!(
                                    "unknown auto-redact pattern '{other}': expected email, phone_us, ssn, credit_card, text:<literal>, or regex:<pattern>"
                                ),
                            })
                        }
                    },
                },
            };
            Regex::new(&source).map_err(|e| EngineError::WriteFailed {
                reason: format!("invalid pattern '{p}': {e}"),
            })
        })
        .collect()
}

fn find_matches(
    input_path: &str,
    pdfium_lib_dir: &str,
    regexes: &[Regex],
) -> Result<Vec<RedactionArea>, EngineError> {
    let pdfium = bind_pdfium(pdfium_lib_dir)?;
    let document =
        pdfium
            .load_pdf_from_file(input_path, None)
            .map_err(|e| EngineError::ReadFailed {
                path: input_path.to_string(),
                reason: e.to_string(),
            })?;

    let mut redactions = Vec::new();
    for (index, page) in document.pages().iter().enumerate() {
        let text = page.text().map_err(|e| EngineError::ReadFailed {
            path: input_path.to_string(),
            reason: format!("failed to read text on page {}: {e}", index + 1),
        })?;
        let all_chars = text.chars();
        let chars: Vec<_> = all_chars.iter().collect();
        // Build the searchable string FROM the glyph list, so a regex match's
        // byte range maps back to exactly the glyphs that produced it. Using
        // pdfium's text.all() instead assumed all() and chars() had the same
        // count/order - false whenever a glyph carries no Unicode (FontAwesome
        // icons, CMSY bullets) or a ligature/space differs, which shifted every
        // later glyph's bounds and painted redaction boxes over the wrong text
        // (e.g. a Typst/LaTeX CV whose contact line is full of icon glyphs).
        // `char_start[i]` is glyph i's byte offset in `full`; a glyph with no
        // Unicode contributes an empty range and is never matched.
        let mut full = String::new();
        let mut char_start: Vec<usize> = Vec::with_capacity(chars.len() + 1);
        for c in &chars {
            char_start.push(full.len());
            if let Some(ch) = c.unicode_char() {
                full.push(ch);
            }
        }
        char_start.push(full.len());

        for regex in regexes {
            for m in regex.find_iter(&full) {
                let mut min_x = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_y = f32::NEG_INFINITY;
                for (i, c) in chars.iter().enumerate() {
                    let (cs, ce) = (char_start[i], char_start[i + 1]);
                    // Non-empty glyph whose byte span overlaps the match.
                    if ce <= cs || cs >= m.end() || ce <= m.start() {
                        continue;
                    }
                    let Ok(bounds) = c.tight_bounds() else {
                        continue;
                    };
                    min_x = min_x.min(bounds.left().value);
                    max_x = max_x.max(bounds.right().value);
                    min_y = min_y.min(bounds.bottom().value);
                    max_y = max_y.max(bounds.top().value);
                }
                if !min_x.is_finite() {
                    continue;
                }

                redactions.push(RedactionArea {
                    page: index as u32 + 1,
                    x: min_x,
                    y: min_y,
                    width: max_x - min_x,
                    height: max_y - min_y,
                });
            }
        }
    }
    Ok(redactions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, dictionary, Document, Object};
    use std::env::temp_dir;

    #[test]
    fn compiles_named_patterns_and_matches_expected_text() {
        let regexes = compile_patterns(&["email".to_string(), "ssn".to_string()]).unwrap();
        assert!(regexes[0].is_match("contact me at a.b+c@example.co.uk please"));
        assert!(!regexes[0].is_match("not an email at all"));
        assert!(regexes[1].is_match("ssn is 123-45-6789 on file"));
    }

    #[test]
    fn accepts_raw_regex_prefix() {
        let regexes = compile_patterns(&["regex:foo\\d+".to_string()]).unwrap();
        assert!(regexes[0].is_match("foo123"));
    }

    #[test]
    fn text_prefix_matches_literal_and_escapes_special_chars() {
        let regexes = compile_patterns(&["text:a.b (co)".to_string()]).unwrap();
        assert!(regexes[0].is_match("contact a.b (co) today"));
        assert!(!regexes[0].is_match("contact axb (co) today"));
    }

    #[test]
    fn rejects_unknown_pattern_name() {
        assert!(compile_patterns(&["not_a_real_pattern".to_string()]).is_err());
    }

    #[test]
    fn rejects_invalid_regex() {
        assert!(compile_patterns(&["regex:(unclosed".to_string()]).is_err());
    }

    fn one_page_pdf(path: &std::path::Path) {
        let mut doc = Document::with_version("1.7");
        let content_id = doc.add_object(lopdf::Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
        ));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Count" => 1, "Kids" => vec![Object::Reference(page_id)] }),
        );
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", Object::Reference(catalog_id));
        save_document(&mut doc, path).unwrap();
    }

    /// Same limitation as rasterize.rs's tests: no host-platform libpdfium
    /// is vendored (only Android .so's, in jniLibs), so the actual text
    /// search can only be verified on-device. This covers the clean-failure
    /// path when the library isn't found.
    #[test]
    fn fails_cleanly_without_pdfium_lib() {
        let input = temp_dir().join("auto_redact_test_input.pdf");
        let output = temp_dir().join("auto_redact_test_output.pdf");
        one_page_pdf(&input);

        let result = content_auto_redact(
            input.to_string_lossy().into_owned(),
            "/nonexistent/dir".to_string(),
            vec!["email".to_string()],
            output.to_string_lossy().into_owned(),
        );
        assert!(result.is_err());
    }
}
