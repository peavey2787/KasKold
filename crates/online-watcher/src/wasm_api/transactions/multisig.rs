use crate::wasm_api::utilities::common::{
    js_error, parse_request, parse_u64_field, parse_utxo_indices,
};
use crate::wasm_api::JsValue;
use crate::{transaction_builder, WatchWallet};
use serde::Deserialize;

#[derive(Deserialize)]
struct MultisigApiRequest {
    descriptor: String,
    source_address: String,
    dest_address: String,
    amount_sompi: String,
    fee_sompi: String,
    change_address: String,
    ws_url: String,
    addr_index: u32,
    #[serde(default = "default_change_index_hint")]
    change_index_hint: u32,
    #[serde(default)]
    utxo_csv: String,
}

impl MultisigApiRequest {
    fn automatic(&self) -> Result<MultisigRequest<'_>, JsValue> {
        Ok(MultisigRequest {
            descriptor: &self.descriptor,
            source_address: &self.source_address,
            dest_address: &self.dest_address,
            amount_sompi: parse_u64_field(&self.amount_sompi, "amount_sompi")?,
            fee_sompi: parse_u64_field(&self.fee_sompi, "fee_sompi")?,
            change_address: &self.change_address,
            ws_url: &self.ws_url,
            addr_index: self.addr_index,
            change_index_hint: self.change_index_hint,
            selection: transaction_builder::MultisigSelection::Automatic,
        })
    }
}

struct MultisigRequest<'a> {
    descriptor: &'a str,
    source_address: &'a str,
    dest_address: &'a str,
    amount_sompi: u64,
    fee_sompi: u64,
    change_address: &'a str,
    ws_url: &'a str,
    addr_index: u32,
    change_index_hint: u32,
    selection: transaction_builder::MultisigSelection<'a>,
}

async fn create_multisig(request: MultisigRequest<'_>) -> Result<String, JsValue> {
    WatchWallet::new()
        .build_multisig_transaction(
            transaction_builder::MultisigTransactionRequest {
                descriptor_text: request.descriptor,
                source_address: request.source_address,
                destination_address: request.dest_address,
                amount: request.amount_sompi,
                fee: request.fee_sompi,
                change_address: request.change_address,
                requested_index: request.addr_index,
                change_index_hint: request.change_index_hint,
                selection: request.selection,
            },
            request.ws_url,
        )
        .await
        .map_err(js_error)
}

/// Build an unsigned multisig PSKB for review and signing.
///
/// The output goes directly to `openPsktReview` on the JS side,
/// landing the user on the Review PSKB screen with 0/M sigs where
/// they can pick Relay → (Any wallet | KasKold compact).
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn create_multisig_pskb(request_json: &str) -> Result<String, JsValue> {
    let request: MultisigApiRequest = parse_request(request_json, "multisig request")?;
    create_multisig(request.automatic()?).await
}

/// Same as `create_multisig_pskb` but with explicit UTXO indices
/// instead of greedy auto-selection.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn create_multisig_pskb_selected(request_json: &str) -> Result<String, JsValue> {
    let request: MultisigApiRequest = parse_request(request_json, "selected multisig request")?;
    let indices = parse_utxo_indices(&request.utxo_csv)?;
    create_multisig(MultisigRequest {
        descriptor: &request.descriptor,
        source_address: &request.source_address,
        dest_address: &request.dest_address,
        amount_sompi: parse_u64_field(&request.amount_sompi, "amount_sompi")?,
        fee_sompi: parse_u64_field(&request.fee_sompi, "fee_sompi")?,
        change_address: &request.change_address,
        ws_url: &request.ws_url,
        addr_index: request.addr_index,
        change_index_hint: request.change_index_hint,
        selection: transaction_builder::MultisigSelection::Explicit(&indices),
    })
    .await
}

fn default_change_index_hint() -> u32 {
    u32::MAX
}

#[derive(Deserialize)]
struct MultisigBranchScanRequest {
    descriptor: String,
    cosigner_index: u32,
    #[serde(default = "default_scan_depth")]
    depth: u32,
    ws_url: String,
    #[serde(default = "default_address_prefix")]
    address_prefix: String,
}
fn default_scan_depth() -> u32 {
    kaspa_portal::transaction::builder::MULTISIG_BRANCH_SCAN_DEPTH
}
fn default_address_prefix() -> String {
    "kaspa".to_string()
}

/// Scan receive/change addresses for one 45' cosigner branch in one node query.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn scan_multisig_branch_js(request_json: &str) -> Result<String, JsValue> {
    let request: MultisigBranchScanRequest = parse_request(request_json, "multisig branch scan")?;
    crate::multisig::require_signer_descriptor(&request.descriptor).map_err(js_error)?;
    let client = crate::network::client(&request.ws_url).map_err(js_error)?;
    kaspa_portal::transaction::builder::scan_multisig_branch(
        &request.descriptor,
        request.cosigner_index,
        request.depth,
        &client,
        &request.address_prefix,
    )
    .await
    .map_err(js_error)
}

#[derive(Deserialize)]
struct MultisigMultiRequest {
    descriptor: String,
    sources_json: String,
    dest_address: String,
    amount_sompi: String,
    fee_sompi: String,
    cosigner_index: u32,
    #[serde(default = "default_change_index_hint")]
    change_index_hint: u32,
    ws_url: String,
}

/// Build a 45' multisig PSKB from selected UTXOs across several addresses.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn create_multisig_pskb_multi_js(request_json: &str) -> Result<String, JsValue> {
    let request: MultisigMultiRequest =
        parse_request(request_json, "multi-address multisig request")?;
    crate::multisig::require_signer_descriptor(&request.descriptor).map_err(js_error)?;
    let consolidation = kaspa_portal::transaction::builder::MultisigConsolidationRequest {
        descriptor_text: &request.descriptor,
        sources_json: &request.sources_json,
        destination_address: &request.dest_address,
        amount: parse_u64_field(&request.amount_sompi, "amount_sompi")?,
        fee: parse_u64_field(&request.fee_sompi, "fee_sompi")?,
        cosigner: request.cosigner_index,
        change_index_hint: request.change_index_hint,
    };
    let client = crate::network::client(&request.ws_url).map_err(js_error)?;
    kaspa_portal::transaction::builder::create_multisig_consolidation(&client, &consolidation)
        .await
        .map_err(js_error)
}

#[cfg(test)]
mod unit_tests;
