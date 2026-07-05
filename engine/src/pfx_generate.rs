use crate::content_util::save_document;
use crate::EngineError;
use rsa::pkcs8::EncodePrivateKey;
use rsa::RsaPrivateKey;
use secrecy::{ExposeSecret, SecretString};

/// Generates a fresh self-signed RSA-2048 identity and bundles it as a
/// PKCS#12 (.pfx) file at `output_path`, protected by `password`. Since
/// this app can't act as a trusted CA, readers will always show "issuer
/// unknown" for the result - fine for personal/internal signing (the same
/// limitation any self-signed cert has), not a substitute for a CA-issued
/// certificate where cross-organization trust matters.
#[uniffi::export]
pub fn generate_self_signed_pfx(
    common_name: String,
    password: String,
    output_path: String,
) -> Result<(), EngineError> {
    let password = SecretString::from(password);

    let mut rng = rand::rngs::OsRng;
    let private_key = RsaPrivateKey::new(&mut rng, 2048).map_err(|e| EngineError::WriteFailed {
        reason: format!("key generation failed: {e}"),
    })?;
    let key_der = private_key
        .to_pkcs8_der()
        .map_err(|e| EngineError::WriteFailed {
            reason: format!("key encoding failed: {e}"),
        })?
        .as_bytes()
        .to_vec();

    let pkcs8 = rustls_pki_types::PrivatePkcs8KeyDer::from(key_der.clone());
    let key_pair = rcgen::KeyPair::from_pkcs8_der_and_sign_algo(&pkcs8, &rcgen::PKCS_RSA_SHA256)
        .map_err(|e| EngineError::WriteFailed {
            reason: format!("key conversion failed: {e}"),
        })?;
    let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(|e| {
        EngineError::WriteFailed {
            reason: format!("certificate parameters failed: {e}"),
        }
    })?;
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, common_name);
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| EngineError::WriteFailed {
            reason: format!("certificate generation failed: {e}"),
        })?;
    let cert_der = cert.der().to_vec();

    let pfx = p12::PFX::new(
        &cert_der,
        &key_der,
        None,
        password.expose_secret(),
        "stirling-mobile",
    )
    .ok_or_else(|| EngineError::WriteFailed {
        reason: "PFX packaging failed".to_string(),
    })?;

    std::fs::write(&output_path, pfx.to_der()).map_err(|e| EngineError::WriteFailed {
        reason: e.to_string(),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sign::sign_pdf;
    use lopdf::{content::Content, dictionary, Document, Object};
    use std::env::temp_dir;
    use std::process::Command;

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
        save_document(&mut doc, path).unwrap();
    }

    /// End-to-end: generate a PFX with our own tool, then feed it straight
    /// into `sign_pdf` and verify with `pdfsig` - proves the generated PFX
    /// is actually usable for real signing, not just structurally present.
    #[test]
    fn generated_pfx_can_sign_and_verifies() {
        let dir = temp_dir();
        let pfx = dir.join("generate_test_identity.pfx");
        let input = dir.join("generate_test_input.pdf");
        let output = dir.join("generate_test_output.pdf");
        one_page_pdf(&input);

        generate_self_signed_pfx(
            "Stirling Mobile Generated Test".to_string(),
            "genpass123".to_string(),
            pfx.to_string_lossy().into_owned(),
        )
        .unwrap();

        sign_pdf(
            input.to_string_lossy().into_owned(),
            pfx.to_string_lossy().into_owned(),
            "genpass123".to_string(),
            output.to_string_lossy().into_owned(),
        )
        .unwrap();

        let result = Command::new("pdfsig").arg(&output).output().unwrap();
        let stdout = String::from_utf8_lossy(&result.stdout);
        assert!(stdout.contains("Signature Validation: Signature is Valid"));
        assert!(stdout.contains("Stirling Mobile Generated Test"));
    }
}
