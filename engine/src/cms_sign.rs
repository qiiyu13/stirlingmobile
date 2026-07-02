use crate::EngineError;
use const_oid::db::rfc5911::{ID_CONTENT_TYPE, ID_DATA, ID_MESSAGE_DIGEST, ID_SIGNED_DATA};
use const_oid::db::rfc5912::{ID_SHA_256, RSA_ENCRYPTION, SHA_256_WITH_RSA_ENCRYPTION};
use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::{CmsVersion, ContentInfo};
use cms::signed_data::{
    CertificateSet, DigestAlgorithmIdentifiers, EncapsulatedContentInfo, SignedData, SignerIdentifier, SignerInfo,
    SignerInfos,
};
use der::asn1::{OctetString, SetOfVec};
use der::{Any, Decode, Encode};
use rsa::pkcs1v15::SigningKey;
use rsa::RsaPrivateKey;
use sha2::Sha256;
use signature::{SignatureEncoding, Signer};
use spki::AlgorithmIdentifierOwned;
use x509_cert::attr::{Attribute, AttributeValue};
use x509_cert::Certificate;

/// Builds a detached PKCS#7/CMS `SignedData` (`ContentInfo`, DER-encoded)
/// over `content_digest` - the SHA-256 digest of the PDF's signed byte
/// range - using `private_key` and the signer's `cert_der`. Matches the PDF
/// `adbe.pkcs7.detached` convention: signed attributes carry the digest,
/// the actual PDF bytes are never embedded in the CMS blob.
pub fn build_detached_signature(
    cert_der: &[u8],
    private_key: &RsaPrivateKey,
    content_digest: &[u8],
) -> Result<Vec<u8>, EngineError> {
    let cert = Certificate::from_der(cert_der).map_err(|e| EngineError::WriteFailed {
        reason: format!("failed to parse signer certificate: {e}"),
    })?;

    let sha256_alg = AlgorithmIdentifierOwned {
        oid: ID_SHA_256,
        parameters: Some(Any::from(der::asn1::Null)),
    };

    let content_type_attr = Attribute {
        oid: ID_CONTENT_TYPE,
        values: SetOfVec::try_from(vec![AttributeValue::from(
            Any::encode_from(&ID_DATA).map_err(der_err)?,
        )])
        .map_err(der_err)?,
    };
    let message_digest_attr = Attribute {
        oid: ID_MESSAGE_DIGEST,
        values: SetOfVec::try_from(vec![AttributeValue::from(
            Any::encode_from(&OctetString::new(content_digest.to_vec()).map_err(der_err)?).map_err(der_err)?,
        )])
        .map_err(der_err)?,
    };
    let signed_attrs = SetOfVec::try_from(vec![content_type_attr, message_digest_attr]).map_err(der_err)?;
    let signed_attrs_der = signed_attrs.to_der().map_err(der_err)?;

    let signing_key = SigningKey::<Sha256>::new(private_key.clone());
    let signature = signing_key.try_sign(&signed_attrs_der).map_err(|e| EngineError::WriteFailed {
        reason: format!("signing failed: {e}"),
    })?;

    let signer_info = SignerInfo {
        version: CmsVersion::V1,
        sid: SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
            issuer: cert.tbs_certificate.issuer.clone(),
            serial_number: cert.tbs_certificate.serial_number.clone(),
        }),
        digest_alg: sha256_alg.clone(),
        signed_attrs: Some(signed_attrs),
        signature_algorithm: AlgorithmIdentifierOwned {
            oid: SHA_256_WITH_RSA_ENCRYPTION,
            parameters: Some(Any::from(der::asn1::Null)),
        },
        signature: OctetString::new(signature.to_bytes().to_vec()).map_err(der_err)?,
        unsigned_attrs: None,
    };
    let _ = RSA_ENCRYPTION; // signature_algorithm above documents intent; rsaEncryption unused directly

    let mut certificates = CertificateSet(Default::default());
    certificates
        .0
        .insert(CertificateChoices::Certificate(cert))
        .map_err(der_err)?;

    let signed_data = SignedData {
        version: CmsVersion::V1,
        digest_algorithms: DigestAlgorithmIdentifiers::try_from(vec![sha256_alg]).map_err(der_err)?,
        encap_content_info: EncapsulatedContentInfo {
            econtent_type: ID_DATA,
            econtent: None,
        },
        certificates: Some(certificates),
        crls: None,
        signer_infos: SignerInfos(SetOfVec::try_from(vec![signer_info]).map_err(der_err)?),
    };

    let content_info = ContentInfo {
        content_type: ID_SIGNED_DATA,
        content: Any::encode_from(&signed_data).map_err(der_err)?,
    };

    content_info.to_der().map_err(der_err)
}

fn der_err(e: der::Error) -> EngineError {
    EngineError::WriteFailed {
        reason: format!("CMS encoding failed: {e}"),
    }
}
