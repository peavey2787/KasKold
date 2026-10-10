//! Standard sends over Kaspa Portal's builder, bounded by what the KasKold
//! signer can co-sign and addressed at the Companion's node.

use kaspa_portal::transaction::builder as portal;

use crate::account::{bip32::WalletData, utxo::UtxoEntry};

const SIGNER_MAX_INPUTS: usize = kaskold_protocol::SIGNER_CAPABILITIES.max_inputs as usize;

pub(super) fn validate_signer_input_count(count: usize) -> Result<(), String> {
    if count == 0 {
        return Err("No UTXOs provided".into());
    }
    if count > SIGNER_MAX_INPUTS {
        return Err(format!(
            "transaction uses {count} inputs but KasKold supports at most {SIGNER_MAX_INPUTS}"
        ));
    }
    Ok(())
}

pub async fn create_send(
    wallet: &WalletData,
    destination: &str,
    amount: u64,
    fee: u64,
    websocket_url: &str,
) -> Result<String, String> {
    let client = crate::network::client(websocket_url)?;
    portal::create_send(wallet, destination, amount, fee, &client).await
}

pub async fn create_send_limited(
    wallet: &WalletData,
    destination: &str,
    amount: u64,
    fee: u64,
    max_inputs: usize,
    websocket_url: &str,
) -> Result<String, String> {
    validate_signer_input_count(max_inputs)?;
    let client = crate::network::client(websocket_url)?;
    portal::create_send_limited(wallet, destination, amount, fee, max_inputs, &client).await
}

pub async fn create_send_selected(
    wallet: &WalletData,
    destination: &str,
    amount: u64,
    fee: u64,
    indices: &[usize],
    websocket_url: &str,
) -> Result<String, String> {
    validate_signer_input_count(indices.len())?;
    let client = crate::network::client(websocket_url)?;
    portal::create_send_selected(wallet, destination, amount, fee, indices, &client).await
}

pub async fn create_consolidation(
    wallet: &WalletData,
    fee: u64,
    websocket_url: &str,
) -> Result<String, String> {
    let client = crate::network::client(websocket_url)?;
    portal::create_consolidation(wallet, fee, &client).await
}

pub fn create_pskb_with_utxos(
    wallet: &WalletData,
    destination: &str,
    amount: u64,
    requested_fee: u64,
    selected: Vec<UtxoEntry>,
) -> Result<String, String> {
    validate_signer_input_count(selected.len())?;
    portal::create_pskb_with_utxos(wallet, destination, amount, requested_fee, selected)
}

#[cfg(test)]
mod unit_tests;
