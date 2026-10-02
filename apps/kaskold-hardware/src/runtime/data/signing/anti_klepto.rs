//! Anti-klepto signing-session state.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AntiKleptoPhase {
    Inactive,
    Reviewing,
    AwaitingReveal,
    FinalResponse,
}

pub struct AntiKleptoSigningState {
    pub phase: AntiKleptoPhase,
    pub session_id: [u8; shared_signer::anti_klepto::SESSION_ID_LEN],
    pub host_commitment: [u8; 32],
    pub transaction_digest: [u8; 32],
    pub initial_sig_counts: [u8; kaskold_protocol::SIGNER_MAX_INPUTS],
}

impl AntiKleptoSigningState {
    pub(super) fn new() -> Self {
        Self {
            phase: AntiKleptoPhase::Inactive,
            session_id: [0; shared_signer::anti_klepto::SESSION_ID_LEN],
            host_commitment: [0; 32],
            transaction_digest: [0; 32],
            initial_sig_counts: [0u8; kaskold_protocol::SIGNER_MAX_INPUTS],
        }
    }

    /// Start a session. Refuses (returns `false`) when the transaction has more
    /// inputs than the device signs, which parsing with the device limits
    /// already rules out.
    #[must_use]
    pub fn begin(
        &mut self,
        session_id: [u8; shared_signer::anti_klepto::SESSION_ID_LEN],
        host_commitment: [u8; 32],
        transaction_digest: [u8; 32],
        initial_sig_counts: &[u8],
    ) -> bool {
        let Some(slots) = self.initial_sig_counts.get_mut(..initial_sig_counts.len()) else {
            return false;
        };
        slots.copy_from_slice(initial_sig_counts);
        self.phase = AntiKleptoPhase::Reviewing;
        self.session_id = session_id;
        self.host_commitment = host_commitment;
        self.transaction_digest = transaction_digest;
        true
    }

    pub fn reset(&mut self) {
        shared_signer::bytes::zeroize_bytes(&mut self.session_id);
        shared_signer::bytes::zeroize_bytes(&mut self.host_commitment);
        shared_signer::bytes::zeroize_bytes(&mut self.transaction_digest);
        self.initial_sig_counts.fill(0);
        self.phase = AntiKleptoPhase::Inactive;
    }
}
