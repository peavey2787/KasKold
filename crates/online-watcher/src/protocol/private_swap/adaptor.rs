//! Public-only adaptor-signature checks for Private Swap, from Kaspa Portal.
//!
//! This module never creates an adaptor pre-signature and never handles a
//! claim private key. Companion may verify pre-signatures, verify the host nonce
//! contribution, complete a stored pre-signature after the adaptor secret is
//! revealed on-chain, and extract that public secret from a completed signature.
//! The math and its domain separation are Kaspa Portal's, shared with the
//! hardware signer that produces the pre-signatures.

use kaspa_portal::crypto::{
    adaptor::{self, AdaptorError},
    schnorr::{schnorr_verify, SchnorrSignature},
};

pub(crate) use kaspa_portal::crypto::adaptor::AdaptorPreSignature;

fn message(error: AdaptorError) -> String {
    match error {
        AdaptorError::InvalidPrivateKey => "invalid adaptor private key",
        AdaptorError::InvalidAdaptorPoint => "invalid adaptor secret or point",
        AdaptorError::InvalidNonce => "invalid adaptor nonce",
        AdaptorError::InvalidPreSignature => "invalid adaptor pre-signature",
        AdaptorError::InvalidCompletedSignature => {
            "completed signature does not match adaptor pre-signature"
        }
        AdaptorError::InvalidHostContribution => "adaptor nonce does not include host contribution",
    }
    .to_string()
}

pub(crate) fn verify_presignature(
    public_x: &[u8; 32],
    message_hash: &[u8; 32],
    presig: &AdaptorPreSignature,
    adaptor_point_x: &[u8; 32],
) -> Result<(), String> {
    adaptor::verify_adaptor_presignature(public_x, message_hash, presig, adaptor_point_x)
        .map_err(message)
}

pub(crate) fn verify_host_nonce_relation(
    public_x: &[u8; 32],
    message_hash: &[u8; 32],
    adaptor_point_x: &[u8; 32],
    session_id: &[u8; 16],
    host_secret: &[u8; 32],
    base_nonce_point: &[u8; 33],
    presig: &AdaptorPreSignature,
) -> Result<(), String> {
    adaptor::verify_host_nonce_relation(
        public_x,
        message_hash,
        adaptor_point_x,
        session_id,
        host_secret,
        base_nonce_point,
        presig,
    )
    .map_err(message)
}

pub(crate) fn complete_presignature(
    presig: &AdaptorPreSignature,
    adaptor_secret: &[u8; 32],
) -> Result<[u8; 64], String> {
    adaptor::complete_adaptor_presignature(presig, adaptor_secret).map_err(message)
}

pub(crate) fn extract_secret(
    completed: &[u8; 64],
    presig: &AdaptorPreSignature,
) -> Result<[u8; 32], String> {
    adaptor::extract_adaptor_secret(completed, presig).map_err(message)
}

pub(crate) fn verify_bip340(
    public_x: &[u8; 32],
    message_hash: &[u8; 32],
    signature: &[u8; 64],
) -> Result<(), String> {
    schnorr_verify(
        public_x,
        message_hash,
        &SchnorrSignature { bytes: *signature },
    )
    .map_err(|_| "invalid completed BIP340 signature".to_string())
}

#[cfg(test)]
mod unit_tests;
