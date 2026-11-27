//! Per-input signing entropy: fresh from the device, and bound to the host's
//! commitment while an anti-klepto review is open.

use crate::crypto::entropy::{self, EntropyError};
use crate::runtime::data::{AntiKleptoPhase, AppData};

/// Fresh device entropy for one input; under anti-klepto review it is bound
/// to the host's commitment.
pub(super) fn input_signing_entropy(ad: &AppData) -> Result<[u8; 32], EntropyError> {
    match ad.signing.anti_klepto.session_in(AntiKleptoPhase::Reviewing) {
        Some(session) => entropy::fill_bound_to(&session.host_commitment),
        None => {
            let mut device = [0u8; 32];
            entropy::fill(&mut device)?;
            Ok(device)
        }
    }
}
