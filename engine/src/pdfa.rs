//! Convert a PDF to PDF/A (1b/2b/3b) and validate PDF/A conformance (F-092).
//!
//! ponytail: covers the checks that don't need a full ISO 19005 validator
//! (veraPDF-class tooling is far too heavy for this engine): encryption,
//! PDF version, `OutputIntent`/ICC profile, XMP `pdfaid` metadata, and font
//! embedding. It does not reflow content to fix violations (e.g. it can't
//! embed a font that isn't already embedded) - `convert_pdf_to_pdfa` only
//! adds what's structurally missing; `pdfa_validate` reports the rest as
//! errors so the user knows the source PDF needs fixing first.

use crate::icc_srgb::srgb_icc_profile;
use crate::EngineError;
use lopdf::{dictionary, Dictionary, Document, Object, Stream, StringFormat};

/// Result of validating a PDF against a PDF/A standard.
#[derive(Debug, Clone, uniffi::Record)]
pub struct PdfaValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug)]
struct Standard {
    part: &'static str,
    min_version: &'static str,
}

fn standard_info(standard: &str) -> Result<Standard, EngineError> {
    match standard {
        "1b" => Ok(Standard {
            part: "1",
            min_version: "1.4",
        }),
        "2b" => Ok(Standard {
            part: "2",
            min_version: "1.7",
        }),
        "3b" => Ok(Standard {
            part: "3",
            min_version: "1.7",
        }),
        other => Err(EngineError::WriteFailed {
            reason: format!("unsupported PDF/A standard '{other}' (expected 1b, 2b, or 3b)"),
        }),
    }
}

fn catalog_id(doc: &Document) -> Result<lopdf::ObjectId, EngineError> {
    doc.trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| EngineError::WriteFailed {
            reason: "PDF has no /Root catalog".into(),
        })
}

/// Converts the PDF at `input_path` to PDF/A `standard` ("1b"/"2b"/"3b"),
/// writing the result to `output_path`. Sets the PDF version, adds a
/// `GTS_PDFA1` `OutputIntent` (sRGB ICC profile), and adds an XMP metadata
/// stream declaring `pdfaid:part`/`pdfaid:conformance`. Does not embed fonts
/// or otherwise rewrite content - run `pdfa_validate` after to confirm the
/// source PDF didn't have other conformance gaps (e.g. non-embedded fonts).
#[uniffi::export]
pub fn convert_pdf_to_pdfa(
    input_path: String,
    standard: String,
    output_path: String,
) -> Result<(), EngineError> {
    let info = standard_info(&standard)?;
    let mut doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    if doc.is_encrypted() {
        return Err(EngineError::WriteFailed {
            reason: "cannot convert an encrypted PDF to PDF/A; remove the password first".into(),
        });
    }

    doc.version = info.min_version.to_string();
    let root_id = catalog_id(&doc)?;

    let icc_dict = dictionary! { "N" => 3 };
    let icc_id = doc.add_object(Stream::new(icc_dict, srgb_icc_profile()));

    let output_intent_id = doc.add_object(dictionary! {
        "Type" => "OutputIntent",
        "S" => "GTS_PDFA1",
        "OutputConditionIdentifier" => Object::String(b"sRGB IEC61966-2.1".to_vec(), StringFormat::Literal),
        "Info" => Object::String(b"sRGB IEC61966-2.1".to_vec(), StringFormat::Literal),
        "DestOutputProfile" => Object::Reference(icc_id),
    });

    let xmp = build_xmp_packet(info.part);
    let metadata_id = doc.add_object(Stream::new(
        dictionary! { "Type" => "Metadata", "Subtype" => "XML" },
        xmp.into_bytes(),
    ));

    let catalog = doc
        .get_object_mut(root_id)
        .and_then(Object::as_dict_mut)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    catalog.set(
        "OutputIntents",
        Object::Array(vec![Object::Reference(output_intent_id)]),
    );
    catalog.set("Metadata", Object::Reference(metadata_id));

    doc.compress();
    doc.save(&output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

/// Validates the PDF at `input_path` against PDF/A `standard`
/// ("1b"/"2b"/"3b"). Best-effort: checks encryption, PDF version,
/// `OutputIntent`/ICC profile, XMP `pdfaid` metadata, and font embedding.
#[uniffi::export]
pub fn pdfa_validate(input_path: String, standard: String) -> Result<PdfaValidation, EngineError> {
    let info = standard_info(&standard)?;
    let doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    let mut errors = Vec::new();

    if doc.is_encrypted() {
        errors.push("document is encrypted; PDF/A forbids encryption".to_string());
    }

    let actual_version: f32 = doc.version.parse().unwrap_or(0.0);
    let min_version: f32 = info.min_version.parse().unwrap_or(0.0);
    if actual_version < min_version {
        errors.push(format!(
            "PDF version {} is below the {} minimum required by PDF/A-{}",
            doc.version, info.min_version, standard
        ));
    }

    let Ok(root_id) = catalog_id(&doc) else {
        errors.push("PDF has no /Root catalog".to_string());
        return Ok(PdfaValidation {
            valid: false,
            errors,
        });
    };
    let Ok(catalog) = doc.get_dictionary(root_id) else {
        errors.push("PDF /Root catalog is not a dictionary".to_string());
        return Ok(PdfaValidation {
            valid: false,
            errors,
        });
    };

    check_output_intent(&doc, catalog, &mut errors);
    check_xmp_metadata(&doc, catalog, info.part, &mut errors);
    check_fonts_embedded(&doc, &mut errors);

    let valid = errors.is_empty();
    Ok(PdfaValidation { valid, errors })
}

fn check_output_intent(doc: &Document, catalog: &Dictionary, errors: &mut Vec<String>) {
    let intents = catalog
        .get(b"OutputIntents")
        .ok()
        .and_then(|o| o.as_array().ok());
    let Some(intents) = intents else {
        errors.push("missing /OutputIntents (required GTS_PDFA1 ICC output profile)".to_string());
        return;
    };

    let pdfa_intent = intents.iter().find_map(|o| {
        let dict = match o {
            Object::Reference(id) => doc.get_dictionary(*id).ok()?,
            Object::Dictionary(d) => d,
            _ => return None,
        };
        let s = dict.get(b"S").ok()?.as_name_str().ok()?;
        (s == "GTS_PDFA1").then_some(dict)
    });
    let Some(intent) = pdfa_intent else {
        errors.push("no /OutputIntent with /S /GTS_PDFA1 found".to_string());
        return;
    };

    let profile_stream = intent
        .get(b"DestOutputProfile")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .and_then(|id| doc.get_object(id).ok())
        .and_then(|o| o.as_stream().ok());
    match profile_stream.and_then(|s| s.decompressed_content().ok()) {
        Some(bytes) if bytes.len() >= 40 && &bytes[36..40] == b"acsp" => {}
        _ => errors.push(
            "GTS_PDFA1 OutputIntent's /DestOutputProfile is missing or not a valid ICC profile"
                .to_string(),
        ),
    }
}

fn check_xmp_metadata(doc: &Document, catalog: &Dictionary, part: &str, errors: &mut Vec<String>) {
    let metadata = catalog
        .get(b"Metadata")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .and_then(|id| doc.get_object(id).ok())
        .and_then(|o| o.as_stream().ok())
        .and_then(|s| s.decompressed_content().ok())
        .and_then(|bytes| String::from_utf8(bytes).ok());

    let Some(xml) = metadata else {
        errors.push("missing XMP metadata stream declaring pdfaid:part/conformance".to_string());
        return;
    };

    let has_part = xml.contains(&format!("pdfaid:part=\"{part}\""))
        || xml.contains(&format!("<pdfaid:part>{part}</pdfaid:part>"));
    let has_conformance = xml.contains("pdfaid:conformance=\"B\"")
        || xml.contains("<pdfaid:conformance>B</pdfaid:conformance>");
    if !has_part || !has_conformance {
        errors.push(format!(
            "XMP metadata does not declare pdfaid:part=\"{part}\" / pdfaid:conformance=\"B\""
        ));
    }
}

fn check_fonts_embedded(doc: &Document, errors: &mut Vec<String>) {
    let mut missing: Vec<String> = Vec::new();
    for (_, page_id) in doc.get_pages() {
        let Ok(fonts) = doc.get_dictionary(page_id).and_then(|p| {
            let res = match p.get(b"Resources")? {
                Object::Reference(id) => doc.get_dictionary(*id)?,
                Object::Dictionary(d) => d,
                _ => return Err(lopdf::Error::DictKey),
            };
            res.get(b"Font").and_then(|f| match f {
                Object::Reference(id) => doc.get_dictionary(*id),
                Object::Dictionary(d) => Ok(d),
                _ => Err(lopdf::Error::DictKey),
            })
        }) else {
            continue;
        };
        for (_, font_ref) in fonts.iter() {
            let Some(font_dict) = font_ref
                .as_reference()
                .ok()
                .and_then(|id| doc.get_dictionary(id).ok())
            else {
                continue;
            };
            let is_type3 = font_dict
                .get(b"Subtype")
                .and_then(|o| o.as_name_str())
                .map(|s| s == "Type3")
                .unwrap_or(false);
            if is_type3 {
                continue; // Type3 glyphs are self-contained, not "embedded" in the usual sense.
            }
            if !font_is_embedded(doc, font_dict) {
                let name = font_dict
                    .get(b"BaseFont")
                    .and_then(|o| o.as_name_str())
                    .unwrap_or("(unknown)")
                    .to_string();
                if !missing.contains(&name) {
                    missing.push(name);
                }
            }
        }
    }
    for name in missing {
        errors.push(format!(
            "font '{name}' is not embedded (PDF/A requires all fonts embedded)"
        ));
    }
}

fn font_is_embedded(doc: &Document, font_dict: &Dictionary) -> bool {
    let descriptor = font_descriptor(doc, font_dict);
    let Some(descriptor) = descriptor else {
        return false;
    };
    descriptor.has(b"FontFile") || descriptor.has(b"FontFile2") || descriptor.has(b"FontFile3")
}

fn font_descriptor<'a>(doc: &'a Document, font_dict: &'a Dictionary) -> Option<&'a Dictionary> {
    if let Ok(d) = font_dict
        .get(b"FontDescriptor")
        .and_then(|o| o.as_reference())
        .and_then(|id| doc.get_dictionary(id))
    {
        return Some(d);
    }
    // Type0 composite fonts carry the descriptor on their one DescendantFont.
    let descendant = font_dict
        .get(b"DescendantFonts")
        .ok()
        .and_then(|o| o.as_array().ok())
        .and_then(|arr| arr.first())
        .and_then(|o| o.as_reference().ok())
        .and_then(|id| doc.get_dictionary(id).ok())?;
    descendant
        .get(b"FontDescriptor")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .and_then(|id| doc.get_dictionary(id).ok())
}

fn build_xmp_packet(part: &str) -> String {
    format!(
        r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
        xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/"
        xmlns:dc="http://purl.org/dc/elements/1.1/"
        xmlns:pdf="http://ns.adobe.com/pdf/1.3/">
      <pdfaid:part>{part}</pdfaid:part>
      <pdfaid:conformance>B</pdfaid:conformance>
      <dc:format>application/pdf</dc:format>
      <pdf:Producer>Stirling Mobile</pdf:Producer>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, dictionary, Document, Object};

    fn one_page_pdf(path: &str) {
        let mut doc = Document::with_version("1.7");
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
        ));
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => Object::Reference(font_id) },
        });
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => Object::Reference(resources_id),
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
    fn convert_then_validate_reports_valid_except_font_embedding() {
        let dir = std::env::temp_dir();
        let input = dir.join("pdfa_test_in.pdf");
        let output = dir.join("pdfa_test_out.pdf");
        one_page_pdf(input.to_str().unwrap());

        convert_pdf_to_pdfa(
            input.to_str().unwrap().to_string(),
            "1b".to_string(),
            output.to_str().unwrap().to_string(),
        )
        .unwrap();

        let result = pdfa_validate(output.to_str().unwrap().to_string(), "1b".to_string()).unwrap();
        // The test fixture uses a non-embedded standard-14 font, which real
        // PDF/A forbids - that's the one error we expect to remain.
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("not embedded"));
    }

    #[test]
    fn validate_rejects_pdf_with_no_pdfa_markers() {
        let dir = std::env::temp_dir();
        let input = dir.join("pdfa_test_plain.pdf");
        one_page_pdf(input.to_str().unwrap());

        let result = pdfa_validate(input.to_str().unwrap().to_string(), "1b".to_string()).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("OutputIntent")));
        assert!(result.errors.iter().any(|e| e.contains("XMP metadata")));
    }

    #[test]
    fn rejects_unsupported_standard() {
        let err = standard_info("4b").unwrap_err();
        assert!(matches!(err, EngineError::WriteFailed { .. }));
    }
}
