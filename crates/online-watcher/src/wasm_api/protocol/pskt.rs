use crate::wasm_api::utilities::common::{js_error, network_to_prefix};
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

/// Parse a PSKT/PSKB payload into a review summary (JSON string).
///
/// `network` is one of "mainnet", "testnet-10/11/12", "simnet",
/// "devnet" — used to format decoded output addresses for display.
pub(super) fn pskt_summary_string(wire_hex: &str, network: &str) -> Result<String, String> {
    let prefix = network_to_prefix(network);
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
