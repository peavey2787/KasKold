//! Shared stateless privacy-pairing response derivation.
//!
//! The request/response wire format lives in `shared-signer::pairing`; this
//! module owns the private BIP32 derivation needed by both M5 and software Vaults.

use alloc::vec::Vec;

use crate::derivation::bip32::{self, ExtendedPrivKey};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivacyPairingError {
    InvalidRequest,
    Derivation,
    Encoding,
}

impl PrivacyPairingError {
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid pairing request",
            Self::Derivation => "pairing derivation failed",
            Self::Encoding => "pairing response encoding failed",
        }
    }
}

pub fn respond(account: &ExtendedPrivKey, input: &[u8]) -> Result<Vec<u8>, PrivacyPairingError> {
    respond_with_progress(account, input, |_, _| {})
}

pub fn respond_with_progress(
    account: &ExtendedPrivKey,
    input: &[u8],
    mut progress: impl FnMut(usize, usize),
) -> Result<Vec<u8>, PrivacyPairingError> {
    let request = shared_signer::pairing::parse_request(input)
        .map_err(|_| PrivacyPairingError::InvalidRequest)?;
    let response_len = request.response_len();
    let mut output = Vec::new();
    output
        .try_reserve_exact(response_len)
        .map_err(|_| PrivacyPairingError::Encoding)?;
    output.resize(response_len, 0);
    let compressed = account
        .public_key_compressed()
        .map_err(|_| PrivacyPairingError::Derivation)?;
    let fingerprint =
        shared_signer::pairing::account_fingerprint(&compressed, account.chain_code_bytes());
    let mut cursor =
        shared_signer::pairing::encode_response_header(request, fingerprint, &mut output)
            .map_err(|_| PrivacyPairingError::Encoding)?;
    let total = request.key_count();
    let mut completed = 0usize;
    cursor = derive_chain_keys_into(
        &mut output,
        cursor,
        account,
        ChainRange::receive(request.receive_start, request.receive_count),
        &mut completed,
        total,
        &mut progress,
    )?;
    cursor = derive_chain_keys_into(
        &mut output,
        cursor,
        account,
        ChainRange::change(request.change_start, request.change_count),
        &mut completed,
        total,
        &mut progress,
    )?;
    if cursor != output.len() {
        return Err(PrivacyPairingError::Encoding);
    }
    Ok(output)
}

#[derive(Clone, Copy)]
struct ChainRange {
    start: u32,
    count: u8,
    change: bool,
}

impl ChainRange {
    const fn receive(start: u32, count: u8) -> Self {
        Self {
            start,
            count,
            change: false,
        }
    }

    const fn change(start: u32, count: u8) -> Self {
        Self {
            start,
            count,
            change: true,
        }
    }
}

fn derive_chain_keys_into(
    output: &mut [u8],
    mut cursor: usize,
    account: &ExtendedPrivKey,
    range: ChainRange,
    completed: &mut usize,
    total: usize,
    progress: &mut impl FnMut(usize, usize),
) -> Result<usize, PrivacyPairingError> {
    for offset in 0..u32::from(range.count) {
        let index = range
            .start
            .checked_add(offset)
            .ok_or(PrivacyPairingError::Derivation)?;
        let key = if range.change {
            bip32::derive_change_key(account, index)
        } else {
            bip32::derive_address_key(account, index)
        }
        .map_err(|_| PrivacyPairingError::Derivation)?;
        let public_key = key
            .public_key_x_only()
            .map_err(|_| PrivacyPairingError::Derivation)?;
        let end = cursor
            .checked_add(public_key.len())
            .ok_or(PrivacyPairingError::Encoding)?;
        output
            .get_mut(cursor..end)
            .ok_or(PrivacyPairingError::Encoding)?
            .copy_from_slice(&public_key);
        cursor = end;
        *completed += 1;
        progress(*completed, total);
    }
    Ok(cursor)
}
