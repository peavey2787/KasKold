//! PSKT/KSPT translation and response-processing primitives.

mod compact;
mod relay;
mod relay_fields;
mod schema_validate;
mod specialized;
pub(crate) mod verified;
mod verified_finalize;
mod wire;

use crate::{AddressBranch, Network};

pub fn encode_pskt(pskt_hex: &str, network: Network) -> Result<Vec<u8>, String> {
    relay::encode_pskt(pskt_hex, network)
}

pub fn merge_signed_kspt(
    original_pskt_hex: &str,
    signed_kspt: &[u8],
    network: Network,
) -> Result<String, String> {
    compact::validate_and_merge(original_pskt_hex, signed_kspt, network)
}

pub fn is_complete(pskt_hex: &str, network: Network) -> Result<bool, String> {
    relay::is_complete(pskt_hex, network)
}

pub fn verified_signature_counts(pskt_hex: &str, network: Network) -> Result<Vec<u8>, String> {
    relay::verified_signature_counts(pskt_hex, network)
}

pub fn finalize_json(pskt_hex: &str) -> Result<String, String> {
    // Parse and authorize exactly once. The final transaction is serialized only
    // from this typed result; the original PSKT is never reparsed after approval.
    let transaction = relay::verify_complete_transaction(pskt_hex, Network::Mainnet)?;
    let verified = verified::from_compact(transaction)?;
    verified_finalize::finalize_json(&verified)
}

pub fn attach_input_derivation(
    pskt_hex: &str,
    input_index: usize,
    branch: AddressBranch,
    index: u32,
) -> Result<String, String> {
    wire::attach_input_derivation(pskt_hex, input_index, branch, index)
}

pub fn attach_output_derivation(
    pskt_hex: &str,
    output_index: usize,
    branch: AddressBranch,
    index: u32,
) -> Result<String, String> {
    wire::attach_output_derivation(pskt_hex, output_index, branch, index)
}

#[cfg(feature = "companion-compat")]
pub(crate) fn compat_decode_json_body(body_hex: &[u8]) -> Result<serde_json::Value, String> {
    wire::decode_json_body_hex(body_hex)
}

#[cfg(feature = "companion-compat")]
pub(crate) fn compat_encode_json_body(root: &serde_json::Value) -> Result<Vec<u8>, String> {
    wire::encode_json_body_hex(root)
}

#[cfg(feature = "companion-compat")]
pub(crate) fn compat_verify_complete_kspt(
    data: &[u8],
) -> Result<verified::VerifiedTransaction, String> {
    let transaction = compact::parse(data)?;
    if !crate::wire::pskt_schema::supported_tx_version(transaction.version) {
        return Err(format!(
            "unsupported transaction version: {}",
            transaction.version
        ));
    }
    if transaction.flags != crate::wire::kspt::FLAG_SIGNED_OR_COMPLETE {
        return Err("Compact KSPT is not marked fully signed".to_string());
    }
    if transaction.inputs.iter().any(|input| {
        !input.redeem.is_empty() && relay_fields::parse_multisig_redeem(&input.redeem).is_none()
    }) {
        return Err(
            "Raw covenant KSPT broadcast is disabled; merge the signed KSPT into its original PSKT so the typed covenant witness plan can be verified"
                .to_string(),
        );
    }
    if !compact::verified_complete(&transaction)? {
        return Err("Compact KSPT is not cryptographically complete".to_string());
    }
    verified::from_compact(transaction)
}

#[cfg(feature = "companion-compat")]
pub(crate) fn compat_verify_complete_pskt(
    pskt_hex: &str,
    network: Network,
) -> Result<verified::VerifiedTransaction, String> {
    let transaction = relay::verify_complete_transaction(pskt_hex, network)?;
    verified::from_compact(transaction)
}

#[cfg(feature = "companion-compat")]
pub(crate) const COMPAT_MAX_PSKT_JSON_BYTES: usize = wire::MAX_PSKT_JSON_BYTES;

#[cfg(feature = "companion-compat")]
pub(crate) const COMPAT_MAX_PSKT_WIRE_HEX_CHARS: usize = wire::MAX_PSKT_WIRE_HEX_CHARS;

#[cfg(test)]
pub(crate) mod test_support {
    pub(crate) use super::relay_fields::{
        find_pubkey_position, parse_ms45, parse_multisig_redeem, InputFields,
    };
    pub(crate) use super::wire::{
        decode, document, encode, parse_derivation, Format, MAX_PSKT_WIRE_HEX_CHARS,
    };

    pub(crate) fn sighash_all_for_pskt(
        pskt_hex: &str,
        network: crate::Network,
        input_index: usize,
    ) -> Result<[u8; 32], String> {
        super::compact::test_sighash_all_for_pskt(pskt_hex, network, input_index)
    }

    pub(crate) fn compact_covenant_execution_for_test(
        data: &[u8],
        input_index: usize,
    ) -> Result<Option<(u16, u16)>, String> {
        let transaction = super::compact::parse(data)?;
        transaction
            .inputs
            .get(input_index)
            .map(|input| input.covenant_execution)
            .ok_or_else(|| "KSPT input index out of range".to_string())
    }
}
