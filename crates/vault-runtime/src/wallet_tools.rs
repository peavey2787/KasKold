//! Non-transaction wallet workflows shared by Vault presentation shells.
//!
//! UI code owns navigation only. Address derivation, recovery, multisig and
//! advanced cryptographic tools remain behind the Rust custody boundary.

use hot_wallet::{CommittedSecret, HotWallet, MessageSignature, MultisigView, WalletNetwork};
use zeroize::Zeroizing;

use super::{VaultCreation, VaultRuntime, VaultRuntimeError};

pub const MAX_SOFTWARE_WALLETS: usize = 5;
pub const WALLET_NAME_MAX: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalletSummary {
    pub index: usize,
    pub name: String,
    pub active: bool,
    pub kind: hot_wallet::WalletKind,
    pub fingerprint: Option<String>,
    pub kpub: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMessage {
    pub signature: [u8; 64],
    pub digest: [u8; 32],
}

#[derive(Debug)]
pub struct SecretCommitment {
    pub payload: Zeroizing<Vec<u8>>,
    pub commitment: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigResult {
    pub descriptor: String,
    pub address: String,
    pub threshold: u8,
    pub participants: u8,
    pub chain: u8,
    pub index: u32,
}

impl VaultRuntime {
    pub fn add_wallet_12(&mut self) -> Result<VaultCreation, VaultRuntimeError> {
        self.add_created(HotWallet::create_12().map_err(VaultRuntimeError::Custody)?)
    }

    pub fn add_wallet_24(&mut self) -> Result<VaultCreation, VaultRuntimeError> {
        self.add_created(HotWallet::create_24().map_err(VaultRuntimeError::Custody)?)
    }

    pub fn add_wallet_with_additive_entropy(
        &mut self,
        word_count: u8,
        dice_rolls: &[u8],
        touch_transcript: &[u8],
        passphrase: &str,
    ) -> Result<VaultCreation, VaultRuntimeError> {
        self.add_created(
            HotWallet::create_with_additive_entropy(
                word_count,
                dice_rolls,
                touch_transcript,
                passphrase,
            )
            .map_err(VaultRuntimeError::Custody)?,
        )
    }

    pub fn add_restored_wallet(
        &mut self,
        recovery_phrase: &str,
        passphrase: &str,
    ) -> Result<String, VaultRuntimeError> {
        self.ensure_wallet_capacity()?;
        let wallet =
            HotWallet::restore(recovery_phrase, passphrase).map_err(VaultRuntimeError::Custody)?;
        self.add_wallet(wallet)
    }

    pub fn add_raw_private_key(
        &mut self,
        private_key_hex: &str,
    ) -> Result<WalletSummary, VaultRuntimeError> {
        self.ensure_wallet_capacity()?;
        let wallet = HotWallet::import_raw_private_key_hex(private_key_hex)
            .map_err(VaultRuntimeError::Custody)?;
        self.push_wallet_summary(wallet)
    }

    pub fn add_account_xprv(&mut self, xprv: &str) -> Result<WalletSummary, VaultRuntimeError> {
        self.ensure_wallet_capacity()?;
        let wallet = HotWallet::import_account_xprv(xprv).map_err(VaultRuntimeError::Custody)?;
        self.push_wallet_summary(wallet)
    }

    pub fn delete_wallet(&mut self, index: usize) -> Result<(), VaultRuntimeError> {
        if index >= self.wallets.len() {
            return Err(VaultRuntimeError::InvalidWalletIndex);
        }
        self.clear_signing_session();
        self.covenant.clear_all();
        self.wallets.remove(index);
        self.wallet_names.remove(index);
        self.active_wallet = if self.wallets.is_empty() {
            None
        } else {
            Some(index.min(self.wallets.len() - 1))
        };
        Ok(())
    }

    pub fn add_recovery_material(
        &mut self,
        data: &[u8],
        passphrase: &str,
    ) -> Result<String, VaultRuntimeError> {
        self.ensure_wallet_capacity()?;
        let wallet = HotWallet::restore_recovery_material(data, passphrase)
            .map_err(VaultRuntimeError::Custody)?;
        self.add_wallet(wallet)
    }

    pub fn wallet_summaries(&self) -> Result<Vec<WalletSummary>, VaultRuntimeError> {
        if self.wallets.is_empty() {
            return Err(VaultRuntimeError::Locked);
        }
        self.wallets
            .iter()
            .enumerate()
            .map(|(index, wallet)| {
                Ok(WalletSummary {
                    index,
                    name: self
                        .wallet_names
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| default_wallet_name(index)),
                    active: self.active_wallet == Some(index),
                    kind: wallet.wallet_kind(),
                    fingerprint: wallet.fingerprint_hex().ok(),
                    kpub: wallet.export_kpub().ok(),
                })
            })
            .collect()
    }

    pub fn set_wallet_name(&mut self, index: usize, name: &str) -> Result<(), VaultRuntimeError> {
        if index >= self.wallets.len() {
            return Err(VaultRuntimeError::InvalidWalletIndex);
        }
        let name = validate_wallet_name(name)?;
        let slot = self
            .wallet_names
            .get_mut(index)
            .ok_or(VaultRuntimeError::InvalidWalletIndex)?;
        *slot = name;
        Ok(())
    }

    pub fn set_active_wallet_name(&mut self, name: &str) -> Result<(), VaultRuntimeError> {
        let index = self.active_wallet.ok_or(VaultRuntimeError::Locked)?;
        self.set_wallet_name(index, name)
    }

    pub fn switch_wallet(&mut self, index: usize) -> Result<Option<String>, VaultRuntimeError> {
        let wallet = self
            .wallets
            .get(index)
            .ok_or(VaultRuntimeError::InvalidWalletIndex)?;
        let kpub = wallet.export_kpub().ok();
        self.clear_signing_session();
        self.covenant.clear_all();
        self.active_wallet = Some(index);
        Ok(kpub)
    }

    pub fn normalize_watch_kpub(value: &[u8]) -> Result<String, VaultRuntimeError> {
        hot_wallet::normalize_public_account(value).map_err(VaultRuntimeError::Custody)
    }

    pub fn validate_multisig_address(value: &str) -> Result<String, VaultRuntimeError> {
        hot_wallet::validate_address_text(value).map_err(VaultRuntimeError::Custody)
    }

    /// Normalize an M5 covenant backup file into its canonical raw COVB/COVI
    /// payload. File selection belongs to the platform shell; byte validation
    /// remains shared with the hardware QR/file classifier.
    pub fn normalize_covenant_backup(data: &[u8]) -> Result<Vec<u8>, VaultRuntimeError> {
        let mut output = [0u8; shared_signer::covenant_backup::MAX_RAW_LEN];
        let len = shared_signer::covenant_backup::normalize(data, &mut output)
            .map_err(|_| VaultRuntimeError::InvalidCovenantBackup)?;
        Ok(output[..len].to_vec())
    }

    pub fn derive_receive_address(
        &self,
        network: &str,
        change: bool,
        index: u32,
    ) -> Result<String, VaultRuntimeError> {
        self.active_wallet()?
            .derive_address(parse_network(network)?, change, index)
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn export_multisig_kpub(&self) -> Result<String, VaultRuntimeError> {
        self.active_wallet()?
            .export_multisig_kpub()
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn create_multisig(
        &mut self,
        threshold: u8,
        cosigner_kpubs: &[&str],
        network: &str,
        chain: u8,
        index: u32,
    ) -> Result<MultisigResult, VaultRuntimeError> {
        self.active_wallet_mut()?
            .create_multisig(
                threshold,
                cosigner_kpubs,
                parse_network(network)?,
                chain,
                index,
            )
            .map(MultisigResult::from)
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn import_multisig_descriptor(
        &mut self,
        descriptor: &str,
        network: &str,
        chain: u8,
        index: u32,
    ) -> Result<MultisigResult, VaultRuntimeError> {
        self.active_wallet_mut()?
            .import_multisig_descriptor(descriptor, parse_network(network)?, chain, index)
            .map(MultisigResult::from)
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn derive_bip85_phrase(
        &self,
        word_count: u8,
        index: u32,
    ) -> Result<Zeroizing<String>, VaultRuntimeError> {
        self.active_wallet()?
            .derive_bip85_phrase(word_count, index)
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn sign_message(&self, message: &[u8]) -> Result<SignedMessage, VaultRuntimeError> {
        self.active_wallet()?
            .sign_message(message)
            .map(SignedMessage::from)
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn commit_secret(&self, secret: &[u8]) -> Result<SecretCommitment, VaultRuntimeError> {
        self.active_wallet()?
            .commit_secret(secret)
            .map(SecretCommitment::from)
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn decrypt_secret(&self, payload: &[u8]) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        self.active_wallet()?
            .decrypt_secret(payload)
            .map_err(VaultRuntimeError::Custody)
    }

    /// Process an M5-compatible STLH privacy scan request using the active HD wallet.
    pub fn stealth_scan_request(&self, request: &[u8]) -> Result<Vec<u8>, VaultRuntimeError> {
        self.active_wallet()?
            .stealth_scan_request(request)
            .map_err(VaultRuntimeError::Custody)
    }

    /// Respond to a stateless KSPR privacy-pairing request with only the requested
    /// receive/change public-key ranges. Raw-key wallets fail closed.
    pub fn privacy_pairing_response(&self, request: &[u8]) -> Result<Vec<u8>, VaultRuntimeError> {
        self.active_wallet()?
            .privacy_pairing_response(request)
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn portable_backup(&self, password: &str) -> Result<Vec<u8>, VaultRuntimeError> {
        self.active_wallet()?
            .portable_backup(password)
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn portable_xprv_backup(&self, password: &str) -> Result<Vec<u8>, VaultRuntimeError> {
        self.active_wallet()?
            .portable_xprv_backup(password)
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn add_portable_backup(
        &mut self,
        data: &[u8],
        password: &str,
    ) -> Result<WalletSummary, VaultRuntimeError> {
        self.ensure_wallet_capacity()?;
        let wallet = HotWallet::restore_portable_backup(data, password)
            .map_err(VaultRuntimeError::Custody)?;
        self.push_wallet_summary(wallet)
    }

    pub fn stego_backup(&self, jpeg: &[u8], password: &str) -> Result<Vec<u8>, VaultRuntimeError> {
        self.active_wallet()?
            .stego_backup(jpeg, password)
            .map_err(VaultRuntimeError::Custody)
    }

    pub fn add_stego_backup(
        &mut self,
        jpeg: &[u8],
        password: &str,
    ) -> Result<WalletSummary, VaultRuntimeError> {
        self.ensure_wallet_capacity()?;
        let wallet =
            HotWallet::restore_stego_backup(jpeg, password).map_err(VaultRuntimeError::Custody)?;
        self.push_wallet_summary(wallet)
    }

    fn add_created(
        &mut self,
        created: hot_wallet::CreatedWallet,
    ) -> Result<VaultCreation, VaultRuntimeError> {
        self.ensure_wallet_capacity()?;
        let kpub = created
            .wallet
            .export_kpub()
            .map_err(VaultRuntimeError::Custody)?;
        self.clear_signing_session();
        self.covenant.clear_all();
        self.wallets.push(created.wallet);
        self.wallet_names
            .push(default_wallet_name(self.wallets.len() - 1));
        self.active_wallet = Some(self.wallets.len() - 1);
        Ok(VaultCreation {
            recovery_phrase: created.recovery_phrase,
            kpub,
        })
    }

    fn add_wallet(&mut self, wallet: HotWallet) -> Result<String, VaultRuntimeError> {
        let kpub = wallet.export_kpub().map_err(VaultRuntimeError::Custody)?;
        self.clear_signing_session();
        self.covenant.clear_all();
        self.wallets.push(wallet);
        self.wallet_names
            .push(default_wallet_name(self.wallets.len() - 1));
        self.active_wallet = Some(self.wallets.len() - 1);
        Ok(kpub)
    }

    fn push_wallet_summary(
        &mut self,
        wallet: HotWallet,
    ) -> Result<WalletSummary, VaultRuntimeError> {
        self.clear_signing_session();
        self.covenant.clear_all();
        self.wallets.push(wallet);
        let index = self.wallets.len() - 1;
        self.wallet_names.push(default_wallet_name(index));
        self.active_wallet = Some(index);
        let wallet = &self.wallets[index];
        Ok(WalletSummary {
            index,
            name: self.wallet_names[index].clone(),
            active: true,
            kind: wallet.wallet_kind(),
            fingerprint: wallet.fingerprint_hex().ok(),
            kpub: wallet.export_kpub().ok(),
        })
    }

    fn ensure_wallet_capacity(&self) -> Result<(), VaultRuntimeError> {
        if self.wallets.len() >= MAX_SOFTWARE_WALLETS {
            Err(VaultRuntimeError::WalletCapacity)
        } else {
            Ok(())
        }
    }
}

impl From<MessageSignature> for SignedMessage {
    fn from(value: MessageSignature) -> Self {
        Self {
            signature: value.signature,
            digest: value.digest,
        }
    }
}

impl From<CommittedSecret> for SecretCommitment {
    fn from(value: CommittedSecret) -> Self {
        Self {
            payload: value.payload,
            commitment: value.commitment,
        }
    }
}

impl From<MultisigView> for MultisigResult {
    fn from(value: MultisigView) -> Self {
        Self {
            descriptor: value.descriptor,
            address: value.address,
            threshold: value.threshold,
            participants: value.participants,
            chain: value.chain,
            index: value.index,
        }
    }
}

fn default_wallet_name(index: usize) -> String {
    format!("Wallet {}", index + 1)
}

pub(crate) fn validate_wallet_name(name: &str) -> Result<String, VaultRuntimeError> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.len() > WALLET_NAME_MAX
        || trimmed.chars().any(char::is_control)
    {
        return Err(VaultRuntimeError::InvalidWalletName);
    }
    Ok(trimmed.to_owned())
}

fn parse_network(network: &str) -> Result<WalletNetwork, VaultRuntimeError> {
    WalletNetwork::from_name(&network.to_ascii_lowercase()).ok_or(VaultRuntimeError::InvalidNetwork)
}
