use crate::cms_sign::build_detached_signature;
use crate::pfx::load_pfx;
use crate::EngineError;
use lopdf::{dictionary, Document, Object, ObjectId, StringFormat};
use secrecy::SecretString;
use sha2::{Digest, Sha256};
use std::io::Cursor;

/// Reserved size (raw bytes, doubled for hex) for the `/Contents` signature
/// placeholder. Comfortably covers RSA-4096 + a multi-cert chain; the real
/// CMS blob is hex-encoded into this space and zero-padded on the right -
/// PDF/CMS readers stop at the DER structure's own encoded length and
/// ignore the padding, same convention Adobe/PDFBox/iText use.
const CONTENTS_PLACEHOLDER_BYTES: usize = 16384;
/// Decimal digit width reserved per `/ByteRange` entry (covers files up to
/// ~9.9GB). Values are right-padded... left-padded with spaces to this
/// width when patched in, since PDF integers can't carry leading zeros but
/// leading whitespace is a harmless separator.
const BYTE_RANGE_DIGIT_WIDTH: usize = 10;
const BYTE_RANGE_SENTINEL: i64 = 1_111_111_111;

/// Adds a detached PKCS#7/CMS digital signature (RSA + SHA-256,
/// `adbe.pkcs7.detached`) to the PDF at `input_path`, using the identity in
/// the PKCS#12 file at `pfx_path`, and writes the result to `output_path`.
/// This is an approval signature (does not set `/DocMDP` permissions); see
/// `certify_pdf` for a certifying signature.
#[uniffi::export]
pub fn sign_pdf(
    input_path: String,
    pfx_path: String,
    pfx_password: String,
    output_path: String,
) -> Result<(), EngineError> {
    let identity = load_pfx(&pfx_path, SecretString::from(pfx_password))?;

    let mut doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;
    add_signature_placeholder(&mut doc)?;

    let mut buffer = Vec::new();
    doc.save_to(&mut Cursor::new(&mut buffer)).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;

    let byte_range_spans = locate_byte_range_placeholder(&buffer)?;
    let contents_span = locate_contents_placeholder(&buffer)?;

    let real_byte_range = [
        0i64,
        contents_span.hex_start as i64,
        contents_span.hex_end as i64,
        (buffer.len() - contents_span.hex_end) as i64,
    ];
    for (span, value) in byte_range_spans.iter().zip(&real_byte_range[1..]) {
        patch_decimal(&mut buffer, span, *value)?;
    }

    let mut hasher = Sha256::new();
    hasher.update(&buffer[..contents_span.hex_start]);
    hasher.update(&buffer[contents_span.hex_end..]);
    let digest = hasher.finalize();

    let cms_der = build_detached_signature(&identity.cert_der, &identity.private_key, &digest)?;
    if cms_der.len() > CONTENTS_PLACEHOLDER_BYTES {
        return Err(EngineError::WriteFailed {
            reason: format!(
                "signature ({} bytes) exceeds reserved placeholder ({CONTENTS_PLACEHOLDER_BYTES} bytes)",
                cms_der.len()
            ),
        });
    }

    let mut hex = String::with_capacity(CONTENTS_PLACEHOLDER_BYTES * 2);
    for byte in &cms_der {
        hex.push_str(&format!("{byte:02X}"));
    }
    hex.push_str(&"00".repeat(CONTENTS_PLACEHOLDER_BYTES - cms_der.len()));
    buffer[contents_span.hex_start..contents_span.hex_end].copy_from_slice(hex.as_bytes());

    std::fs::write(&output_path, &buffer).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

/// Inserts an unsigned `/Sig` dictionary (with placeholder `/Contents` and
/// `/ByteRange`) plus an invisible signature field widget on page 1,
/// wiring it into `/AcroForm` (merging with an existing one if present so
/// pre-existing form fields aren't clobbered).
fn add_signature_placeholder(doc: &mut Document) -> Result<(), EngineError> {
    let page_id = *doc.get_pages().get(&1).ok_or_else(|| EngineError::WriteFailed {
        reason: "PDF has no pages to sign".to_string(),
    })?;

    let sig_id = doc.new_object_id();
    doc.objects.insert(
        sig_id,
        Object::Dictionary(dictionary! {
            "Type" => "Sig",
            "Filter" => "Adobe.PPKLite",
            "SubFilter" => "adbe.pkcs7.detached",
            "ByteRange" => vec![
                0.into(),
                Object::Integer(BYTE_RANGE_SENTINEL),
                Object::Integer(BYTE_RANGE_SENTINEL),
                Object::Integer(BYTE_RANGE_SENTINEL),
            ],
            "Contents" => Object::String(vec![0u8; CONTENTS_PLACEHOLDER_BYTES], StringFormat::Hexadecimal),
        }),
    );

    let widget_id = doc.new_object_id();
    doc.objects.insert(
        widget_id,
        Object::Dictionary(dictionary! {
            "Type" => "Annot",
            "Subtype" => "Widget",
            "FT" => "Sig",
            "Rect" => vec![0.into(), 0.into(), 0.into(), 0.into()],
            "V" => Object::Reference(sig_id),
            "T" => Object::string_literal("Signature1"),
            "P" => Object::Reference(page_id),
            "F" => 132, // Print (bit 3) + Locked (bit 8)
        }),
    );

    let mut annots = doc
        .get_dictionary(page_id)
        .ok()
        .and_then(|d| d.get(b"Annots").ok())
        .and_then(|o| o.as_array().ok())
        .cloned()
        .unwrap_or_default();
    annots.push(Object::Reference(widget_id));
    if let Ok(page_dict) = doc.get_dictionary_mut(page_id) {
        page_dict.set("Annots", annots);
    }

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| EngineError::WriteFailed {
            reason: "PDF has no document catalog".to_string(),
        })?;

    let existing_acroform: Option<ObjectId> = doc
        .get_dictionary(catalog_id)
        .ok()
        .and_then(|d| d.get(b"AcroForm").ok())
        .and_then(|o| o.as_reference().ok());

    if let Some(acroform_id) = existing_acroform {
        let mut fields = doc
            .get_dictionary(acroform_id)
            .ok()
            .and_then(|d| d.get(b"Fields").ok())
            .and_then(|o| o.as_array().ok())
            .cloned()
            .unwrap_or_default();
        fields.push(Object::Reference(widget_id));
        if let Ok(acroform_dict) = doc.get_dictionary_mut(acroform_id) {
            acroform_dict.set("Fields", fields);
            acroform_dict.set("SigFlags", 3);
        }
    } else {
        let acroform_id = doc.add_object(dictionary! {
            "Fields" => vec![Object::Reference(widget_id)],
            "SigFlags" => 3,
        });
        if let Ok(catalog_dict) = doc.get_dictionary_mut(catalog_id) {
            catalog_dict.set("AcroForm", Object::Reference(acroform_id));
        }
    }

    Ok(())
}

struct ByteSpan {
    start: usize,
    end: usize,
}

struct ContentsSpan {
    hex_start: usize,
    hex_end: usize,
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn locate_byte_range_placeholder(buffer: &[u8]) -> Result<[ByteSpan; 3], EngineError> {
    let sentinel = BYTE_RANGE_SENTINEL.to_string();
    debug_assert_eq!(sentinel.len(), BYTE_RANGE_DIGIT_WIDTH);
    let pattern = format!("{sentinel} {sentinel} {sentinel}");
    let start = find(buffer, pattern.as_bytes()).ok_or_else(|| EngineError::WriteFailed {
        reason: "internal error: ByteRange placeholder not found after save".to_string(),
    })?;
    let w = BYTE_RANGE_DIGIT_WIDTH;
    Ok([
        ByteSpan { start, end: start + w },
        ByteSpan { start: start + w + 1, end: start + 2 * w + 1 },
        ByteSpan { start: start + 2 * w + 2, end: start + 3 * w + 2 },
    ])
}

fn locate_contents_placeholder(buffer: &[u8]) -> Result<ContentsSpan, EngineError> {
    let marker = b"/Contents<";
    let marker_pos = find(buffer, marker).ok_or_else(|| EngineError::WriteFailed {
        reason: "internal error: Contents placeholder not found after save".to_string(),
    })?;
    let hex_start = marker_pos + marker.len();
    let hex_end = hex_start + CONTENTS_PLACEHOLDER_BYTES * 2;
    let placeholder_all_zero = buffer
        .get(hex_start..hex_end)
        .map(|s| s.iter().all(|&b| b == b'0'))
        .unwrap_or(false);
    if !placeholder_all_zero || buffer.get(hex_end) != Some(&b'>') {
        return Err(EngineError::WriteFailed {
            reason: "internal error: Contents placeholder shape mismatch after save".to_string(),
        });
    }
    Ok(ContentsSpan { hex_start, hex_end })
}

fn patch_decimal(buffer: &mut [u8], span: &ByteSpan, value: i64) -> Result<(), EngineError> {
    let text = format!("{value:>width$}", width = span.end - span.start);
    if text.len() != span.end - span.start {
        return Err(EngineError::WriteFailed {
            reason: format!("PDF too large to fit in a {}-digit ByteRange field", span.end - span.start),
        });
    }
    buffer[span.start..span.end].copy_from_slice(text.as_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Content;
    use rsa::pkcs8::EncodePrivateKey;
    use rsa::RsaPrivateKey;
    use std::env::temp_dir;
    use std::process::Command;

    /// Self-signed RSA-2048 identity, packaged as a real PKCS#12 file via
    /// the `p12` crate - an independently-implemented PFX writer, not our
    /// own code, so this fixture doesn't just test that we can read back
    /// what we wrote.
    fn make_test_pfx() -> (Vec<u8>, &'static str) {
        let mut rng = rand::rngs::OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let key_der = private_key.to_pkcs8_der().unwrap().as_bytes().to_vec();

        let pkcs8 = rustls_pki_types::PrivatePkcs8KeyDer::from(key_der.clone());
        let key_pair = rcgen::KeyPair::from_pkcs8_der_and_sign_algo(&pkcs8, &rcgen::PKCS_RSA_SHA256).unwrap();
        let params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
        let cert = params.self_signed(&key_pair).unwrap();
        let cert_der = cert.der().to_vec();

        let password = "testpass";
        let pfx = p12::PFX::new(&cert_der, &key_der, None, password, "stirling-mobile-test").unwrap();
        (pfx.to_der(), password)
    }

    /// Same identity, but bundled with the *system* `openssl` CLI using
    /// its modern default (PBES2/AES-256/PBKDF2-SHA256, not the legacy
    /// RC2/3DES the `p12` crate itself writes) - the format real CAs,
    /// current OpenSSL, and Windows cert export now produce. Returns
    /// `None` if `openssl` isn't installed, so this test skips gracefully
    /// rather than failing on machines without it.
    fn make_test_pfx_pbes2() -> Option<(Vec<u8>, &'static str)> {
        if !Command::new("openssl").arg("version").status().map(|s| s.success()).unwrap_or(false) {
            return None;
        }
        let dir = temp_dir();
        let key_pem = dir.join("pbes2_fixture_key.pem");
        let cert_pem = dir.join("pbes2_fixture_cert.pem");
        let pfx_path = dir.join("pbes2_fixture.pfx");
        let password = "testpass123";

        let ok = Command::new("openssl")
            .args([
                "req", "-x509", "-newkey", "rsa:2048",
                "-keyout", key_pem.to_str()?,
                "-out", cert_pem.to_str()?,
                "-days", "365", "-nodes", "-subj", "/CN=PBES2 Test Signer",
            ])
            .status()
            .ok()?
            .success();
        if !ok {
            return None;
        }
        let ok = Command::new("openssl")
            .args([
                "pkcs12", "-export",
                "-out", pfx_path.to_str()?,
                "-inkey", key_pem.to_str()?,
                "-in", cert_pem.to_str()?,
                "-passout", &format!("pass:{password}"),
            ])
            .status()
            .ok()?
            .success();
        if !ok {
            return None;
        }
        Some((std::fs::read(&pfx_path).ok()?, password))
    }

    fn one_page_pdf(path: &std::path::Path) {
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
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    /// Independent oracle: verifies with poppler's `pdfsig`, not our own
    /// signing code, so a bug in our `/ByteRange`/CMS construction that
    /// "looks right" to us still gets caught.
    fn pdfsig_says_valid(path: &std::path::Path) -> bool {
        let output = Command::new("pdfsig").arg(path).output().expect("pdfsig must be installed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.contains("Signature Validation: Signature is Valid")
    }

    #[test]
    fn signs_pdf_and_pdfsig_verifies_it() {
        let dir = temp_dir();
        let input = dir.join("sign_test_input.pdf");
        let pfx = dir.join("sign_test_identity.pfx");
        let output = dir.join("sign_test_output.pdf");
        one_page_pdf(&input);
        let (pfx_bytes, password) = make_test_pfx();
        std::fs::write(&pfx, &pfx_bytes).unwrap();

        sign_pdf(
            input.to_string_lossy().into_owned(),
            pfx.to_string_lossy().into_owned(),
            password.to_string(),
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        assert!(pdfsig_says_valid(&output), "pdfsig did not report a valid signature");
    }

    #[test]
    fn signs_pdf_with_pbes2_pfx() {
        let Some((pfx_bytes, password)) = make_test_pfx_pbes2() else {
            eprintln!("openssl not available, skipping PBES2 PFX test");
            return;
        };
        let dir = temp_dir();
        let input = dir.join("sign_test_pbes2_input.pdf");
        let pfx = dir.join("sign_test_pbes2_identity.pfx");
        let output = dir.join("sign_test_pbes2_output.pdf");
        one_page_pdf(&input);
        std::fs::write(&pfx, &pfx_bytes).unwrap();

        sign_pdf(
            input.to_string_lossy().into_owned(),
            pfx.to_string_lossy().into_owned(),
            password.to_string(),
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        assert!(pdfsig_says_valid(&output), "pdfsig did not report a valid signature for a PBES2/AES PFX");
    }

    #[test]
    fn tampering_after_signing_breaks_verification() {
        let dir = temp_dir();
        let input = dir.join("sign_test_tamper_input.pdf");
        let pfx = dir.join("sign_test_tamper_identity.pfx");
        let output = dir.join("sign_test_tamper_output.pdf");
        one_page_pdf(&input);
        let (pfx_bytes, password) = make_test_pfx();
        std::fs::write(&pfx, &pfx_bytes).unwrap();

        sign_pdf(
            input.to_string_lossy().into_owned(),
            pfx.to_string_lossy().into_owned(),
            password.to_string(),
            output.to_string_lossy().into_owned(),
        )
        .unwrap();
        assert!(pdfsig_says_valid(&output));

        // Flip a byte inside the signed content stream (well outside the
        // /Contents hex blob) - a correct /ByteRange must cover this, so
        // verification must now fail.
        let mut bytes = std::fs::read(&output).unwrap();
        let marker = b"endstream";
        let pos = bytes.windows(marker.len()).position(|w| w == marker).unwrap();
        bytes[pos - 1] ^= 0xFF;
        std::fs::write(&output, &bytes).unwrap();

        assert!(!pdfsig_says_valid(&output), "tampering after signing should invalidate the signature");
    }

    #[test]
    fn rejects_wrong_password() {
        let dir = temp_dir();
        let input = dir.join("sign_test_badpw_input.pdf");
        let pfx = dir.join("sign_test_badpw_identity.pfx");
        let output = dir.join("sign_test_badpw_output.pdf");
        one_page_pdf(&input);
        let (pfx_bytes, _password) = make_test_pfx();
        std::fs::write(&pfx, &pfx_bytes).unwrap();

        let result = sign_pdf(
            input.to_string_lossy().into_owned(),
            pfx.to_string_lossy().into_owned(),
            "wrong-password".to_string(),
            output.to_string_lossy().into_owned(),
        );
        assert!(result.is_err());
    }
}
