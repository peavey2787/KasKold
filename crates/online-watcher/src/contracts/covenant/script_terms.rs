//! Covenant terms read from redeem scripts: every term the wallet shows for an
//! invite or a stored covenant comes from its script, never from a record's
//! own claims.
//!
//! A record carries a covenant type, a redeem script and an address. The
//! script must be exactly what that family's builder produces and must hash to
//! the address on the selected network; the terms are then read back out of
//! the script. Types with no creation builder are refused.

use kaspa_portal::primitives::address::{
    encode_p2pk_address, encode_p2sh_address, p2pk_xonly, p2sh_hash,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::account::address::network_prefix;
use crate::contracts::covenant::script as covenant;

type Terms = Map<String, Value>;
type Reader = fn(&NetworkScript) -> Option<Terms>;

/// A redeem script and the address prefix of the network it is spent on.
struct NetworkScript<'a> {
    bytes: &'a [u8],
    prefix: &'a str,
}

const ORACLE_V1: &str = "oracle-v1";
const STATEMENT_PREFIX: &str = "KaspaPortal Oracle v1 ";

const FAMILIES: [(&str, Reader); 8] = [
    ("dms", dms_terms),
    ("timelocked-savings", timelocked_savings_terms),
    ("additive", piggy_bank_terms),
    ("escrow", escrow_terms),
    ("global-allowance", global_allowance_terms),
    ("global-spending-limit", global_spending_limit_terms),
    ("payjoin", payjoin_terms),
    (ORACLE_V1, oracle_v1_terms),
];

/// Verify a covenant record and return its terms as read from the redeem script.
///
/// `attestation_statement` is the record's oracle statement; it is only read
/// for `oracle-v1`, where it must hash to the script's message commitment.
pub(crate) fn verified_terms_json(
    covenant_type: &str,
    redeem_script_hex: &str,
    address: &str,
    network: &str,
    attestation_statement: &str,
) -> Result<String, String> {
    let reader = family_reader(covenant_type)?;
    let script = hex::decode(redeem_script_hex)
        .map_err(|error| format!("Covenant redeem script is not hex: {error}"))?;
    let prefix = network_prefix(network)?;
    let mut terms = reader(&NetworkScript {
        bytes: &script,
        prefix,
    })
    .ok_or_else(|| format!("Redeem script is not a {covenant_type} covenant"))?;
    if crate::protocol::script::p2sh::script_to_address(&script, prefix)? != address {
        return Err("Covenant address does not match its redeem script".to_string());
    }
    if covenant_type == ORACLE_V1 {
        bind_attestation_statement(&mut terms, attestation_statement)?;
    }
    terms.insert("type".into(), json!(covenant_type));
    terms.insert("address".into(), json!(address));
    terms.insert("redeem_script_hex".into(), json!(hex::encode(&script)));
    Ok(Value::Object(terms).to_string())
}

fn family_reader(covenant_type: &str) -> Result<Reader, String> {
    FAMILIES
        .iter()
        .find(|(name, _)| *name == covenant_type)
        .map(|(_, reader)| *reader)
        .ok_or_else(|| format!("Unsupported covenant type: {covenant_type:?}"))
}

fn terms(value: Value) -> Option<Terms> {
    match value {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

fn dms_terms(script: &NetworkScript) -> Option<Terms> {
    let fields = covenant::dms_script_fields(script.bytes)?;
    terms(json!({
        "owner_pubkey_hex": hex::encode(fields.owner_pubkey),
        "heir_pubkey_hex": hex::encode(fields.heir_pubkey),
        "inactivity_daa": fields.inactivity_daa.to_string(),
    }))
}

fn timelocked_savings_terms(script: &NetworkScript) -> Option<Terms> {
    let fields = covenant::timelocked_savings_script_fields(script.bytes)?;
    terms(json!({
        "wallet1_pubkey_hex": hex::encode(fields.wallet1_pubkey),
        "wallet2_pubkey_hex": hex::encode(fields.wallet2_pubkey),
        "locktime_daa": fields.locktime_daa.to_string(),
    }))
}

fn piggy_bank_terms(script: &NetworkScript) -> Option<Terms> {
    let fields = covenant::piggy_bank_script_fields(script.bytes)?;
    terms(json!({
        "owner_pubkey_hex": hex::encode(fields.owner_pubkey),
        "threshold_sompi": fields.threshold_sompi.to_string(),
        "deadline_daa": fields.deadline_daa.to_string(),
        "salt": hex::encode(fields.salt),
    }))
}

fn escrow_terms(script: &NetworkScript) -> Option<Terms> {
    let fields = covenant::escrow_script_fields(script.bytes)?;
    terms(json!({
        "alice_pk": hex::encode(fields.alice_pubkey),
        "bob_pk": hex::encode(fields.bob_pubkey),
        "arbiter_pk": hex::encode(fields.arbiter_pubkey),
        "alice_address": script_pubkey_address(&fields.alice_spk, script.prefix)?,
        "bob_address": script_pubkey_address(&fields.bob_spk, script.prefix)?,
        "salt": hex::encode(fields.salt),
    }))
}

fn global_allowance_terms(script: &NetworkScript) -> Option<Terms> {
    let fields = covenant::global_allowance_script_fields(script.bytes)?;
    terms(json!({
        "owner_pubkey_hex": hex::encode(fields.owner_pubkey),
        "beneficiary_pubkey_hex": hex::encode(fields.beneficiary_pubkey),
        "max_withdraw_sompi": fields.max_withdraw_sompi.to_string(),
        "cooldown_daa": fields.cooldown_daa.to_string(),
        "start_daa": fields.start_daa.to_string(),
        "salt": hex::encode(fields.salt),
    }))
}

fn global_spending_limit_terms(script: &NetworkScript) -> Option<Terms> {
    let fields = covenant::global_spending_limit_script_fields(script.bytes)?;
    terms(json!({
        "owner_pubkey_hex": hex::encode(fields.owner_pubkey),
        "max_withdraw_sompi": fields.max_withdraw_sompi.to_string(),
        "cooldown_daa": fields.cooldown_daa.to_string(),
        "salt": hex::encode(fields.salt),
    }))
}

fn payjoin_terms(script: &NetworkScript) -> Option<Terms> {
    let fields = covenant::payjoin_script_fields(script.bytes)?;
    terms(json!({
        "owner_pubkey_hex": hex::encode(fields.owner_pubkey),
        "beneficiary_pubkey_hex": hex::encode(fields.beneficiary_pubkey),
        "locktime_daa": fields.locktime_daa.to_string(),
        "min_inputs": fields.min_inputs.to_string(),
        "min_outputs": fields.min_outputs.to_string(),
    }))
}

fn oracle_v1_terms(script: &NetworkScript) -> Option<Terms> {
    let fields = covenant::oracle_v1_script_fields(script.bytes)?;
    terms(json!({
        "owner_pubkey_hex": hex::encode(fields.owner_pubkey),
        "beneficiary_pubkey_hex": hex::encode(fields.beneficiary_pubkey),
        "oracle_pubkey_hex": hex::encode(fields.oracle_pubkey),
        "message_commitment_hex": hex::encode(fields.message_commitment),
        "locktime_daa": fields.locktime_daa.to_string(),
        "salt": hex::encode(fields.salt),
    }))
}

/// The address an escrow pays out to; only standard P2PK and P2SH outputs are
/// recognized, so an unreadable destination refuses the whole invite.
fn script_pubkey_address(script_pubkey: &[u8], prefix: &str) -> Option<String> {
    if let Some(xonly) = p2pk_xonly(script_pubkey) {
        return Some(encode_p2pk_address(&xonly, prefix));
    }
    p2sh_hash(script_pubkey).map(|hash| encode_p2sh_address(&hash, prefix))
}

/// The statement must be the one the script commits to: bound to the
/// covenant's salt and hashing to its message commitment.
fn bind_attestation_statement(terms: &mut Terms, statement: &str) -> Result<(), String> {
    let salt = terms
        .get("salt")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let commitment = terms
        .get("message_commitment_hex")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let bound = statement.starts_with(&format!("{STATEMENT_PREFIX}{salt}: "));
    if !bound || hex::encode(Sha256::digest(statement.as_bytes())) != commitment {
        return Err("Oracle statement does not match the covenant commitment".to_string());
    }
    terms.insert("attestation_statement".into(), json!(statement));
    Ok(())
}
