use crate::account::address::network_prefix;
use crate::wasm_api::utilities::common::js_error;
use crate::wasm_api::JsValue;
use crate::WatchWallet;
use kaspa_portal::transaction::interchange::pskt;

/// Inspect a hex payload (output of the multi-frame QR decoder) and
/// return the detected format as a short string: "pskb", "pskt", or
/// "unknown". JS uses this to route a decoded payload to either the
/// PSKT review screen (this module) or the compact KSPT return flow.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn pskt_detect(wire_hex: &str) -> String {
    match pskt::detect_format_hex(wire_hex) {
        pskt::PsktFormat::Pskb => "pskb".into(),
        pskt::PsktFormat::PsktSingle => "pskt".into(),
        pskt::PsktFormat::Unknown => "unknown".into(),
    }
}

/// Classify a returned compact KSPT by its cryptographically verified
/// signatures: "signed", "partial", "unsigned", or "unknown" when the hex is
/// not a KSPT this build accepts.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn kspt_signature_status(wire_hex: &str) -> String {
    use kaspa_portal::transaction::{
        interchange::kspt::{parse_compact_kspt, signature_status},
        model::Transaction,
    };
    let Ok(bytes) = hex::decode(wire_hex) else {
        return "unknown".into();
    };
    let Ok(mut transaction) = Transaction::try_new() else {
        return "unknown".into();
    };
    if parse_compact_kspt(&bytes, &mut transaction).is_err() {
        return "unknown".into();
    }
    match signature_status(&transaction) {
        (0, _) => "unsigned".into(),
        (present, required) if present >= required => "signed".into(),
        _ => "partial".into(),
    }
}

/// Parse a PSKT/PSKB payload into a review summary (JSON string).
///
/// `network` is one of "mainnet", "testnet-10/11/12", "simnet",
/// "devnet" — used to format decoded output addresses for display.
pub(super) fn pskt_summary_string(wire_hex: &str, network: &str) -> Result<String, String> {
    let prefix = network_prefix(network)?;
    let summary = pskt::parse_summary(wire_hex, prefix)?;
    serde_json::to_string(&summary).map_err(|error| error.to_string())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn pskt_summary(wire_hex: &str, network: &str) -> Result<String, JsValue> {
    pskt_summary_string(wire_hex, network).map_err(js_error)
}

/// Re-emit a PSKB/PSKT as a compact KSPT "partial" hex blob for relay to
/// KasKold over QR. Does NOT require M sigs — accepts 0..=N partial
/// sigs per input. Flags byte = 0x00 (partial).
///
pub(super) fn pskt_relay_to_kspt_string(wire_hex: &str, network: &str) -> Result<String, String> {
    let network = kaskold_protocol::Network::parse(network).map_err(|error| error.to_string())?;
    kaskold_protocol::encode_pskt_hex(wire_hex, network).map_err(|error| error.to_string())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn pskt_relay_to_kspt(wire_hex: &str, network: &str) -> Result<String, JsValue> {
    pskt_relay_to_kspt_string(wire_hex, network).map_err(js_error)
}

/// Inverse of `pskt_relay_to_kspt`: merge the partial sigs from a
/// device-returned compact KSPT blob into the canonical PSKB and return
/// the updated PSKB wire hex. Idempotent — existing sigs are not
/// clobbered.
///
/// Accepts `flags = 0x00` (partial) and `flags = 0x01` (fully signed)
/// equally. Caller must still check whether the merged PSKB has ≥M
/// sigs before finalizing/broadcasting.
pub(super) fn pskt_merge_signed_kspt_string(
    signed_kspt_hex: &str,
    pskb_wire_hex: &str,
) -> Result<String, String> {
    let signed = hex::decode(signed_kspt_hex).map_err(|error| format!("KSPT hex: {error}"))?;
    kaskold_protocol::compat::merge_signed_kspt_at_trailer_network(pskb_wire_hex, &signed)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn pskt_merge_signed_kspt(
    signed_kspt_hex: &str,
    pskb_wire_hex: &str,
) -> Result<String, JsValue> {
    pskt_merge_signed_kspt_string(signed_kspt_hex, pskb_wire_hex).map_err(js_error)
}

/// PSKT-native finalize + broadcast. Walks the PSKB JSON once,
/// assembles a consensus Transaction directly (sig_scripts per input,
/// with partial sigs + redeem script for P2SH multisig), and submits
/// via Borsh wRPC. No KSPT intermediate format, no shim — PSKB JSON
/// in, Kaspa consensus transaction out, TX ID returned on acceptance.
pub(super) async fn pskt_finalize_and_broadcast_string(
    wire_hex: &str,
    ws_url: &str,
) -> Result<String, String> {
    WatchWallet::new()
        .finalize_and_broadcast(wire_hex, ws_url)
        .await
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn pskt_finalize_and_broadcast(wire_hex: &str, ws_url: &str) -> Result<String, JsValue> {
    pskt_finalize_and_broadcast_string(wire_hex, ws_url)
        .await
        .map_err(js_error)
}

/// Wrap one PSKT JSON document in the canonical PSKB wire through Portal,
/// which checks its exact u64 fields first.
pub(super) fn pskb_encode_document_string(pskt_json: &str) -> Result<String, String> {
    let document: serde_json::Value =
        serde_json::from_str(pskt_json).map_err(|error| format!("PSKT JSON: {error}"))?;
    kaspa_portal::transaction::interchange::pskt::pskb::encode_pskt_document(document)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn pskb_encode_document(pskt_json: &str) -> Result<String, JsValue> {
    pskb_encode_document_string(pskt_json).map_err(js_error)
}

/// Append a plain output paying `amount` to a version-0 script public key
/// (`0000` + script hex) on a PSKB wire.
pub(super) fn pskb_append_output_string(
    wire_hex: &str,
    amount: u64,
    script_public_key_hex: &str,
) -> Result<String, String> {
    let script = script_public_key_hex
        .strip_prefix("0000")
        .ok_or_else(|| "script public key must be version 0".to_string())
        .and_then(|script| hex::decode(script).map_err(|error| format!("script hex: {error}")))?;
    pskt::append_output(wire_hex, amount, &script)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn pskb_append_output(
    wire_hex: &str,
    amount: u64,
    script_public_key_hex: &str,
) -> Result<String, JsValue> {
    pskb_append_output_string(wire_hex, amount, script_public_key_hex).map_err(js_error)
}
