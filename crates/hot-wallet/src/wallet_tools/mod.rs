//! Explicit wallet tools shared by hardware-compatible software Vault shells.
//!
//! This module owns key-derivation-backed utilities that require the active
//! private seed. UI shells call these through `vault-runtime`; JavaScript,
//! Swift, and Kotlin never reimplement derivation or cryptography.

mod privacy_pairing;
mod stealth;
#[cfg(test)]
mod unit_tests;

use offline_signer::{
    address::{encode_address_for_network, AddressType, KaspaNetwork, MAX_ADDR_LEN},
    crypto::{ecies, message},
    derivation::{bip32, bip85, xpub},
    transaction::sighash::blake2b_hash,
};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::{entropy::fill_random, phrase_from_indices, HotWallet, HotWalletError};

const SECRET_SALT_LEN: usize = 8;
const MAX_COMMIT_SECRET_LEN: usize = 33;

pub struct MessageSignature {
    pub signature: [u8; 64],
    pub digest: [u8; 32],
}

pub struct CommittedSecret {
    /// `commitment || ECIES(salt || plaintext)`; compatible with the hardware
    /// commit/reveal QR payload.
    pub payload: Zeroizing<Vec<u8>>,
    pub commitment: [u8; 32],
}

/// Normalize and validate a public watch-account kpub with the shared signer codec.
pub fn normalize_public_account(input: &[u8]) -> Result<String, HotWalletError> {
    let mut output = [0u8; xpub::KPUB_MAX_LEN];
    let length = xpub::normalize_kpub_text(input, &mut output)
        .map_err(|_| HotWalletError::InvalidKpubEncoding)?;
    let value = core::str::from_utf8(&output[..length])
        .map_err(|_| HotWalletError::InvalidKpubEncoding)?
        .to_owned();
    output.zeroize();
    Ok(value)
}

/// Validate a Kaspa address using the same checksum-aware codec as signing.
pub fn validate_address_text(input: &str) -> Result<String, HotWalletError> {
    let trimmed = input.trim();
    if !offline_signer::address::validate_kaspa_address(trimmed.as_bytes()) {
        return Err(HotWalletError::InvalidToolInput);
    }
    Ok(trimmed.to_owned())
}

impl HotWallet {
    /// Four-byte visual wallet fingerprint using the same source-material
    /// rules as the M5 wallet-details screen.
    pub fn fingerprint_hex(&self) -> Result<String, HotWalletError> {
        let digest = if let Some(raw_key) = self.raw_key_bytes() {
            Sha256::digest(raw_key)
        } else if let Some(account) = self.account_key() {
            let mut raw = account.to_raw();
            let mut hasher = Sha256::new();
            hasher.update(raw);
            hasher.update(self.account_parent_fingerprint());
            let digest = hasher.finalize();
            raw.zeroize();
            digest
        } else {
            let recovery = self
                .recovery
                .as_ref()
                .ok_or(HotWalletError::RecoveryUnavailable)?;
            let word_count = usize::from(recovery.word_count);
            let entropy_len = if word_count == 12 { 16 } else { 32 };
            let mut entropy = [0u8; 33];
            let mut bit_position = 0usize;
            for index in recovery.indices.iter().take(word_count) {
                for bit in (0..11).rev() {
                    let byte_index = bit_position / 8;
                    let bit_index = 7 - bit_position % 8;
                    if (index >> bit) & 1 == 1 {
                        entropy[byte_index] |= 1 << bit_index;
                    }
                    bit_position += 1;
                }
            }
            let mut hasher = Sha256::new();
            hasher.update(&entropy[..entropy_len]);
            hasher.update(&recovery.passphrase[..usize::from(recovery.passphrase_len)]);
            let digest = hasher.finalize();
            entropy.zeroize();
            digest
        };
        Ok(format!(
            "{:02x}{:02x}{:02x}{:02x}",
            digest[0], digest[1], digest[2], digest[3]
        ))
    }

    /// Derive a normal Kaspa receive/change address using the same BIP44 paths
    /// as the hardware Receive workflow.
    pub fn derive_address(
        &self,
        network: KaspaNetwork,
        change: bool,
        index: u32,
    ) -> Result<String, HotWalletError> {
        validate_address_request(network, index)?;
        let public_key = self.address_public_key(change, index)?;
        encode_p2pk_address(&public_key, network)
    }

    fn address_public_key(&self, change: bool, index: u32) -> Result<[u8; 32], HotWalletError> {
        if let Some(raw_key) = self.raw_key_bytes() {
            return raw_address_public_key(raw_key, change, index);
        }
        if let Some(account) = self.account_key() {
            return account_address_public_key(account, change, index);
        }
        let mut account = bip32::derive_account_key(self.seed_bytes()?)?;
        let result = account_address_public_key(&account, change, index);
        account.zeroize();
        result
    }

    /// Export the dedicated BIP45 cosigner kpub used by the M5 Multisig QR.
    pub fn export_multisig_kpub(&self) -> Result<String, HotWalletError> {
        let mut output = [0u8; xpub::KPUB_MAX_LEN];
        let length = xpub::derive_and_serialize_multisig_kpub(self.seed_bytes()?, &mut output)?;
        let value = core::str::from_utf8(&output[..length])
            .map_err(|_| HotWalletError::InvalidKpubEncoding)?
            .to_owned();
        output.zeroize();
        Ok(value)
    }

    /// Deterministically derive a BIP85 child recovery phrase.
    pub fn derive_bip85_phrase(
        &self,
        word_count: u8,
        index: u32,
    ) -> Result<Zeroizing<String>, HotWalletError> {
        let phrase = match word_count {
            12 => {
                let mut mnemonic = bip85::derive_mnemonic_12(self.seed_bytes()?, index)
                    .map_err(|_| HotWalletError::InvalidToolInput)?;
                let phrase = phrase_from_indices(&mnemonic.indices);
                mnemonic.zeroize();
                phrase
            }
            24 => {
                let mut mnemonic = bip85::derive_mnemonic_24(self.seed_bytes()?, index)
                    .map_err(|_| HotWalletError::InvalidToolInput)?;
                let phrase = phrase_from_indices(&mnemonic.indices);
                mnemonic.zeroize();
                phrase
            }
            _ => return Err(HotWalletError::InvalidMnemonicWordCount),
        };
        Ok(Zeroizing::new(phrase))
    }

    /// Domain-separated BIP340 message signing using the active account key.
    pub fn sign_message(&self, payload: &[u8]) -> Result<MessageSignature, HotWalletError> {
        if payload.is_empty() {
            return Err(HotWalletError::InvalidToolInput);
        }
        let mut private_key = if let Some(raw_key) = self.raw_key_bytes() {
            *raw_key
        } else if let Some(account) = self.account_key() {
            *account.private_key_bytes()
        } else {
            let mut account = bip32::derive_account_key(self.seed_bytes()?)?;
            let private_key = *account.private_key_bytes();
            account.zeroize();
            private_key
        };
        let mut entropy = [0u8; 32];
        fill_random(&mut entropy)?;
        let result = message::sign_message_with_entropy(&private_key, payload, &entropy)
            .map_err(|_| HotWalletError::CryptoOperationFailed);
        private_key.zeroize();
        entropy.zeroize();
        let signature = result?;
        Ok(MessageSignature {
            signature: signature.bytes,
            digest: message::message_digest(payload),
        })
    }

    /// Create the M5-compatible commit/reveal payload for a short secret.
    pub fn commit_secret(&self, secret: &[u8]) -> Result<CommittedSecret, HotWalletError> {
        validate_commit_secret(secret)?;
        let salted = salted_secret(secret)?;
        let commitment = blake2b_hash(&salted);
        let recipient = self.commit_recipient_public_key()?;
        let payload = encrypt_committed_secret(&recipient, &salted, &commitment)?;
        Ok(CommittedSecret {
            payload,
            commitment,
        })
    }

    fn commit_recipient_public_key(&self) -> Result<[u8; 32], HotWalletError> {
        if let Some(account) = self.account_key() {
            return account.public_key_x_only().map_err(HotWalletError::from);
        }
        let mut account = bip32::derive_account_key(self.seed_bytes()?)?;
        let result = account.public_key_x_only().map_err(HotWalletError::from);
        account.zeroize();
        result
    }

    /// Decrypt and authenticate a commit/reveal payload produced for this wallet.
    pub fn decrypt_secret(&self, payload: &[u8]) -> Result<Zeroizing<Vec<u8>>, HotWalletError> {
        if payload.len() <= 32 + 61 {
            return Err(HotWalletError::InvalidToolInput);
        }
        let mut expected = [0u8; 32];
        expected.copy_from_slice(&payload[..32]);
        let mut private_key = if let Some(account) = self.account_key() {
            *account.private_key_bytes()
        } else {
            let mut account = bip32::derive_account_key(self.seed_bytes()?)?;
            let private_key = *account.private_key_bytes();
            account.zeroize();
            private_key
        };
        let decrypted = ecies::decrypt(&private_key, &payload[32..])
            .map_err(|_| HotWalletError::CryptoOperationFailed);
        private_key.zeroize();
        let mut plaintext = Zeroizing::new(decrypted?);
        if plaintext.len() <= SECRET_SALT_LEN || blake2b_hash(&plaintext) != expected {
            return Err(HotWalletError::CryptoOperationFailed);
        }
        let secret = Zeroizing::new(plaintext.split_off(SECRET_SALT_LEN));
        Ok(secret)
    }
}

fn validate_address_request(network: KaspaNetwork, index: u32) -> Result<(), HotWalletError> {
    if network == KaspaNetwork::Unknown {
        return Err(HotWalletError::InvalidToolInput);
    }
    if index >= 0x8000_0000 {
        return Err(HotWalletError::InvalidToolInput);
    }
    Ok(())
}

fn raw_address_public_key(
    raw_key: &[u8; 32],
    change: bool,
    index: u32,
) -> Result<[u8; 32], HotWalletError> {
    if change || index != 0 {
        return Err(HotWalletError::UnsupportedForWalletType);
    }
    bip32::pubkey_from_raw_key(raw_key).map_err(HotWalletError::from)
}

fn account_address_public_key(
    account: &bip32::ExtendedPrivKey,
    change: bool,
    index: u32,
) -> Result<[u8; 32], HotWalletError> {
    let mut child = if change {
        bip32::derive_change_key(account, index)?
    } else {
        bip32::derive_address_key(account, index)?
    };
    let result = child.public_key_x_only().map_err(HotWalletError::from);
    child.zeroize();
    result
}

fn encode_p2pk_address(
    public_key: &[u8; 32],
    network: KaspaNetwork,
) -> Result<String, HotWalletError> {
    let mut encoded = [0u8; MAX_ADDR_LEN];
    let length = encode_address_for_network(public_key, AddressType::P2pk, network, &mut encoded);
    if length == 0 {
        return Err(HotWalletError::InvalidToolInput);
    }
    core::str::from_utf8(&encoded[..length])
        .map(str::to_owned)
        .map_err(|_| HotWalletError::InvalidToolInput)
}

fn validate_commit_secret(secret: &[u8]) -> Result<(), HotWalletError> {
    if secret.is_empty() || secret.len() > MAX_COMMIT_SECRET_LEN {
        return Err(HotWalletError::InvalidToolInput);
    }
    Ok(())
}

fn salted_secret(secret: &[u8]) -> Result<Zeroizing<Vec<u8>>, HotWalletError> {
    let mut salted = Zeroizing::new(Vec::with_capacity(SECRET_SALT_LEN + secret.len()));
    let mut salt = [0u8; SECRET_SALT_LEN];
    fill_random(&mut salt)?;
    salted.extend_from_slice(&salt);
    salted.extend_from_slice(secret);
    salt.zeroize();
    Ok(salted)
}

fn encrypt_committed_secret(
    recipient: &[u8; 32],
    salted: &[u8],
    commitment: &[u8; 32],
) -> Result<Zeroizing<Vec<u8>>, HotWalletError> {
    let mut randomness = [0u8; 44];
    fill_random(&mut randomness)?;
    let encrypted = ecies::encrypt(recipient, salted, &randomness)
        .map_err(|_| HotWalletError::CryptoOperationFailed);
    randomness.zeroize();
    let mut ciphertext = encrypted?;
    let mut payload = Zeroizing::new(Vec::with_capacity(32 + ciphertext.len()));
    payload.extend_from_slice(commitment);
    payload.extend_from_slice(&ciphertext);
    ciphertext.zeroize();
    Ok(payload)
}
