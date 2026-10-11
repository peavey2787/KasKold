//! Authenticated native-platform persistence for software Vault custody state.
//!
//! KHV3 persists the wallet source type plus secret/recovery metadata required
//! by explicit backup workflows.

use aes_gcm::{
    aead::{generic_array::GenericArray, AeadInPlace, KeyInit},
    Aes256Gcm,
};
use offline_signer::{derivation::bip39::Seed, transaction::model::MultisigStore};
use zeroize::Zeroize;

use crate::entropy::fill_random;

use super::{HotWallet, HotWalletError, RecoveryMaterial, WalletKind, MAX_BIP39_PASSPHRASE_LEN};

pub const PLATFORM_WRAPPING_KEY_LEN: usize = 32;
pub const SEALED_WALLET_NONCE_LEN: usize = 12;
pub const SEALED_WALLET_TAG_LEN: usize = 16;
pub(crate) const RECOVERY_RECORD_LEN: usize = 1 + 1 + 24 * 2 + MAX_BIP39_PASSPHRASE_LEN;
const SECRET_RECORD_LEN: usize = 69;
const SOURCE_MNEMONIC: u8 = 0;
const SOURCE_RAW_PRIVATE_KEY: u8 = 1;
const SOURCE_ACCOUNT_XPRV: u8 = 2;
const SEALED_WALLET_PLAINTEXT_LEN: usize = 1 + SECRET_RECORD_LEN + RECOVERY_RECORD_LEN;
pub const SEALED_WALLET_LEN: usize =
    4 + SEALED_WALLET_NONCE_LEN + SEALED_WALLET_PLAINTEXT_LEN + SEALED_WALLET_TAG_LEN;
const SEALED_WALLET_MAGIC: &[u8; 4] = b"KHV3";
const SEALED_WALLET_AAD: &[u8] = b"KasKold/vault/platform-sealed-wallet/v3";

impl HotWallet {
    /// Seal wallet custody state under a short-lived platform wrapping key.
    ///
    /// Native shells may persist the returned authenticated ciphertext, but the
    /// plaintext seed/recovery material never crosses this Rust custody boundary.
    /// The platform wrapping key is not wallet material; Android/iOS are responsible for
    /// protecting it with Keystore/Keychain device security and zeroizing the
    /// temporary key bytes after each call.
    pub fn seal_for_platform(
        &self,
        wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
    ) -> Result<Vec<u8>, HotWalletError> {
        let mut nonce = [0u8; SEALED_WALLET_NONCE_LEN];
        fill_random(&mut nonce)?;

        let cipher = Aes256Gcm::new(GenericArray::from_slice(wrapping_key));
        let mut plaintext = encode_platform_plaintext(self)?;
        let tag = match cipher.encrypt_in_place_detached(
            GenericArray::from_slice(&nonce),
            SEALED_WALLET_AAD,
            &mut plaintext,
        ) {
            Ok(tag) => tag,
            Err(_) => {
                plaintext.zeroize();
                return Err(HotWalletError::SealedWalletEncryptionFailed);
            }
        };

        let mut sealed = Vec::with_capacity(SEALED_WALLET_LEN);
        sealed.extend_from_slice(SEALED_WALLET_MAGIC);
        sealed.extend_from_slice(&nonce);
        sealed.extend_from_slice(&plaintext);
        sealed.extend_from_slice(tag.as_ref());
        plaintext.zeroize();
        Ok(sealed)
    }

    /// Restore a wallet from an authenticated platform-sealed custody container.
    ///
    /// KHV3 is the only accepted format; it retains wallet type and recovery material.
    pub fn restore_platform_sealed(
        sealed: &[u8],
        wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
    ) -> Result<Self, HotWalletError> {
        if sealed.len() == SEALED_WALLET_LEN && &sealed[..4] == SEALED_WALLET_MAGIC {
            return restore_platform_sealed_v3(sealed, wrapping_key);
        }
        Err(HotWalletError::InvalidSealedWallet)
    }
}

fn restore_platform_sealed_v3(
    sealed: &[u8],
    wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
) -> Result<HotWallet, HotWalletError> {
    let mut plaintext = decrypt_platform_v3(sealed, wrapping_key)?;
    let recovery = decode_recovery(&plaintext[1 + SECRET_RECORD_LEN..]);
    let result = recovery.and_then(|recovery| decode_platform_v3_wallet(&plaintext, recovery));
    plaintext.zeroize();
    result
}

fn encode_platform_plaintext(
    wallet: &HotWallet,
) -> Result<[u8; SEALED_WALLET_PLAINTEXT_LEN], HotWalletError> {
    let mut plaintext = [0u8; SEALED_WALLET_PLAINTEXT_LEN];
    match wallet.wallet_kind() {
        WalletKind::Mnemonic => encode_mnemonic_plaintext(wallet, &mut plaintext)?,
        WalletKind::AccountXprv => encode_account_plaintext(wallet, &mut plaintext)?,
        WalletKind::RawPrivateKey => encode_raw_key_plaintext(wallet, &mut plaintext)?,
    }
    Ok(plaintext)
}

fn encode_mnemonic_plaintext(
    wallet: &HotWallet,
    plaintext: &mut [u8; SEALED_WALLET_PLAINTEXT_LEN],
) -> Result<(), HotWalletError> {
    plaintext[0] = SOURCE_MNEMONIC;
    plaintext[1..65].copy_from_slice(wallet.seed_bytes()?);
    if let Some(recovery) = &wallet.recovery {
        encode_recovery(recovery, &mut plaintext[1 + SECRET_RECORD_LEN..]);
    }
    Ok(())
}

fn encode_account_plaintext(
    wallet: &HotWallet,
    plaintext: &mut [u8; SEALED_WALLET_PLAINTEXT_LEN],
) -> Result<(), HotWalletError> {
    plaintext[0] = SOURCE_ACCOUNT_XPRV;
    let account = wallet
        .account_key()
        .ok_or(HotWalletError::UnsupportedForWalletType)?;
    let mut raw = account.to_raw();
    plaintext[1..66].copy_from_slice(&raw);
    plaintext[66..70].copy_from_slice(&wallet.account_parent_fingerprint());
    raw.zeroize();
    Ok(())
}

fn encode_raw_key_plaintext(
    wallet: &HotWallet,
    plaintext: &mut [u8; SEALED_WALLET_PLAINTEXT_LEN],
) -> Result<(), HotWalletError> {
    plaintext[0] = SOURCE_RAW_PRIVATE_KEY;
    let raw_key = wallet
        .raw_key_bytes()
        .ok_or(HotWalletError::RawKeyInvalid)?;
    plaintext[1..33].copy_from_slice(raw_key);
    Ok(())
}

fn decrypt_platform_v3(
    sealed: &[u8],
    wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
) -> Result<[u8; SEALED_WALLET_PLAINTEXT_LEN], HotWalletError> {
    let nonce_end = 4 + SEALED_WALLET_NONCE_LEN;
    let payload_end = nonce_end + SEALED_WALLET_PLAINTEXT_LEN;
    let mut plaintext = [0u8; SEALED_WALLET_PLAINTEXT_LEN];
    plaintext.copy_from_slice(&sealed[nonce_end..payload_end]);
    let tag = GenericArray::from_slice(&sealed[payload_end..]);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(wrapping_key));
    if cipher
        .decrypt_in_place_detached(
            GenericArray::from_slice(&sealed[4..nonce_end]),
            SEALED_WALLET_AAD,
            &mut plaintext,
            tag,
        )
        .is_err()
    {
        plaintext.zeroize();
        return Err(HotWalletError::SealedWalletAuthenticationFailed);
    }
    Ok(plaintext)
}

fn decode_platform_v3_wallet(
    plaintext: &[u8; SEALED_WALLET_PLAINTEXT_LEN],
    recovery: Option<RecoveryMaterial>,
) -> Result<HotWallet, HotWalletError> {
    match plaintext[0] {
        SOURCE_MNEMONIC => decode_v3_mnemonic(plaintext, recovery),
        SOURCE_RAW_PRIVATE_KEY => decode_v3_raw_key(plaintext, recovery),
        SOURCE_ACCOUNT_XPRV => decode_v3_account(plaintext, recovery),
        _ => Err(HotWalletError::InvalidSealedWallet),
    }
}

fn decode_v3_mnemonic(
    plaintext: &[u8; SEALED_WALLET_PLAINTEXT_LEN],
    recovery: Option<RecoveryMaterial>,
) -> Result<HotWallet, HotWalletError> {
    if plaintext[65..1 + SECRET_RECORD_LEN]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(HotWalletError::InvalidSealedWallet);
    }
    let mut seed_bytes = [0u8; 64];
    seed_bytes.copy_from_slice(&plaintext[1..65]);
    Ok(HotWallet {
        seed: Seed { bytes: seed_bytes },
        account_key: None,
        account_parent_fingerprint: [0u8; 4],
        raw_key: None,
        recovery,
        multisig_store: MultisigStore::new(),
    })
}

fn decode_v3_raw_key(
    plaintext: &[u8; SEALED_WALLET_PLAINTEXT_LEN],
    recovery: Option<RecoveryMaterial>,
) -> Result<HotWallet, HotWalletError> {
    if recovery.is_some()
        || plaintext[33..1 + SECRET_RECORD_LEN]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(HotWalletError::InvalidSealedWallet);
    }
    let mut raw_key = [0u8; 32];
    raw_key.copy_from_slice(&plaintext[1..33]);
    if offline_signer::derivation::bip32::pubkey_from_raw_key(&raw_key).is_err() {
        raw_key.zeroize();
        return Err(HotWalletError::InvalidSealedWallet);
    }
    Ok(HotWallet {
        seed: Seed { bytes: [0u8; 64] },
        account_key: None,
        account_parent_fingerprint: [0u8; 4],
        raw_key: Some(zeroize::Zeroizing::new(raw_key)),
        recovery: None,
        multisig_store: MultisigStore::new(),
    })
}

fn decode_v3_account(
    plaintext: &[u8; SEALED_WALLET_PLAINTEXT_LEN],
    recovery: Option<RecoveryMaterial>,
) -> Result<HotWallet, HotWalletError> {
    if recovery.is_some() {
        return Err(HotWalletError::InvalidSealedWallet);
    }
    let mut raw = [0u8; 65];
    raw.copy_from_slice(&plaintext[1..66]);
    let account = offline_signer::derivation::bip32::ExtendedPrivKey::from_raw(&raw);
    raw.zeroize();
    if account.depth != 3 || account.public_key_compressed().is_err() {
        return Err(HotWalletError::InvalidSealedWallet);
    }
    let mut parent_fingerprint = [0u8; 4];
    parent_fingerprint.copy_from_slice(&plaintext[66..70]);
    Ok(HotWallet {
        seed: Seed { bytes: [0u8; 64] },
        account_key: Some(account),
        account_parent_fingerprint: parent_fingerprint,
        raw_key: None,
        recovery: None,
        multisig_store: MultisigStore::new(),
    })
}

pub(crate) fn encode_recovery(recovery: &RecoveryMaterial, output: &mut [u8]) {
    debug_assert_eq!(output.len(), RECOVERY_RECORD_LEN);
    output.fill(0);
    output[0] = recovery.word_count;
    output[1] = recovery.passphrase_len;
    for (index, word) in recovery.indices.iter().copied().enumerate() {
        let offset = 2 + index * 2;
        output[offset..offset + 2].copy_from_slice(&word.to_le_bytes());
    }
    output[50..].copy_from_slice(&recovery.passphrase);
}

pub(crate) fn decode_recovery(input: &[u8]) -> Result<Option<RecoveryMaterial>, HotWalletError> {
    if input.len() != RECOVERY_RECORD_LEN {
        return Err(HotWalletError::InvalidSealedWallet);
    }
    let word_count = input[0];
    if word_count == 0 {
        return decode_empty_recovery(input);
    }
    if !matches!(word_count, 12 | 24) {
        return Err(HotWalletError::InvalidSealedWallet);
    }
    let passphrase_len = usize::from(input[1]);
    if passphrase_len > MAX_BIP39_PASSPHRASE_LEN {
        return Err(HotWalletError::InvalidSealedWallet);
    }
    let mut indices = decode_recovery_indices(input);
    validate_recovery_indices(&indices, word_count)?;
    let mut passphrase = [0u8; MAX_BIP39_PASSPHRASE_LEN];
    passphrase.copy_from_slice(&input[50..]);
    if !valid_recovery_passphrase(&passphrase, passphrase_len) {
        passphrase.zeroize();
        indices.zeroize();
        return Err(HotWalletError::InvalidSealedWallet);
    }
    Ok(Some(RecoveryMaterial {
        word_count,
        indices,
        passphrase,
        passphrase_len: passphrase_len as u8,
    }))
}

fn decode_empty_recovery(input: &[u8]) -> Result<Option<RecoveryMaterial>, HotWalletError> {
    if input.iter().any(|byte| *byte != 0) {
        return Err(HotWalletError::InvalidSealedWallet);
    }
    Ok(None)
}

fn decode_recovery_indices(input: &[u8]) -> [u16; 24] {
    let mut indices = [0u16; 24];
    for (index, word) in indices.iter_mut().enumerate() {
        let offset = 2 + index * 2;
        *word = u16::from_le_bytes([input[offset], input[offset + 1]]);
    }
    indices
}

fn validate_recovery_indices(indices: &[u16; 24], word_count: u8) -> Result<(), HotWalletError> {
    let count = usize::from(word_count);
    if indices[..count].iter().any(|index| *index >= 2048)
        || indices[count..].iter().any(|index| *index != 0)
    {
        return Err(HotWalletError::InvalidSealedWallet);
    }
    Ok(())
}

fn valid_recovery_passphrase(passphrase: &[u8; MAX_BIP39_PASSPHRASE_LEN], len: usize) -> bool {
    passphrase[len..].iter().all(|byte| *byte == 0)
        && core::str::from_utf8(&passphrase[..len]).is_ok()
}

#[cfg(test)]
mod unit_tests;
