use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::constants;

/// Decoded license payload from the encrypted key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    pub v: u8,
    pub e: String,
    pub t: i64,
    pub d: u32,
    pub n: String,
}

/// License validation result sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatus {
    pub valid: bool,
    pub email: String,
    pub expires_at: i64,
    pub max_discussions: u32,
    pub discussions_used: u32,
    pub error: Option<String>,
}

impl LicenseStatus {
    pub fn invalid(reason: &str) -> Self {
        Self {
            valid: false,
            email: String::new(),
            expires_at: 0,
            max_discussions: 0,
            discussions_used: 0,
            error: Some(reason.to_string()),
        }
    }
}

/// Decode a hex string to bytes.
fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("Odd-length hex string".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

/// Decode and verify a license key string.
///
/// Flow: strip prefix → remove dashes → Base64 decode → AES-GCM decrypt → Ed25519 verify → parse JSON.
pub fn decode_license_key(key: &str) -> Result<LicensePayload, String> {
    // 1. Strip AIRENA- prefix
    let without_prefix = key
        .strip_prefix("AIRENA-")
        .ok_or("Missing AIRENA- prefix")?;

    // 2. Remove segment dashes → Base64 standard decode
    let b64: String = without_prefix.chars().filter(|c| *c != '-').collect();
    let blob = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64)
        .map_err(|e| format!("Base64 decode error: {e}"))?;

    // 3. Minimum size: 12 (nonce) + 16 (GCM tag) + 64 (signature) + 1 (min payload)
    if blob.len() < 93 {
        return Err("Key too short".to_string());
    }

    // 4. Split nonce + ciphertext+tag
    let nonce_bytes = &blob[..12];
    let ciphertext_with_tag = &blob[12..];

    // 5. AES-256-GCM decrypt
    let aes_key_bytes = decode_hex(constants::LICENSE_AES_KEY_HEX)?;
    let cipher = Aes256Gcm::new_from_slice(&aes_key_bytes)
        .map_err(|e| format!("AES key error: {e}"))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext_with_tag)
        .map_err(|_| "Decryption failed (invalid or tampered key)".to_string())?;

    // 6. Split payload + signature (last 64 bytes)
    if plaintext.len() < 65 {
        return Err("Decrypted content too short".to_string());
    }
    let sig_offset = plaintext.len() - 64;
    let payload_bytes = &plaintext[..sig_offset];
    let sig_bytes = &plaintext[sig_offset..];

    // 7. Ed25519 verify
    let pub_key_bytes = decode_hex(constants::LICENSE_ED25519_PUBLIC_KEY_HEX)?;
    let pub_key_array: [u8; 32] = pub_key_bytes
        .try_into()
        .map_err(|_| "Public key must be 32 bytes")?;
    let verifying_key = VerifyingKey::from_bytes(&pub_key_array)
        .map_err(|e| format!("Invalid public key: {e}"))?;
    let signature = Signature::from_bytes(
        sig_bytes
            .try_into()
            .map_err(|_| "Signature must be 64 bytes")?,
    );
    verifying_key
        .verify_strict(payload_bytes, &signature)
        .map_err(|_| "Signature verification failed".to_string())?;

    // 8. Parse JSON payload
    let payload: LicensePayload = serde_json::from_slice(payload_bytes)
        .map_err(|e| format!("Payload parse error: {e}"))?;

    if payload.v != constants::LICENSE_VERSION {
        return Err(format!(
            "Unsupported license version: {} (expected {})",
            payload.v,
            constants::LICENSE_VERSION
        ));
    }

    Ok(payload)
}

/// Compute SHA-256 hex hash of a license key string (used as DB key for quota tracking).
pub fn hash_license_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

/// Full license status check: decode + expiration + clock drift + quota.
pub fn check_license_status(
    key: &str,
    stored_hash: &str,
    disc_count: u32,
    last_check_ts: i64,
) -> LicenseStatus {
    // 1. Decode & verify
    let payload = match decode_license_key(key) {
        Ok(p) => p,
        Err(e) => return LicenseStatus::invalid(&e),
    };

    let now = chrono::Utc::now().timestamp();
    let expires_at = payload.t + payload.d as i64 * 3600;
    let max_discussions = ((constants::LICENSE_DISCUSSIONS_PER_DAY as u64 * payload.d as u64)
        .div_ceil(24)) as u32;

    // 2. Anti-clock manipulation
    if last_check_ts > 0 && now < last_check_ts - constants::LICENSE_CLOCK_TOLERANCE_SECS {
        return LicenseStatus {
            valid: false,
            email: payload.e,
            expires_at,
            max_discussions,
            discussions_used: disc_count,
            error: Some("Clock moved backward".to_string()),
        };
    }

    // 3. Expiration check
    if now > expires_at {
        return LicenseStatus {
            valid: false,
            email: payload.e,
            expires_at,
            max_discussions,
            discussions_used: disc_count,
            error: Some("License expired".to_string()),
        };
    }

    // 4. Counter: reset implicitly if key changed
    let effective_count = if hash_license_key(key) == stored_hash {
        disc_count
    } else {
        0
    };

    // 5. Quota check
    if effective_count >= max_discussions {
        return LicenseStatus {
            valid: false,
            email: payload.e,
            expires_at,
            max_discussions,
            discussions_used: effective_count,
            error: Some("Discussion quota exhausted".to_string()),
        };
    }

    LicenseStatus {
        valid: true,
        email: payload.e,
        expires_at,
        max_discussions,
        discussions_used: effective_count,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_hex() {
        assert_eq!(decode_hex("48656c6c6f").unwrap(), b"Hello");
        assert_eq!(decode_hex("00ff").unwrap(), vec![0x00, 0xff]);
        assert!(decode_hex("0").is_err()); // odd length
        assert!(decode_hex("zz").is_err()); // invalid chars
    }

    #[test]
    fn test_hash_license_key() {
        let h1 = hash_license_key("AIRENA-test1");
        let h2 = hash_license_key("AIRENA-test2");
        assert_eq!(h1.len(), 64); // SHA-256 = 64 hex chars
        assert_ne!(h1, h2);
        // Deterministic
        assert_eq!(h1, hash_license_key("AIRENA-test1"));
    }

    #[test]
    fn test_invalid_key_format() {
        assert!(decode_license_key("").is_err());
        assert!(decode_license_key("INVALID-KEY").is_err());
        assert!(decode_license_key("AIRENA-short").is_err());
    }
}
