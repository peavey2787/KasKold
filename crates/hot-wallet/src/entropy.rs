//! Cryptographically secure randomness for software-wallet custody operations.
//!
//! Keep OS/browser CSPRNG access behind this module so wallet creation,
//! signing, backup, covenant, and privacy flows share one fail-closed source.

use crate::HotWalletError;

pub(crate) fn fill_random(output: &mut [u8]) -> Result<(), HotWalletError> {
    getrandom::getrandom(output).map_err(|_| HotWalletError::EntropyUnavailable)
}
