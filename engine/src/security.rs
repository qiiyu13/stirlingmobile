use crate::content_util::save_document;
use crate::EngineError;
use aes::Aes128;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use lopdf::{dictionary, Document, Object, ObjectId, StringFormat};
use md5::{Digest, Md5};

/// Standard security handler, revision 4, AES-128 (`/CFM /AESV2`).
/// lopdf can only *decrypt* RC4-encrypted PDFs (V1/V2), so both directions
/// here are implemented against the spec (ISO 32000-1 §7.6) directly rather
/// than reusing lopdf's encryption module.
const PAD_BYTES: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];
const KEY_LEN: usize = 16; // AES-128
/// "Allow everything" permissions bitfield: reserved bits 1-2 clear, all
/// defined permission bits set. Matches the P value used by most PDF
/// tools when the caller doesn't need granular restrictions.
const FULL_PERMISSIONS: i32 = -4;

/// Adds standard-security-handler AES-128 password protection to the PDF at
/// `input_path`. If `owner_password` is empty, it defaults to
/// `user_password` (so either unlocks the file identically - the common
/// case when the caller only wants "require a password to open").
#[uniffi::export]
pub fn add_password(
    input_path: String,
    user_password: String,
    owner_password: String,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;
    if doc.is_encrypted() {
        return Err(EngineError::WriteFailed {
            reason: "document is already password protected; remove the existing password first"
                .into(),
        });
    }

    let owner_password = if owner_password.is_empty() {
        user_password.clone()
    } else {
        owner_password
    };

    let file_id = random_bytes(16);
    doc.trailer.set(
        "ID",
        Object::Array(vec![
            Object::String(file_id.clone(), StringFormat::Hexadecimal),
            Object::String(random_bytes(16), StringFormat::Hexadecimal),
        ]),
    );

    let owner_hash = compute_owner_hash(owner_password.as_bytes(), user_password.as_bytes());
    let base_key = compute_base_key(
        user_password.as_bytes(),
        &owner_hash,
        FULL_PERMISSIONS,
        &file_id,
    );
    let user_hash = compute_user_hash(&base_key, &file_id);

    let ids: Vec<ObjectId> = doc.objects.keys().copied().collect();
    for id in ids {
        let object_key = derive_object_key(&base_key, id);
        if let Some(obj) = doc.objects.get_mut(&id) {
            encrypt_strings_and_streams(obj, &object_key);
        }
    }

    let encrypt_dict = dictionary! {
        "Filter" => "Standard",
        "V" => 4,
        "R" => 4,
        "Length" => 128,
        "CF" => dictionary! {
            "StdCF" => dictionary! {
                "AuthEvent" => "DocOpen",
                "CFM" => "AESV2",
                "Length" => 16,
            },
        },
        "StmF" => "StdCF",
        "StrF" => "StdCF",
        "O" => Object::String(owner_hash.to_vec(), StringFormat::Hexadecimal),
        "U" => Object::String(user_hash.to_vec(), StringFormat::Hexadecimal),
        "P" => FULL_PERMISSIONS as i64,
        "EncryptMetadata" => true,
    };
    let encrypt_id = doc.add_object(Object::Dictionary(encrypt_dict));
    doc.trailer.set("Encrypt", Object::Reference(encrypt_id));

    save_document(&mut doc, &output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

/// Removes password protection from the PDF at `input_path`, given the
/// correct password, and writes the plain PDF to `output_path`. Only the
/// user password is checked (matches `add_password`'s default of owner ==
/// user when no separate owner password is set).
#[uniffi::export]
pub fn remove_password(
    input_path: String,
    password: String,
    output_path: String,
) -> Result<(), EngineError> {
    let mut doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path,
        reason: e.to_string(),
    })?;

    if !doc.is_encrypted() {
        save_document(&mut doc, &output_path)
            .map_err(|e| EngineError::WriteFailed {
                reason: e.to_string(),
            })?;
        return Ok(());
    }

    let encrypt_dict = doc
        .get_encrypted()
        .map_err(|_| EngineError::ReadFailed {
            path: String::new(),
            reason: "missing /Encrypt dictionary".into(),
        })?
        .clone();

    let owner_hash = encrypt_dict
        .get(b"O")
        .and_then(Object::as_str)
        .map_err(|_| EngineError::ReadFailed {
            path: String::new(),
            reason: "missing /O entry".into(),
        })?
        .to_vec();
    let expected_user_hash = encrypt_dict
        .get(b"U")
        .and_then(Object::as_str)
        .map_err(|_| EngineError::ReadFailed {
            path: String::new(),
            reason: "missing /U entry".into(),
        })?
        .to_vec();
    let permissions = encrypt_dict
        .get(b"P")
        .and_then(Object::as_i64)
        .map_err(|_| EngineError::ReadFailed {
            path: String::new(),
            reason: "missing /P entry".into(),
        })? as i32;
    let file_id = doc
        .trailer
        .get(b"ID")
        .and_then(Object::as_array)
        .ok()
        .and_then(|arr| arr.first())
        .and_then(|o| Object::as_str(o).ok())
        .map(|s| s.to_vec())
        .ok_or_else(|| EngineError::ReadFailed {
            path: String::new(),
            reason: "missing /ID entry".into(),
        })?;

    let base_key = compute_base_key(password.as_bytes(), &owner_hash, permissions, &file_id);
    let actual_user_hash = compute_user_hash(&base_key, &file_id);
    if actual_user_hash[..16] != expected_user_hash[..16] {
        return Err(EngineError::ReadFailed {
            path: String::new(),
            reason: "incorrect password".into(),
        });
    }

    let ids: Vec<ObjectId> = doc.objects.keys().copied().collect();
    for id in ids {
        let object_key = derive_object_key(&base_key, id);
        if let Some(obj) = doc.objects.get_mut(&id) {
            decrypt_strings_and_streams(obj, &object_key);
        }
    }

    doc.trailer.remove(b"Encrypt");
    doc.trailer.remove(b"ID");
    save_document(&mut doc, &output_path)
        .map_err(|e| EngineError::WriteFailed {
            reason: e.to_string(),
        })?;
    Ok(())
}

fn random_bytes(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    getrandom::getrandom(&mut buf).expect("OS RNG unavailable");
    buf
}

fn padded_password(password: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let n = password.len().min(32);
    out[..n].copy_from_slice(&password[..n]);
    out[n..].copy_from_slice(&PAD_BYTES[..32 - n]);
    out
}

/// Algorithm 3 (ISO 32000-1 §7.6.3.4): computes the /O value. Always RC4,
/// even under an AES crypt filter - only per-object content encryption
/// (Algorithm 1) differs between RC4 and AESV2.
fn compute_owner_hash(owner_password: &[u8], user_password: &[u8]) -> [u8; 32] {
    let mut digest = Md5::digest(padded_password(owner_password)).to_vec();
    for _ in 0..50 {
        digest = Md5::digest(&digest[..KEY_LEN]).to_vec();
    }
    let rc4_key = &digest[..KEY_LEN];

    let mut encrypted = rc4(rc4_key, &padded_password(user_password));
    for i in 1..=19u8 {
        let round_key: Vec<u8> = rc4_key.iter().map(|b| b ^ i).collect();
        encrypted = rc4(&round_key, &encrypted);
    }
    encrypted.try_into().expect("RC4 preserves length")
}

/// Algorithm 2 (ISO 32000-1 §7.6.3.3): computes the base file encryption key.
fn compute_base_key(
    user_password: &[u8],
    owner_hash: &[u8],
    permissions: i32,
    file_id: &[u8],
) -> Vec<u8> {
    let mut key = Vec::with_capacity(32 + 32 + 4 + file_id.len());
    key.extend_from_slice(&padded_password(user_password));
    key.extend_from_slice(owner_hash);
    key.extend_from_slice(&(permissions as u32).to_le_bytes());
    key.extend_from_slice(file_id);
    // R4 always encrypts metadata in this implementation, so no +0xFFFFFFFF suffix.

    let mut digest = Md5::digest(&key).to_vec();
    for _ in 0..50 {
        digest = Md5::digest(&digest[..KEY_LEN]).to_vec();
    }
    digest.truncate(KEY_LEN);
    digest
}

/// Algorithm 5 (ISO 32000-1 §7.6.3.4, revision 3/4): computes the /U value.
fn compute_user_hash(base_key: &[u8], file_id: &[u8]) -> [u8; 32] {
    let mut hasher = Md5::new();
    hasher.update(PAD_BYTES);
    hasher.update(file_id);
    let digest = hasher.finalize();

    let mut encrypted = rc4(base_key, &digest);
    for i in 1..=19u8 {
        let round_key: Vec<u8> = base_key.iter().map(|b| b ^ i).collect();
        encrypted = rc4(&round_key, &encrypted);
    }
    encrypted.extend_from_slice(&PAD_BYTES[..16]);
    encrypted.try_into().expect("16 + 16 == 32")
}

/// Algorithm 1 (ISO 32000-1 §7.6.2), AES branch: per-object key = first
/// min(base_len+5, 16) bytes of MD5(base_key || obj_num[3] || gen_num[2] || "sAlT").
fn derive_object_key(base_key: &[u8], id: ObjectId) -> Vec<u8> {
    let mut input = base_key.to_vec();
    input.extend_from_slice(&id.0.to_le_bytes()[..3]);
    input.extend_from_slice(&id.1.to_le_bytes()[..2]);
    input.extend_from_slice(b"sAlT");
    let digest = Md5::digest(&input);
    let len = (base_key.len() + 5).min(16);
    digest[..len].to_vec()
}

fn aes_encrypt(key: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let iv = random_bytes(16);
    let mut buf = vec![0u8; plaintext.len() + 16]; // room for PKCS7 padding block
    buf[..plaintext.len()].copy_from_slice(plaintext);
    let ciphertext = cbc::Encryptor::<Aes128>::new(key.into(), iv.as_slice().into())
        .encrypt_padded_mut::<Pkcs7>(&mut buf, plaintext.len())
        .expect("buffer sized with padding headroom");
    let mut out = iv;
    out.extend_from_slice(ciphertext);
    out
}

fn aes_decrypt(key: &[u8], iv_and_ciphertext: &[u8]) -> Option<Vec<u8>> {
    if iv_and_ciphertext.len() < 16 {
        return None;
    }
    let (iv, ciphertext) = iv_and_ciphertext.split_at(16);
    let mut buf = ciphertext.to_vec();
    cbc::Decryptor::<Aes128>::new(key.into(), iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .ok()
        .map(|plaintext| plaintext.to_vec())
}

fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut s: [u8; 256] = std::array::from_fn(|i| i as u8);
    let mut j = 0u8;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }
    let mut out = Vec::with_capacity(data.len());
    let (mut i, mut j) = (0u8, 0u8);
    for &byte in data {
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[i as usize]);
        s.swap(i as usize, j as usize);
        let k = s[(s[i as usize].wrapping_add(s[j as usize])) as usize];
        out.push(byte ^ k);
    }
    out
}

fn encrypt_strings_and_streams(obj: &mut Object, key: &[u8]) {
    match obj {
        Object::String(bytes, _) => *bytes = aes_encrypt(key, bytes),
        Object::Stream(stream) => {
            let encrypted = aes_encrypt(key, &stream.content);
            stream.set_content(encrypted);
            for value in stream.dict.iter_mut().map(|(_, v)| v) {
                encrypt_strings_and_streams(value, key);
            }
        }
        Object::Array(items) => {
            for item in items {
                encrypt_strings_and_streams(item, key);
            }
        }
        Object::Dictionary(dict) => {
            for value in dict.iter_mut().map(|(_, v)| v) {
                encrypt_strings_and_streams(value, key);
            }
        }
        _ => {}
    }
}

fn decrypt_strings_and_streams(obj: &mut Object, key: &[u8]) {
    match obj {
        Object::String(bytes, _) => {
            if let Some(plain) = aes_decrypt(key, bytes) {
                *bytes = plain;
            }
        }
        Object::Stream(stream) => {
            if let Some(plain) = aes_decrypt(key, &stream.content) {
                stream.set_content(plain);
            }
            for value in stream.dict.iter_mut().map(|(_, v)| v) {
                decrypt_strings_and_streams(value, key);
            }
        }
        Object::Array(items) => {
            for item in items {
                decrypt_strings_and_streams(item, key);
            }
        }
        Object::Dictionary(dict) => {
            for value in dict.iter_mut().map(|(_, v)| v) {
                decrypt_strings_and_streams(value, key);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Content;
    use std::env::temp_dir;

    fn one_page_pdf_with_text(path: &std::path::Path, text: &str) {
        let mut doc = Document::with_version("1.7");
        let content_id = doc.add_object(lopdf::Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
        ));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
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
        // A string object so encryption round-trip has something to check.
        let string_id = doc.new_object_id();
        doc.objects.insert(
            string_id,
            Object::String(text.as_bytes().to_vec(), StringFormat::Literal),
        );
        save_document(&mut doc, path).unwrap();
    }

    #[test]
    fn rc4_is_involution_with_same_key() {
        let key = b"secret";
        let plaintext = b"hello world, this is a test message";
        let encrypted = rc4(key, plaintext);
        let decrypted = rc4(key, &encrypted);
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_round_trips() {
        let key = vec![0x42u8; 16];
        let plaintext = b"some pdf stream content, longer than one AES block for sure";
        let encrypted = aes_encrypt(&key, plaintext);
        let decrypted = aes_decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn add_then_remove_password_round_trips() {
        let input = temp_dir().join("security_test_input.pdf");
        let protected = temp_dir().join("security_test_protected.pdf");
        let recovered = temp_dir().join("security_test_recovered.pdf");
        one_page_pdf_with_text(&input, "a marker string to verify decryption");

        add_password(
            input.to_string_lossy().into_owned(),
            "hunter2".to_string(),
            String::new(),
            protected.to_string_lossy().into_owned(),
        )
        .unwrap();

        let protected_doc = Document::load(&protected).unwrap();
        assert!(protected_doc.is_encrypted());

        // Wrong password must be rejected.
        let wrong = remove_password(
            protected.to_string_lossy().into_owned(),
            "wrong".to_string(),
            recovered.to_string_lossy().into_owned(),
        );
        assert!(wrong.is_err());

        remove_password(
            protected.to_string_lossy().into_owned(),
            "hunter2".to_string(),
            recovered.to_string_lossy().into_owned(),
        )
        .unwrap();

        let recovered_doc = Document::load(&recovered).unwrap();
        assert!(!recovered_doc.is_encrypted());
        assert_eq!(recovered_doc.get_pages().len(), 1);

        let found_marker = recovered_doc.objects.values().any(|obj| {
            matches!(obj, Object::String(bytes, _) if bytes == b"a marker string to verify decryption")
        });
        assert!(
            found_marker,
            "decrypted string content should match the original plaintext"
        );
    }
}
