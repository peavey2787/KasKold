//! Cryptographically secure randomness for software-wallet custody operations.
//!
//! Keep OS/browser CSPRNG access behind this module so wallet creation,
//! signing, backup, covenant, and privacy flows share one fail-closed source.

use crate::HotWalletError;

pub(crate) fn fill_random(output: &mut [u8]) -> Result<(), HotWalletError> {
    getrandom::getrandom(output).map_err(|_| HotWalletError::EntropyUnavailable)
}

/// Fresh entropy bound to an anti-klepto host commitment, for a provisional
/// nonce: a repeated CSPRNG sample then cannot repeat a nonce that the host
/// could finalize under a second secret.
pub(crate) fn fill_random_bound_to(host_commitment: &[u8; 32]) -> Result<[u8; 32], HotWalletError> {
    let mut device = [0u8; 32];
    fill_random(&mut device)?;
    let bound =
        offline_signer::crypto::anti_klepto::session_signing_entropy(&device, host_commitment);
    zeroize::Zeroize::zeroize(&mut device);
    Ok(bound)
}
