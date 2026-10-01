//! Software-wallet custody backend for KasKold Companion.
//!
//! This crate is intentionally separate from `online-watcher`, `kaskold-sdk`,
//! and `kaskold-protocol`: those crates remain structurally unable to own or
//! derive private keys. Secret material stays inside this Rust boundary and is
//! exposed only by explicit wallet creation/restore flows.

use offline_signer::{
    derivation::{
        bip32,
        bip39::{self, Mnemonic12, Mnemonic24, Seed},
    },
    transaction::{
        kspt,
        model::{MultisigStore, TransactionStorageError},
    },
    OfflineSigner,
};
use zeroize::{Zeroize, Zeroizing};

mod covenant_tools;
mod entropy;
mod platform_sealed;
mod portable_backup;
mod private_swap;
mod recovery_import;
mod stego_backup;
mod transaction_anti_klepto;
mod transaction_codec;
mod transaction_review;
mod transaction_signing;
mod wallet_creation;
mod wallet_multisig;
mod wallet_source;
mod wallet_tools;

pub use covenant_tools::CovenantProvisional;
pub use offline_signer::address::KaspaNetwork as WalletNetwork;
#[cfg(test)]
use platform_sealed::{decode_recovery, encode_recovery, RECOVERY_RECORD_LEN};
pub use wallet_multisig::MultisigView;
pub use wallet_tools::{
    normalize_public_account, validate_address_text, CommittedSecret, MessageSignature,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WalletKind {
    Mnemonic,
    AccountXprv,
    RawPrivateKey,
}

pub use platform_sealed::{
    PLATFORM_WRAPPING_KEY_LEN, SEALED_WALLET_LEN, SEALED_WALLET_NONCE_LEN, SEALED_WALLET_TAG_LEN,
};
pub use private_swap::{
    PrivateSwapMode, PrivateSwapPhase, PrivateSwapPrepared, PrivateSwapReview, PrivateSwapSession,
};

pub const MAX_BIP39_PASSPHRASE_LEN: usize = 64;

/// A newly created wallet and the recovery phrase shown by the creation flow.
///
/// The phrase is wrapped in `Zeroizing` so this Rust-side copy is erased when
/// dropped. The custody object separately retains encrypted recovery material
/// so an authenticated explicit backup flow can reveal the words again later.
pub struct CreatedWallet {
    pub wallet: HotWallet,
    pub recovery_phrase: Zeroizing<String>,
}

/// Failures at the hot-wallet custody boundary.
#[derive(Debug)]
pub enum HotWalletError {
    EntropyUnavailable,
    InvalidMnemonicWordCount,
    Bip39(bip39::Bip39Error),
    Bip32(bip32::Bip32Error),
    TransactionStorage(TransactionStorageError),
    Kspt(kspt::PsktError),
    InvalidKpubEncoding,
    RecoveryUnavailable,
    Bip39PassphraseTooLong,
    InvalidSealedWallet,
    SealedWalletEncryptionFailed,
    SealedWalletAuthenticationFailed,
    InvalidToolInput,
    CryptoOperationFailed,
    MultisigInvalid,
    BackupCryptoFailed,
    RawKeyInvalid,
    UnsupportedForWalletType,
    StandardPskt(offline_signer::transaction::std_pskt::PskError),
}

impl From<bip39::Bip39Error> for HotWalletError {
    fn from(value: bip39::Bip39Error) -> Self {
        Self::Bip39(value)
    }
}

impl From<bip32::Bip32Error> for HotWalletError {
    fn from(value: bip32::Bip32Error) -> Self {
        Self::Bip32(value)
    }
}

impl From<TransactionStorageError> for HotWalletError {
    fn from(value: TransactionStorageError) -> Self {
        Self::TransactionStorage(value)
    }
}

impl From<kspt::PsktError> for HotWalletError {
    fn from(value: kspt::PsktError) -> Self {
        Self::Kspt(value)
    }
}

impl From<offline_signer::transaction::std_pskt::PskError> for HotWalletError {
    fn from(value: offline_signer::transaction::std_pskt::PskError) -> Self {
        Self::StandardPskt(value)
    }
}

/// Opaque software-wallet custody object.
///
/// Normal callers can export public account information and request signing.
/// Secret material is released only through deliberately named backup/export
/// operations that higher layers must place behind explicit user interaction.
pub use transaction_anti_klepto::AntiKleptoSession;
pub use transaction_review::{
    OutputOwnership, TransactionReview, TransactionReviewInput, TransactionReviewOutput,
};

pub struct HotWallet {
    seed: Seed,
    account_key: Option<bip32::ExtendedPrivKey>,
    account_parent_fingerprint: [u8; 4],
    raw_key: Option<Zeroizing<[u8; 32]>>,
    recovery: Option<RecoveryMaterial>,
    multisig_store: MultisigStore,
}

struct RecoveryMaterial {
    word_count: u8,
    indices: [u16; 24],
    passphrase: [u8; MAX_BIP39_PASSPHRASE_LEN],
    passphrase_len: u8,
}

impl Drop for RecoveryMaterial {
    fn drop(&mut self) {
        self.indices.zeroize();
        self.passphrase.zeroize();
        self.word_count.zeroize();
        self.passphrase_len.zeroize();
    }
}

impl HotWallet {
    /// Restore a wallet from an explicitly supplied 12- or 24-word phrase.
    pub fn restore(recovery_phrase: &str, passphrase: &str) -> Result<Self, HotWalletError> {
        if passphrase.len() > MAX_BIP39_PASSPHRASE_LEN {
            return Err(HotWalletError::Bip39PassphraseTooLong);
        }
        let words: Vec<&str> = recovery_phrase.split_whitespace().collect();
        let (seed, recovery) = match words.len() {
            12 => restore_words_12(&words, passphrase)?,
            24 => restore_words_24(&words, passphrase)?,
            _ => return Err(HotWalletError::InvalidMnemonicWordCount),
        };
        Ok(Self {
            seed,
            account_key: None,
            account_parent_fingerprint: [0u8; 4],
            raw_key: None,
            recovery: Some(recovery),
            multisig_store: MultisigStore::new(),
        })
    }

    /// Return the wallet's BIP39 recovery phrase for an explicit backup flow.
    ///
    /// This is deliberately separate from normal signing/public-account APIs.
    /// The returned string zeroizes its allocation when dropped.
    pub fn backup_recovery_phrase(&self) -> Result<Zeroizing<String>, HotWalletError> {
        let recovery = self
            .recovery
            .as_ref()
            .ok_or(HotWalletError::RecoveryUnavailable)?;
        Ok(Zeroizing::new(phrase_from_indices(
            &recovery.indices[..usize::from(recovery.word_count)],
        )))
    }

    /// Copy BIP39 word indices into a caller-owned fixed buffer for explicit
    /// SeedQR/backup operations. No seed or private key bytes are exposed.
    pub fn backup_mnemonic_indices(&self, output: &mut [u16; 24]) -> Result<u8, HotWalletError> {
        let recovery = self
            .recovery
            .as_ref()
            .ok_or(HotWalletError::RecoveryUnavailable)?;
        output.fill(0);
        output.copy_from_slice(&recovery.indices);
        Ok(recovery.word_count)
    }

    /// Serialize the account-level Kaspa XPrv for an explicit advanced backup.
    pub fn backup_account_xprv(&self) -> Result<Zeroizing<String>, HotWalletError> {
        let mut output = [0u8; offline_signer::derivation::xpub::XPRV_MAX_LEN];
        let len = if let Some(account) = self.account_key() {
            offline_signer::derivation::xpub::serialize_account_key_xprv(
                account,
                self.account_parent_fingerprint(),
                &mut output,
            )?
        } else {
            offline_signer::derivation::xpub::derive_and_serialize_xprv(
                self.seed_bytes()?,
                &mut output,
            )?
        };
        let text = core::str::from_utf8(&output[..len])
            .map_err(|_| HotWalletError::InvalidKpubEncoding)?
            .to_owned();
        output.zeroize();
        Ok(Zeroizing::new(text))
    }

    /// Derive one receive-chain private key as lower-case hex for the explicit
    /// hardware-compatible Export Key workflow.
    pub fn backup_receive_private_key_hex(
        &self,
        address_index: u16,
    ) -> Result<Zeroizing<String>, HotWalletError> {
        if let Some(raw_key) = self.raw_key_bytes() {
            if address_index != 0 {
                return Err(HotWalletError::UnsupportedForWalletType);
            }
            return Ok(Zeroizing::new(hex_string(raw_key)));
        }
        let derived_account;
        let account = if let Some(account) = self.account_key() {
            account
        } else {
            derived_account = bip32::derive_account_key(self.seed_bytes()?)?;
            &derived_account
        };
        let mut child = bip32::derive_address_key(account, u32::from(address_index))?;
        let mut key = *child.private_key_bytes();
        child.zeroize();
        let text = hex_string(&key);
        key.zeroize();
        Ok(Zeroizing::new(text))
    }

    /// Export the public account (KPUB) used by Companion's watch-only runtime.
    pub fn export_kpub(&self) -> Result<String, HotWalletError> {
        let mut output = [0u8; offline_signer::derivation::xpub::KPUB_MAX_LEN];
        let len = if let Some(account) = self.account_key() {
            offline_signer::derivation::xpub::serialize_account_kpub(
                account,
                self.account_parent_fingerprint(),
                &mut output,
            )?
        } else {
            OfflineSigner::new().export_watch_account(self.seed_bytes()?, &mut output)?
        };
        let text = core::str::from_utf8(&output[..len])
            .map_err(|_| HotWalletError::InvalidKpubEncoding)?
            .to_owned();
        output.zeroize();
        Ok(text)
    }
}

fn parse_mnemonic_words(words: &[&str], output: &mut [u16]) -> Result<(), HotWalletError> {
    for (slot, word) in output.iter_mut().zip(words.iter().copied()) {
        *slot = bip39::word_to_index(word)?;
    }
    Ok(())
}

fn restore_words_12(
    words: &[&str],
    passphrase: &str,
) -> Result<(Seed, RecoveryMaterial), HotWalletError> {
    let signer = OfflineSigner::new();
    let mut mnemonic = Mnemonic12 {
        indices: [0u16; 12],
    };
    parse_mnemonic_words(words, &mut mnemonic.indices)?;
    let seed = signer.restore_wallet_12(&mnemonic, passphrase)?;
    let recovery = RecoveryMaterial::from_12(&mnemonic, passphrase)?;
    mnemonic.zeroize();
    Ok((seed, recovery))
}

fn restore_words_24(
    words: &[&str],
    passphrase: &str,
) -> Result<(Seed, RecoveryMaterial), HotWalletError> {
    let signer = OfflineSigner::new();
    let mut mnemonic = Mnemonic24 {
        indices: [0u16; 24],
    };
    parse_mnemonic_words(words, &mut mnemonic.indices)?;
    let seed = signer.restore_wallet_24(&mnemonic, passphrase)?;
    let recovery = RecoveryMaterial::from_24(&mnemonic, passphrase)?;
    mnemonic.zeroize();
    Ok((seed, recovery))
}

impl RecoveryMaterial {
    fn from_12(mnemonic: &Mnemonic12, passphrase: &str) -> Result<Self, HotWalletError> {
        let mut indices = [0u16; 24];
        indices[..12].copy_from_slice(&mnemonic.indices);
        Self::from_parts(12, indices, passphrase)
    }

    fn from_24(mnemonic: &Mnemonic24, passphrase: &str) -> Result<Self, HotWalletError> {
        Self::from_parts(24, mnemonic.indices, passphrase)
    }

    fn from_parts(
        word_count: u8,
        indices: [u16; 24],
        passphrase: &str,
    ) -> Result<Self, HotWalletError> {
        if passphrase.len() > MAX_BIP39_PASSPHRASE_LEN {
            return Err(HotWalletError::Bip39PassphraseTooLong);
        }
        let mut passphrase_bytes = [0u8; MAX_BIP39_PASSPHRASE_LEN];
        passphrase_bytes[..passphrase.len()].copy_from_slice(passphrase.as_bytes());
        Ok(Self {
            word_count,
            indices,
            passphrase: passphrase_bytes,
            passphrase_len: passphrase.len() as u8,
        })
    }
}

fn hex_string(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes.iter().copied() {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn mnemonic12_phrase(mnemonic: &Mnemonic12) -> String {
    phrase_from_indices(&mnemonic.indices)
}

fn mnemonic24_phrase(mnemonic: &Mnemonic24) -> String {
    phrase_from_indices(&mnemonic.indices)
}

fn phrase_from_indices(indices: &[u16]) -> String {
    let capacity = indices.len().saturating_mul(9);
    let mut phrase = String::with_capacity(capacity);
    for (position, index) in indices.iter().copied().enumerate() {
        if position != 0 {
            phrase.push(' ');
        }
        phrase.push_str(bip39::index_to_word(index));
    }
    phrase
}

#[cfg(test)]
#[path = "unit_tests/mod.rs"]
mod unit_tests;
