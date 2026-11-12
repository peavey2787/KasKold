//! Anti-klepto signing-session state.
//!
//! A session exists only between the request scan and the final response, so
//! its identifiers, commitment and starting signature counts live inside the
//! active session and are wiped the moment it ends. The phase moves forward
//! only through [`AntiKleptoSigningState::await_reveal`] and
//! [`AntiKleptoSigningState::present_final`].

use shared_signer::anti_klepto::SESSION_ID_LEN;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AntiKleptoPhase {
    Inactive,
    Reviewing,
    AwaitingReveal,
    FinalResponse,
}

/// The facts one anti-klepto exchange is bound to.
pub struct AntiKleptoSession {
    phase: AntiKleptoPhase,
    pub session_id: [u8; SESSION_ID_LEN],
    pub host_commitment: [u8; 32],
    pub transaction_digest: [u8; 32],
    pub initial_sig_counts: [u8; kaskold_protocol::SIGNER_MAX_INPUTS],
}

impl Drop for AntiKleptoSession {
    fn drop(&mut self) {
        shared_signer::bytes::zeroize_bytes(&mut self.session_id);
        shared_signer::bytes::zeroize_bytes(&mut self.host_commitment);
        shared_signer::bytes::zeroize_bytes(&mut self.transaction_digest);
        shared_signer::bytes::zeroize_bytes(&mut self.initial_sig_counts);
    }
}

pub struct AntiKleptoSigningState {
    session: Option<AntiKleptoSession>,
}

impl AntiKleptoSigningState {
    pub(super) const fn new() -> Self {
        Self { session: None }
    }

    /// Start a session in review. Refuses (returns `false`) when the
    /// transaction has more inputs than the device signs, which parsing with
    /// the device limits already rules out.
    #[must_use]
    pub fn begin(
        &mut self,
        session_id: [u8; SESSION_ID_LEN],
        host_commitment: [u8; 32],
        transaction_digest: [u8; 32],
        initial_sig_counts: &[u8],
    ) -> bool {
        let mut counts = [0u8; kaskold_protocol::SIGNER_MAX_INPUTS];
        let Some(slots) = counts.get_mut(..initial_sig_counts.len()) else {
            return false;
        };
        slots.copy_from_slice(initial_sig_counts);
        self.session = Some(AntiKleptoSession {
            phase: AntiKleptoPhase::Reviewing,
            session_id,
            host_commitment,
            transaction_digest,
            initial_sig_counts: counts,
        });
        true
    }

    #[must_use]
    pub fn phase(&self) -> AntiKleptoPhase {
        self.session.as_ref().map_or(AntiKleptoPhase::Inactive, |session| session.phase)
    }

    #[must_use]
    pub const fn session(&self) -> Option<&AntiKleptoSession> {
        self.session.as_ref()
    }

    /// The active session, only while it is in `phase`.
    #[must_use]
    pub fn session_in(&self, phase: AntiKleptoPhase) -> Option<&AntiKleptoSession> {
        self.session.as_ref().filter(|session| session.phase == phase)
    }

    /// The approved commitment went out; the device now waits for the reveal.
    #[must_use]
    pub fn await_reveal(&mut self) -> bool {
        self.advance(AntiKleptoPhase::Reviewing, AntiKleptoPhase::AwaitingReveal)
    }

    /// The reveal checked out; the device now shows the final response.
    #[must_use]
    pub fn present_final(&mut self) -> bool {
        self.advance(AntiKleptoPhase::AwaitingReveal, AntiKleptoPhase::FinalResponse)
    }

    fn advance(&mut self, from: AntiKleptoPhase, to: AntiKleptoPhase) -> bool {
        match self.session.as_mut() {
            Some(session) if session.phase == from => {
                session.phase = to;
                true
            }
            _ => false,
        }
    }

    /// End the session, returning how many signatures each input carried when
    /// it began so the caller can take back the ones it added.
    #[must_use]
    pub fn end(&mut self) -> Option<[u8; kaskold_protocol::SIGNER_MAX_INPUTS]> {
        self.session.take().map(|session| session.initial_sig_counts)
    }

    pub fn reset(&mut self) {
        self.session = None;
    }
}
