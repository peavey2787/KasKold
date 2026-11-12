//! Isolated runtime state for Private Swap v2 adaptor-signature authorization.
//!
//! The phase alone says which request is in flight: a key handout, or one of
//! the three owner-approved actions. Only a pre-signature waits for a host
//! reveal, so that wait carries whether its nonce QR was shown.

use offline_signer::crypto::adaptor::AdaptorPreSignature;

/// An action the owner approves on the review screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateSwapAction {
    Bind,
    PreSign,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateSwapPhase {
    Idle,
    /// A fresh swap key's details are ready to show.
    KeyReady,
    /// An action waits for the owner's approval.
    Prepared(PrivateSwapAction),
    /// The pre-signature nonce is committed; the host's reveal is due once
    /// its QR has been shown.
    AwaitingReveal {
        nonce_shown: bool,
    },
    /// The action's response is ready to show.
    FinalResponse(PrivateSwapAction),
}

impl PrivateSwapPhase {
    /// The approved action this phase belongs to, if any.
    #[must_use]
    pub const fn action(self) -> Option<PrivateSwapAction> {
        match self {
            Self::Prepared(action) | Self::FinalResponse(action) => Some(action),
            Self::AwaitingReveal { .. } => Some(PrivateSwapAction::PreSign),
            Self::Idle | Self::KeyReady => None,
        }
    }
}

const NO_PRESIGNATURE: AdaptorPreSignature = AdaptorPreSignature { bytes: [0; 64], negated: false };

pub struct PrivateSwapState {
    pub phase: PrivateSwapPhase,
    pub session_id: [u8; 16],
    pub host_commitment: [u8; 32],
    pub key_id: [u8; 32],
    pub claim_pubkey: [u8; 32],
    pub binding_token: [u8; 32],
    pub adaptor_point: [u8; 32],
    pub script_hash: [u8; 32],
    pub sighash: [u8; 32],
    pub nonce_point: [u8; 33],
    pub aux_rand: [u8; 32],
    pub presignature: AdaptorPreSignature,
    pub completed_signature: [u8; 64],
    pub input_amount: u64,
    pub output_amount: u64,
    pub fee: u64,
    pub refund_locktime_daa: u64,
    pub destination_hash: [u8; 32],
    pub response: [u8; shared_signer::covenant_sign::private_swap::RESPONSE_LEN],
    pub response_len: usize,
    /// One fresh, device-allocated swap covenant key waiting to be bound.
    pub pending_key_id: [u8; 32],
    pub pending_pubkey: [u8; 32],
    pub pending_adaptor_point: [u8; 32],
}

impl PrivateSwapState {
    pub(super) const fn new() -> Self {
        Self {
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
            presignature: NO_PRESIGNATURE,
            completed_signature: [0; 64],
            input_amount: 0,
            output_amount: 0,
            fee: 0,
            refund_locktime_daa: 0,
            destination_hash: [0; 32],
            response: [0; shared_signer::covenant_sign::private_swap::RESPONSE_LEN],
            response_len: 0,
            pending_key_id: [0; 32],
            pending_pubkey: [0; 32],
            pending_adaptor_point: [0; 32],
        }
    }

    pub fn reset(&mut self) {
        shared_signer::bytes::zeroize_bytes(&mut self.session_id);
        shared_signer::bytes::zeroize_bytes(&mut self.host_commitment);
        shared_signer::bytes::zeroize_bytes(&mut self.key_id);
        shared_signer::bytes::zeroize_bytes(&mut self.claim_pubkey);
        shared_signer::bytes::zeroize_bytes(&mut self.binding_token);
        shared_signer::bytes::zeroize_bytes(&mut self.adaptor_point);
        shared_signer::bytes::zeroize_bytes(&mut self.script_hash);
        shared_signer::bytes::zeroize_bytes(&mut self.sighash);
        shared_signer::bytes::zeroize_bytes(&mut self.nonce_point);
        shared_signer::bytes::zeroize_bytes(&mut self.aux_rand);
        shared_signer::bytes::zeroize_bytes(&mut self.presignature.bytes);
        shared_signer::bytes::zeroize_bytes(&mut self.completed_signature);
        shared_signer::bytes::zeroize_bytes(&mut self.destination_hash);
        shared_signer::bytes::zeroize_bytes(&mut self.response);
        self.phase = PrivateSwapPhase::Idle;
        self.presignature.negated = false;
        self.input_amount = 0;
        self.output_amount = 0;
        self.fee = 0;
        self.refund_locktime_daa = 0;
        self.response_len = 0;
    }

    pub fn replace_pending(&mut self, key_id: [u8; 32], pubkey: [u8; 32], adaptor: [u8; 32]) {
        self.pending_key_id = key_id;
        self.pending_pubkey = pubkey;
        self.pending_adaptor_point = adaptor;
    }

    pub fn clear_pending(&mut self) {
        shared_signer::bytes::zeroize_bytes(&mut self.pending_key_id);
        shared_signer::bytes::zeroize_bytes(&mut self.pending_pubkey);
        shared_signer::bytes::zeroize_bytes(&mut self.pending_adaptor_point);
    }
}
