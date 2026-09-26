//! Shared KasKold stealth-scan protocol implementation.
//!
//! The M5 camera workflow and software Vaults both use this module so the
//! STLH/STLR derivation math has one implementation. Callers own transport,
//! progress UI, and QR presentation only.

use alloc::vec::Vec;

use k256::elliptic_curve::{ops::Reduce, sec1::ToEncodedPoint, ScalarPrimitive};
use k256::{Scalar, U256};
use sha2::{Digest, Sha256};

use crate::derivation::bip32::{self, ExtendedPrivKey};

pub const REQUEST_MAGIC: [u8; 4] = *b"STLH";
pub const RESPONSE_MAGIC: [u8; 4] = *b"STLR";
pub const MAX_CANDIDATES: usize = 64;
pub const REQUEST_HEADER_LEN: usize = 5;
pub const CANDIDATE_LEN: usize = 32;
pub const RESPONSE_RECORD_LEN: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StealthError {
    RequestTooShort,
    InvalidMagic,
    InvalidCount,
    ResponseTooLarge,
    Derivation,
    InvalidAccountKey,
}

impl StealthError {
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::RequestTooShort => "request too short",
            Self::InvalidMagic => "invalid stealth request magic",
            Self::InvalidCount => "bad count or payload length",
            Self::ResponseTooLarge => "response too large",
            Self::Derivation => "stealth key derivation failed",
            Self::InvalidAccountKey => "invalid stealth account key",
        }
    }
}

/// Validate an STLH request and return its candidate count.
///
/// Extra bytes after the declared candidate list are ignored for compatibility
/// with the M5 camera buffer contract; callers should present only the decoded
/// QR payload whenever possible.
pub fn validate_request(data: &[u8]) -> Result<usize, StealthError> {
    if data.len() < REQUEST_HEADER_LEN {
        return Err(StealthError::RequestTooShort);
    }
    if !data.starts_with(&REQUEST_MAGIC) {
        return Err(StealthError::InvalidMagic);
    }
    let count = usize::from(data[4]);
    let expected = REQUEST_HEADER_LEN
        .checked_add(
            count
                .checked_mul(CANDIDATE_LEN)
                .ok_or(StealthError::InvalidCount)?,
        )
        .ok_or(StealthError::InvalidCount)?;
    if count == 0 || count > MAX_CANDIDATES || data.len() < expected {
        return Err(StealthError::InvalidCount);
    }
    Ok(count)
}

/// Build the canonical STLR response for a validated STLH request.
pub fn scan_request(account_key: &ExtendedPrivKey, data: &[u8]) -> Result<Vec<u8>, StealthError> {
    scan_request_with_progress(account_key, data, |_, _| {})
}

/// Build the canonical STLR response while reporting completed candidates.
pub fn scan_request_with_progress(
    account_key: &ExtendedPrivKey,
    data: &[u8],
    mut progress: impl FnMut(usize, usize),
) -> Result<Vec<u8>, StealthError> {
    let count = validate_request(data)?;
    let keys = derive_keys(account_key)?;
    let response_len = count
        .checked_mul(RESPONSE_RECORD_LEN)
        .and_then(|length| length.checked_add(REQUEST_HEADER_LEN))
        .ok_or(StealthError::ResponseTooLarge)?;
    let mut response = Vec::new();
    response
        .try_reserve_exact(response_len)
        .map_err(|_| StealthError::ResponseTooLarge)?;
    response.resize(response_len, 0);
    response[..4].copy_from_slice(&RESPONSE_MAGIC);
    response[4] = count as u8;
    for index in 0..count {
        let request_offset = REQUEST_HEADER_LEN + index * CANDIDATE_LEN;
        let output_offset = REQUEST_HEADER_LEN + index * RESPONSE_RECORD_LEN;
        if let Some((public_key, tweak)) =
            scan_candidate(&data[request_offset..request_offset + CANDIDATE_LEN], &keys)
        {
            response[output_offset..output_offset + 32].copy_from_slice(&public_key);
            response[output_offset + 32..output_offset + 64].copy_from_slice(&tweak);
        }
        progress(index + 1, count);
    }
    Ok(response)
}

struct StealthKeys {
    scan_scalar: k256::Scalar,
    spend_public_key: [u8; 32],
}

fn derive_keys(account_key: &ExtendedPrivKey) -> Result<StealthKeys, StealthError> {
    let scan_branch = bip32::derive_child(account_key, 2).map_err(|_| StealthError::Derivation)?;
    let scan_key = bip32::derive_child(&scan_branch, 0).map_err(|_| StealthError::Derivation)?;
    let scan_primitive =
        ScalarPrimitive::<k256::Secp256k1>::from_slice(scan_key.private_key_bytes())
            .map_err(|_| StealthError::Derivation)?;
    Ok(StealthKeys {
        scan_scalar: k256::Scalar::from(scan_primitive),
        spend_public_key: account_key
            .public_key_x_only()
            .map_err(|_| StealthError::InvalidAccountKey)?,
    })
}

fn scan_candidate(candidate: &[u8], keys: &StealthKeys) -> Option<([u8; 32], [u8; 32])> {
    let ephemeral = parse_xonly_public_key(candidate)?;
    let shared = (ephemeral.to_projective() * keys.scan_scalar)
        .to_affine()
        .to_encoded_point(true);
    let mut hasher = Sha256::new();
    hasher.update(b"KasStealth");
    hasher.update(&shared.as_bytes()[1..33]);
    hasher.update(0u32.to_be_bytes());
    let tweak_hash: [u8; 32] = hasher.finalize().into();
    let tweak_primitive = scalar_primitive(&tweak_hash)?;
    let spend = parse_xonly_public_key(&keys.spend_public_key)?;
    let one_time = (spend.to_projective()
        + k256::ProjectivePoint::GENERATOR * k256::Scalar::from(tweak_primitive))
    .to_affine()
    .to_encoded_point(true);
    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(&one_time.as_bytes()[1..33]);
    Some((public_key, tweak_hash))
}

fn parse_xonly_public_key(value: &[u8]) -> Option<k256::PublicKey> {
    if value.len() != 32 {
        return None;
    }
    let mut compressed = [0u8; 33];
    compressed[0] = 0x02;
    compressed[1..].copy_from_slice(value);
    k256::PublicKey::from_sec1_bytes(&compressed).ok()
}

fn scalar_primitive(value: &[u8; 32]) -> Option<ScalarPrimitive<k256::Secp256k1>> {
    let scalar = <Scalar as Reduce<U256>>::reduce_bytes(&(*value).into());
    let reduced = scalar.to_bytes();
    if reduced.iter().all(|byte| *byte == 0) {
        return None;
    }
    ScalarPrimitive::<k256::Secp256k1>::from_slice(&reduced).ok()
}
