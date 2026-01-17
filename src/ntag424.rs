//! NTAG424 DNA SUN (Secure Unique NFC) message parsing and verification.
//!
//! The NTAG424 DNA chip generates URLs with encrypted and signed data to prevent
//! replay attacks. This module handles:
//! - Decrypting the `picc_data` parameter using AES-128-CBC with the k1 key
//! - Verifying the CMAC signature using the k2 key
//! - Checking the counter for replay protection

use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, KeyIvInit};
use cmac::{Cmac, Mac};
use thiserror::Error;

use crate::db::Database;
use crate::models::{Location, NfcCard};

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

/// Errors that can occur during SUN message verification
#[derive(Debug, Error)]
pub enum SunError {
    #[error("Invalid picc_data format: {0}")]
    InvalidPiccData(String),

    #[error("Invalid CMAC format: {0}")]
    InvalidCmac(String),

    #[error("CMAC verification failed")]
    CmacMismatch,

    #[error("UID mismatch: expected {expected}, got {actual}")]
    UidMismatch { expected: String, actual: String },

    #[error("Replay attack detected: counter {received} is not greater than stored {stored}")]
    ReplayDetected { received: u32, stored: u32 },

    #[error("NFC card not found for location")]
    CardNotFound,

    #[error("NFC card has no UID set (not yet programmed)")]
    CardNotProgrammed,

    #[error("Location not found")]
    LocationNotFound,

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] anyhow::Error),
}

/// Parsed SUN message from NTAG424
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SunMessage {
    /// 7-byte UID as hex string (14 characters)
    pub uid: [u8; 7],
    /// 3-byte counter value
    pub counter: u32,
}

impl SunMessage {
    pub fn new(uid: [u8; 7], counter: u32) -> Self {
        SunMessage { uid, counter }
    }

    pub fn new_hex(uid_hex: &str, counter: u32) -> Result<Self, SunError> {
        let uid = hex::decode(uid_hex)
            .map_err(|e| SunError::InvalidPiccData(format!("uid hex decode error: {}", e)))?
            .try_into()
            .map_err(|_| {
                SunError::InvalidPiccData("uid hex decode error: incorrect length".to_string())
            })?;
        Ok(SunMessage { uid, counter })
    }

    pub fn uid_hex(&self) -> String {
        hex::encode(self.uid)
    }
}

/// Result of successful SUN verification
#[derive(Debug)]
pub struct SunVerification {
    pub location: Location,
    pub nfc_card: NfcCard,
    pub counter: u32,
}

/// Decrypt the picc_data parameter from NTAG424 SUN message.
///
/// The picc_data is encrypted using AES-128-CBC with:
/// - Key: k1 (decrypt key from nfc_cards table)
/// - IV: all zeros (standard for NTAG424 SUN)
///
/// The decrypted data contains:
/// - 7 bytes: UID
/// - 3 bytes: counter (little-endian)
/// - Remaining: padding/random
pub fn decrypt_picc_data(encrypted_hex: &str, k1_hex: &str) -> Result<SunMessage, SunError> {
    // Decode hex inputs
    let encrypted = hex::decode(encrypted_hex)
        .map_err(|e| SunError::InvalidPiccData(format!("hex decode error: {}", e)))?;

    let key = hex::decode(k1_hex)
        .map_err(|e| SunError::InvalidPiccData(format!("key hex decode error: {}", e)))?;

    if key.len() != 16 {
        return Err(SunError::InvalidPiccData(format!(
            "key must be 16 bytes, got {}",
            key.len()
        )));
    }

    if encrypted.len() < 16 {
        return Err(SunError::InvalidPiccData(format!(
            "encrypted data must be at least 16 bytes, got {}",
            encrypted.len()
        )));
    }

    // Use zero IV for NTAG424 SUN
    let iv = [0u8; 16];

    // Decrypt using AES-128-CBC
    let mut buf = encrypted.clone();
    let decrypted = Aes128CbcDec::new(key.as_slice().into(), &iv.into())
        .decrypt_padded_mut::<NoPadding>(&mut buf)
        .map_err(|e| SunError::DecryptionError(format!("{:?}", e)))?;

    if decrypted.len() < 11 {
        return Err(SunError::InvalidPiccData(format!(
            "decrypted data too short: {} bytes",
            decrypted.len()
        )));
    }

    if decrypted[0] != 0xc7 {
        return Err(SunError::InvalidPiccData(format!(
            "invalid PICC type: {:02X}, expected 0xC7",
            decrypted[0]
        )));
    }

    // Extract UID (first 7 bytes)
    let uid = decrypted[1..8].try_into().expect("correct len");

    // Extract counter (next 3 bytes, little-endian)
    let counter = u32::from_le_bytes([decrypted[8], decrypted[9], decrypted[10], 0]);

    Ok(SunMessage { uid, counter })
}

/// Derive session MAC key from master SDM MAC key using SV2 diversification.
///
/// SV2 = [0x3C, 0xC3, 0x00, 0x01, 0x00, 0x80] || UID (7 bytes) || counter (3 bytes LE)
/// Padded to 16 bytes with zeros, then CMAC'd with the master key.
///
/// Note: The prefix 0x3C 0xC3 is for SDM MAC key derivation (per AN12196).
/// The 0x5A 0xA5 prefix is used for encryption key derivation instead.
fn derive_session_mac_key(
    master_key: &[u8],
    uid: &[u8; 7],
    counter: u32,
) -> Result<[u8; 16], SunError> {
    // Build SV2 diversification vector for MAC key
    // Prefix 0x3C 0xC3 indicates MAC key derivation
    let counter_bytes = counter.to_le_bytes();
    let mut sv2 = [0u8; 16];
    sv2[0] = 0x3C;
    sv2[1] = 0xC3;
    sv2[2] = 0x00;
    sv2[3] = 0x01;
    sv2[4] = 0x00;
    sv2[5] = 0x80;
    sv2[6..13].copy_from_slice(uid);
    sv2[13..16].copy_from_slice(&counter_bytes[..3]);

    // Derive session key using CMAC
    let mut mac = <Cmac<aes::Aes128> as Mac>::new_from_slice(master_key)
        .map_err(|e| SunError::InvalidCmac(format!("cmac init error: {}", e)))?;
    mac.update(&sv2);
    let result = mac.finalize().into_bytes();

    Ok(result.into())
}

/// Truncate a 16-byte CMAC to 8 bytes by taking bytes at odd positions.
///
/// NTAG 424 uses bytes at indices 1, 3, 5, 7, 9, 11, 13, 15 (0-indexed).
fn truncate_cmac(full_cmac: &[u8; 16]) -> [u8; 8] {
    [
        full_cmac[1],
        full_cmac[3],
        full_cmac[5],
        full_cmac[7],
        full_cmac[9],
        full_cmac[11],
        full_cmac[13],
        full_cmac[15],
    ]
}

/// Verify the CMAC (C value) from an NTAG424 SUN message.
///
/// The verification process:
/// 1. Derive session MAC key from master key using SV2 with UID and counter
/// 2. Compute CMAC over empty input (for SDM without encrypted file data)
/// 3. Truncate CMAC by taking bytes at odd positions
/// 4. Compare with received CMAC
pub fn verify_cmac(
    sun_message: &SunMessage,
    cmac_hex: &str,
    k2_hex: &str,
) -> Result<bool, SunError> {
    let k2 = hex::decode(k2_hex)
        .map_err(|e| SunError::InvalidCmac(format!("key hex decode error: {}", e)))?;

    if k2.len() != 16 {
        return Err(SunError::InvalidCmac(format!(
            "k2 must be 16 bytes, got {}",
            k2.len()
        )));
    }

    let expected_cmac = hex::decode(cmac_hex)
        .map_err(|e| SunError::InvalidCmac(format!("cmac hex decode error: {}", e)))?;

    if expected_cmac.len() != 8 {
        return Err(SunError::InvalidCmac(format!(
            "cmac must be 8 bytes, got {}",
            expected_cmac.len()
        )));
    }

    // Derive session MAC key using SV2 diversification
    let session_mac_key = derive_session_mac_key(&k2, &sun_message.uid, sun_message.counter)?;

    // Compute CMAC over empty input (SDM without encrypted file data)
    let mut mac = <Cmac<aes::Aes128> as Mac>::new_from_slice(&session_mac_key)
        .map_err(|e| SunError::InvalidCmac(format!("cmac init error: {}", e)))?;
    mac.update(b"");
    let full_cmac: [u8; 16] = mac.finalize().into_bytes().into();

    // Truncate CMAC by taking bytes at odd positions
    let truncated_cmac = truncate_cmac(&full_cmac);

    Ok(truncated_cmac == expected_cmac.as_slice())
}

/// Fully verify a SUN message and return the location and NFC card if valid.
///
/// This performs:
/// 1. Look up the NFC card by location ID
/// 2. Decrypt picc_data using k1
/// 3. Verify CMAC using k2
/// 4. Verify UID matches
/// 5. Verify counter > stored counter (replay protection)
pub async fn verify_sun_message(
    db: &Database,
    location_id: &str,
    picc_data: &str,
    cmac: &str,
) -> Result<SunVerification, SunError> {
    // Get the NFC card for this location
    let nfc_card = db
        .get_nfc_card_by_location(location_id)
        .await?
        .ok_or(SunError::CardNotFound)?;

    // Verify the card has been programmed (has a UID)
    let stored_uid = nfc_card.uid.as_ref().ok_or(SunError::CardNotProgrammed)?;

    // Decrypt the picc_data
    let sun_message = decrypt_picc_data(picc_data, &nfc_card.k1_decrypt_key)?;

    // Verify CMAC
    if !verify_cmac(&sun_message, cmac, &nfc_card.k2_cmac_key)? {
        return Err(SunError::CmacMismatch);
    }

    // Verify UID matches
    let stored_uid_bytes = hex::decode(stored_uid).expect("DB entry malformed");
    if sun_message.uid.as_slice() != stored_uid_bytes.as_slice() {
        return Err(SunError::UidMismatch {
            expected: stored_uid.clone(),
            actual: sun_message.uid_hex(),
        });
    }

    // Verify counter is greater than stored (replay protection)
    if sun_message.counter as i64 <= nfc_card.counter {
        return Err(SunError::ReplayDetected {
            received: sun_message.counter,
            stored: nfc_card.counter as u32,
        });
    }

    // Get the location
    let location = db
        .get_location(location_id)
        .await?
        .ok_or(SunError::LocationNotFound)?;

    Ok(SunVerification {
        location,
        nfc_card,
        counter: sun_message.counter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DECRYPTION_KEY_K1: &str = "1b53525189f66e2e88a3996ae5a87cf3";
    const TEST_AUTHENTICATION_KEY_K2: &str = "e4dae5db65c91efdf74ef3eba21b36c3";
    const TEST_UID: &str = "048D58D2142290";

    const TEST_VECTORS: &[(&str, &str, u32)] = &[
        ("7A4D60F5098CDC5EC25D19592DD90F61", "82E278C1118CEE2F", 10),
        ("3B721FF6E84B8BAB149395CEFDBD465F", "B5939AF5E1DFD702", 11),
        ("79831D41FEAB2E7F54C26FBBB8C72126", "53A929063D0ACD94", 12),
    ];

    #[test]
    fn test_decrypt_picc_data_format() {
        for (p, _c, dec_counter) in TEST_VECTORS {
            let msg = decrypt_picc_data(p, TEST_DECRYPTION_KEY_K1).expect("Decryption can't fail");
            let reference_msg = SunMessage::new_hex(TEST_UID, *dec_counter).expect("Can be parsed");
            assert_eq!(msg, reference_msg);
        }
    }

    #[test]
    fn test_decrypt_picc_data_invalid_hex() {
        let result = decrypt_picc_data("not-hex", "00000000000000000000000000000000");
        assert!(matches!(result, Err(SunError::InvalidPiccData(_))));
    }

    #[test]
    fn test_decrypt_picc_data_invalid_key_length() {
        let result = decrypt_picc_data("00000000000000000000000000000000", "0000");
        assert!(matches!(result, Err(SunError::InvalidPiccData(_))));
    }

    #[test]
    fn test_decrypt_picc_data_too_short() {
        let result = decrypt_picc_data("00000000", "00000000000000000000000000000000");
        assert!(matches!(result, Err(SunError::InvalidPiccData(_))));
    }

    #[test]
    fn test_verify_cmac() {
        for (p, c, _counter) in TEST_VECTORS {
            let msg = decrypt_picc_data(p, TEST_DECRYPTION_KEY_K1).unwrap();
            let valid = verify_cmac(&msg, c, TEST_AUTHENTICATION_KEY_K2).expect("Auth failed");
            assert!(valid, "CMAC verification failed for p={}, c={}", p, c);
        }
    }
}
