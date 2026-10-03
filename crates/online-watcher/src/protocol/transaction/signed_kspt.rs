//! Signed compact-KSPT consumer boundary.
//!
//! Production decoding authorizes the compact transaction once in
//! `kaskold-protocol` and materializes only that typed verified model. The raw
//! KSPT bytes are never parsed a second time after authorization.

use crate::protocol::transaction::consensus::{ConsensusTransaction, InputEncoding};

pub fn decode_signed_kspt(signed_hex: &str) -> Result<ConsensusTransaction, String> {
    let bytes = hex::decode(signed_hex).map_err(|error| format!("Invalid hex: {error}"))?;
    let verified = kaskold_protocol::compat::verify_complete_kspt(&bytes)?;
    super::verified::materialize_verified_transaction(verified, InputEncoding::Compact)
}
