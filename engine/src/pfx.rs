use crate::EngineError;
use const_oid::ObjectIdentifier;
use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;
use secrecy::{ExposeSecret, SecretString};

/// Signer identity extracted from a PKCS#12 (.pfx/.p12) file: the leaf
/// certificate (DER) and its matching RSA private key. The PFX password is
/// only ever held as `SecretString` and never persisted.
pub struct SigningIdentity {
    pub cert_der: Vec<u8>,
    pub private_key: RsaPrivateKey,
}

pub fn load_pfx(pfx_path: &str, password: SecretString) -> Result<SigningIdentity, EngineError> {
    let bytes = std::fs::read(pfx_path).map_err(|e| EngineError::ReadFailed {
        path: pfx_path.to_string(),
        reason: e.to_string(),
    })?;
    let auth_safe = parse_auth_safe(&bytes).ok_or_else(|| EngineError::ReadFailed {
        path: pfx_path.to_string(),
        reason: "not a valid PFX/PKCS#12 file".to_string(),
    })?;

    let password = password.expose_secret();
    let bags = safe_bags(&auth_safe, password).ok_or_else(|| EngineError::ReadFailed {
        path: pfx_path.to_string(),
        reason: "incorrect PFX password, or an unsupported encryption scheme".to_string(),
    })?;

    let cert_der = bags
        .iter()
        .find_map(|b| b.bag.get_x509_cert())
        .ok_or_else(|| EngineError::ReadFailed {
            path: pfx_path.to_string(),
            reason: "PFX contains no certificate".to_string(),
        })?;

    let key_der = bags
        .iter()
        .find_map(|b| match &b.bag {
            p12::SafeBagKind::Pkcs8ShroudedKeyBag(epk) => {
                decrypt_algorithm(&epk.encryption_algorithm, &epk.encrypted_data, password)
            }
            _ => None,
        })
        .ok_or_else(|| EngineError::ReadFailed {
            path: pfx_path.to_string(),
            reason: "PFX contains no private key (or it uses an encryption scheme we don't support - only RSA/PKCS#8 keys under legacy RC2/3DES or PBES2/AES are supported)".to_string(),
        })?;

    let private_key = RsaPrivateKey::from_pkcs8_der(&key_der).map_err(|e| EngineError::ReadFailed {
        path: pfx_path.to_string(),
        reason: format!("unsupported private key (RSA/PKCS#8 only): {e}"),
    })?;

    Ok(SigningIdentity { cert_der, private_key })
}

/// Parses just the outer `PFX ::= SEQUENCE { version, authSafe, macData
/// OPTIONAL }` shape and returns `authSafe`. Deliberately does not use
/// `p12::PFX::parse`: its `MacData::parse` hard-codes
/// `debug_assert_eq!(digest_algorithm, Sha1)` and panics on the SHA-256 MAC
/// that OpenSSL 3.x/Windows now produce by default. We don't need the MAC
/// for security here anyway - a wrong password just fails to produce
/// parseable ASN.1 downstream, which we already surface as an error.
fn parse_auth_safe(bytes: &[u8]) -> Option<p12::ContentInfo> {
    yasna::parse_der(bytes, |r| {
        r.read_sequence(|r| {
            let _version = r.next().read_u8()?;
            let auth_safe = p12::ContentInfo::parse(r.next())?;
            let _mac_data = r.read_optional(|r| r.read_der());
            Ok(auth_safe)
        })
    })
    .ok()
}

/// Walks the PFX's `AuthenticatedSafe` (a nested `ContentInfo`, each layer
/// either a legacy PKCS#12 PBE (RC2-40/3DES, `p12`'s native support) or a
/// PBES2 (PBKDF2 + AES, via `pkcs5` - the modern default for OpenSSL 3.x,
/// Windows cert export, and most CAs) blob, and returns every `SafeBag`
/// found inside. `p12`'s own `PFX::bags` only understands the legacy
/// scheme, so this reimplements that walk with both.
fn safe_bags(auth_safe: &p12::ContentInfo, password: &str) -> Option<Vec<p12::SafeBag>> {
    let top_level = decrypt_content_info(auth_safe, password)?;
    let contents: Vec<p12::ContentInfo> = yasna::parse_der(&top_level, |r| r.collect_sequence_of(p12::ContentInfo::parse)).ok()?;

    let mut bags = Vec::new();
    for content in &contents {
        let data = decrypt_content_info(content, password)?;
        let safe_bags: Vec<p12::SafeBag> = yasna::parse_der(&data, |r| r.collect_sequence_of(p12::SafeBag::parse)).ok()?;
        bags.extend(safe_bags);
    }
    Some(bags)
}

fn decrypt_content_info(content: &p12::ContentInfo, password: &str) -> Option<Vec<u8>> {
    match content {
        p12::ContentInfo::Data(data) => Some(data.clone()),
        p12::ContentInfo::EncryptedData(encrypted) => decrypt_algorithm(
            &encrypted.encrypted_content_info.content_encryption_algorithm,
            &encrypted.encrypted_content_info.encrypted_content,
            password,
        ),
        p12::ContentInfo::OtherContext(_) => None,
    }
}

/// Legacy PKCS#12 PBE uses the RFC 7292 Appendix B KDF over a BMP
/// (UTF-16BE, NUL-terminated) password - `p12`'s native support, kept
/// as-is. PBES2 (RFC 8018) uses PBKDF2 over the raw UTF-8 password bytes
/// instead; `p12` doesn't recognize the OID at all (falls into `OtherAlg`),
/// so that path is decrypted here via `pkcs5`.
fn decrypt_algorithm(alg: &p12::AlgorithmIdentifier, ciphertext: &[u8], password: &str) -> Option<Vec<u8>> {
    match alg {
        p12::AlgorithmIdentifier::PbewithSHAAnd40BitRC2CBC(_) | p12::AlgorithmIdentifier::PbeWithSHAAnd3KeyTripleDESCBC(_) => {
            alg.decrypt_pbe(ciphertext, &bmp_string(password))
        }
        p12::AlgorithmIdentifier::OtherAlg(other) => decrypt_pbes2(other, ciphertext, password.as_bytes()),
        p12::AlgorithmIdentifier::Sha1 => None,
    }
}

fn decrypt_pbes2(other: &p12::OtherAlgorithmIdentifier, ciphertext: &[u8], password_raw: &[u8]) -> Option<Vec<u8>> {
    use der::Decode;
    let params = other.params.as_ref()?;
    let alg_der = algorithm_identifier_der(other.algorithm_type.components(), params)?;
    let scheme = pkcs5::EncryptionScheme::from_der(&alg_der).ok()?;
    scheme.decrypt(password_raw, ciphertext).ok()
}

/// Rebuilds a DER `AlgorithmIdentifier SEQUENCE { OID, params }` from the
/// pieces `p12` gives us (it parses the OID but only keeps the params as
/// raw bytes) so `pkcs5` can decode it as its own `AlgorithmIdentifierRef`.
fn algorithm_identifier_der(oid_arcs: &[u64], params_der: &[u8]) -> Option<Vec<u8>> {
    use der::Encode;
    let arcs: Option<Vec<u32>> = oid_arcs.iter().map(|&a| u32::try_from(a).ok()).collect();
    let oid = ObjectIdentifier::from_arcs(arcs?).ok()?;
    let oid_der = oid.to_der().ok()?;

    let mut inner = Vec::with_capacity(oid_der.len() + params_der.len());
    inner.extend_from_slice(&oid_der);
    inner.extend_from_slice(params_der);

    let mut out = Vec::with_capacity(inner.len() + 4);
    out.push(0x30); // SEQUENCE
    if inner.len() < 0x80 {
        out.push(inner.len() as u8);
    } else {
        let len_bytes: Vec<u8> = inner.len().to_be_bytes().into_iter().skip_while(|&b| b == 0).collect();
        out.push(0x80 | len_bytes.len() as u8);
        out.extend(&len_bytes);
    }
    out.extend(inner);
    Some(out)
}

fn bmp_string(s: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(s.len() * 2 + 2);
    for unit in s.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    bytes.extend_from_slice(&[0, 0]);
    bytes
}
