// Cryptographic entropy façade.
//
// All hardware sampling and structural health tests live in services::entropy.
// This module remains as the crypto-facing boundary so callers do not access
// device registers directly.

pub use crate::services::entropy::EntropyError;

/// Fill `out` with health-checked hardware entropy.
///
/// Callers must handle failure; there is deliberately no infallible fallback.
pub fn fill(out: &mut [u8]) -> Result<(), EntropyError> {
    crate::services::entropy::fill(out)
}

/// Fresh health-checked entropy bound to an anti-klepto host commitment, for
/// a provisional nonce. A repeated hardware sample then cannot repeat a nonce
/// that the host could finalize under a second secret.
pub fn fill_bound_to(host_commitment: &[u8; 32]) -> Result<[u8; 32], EntropyError> {
    let mut device = [0u8; 32];
    fill(&mut device)?;
    let bound = offline_signer::crypto::anti_klepto::session_signing_entropy(&device, host_commitment);
    shared_signer::bytes::zeroize_bytes(&mut device);
    Ok(bound)
}
