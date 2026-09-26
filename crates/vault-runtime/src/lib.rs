//! Network-free signing facade for KasKold Vault mobile applications.
//!
//! This crate intentionally contains no RPC, HTTP, WebSocket, resolver, or
//! broadcast dependency. Native Android/iOS shells provide local storage,
//! camera, QR display, and platform authentication around this state machine.

pub use covenant_signing::{CovenantMode, CovenantPhase, CovenantReview};
pub use hot_wallet::{PrivateSwapMode, PrivateSwapReview, WalletKind};
pub use kaskold_protocol::{QrFrame, QrProgress};

use hot_wallet::{HotWallet, HotWalletError, TransactionReview};
use kaskold_protocol::QrDecoder;
use shared_signer::{
    advanced_policy::SigningPolicy,
    seed_qr::{encode_compact_seedqr, encode_seedqr},
};
use zeroize::{Zeroize, Zeroizing};

pub use creation_flow::creation_flow_json;
pub use wallet_tools::{
    MultisigResult, SecretCommitment, SignedMessage, WalletSummary, MAX_SOFTWARE_WALLETS,
};

/// Wallet creation result. The recovery phrase is displayed by the explicit
/// creation backup flow and is zeroized on Rust drop; authenticated backup
/// operations can reveal the retained mnemonic again later.
pub struct VaultCreation {
    pub recovery_phrase: Zeroizing<String>,
    pub kpub: String,
}

/// Transaction data safe to show on the native review screen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultReviewInput {
    pub index: usize,
    pub outpoint: String,
    pub amount: u64,
    pub script_type: &'static str,
    pub address: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultReviewOutput {
    pub index: usize,
    pub amount: u64,
    pub ownership: &'static str,
    pub address: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultReview {
    pub network: &'static str,
    pub input_count: usize,
    pub output_count: usize,
    pub input_total: u64,
    pub output_total: u64,
    pub external_total: u64,
    pub change_total: u64,
    pub own_receive_total: u64,
    pub fee: u64,
    pub inputs: Vec<VaultReviewInput>,
    pub outputs: Vec<VaultReviewOutput>,
}

impl From<TransactionReview> for VaultReview {
    fn from(value: TransactionReview) -> Self {
        Self {
            network: value.network.label(),
            input_count: value.input_count,
            output_count: value.output_count,
            input_total: value.input_total,
            output_total: value.output_total,
            external_total: value.external_total,
            change_total: value.change_total,
            own_receive_total: value.own_receive_total,
            fee: value.fee,
            inputs: value
                .inputs
                .into_iter()
                .map(|input| VaultReviewInput {
                    index: input.index,
                    outpoint: input.outpoint,
                    amount: input.amount,
                    script_type: input.script_type,
                    address: input.address,
                })
                .collect(),
            outputs: value
                .outputs
                .into_iter()
                .map(|output| VaultReviewOutput {
                    index: output.index,
                    amount: output.amount,
                    ownership: output.ownership.label(),
                    address: output.address,
                })
                .collect(),
        }
    }
}

/// Result of accepting a QR frame. Completion moves the request into review
/// state but never signs it automatically.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScanResult {
    Progress(QrProgress),
    Ready(VaultReview),
}

/// Network-free Vault state machine.
///
/// The state transition is intentionally strict:
/// `scan -> review -> explicit approve -> signed response frames`.
/// Any new scan or rejection clears the prior pending/approved transaction.
pub struct VaultRuntime {
    wallets: Vec<HotWallet>,
    wallet_names: Vec<String>,
    active_wallet: Option<usize>,
    decoder: QrDecoder,
    pending_request: Option<Zeroizing<Vec<u8>>>,
    signed_response_frames: Vec<QrFrame>,
    signed_response_wire: Option<Zeroizing<Vec<u8>>>,
    anti_klepto: anti_klepto::AntiKleptoState,
    private_swap: private_swap::PrivateSwapState,
    covenant: covenant_signing::CovenantSession,
    signing_policy: SigningPolicy,
}

impl Default for VaultRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl VaultRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self {
            wallets: Vec::new(),
            wallet_names: Vec::new(),
            active_wallet: None,
            decoder: QrDecoder::new(),
            pending_request: None,
            signed_response_frames: Vec::new(),
            signed_response_wire: None,
            anti_klepto: anti_klepto::AntiKleptoState::new(),
            private_swap: private_swap::PrivateSwapState::new(),
            covenant: covenant_signing::CovenantSession::new(),
            signing_policy: SigningPolicy::disabled(),
        }
    }

    #[must_use]
    pub fn is_unlocked(&self) -> bool {
        self.active_wallet.is_some()
    }

    /// Export only the public account; private material has no accessor.
    pub fn export_public_account(&self) -> Result<String, VaultRuntimeError> {
        self.active_wallet()?
            .export_kpub()
            .map_err(VaultRuntimeError::Custody)
    }

    /// Reveal the BIP39 words only for an explicit backup operation.
    pub fn backup_recovery_phrase(&self) -> Result<Zeroizing<String>, VaultRuntimeError> {
        self.active_wallet()?
            .backup_recovery_phrase()
            .map_err(VaultRuntimeError::Custody)
    }

    /// SeedSigner-compatible numeric SeedQR payload used by the hardware
    /// Wallet -> Backup -> SeedQR Backup workflow.
    pub fn backup_seedqr(&self) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        let wallet = self.active_wallet()?;
        let mut indices = [0u16; 24];
        let word_count = wallet
            .backup_mnemonic_indices(&mut indices)
            .map_err(VaultRuntimeError::Custody)?;
        let mut encoded = [0u8; 96];
        let len = encode_seedqr(&indices, word_count, &mut encoded);
        indices.zeroize();
        if len == 0 {
            return Err(VaultRuntimeError::BackupEncoding);
        }
        let payload = Zeroizing::new(encoded[..len].to_vec());
        encoded.zeroize();
        Ok(payload)
    }

    /// SeedSigner-compatible CompactSeedQR entropy used by the hardware
    /// Wallet -> Backup -> Advanced -> Compact SeedQR workflow.
    pub fn backup_compact_seedqr(&self) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        let wallet = self.active_wallet()?;
        let mut indices = [0u16; 24];
        let word_count = wallet
            .backup_mnemonic_indices(&mut indices)
            .map_err(VaultRuntimeError::Custody)?;
        let mut encoded = [0u8; 32];
        let len = encode_compact_seedqr(&indices, word_count, &mut encoded);
        indices.zeroize();
        if len == 0 {
            return Err(VaultRuntimeError::BackupEncoding);
        }
        let payload = Zeroizing::new(encoded[..len].to_vec());
        encoded.zeroize();
        Ok(payload)
    }

    /// Account-level XPrv for the explicit advanced backup workflow.
    pub fn backup_account_xprv(&self) -> Result<Zeroizing<String>, VaultRuntimeError> {
        self.active_wallet()?
            .backup_account_xprv()
            .map_err(VaultRuntimeError::Custody)
    }

    /// Receive-chain private key export matching the hardware advanced backup
    /// route. Callers must treat the returned value as an explicit secret.
    pub fn backup_receive_private_key_hex(
        &self,
        address_index: u16,
    ) -> Result<Zeroizing<String>, VaultRuntimeError> {
        self.active_wallet()?
            .backup_receive_private_key_hex(address_index)
            .map_err(VaultRuntimeError::Custody)
    }

    fn active_wallet(&self) -> Result<&HotWallet, VaultRuntimeError> {
        let index = self.active_wallet.ok_or(VaultRuntimeError::Locked)?;
        self.wallets.get(index).ok_or(VaultRuntimeError::Locked)
    }

    fn active_wallet_mut(&mut self) -> Result<&mut HotWallet, VaultRuntimeError> {
        let index = self.active_wallet.ok_or(VaultRuntimeError::Locked)?;
        self.wallets.get_mut(index).ok_or(VaultRuntimeError::Locked)
    }
}

impl Drop for VaultRuntime {
    fn drop(&mut self) {
        self.clear_signing_session();
        self.covenant.clear_all();
        self.private_swap.clear_all();
        self.wallets.clear();
        self.wallet_names.clear();
        self.active_wallet = None;
    }
}

pub enum PrivateSwapPrepareResult {
    Review(Box<PrivateSwapReview>),
    Response(Zeroizing<Vec<u8>>),
}

#[derive(Debug)]
pub enum VaultRuntimeError {
    Locked,
    NoPendingReview,
    NoSignedResponse,
    SigningSessionAlreadyComplete,
    BackupEncoding,
    InvalidWalletIndex,
    InvalidWalletName,
    InvalidSealedInventory,
    WalletCapacity,
    InvalidNetwork,
    InvalidLocalTransaction,
    InvalidCovenantBackup,
    InvalidCovenantMessage,
    InvalidCovenantState,
    InvalidCovenantContext,
    CovenantBindingRequired,
    CovenantBindingMismatch,
    CovenantRevealMismatch,
    NoCovenantResponse,
    AntiKleptoRevealNotExpected,
    InvalidSigningPolicy,
    SigningBlockedNotBefore,
    SigningBlockedWeeklyWindow,
    SigningBlockedClockRollback,
    SigningBlockedClockInvalid,
    Custody(HotWalletError),
    Protocol(kaskold_protocol::ProtocolError),
}

mod anti_klepto;
mod covenant_signing;
mod creation_flow;
mod inventory_seal;
mod private_swap;
mod signing_policy;
mod signing_session;
mod wallet_lifecycle;
mod wallet_tools;

#[cfg(any(test, target_os = "android", target_os = "ios", target_os = "macos"))]
mod native_ffi;

#[cfg(test)]
#[path = "unit_tests/mod.rs"]
mod unit_tests;
