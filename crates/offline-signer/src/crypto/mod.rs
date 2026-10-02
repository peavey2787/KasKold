//! Cryptographic primitives. The signing core comes from Kaspa Portal; the
//! encrypted-container formats below are KasKold's own.

pub use kaspa_portal::crypto::{adaptor, anti_klepto, ecies, kdf::password as password_kdf, message, schnorr};

pub mod container_framing;
pub mod credential;
pub mod device_bound_storage;

#[cfg(test)]
#[path = "unit_tests/external_input_hardening.rs"]
mod external_input_hardening_tests;
