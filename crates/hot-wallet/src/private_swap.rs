//! Private Swap v2 authorization session shared by software Vaults.
//!
//! This is intentionally separate from ordinary transaction signing. Only the
//! isolated mnemonic-derived covenant hierarchy participates, exactly like the
//! M5 service: allocate key -> bind script -> anti-klepto adaptor pre-sign ->
//! complete adaptor signature.

use zeroize::Zeroize;

mod operations;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateSwapMode {
    None,
    KeyInfo,
    Bind,
    PreSign,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateSwapPhase {
    Idle,
    Prepared,
    AwaitingReveal,
    FinalResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrivateSwapReview {
    pub mode: PrivateSwapMode,
    pub key_id: [u8; 32],
    pub claim_pubkey: [u8; 32],
    pub adaptor_point: [u8; 32],
    pub script_hash: [u8; 32],
    pub sighash: [u8; 32],
    pub input_amount: u64,
    pub output_amount: u64,
    pub fee: u64,
    pub refund_locktime_daa: u64,
    pub destination_hash: [u8; 32],
}

pub enum PrivateSwapPrepared {
    Review(Box<PrivateSwapReview>),
    Response(Vec<u8>),
}

pub struct PrivateSwapSession {
    mode: PrivateSwapMode,
    phase: PrivateSwapPhase,
    session_id: [u8; 16],
    host_commitment: [u8; 32],
    key_id: [u8; 32],
    claim_pubkey: [u8; 32],
    binding_token: [u8; 32],
    adaptor_point: [u8; 32],
    script_hash: [u8; 32],
    sighash: [u8; 32],
    nonce_point: [u8; 33],
    aux_rand: [u8; 32],
    presignature: [u8; 64],
    presignature_negated: bool,
    input_amount: u64,
    output_amount: u64,
    fee: u64,
    refund_locktime_daa: u64,
    destination_hash: [u8; 32],
    pending_key_id: [u8; 32],
    pending_pubkey: [u8; 32],
    pending_adaptor_point: [u8; 32],
}

impl Default for PrivateSwapSession {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivateSwapSession {
    pub const fn new() -> Self {
        Self {
            mode: PrivateSwapMode::None,
            phase: PrivateSwapPhase::Idle,
            session_id: [0; 16],
            host_commitment: [0; 32],
            key_id: [0; 32],
            claim_pubkey: [0; 32],
            binding_token: [0; 32],
            adaptor_point: [0; 32],
            script_hash: [0; 32],
            sighash: [0; 32],
            nonce_point: [0; 33],
            aux_rand: [0; 32],
            presignature: [0; 64],
            presignature_negated: false,
            input_amount: 0,
            output_amount: 0,
            fee: 0,
            refund_locktime_daa: 0,
            destination_hash: [0; 32],
            pending_key_id: [0; 32],
            pending_pubkey: [0; 32],
            pending_adaptor_point: [0; 32],
        }
    }

    pub fn awaiting_reveal(&self) -> bool {
        self.mode == PrivateSwapMode::PreSign && self.phase == PrivateSwapPhase::AwaitingReveal
    }

    pub fn reset_active(&mut self) {
        self.session_id.zeroize();
        self.host_commitment.zeroize();
        self.key_id.zeroize();
        self.claim_pubkey.zeroize();
        self.binding_token.zeroize();
        self.adaptor_point.zeroize();
        self.script_hash.zeroize();
        self.sighash.zeroize();
        self.nonce_point.zeroize();
        self.aux_rand.zeroize();
        self.presignature.zeroize();
        self.destination_hash.zeroize();
        self.mode = PrivateSwapMode::None;
        self.phase = PrivateSwapPhase::Idle;
        self.presignature_negated = false;
        self.input_amount = 0;
        self.output_amount = 0;
        self.fee = 0;
        self.refund_locktime_daa = 0;
    }

    pub fn clear_all(&mut self) {
        self.reset_active();
        self.pending_key_id.zeroize();
        self.pending_pubkey.zeroize();
        self.pending_adaptor_point.zeroize();
    }
}

impl Drop for PrivateSwapSession {
    fn drop(&mut self) {
        self.clear_all();
    }
}
