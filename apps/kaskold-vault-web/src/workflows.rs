//! Web-facing wallet workflows backed exclusively by `vault-runtime`.

use serde::Serialize;
use wasm_bindgen::prelude::*;

use super::{hex_encode, json, qr_svg, vault_error, VAULT};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddressResponse {
    address: String,
    svg: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreationResponse {
    recovery_phrase: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KpubResponse {
    kpub: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WalletResponse {
    index: usize,
    name: String,
    active: bool,
    kind: &'static str,
    fingerprint: Option<String>,
    kpub: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MultisigKpubResponse {
    kpub: String,
    svg: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MultisigResponse {
    descriptor: String,
    descriptor_svg: String,
    address: String,
    address_svg: String,
    threshold: u8,
    participants: u8,
    chain: u8,
    index: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SecretTextResponse {
    value: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignedMessageResponse {
    signature_hex: String,
    digest_hex: String,
    payload_hex: String,
    svg: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CommitmentResponse {
    commitment_hex: String,
    payload_hex: String,
    svg: String,
}

#[wasm_bindgen]
pub fn kaskold_vault_receive_address(
    network: &str,
    change: bool,
    index: u32,
) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let address = vault
            .borrow()
            .derive_receive_address(network, change, index)
            .map_err(vault_error)?;
        let svg = qr_svg(address.as_bytes())?;
        json(&AddressResponse { address, svg })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_add_create_12() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let created = vault.borrow_mut().add_wallet_12().map_err(vault_error)?;
        json(&CreationResponse {
            recovery_phrase: created.recovery_phrase.to_string(),
        })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_add_create_24() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let created = vault.borrow_mut().add_wallet_24().map_err(vault_error)?;
        json(&CreationResponse {
            recovery_phrase: created.recovery_phrase.to_string(),
        })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_add_restore(
    recovery_phrase: &str,
    passphrase: &str,
) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let kpub = vault
            .borrow_mut()
            .add_restored_wallet(recovery_phrase, passphrase)
            .map_err(vault_error)?;
        json(&KpubResponse { kpub })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_wallets() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let wallets = vault.borrow().wallet_summaries().map_err(vault_error)?;
        let rows: Vec<WalletResponse> = wallets
            .into_iter()
            .map(wallet_response)
            .collect();
        json(&rows)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_switch_wallet(index: u32) -> Result<String, JsValue> {
    let index = usize::try_from(index).map_err(|_| JsValue::from_str("Invalid wallet index."))?;
    switch_wallet_json(index)
}

fn switch_wallet_json(index: usize) -> Result<String, JsValue> {
    let active = switch_wallet_summary(index)?;
    json(&wallet_response(active))
}

fn switch_wallet_summary(index: usize) -> Result<vault_runtime::WalletSummary, JsValue> {
    VAULT.with(|vault| {
        vault.borrow_mut().switch_wallet(index).map_err(vault_error)?;
        let wallets = vault.borrow().wallet_summaries().map_err(vault_error)?;
        active_wallet_summary(wallets)
    })
}

fn active_wallet_summary(
    wallets: Vec<vault_runtime::WalletSummary>,
) -> Result<vault_runtime::WalletSummary, JsValue> {
    wallets
        .into_iter()
        .find(|wallet| wallet.active)
        .ok_or_else(|| JsValue::from_str("No active wallet."))
}

#[wasm_bindgen]
pub fn kaskold_vault_import_raw_key(private_key_hex: &str) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let wallet = vault
            .borrow_mut()
            .add_raw_private_key(private_key_hex)
            .map_err(vault_error)?;
        json(&wallet_response(wallet))
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_import_xprv(xprv: &str) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let wallet = vault
            .borrow_mut()
            .add_account_xprv(xprv)
            .map_err(vault_error)?;
        json(&wallet_response(wallet))
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_set_wallet_name(index: u32, name: &str) -> Result<(), JsValue> {
    let index = usize::try_from(index).map_err(|_| JsValue::from_str("Invalid wallet index."))?;
    VAULT.with(|vault| vault.borrow_mut().set_wallet_name(index, name).map_err(vault_error))
}

#[wasm_bindgen]
pub fn kaskold_vault_delete_wallet(index: u32) -> Result<(), JsValue> {
    let index = usize::try_from(index).map_err(|_| JsValue::from_str("Invalid wallet index."))?;
    VAULT.with(|vault| vault.borrow_mut().delete_wallet(index).map_err(vault_error))
}

#[wasm_bindgen]
pub fn kaskold_vault_recover_material(data: &[u8], passphrase: &str) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let kpub = vault
            .borrow_mut()
            .add_recovery_material(data, passphrase)
            .map_err(vault_error)?;
        json(&KpubResponse { kpub })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_multisig_kpub() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let kpub = vault.borrow().export_multisig_kpub().map_err(vault_error)?;
        let svg = qr_svg(kpub.as_bytes())?;
        json(&MultisigKpubResponse { kpub, svg })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_create_multisig(
    threshold: u8,
    cosigners: &str,
    network: &str,
    chain: u8,
    index: u32,
) -> Result<String, JsValue> {
    let owned: Vec<String> = cosigners
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    VAULT.with(|vault| {
        let result = vault
            .borrow_mut()
            .create_multisig(threshold, &refs, network, chain, index)
            .map_err(vault_error)?;
        multisig_json(result)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_import_multisig(
    descriptor: &str,
    network: &str,
    chain: u8,
    index: u32,
) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let result = vault
            .borrow_mut()
            .import_multisig_descriptor(descriptor, network, chain, index)
            .map_err(vault_error)?;
        multisig_json(result)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_bip85(word_count: u8, index: u32) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let phrase = vault
            .borrow()
            .derive_bip85_phrase(word_count, index)
            .map_err(vault_error)?;
        json(&SecretTextResponse {
            value: phrase.to_string(),
        })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_sign_message(message: &str) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let signed = vault
            .borrow()
            .sign_message(message.as_bytes())
            .map_err(vault_error)?;
        let mut payload = Vec::with_capacity(96);
        payload.extend_from_slice(&signed.signature);
        payload.extend_from_slice(&signed.digest);
        let response = SignedMessageResponse {
            signature_hex: hex_encode(&signed.signature),
            digest_hex: hex_encode(&signed.digest),
            payload_hex: hex_encode(&payload),
            svg: qr_svg(&payload)?,
        };
        json(&response)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_commit_secret(secret: &str) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let committed = vault
            .borrow()
            .commit_secret(secret.as_bytes())
            .map_err(vault_error)?;
        json(&CommitmentResponse {
            commitment_hex: hex_encode(&committed.commitment),
            payload_hex: hex_encode(&committed.payload),
            svg: qr_svg(&committed.payload)?,
        })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_decrypt_secret(payload_hex: &str) -> Result<String, JsValue> {
    let payload = decode_hex(payload_hex)?;
    VAULT.with(|vault| {
        let secret = vault.borrow().decrypt_secret(&payload).map_err(vault_error)?;
        let value = core::str::from_utf8(&secret)
            .map_err(|_| JsValue::from_str("Decrypted secret is not valid UTF-8."))?
            .to_owned();
        json(&SecretTextResponse { value })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_portable_backup(password: &str) -> Result<Vec<u8>, JsValue> {
    VAULT.with(|vault| vault.borrow().portable_backup(password).map_err(vault_error))
}

#[wasm_bindgen]
pub fn kaskold_vault_portable_xprv_backup(password: &str) -> Result<Vec<u8>, JsValue> {
    VAULT.with(|vault| {
        vault
            .borrow()
            .portable_xprv_backup(password)
            .map_err(vault_error)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_restore_portable(data: &[u8], password: &str) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let wallet = vault
            .borrow_mut()
            .add_portable_backup(data, password)
            .map_err(vault_error)?;
        json(&wallet_response(wallet))
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_stego_backup(jpeg: &[u8], password: &str) -> Result<Vec<u8>, JsValue> {
    VAULT.with(|vault| vault.borrow().stego_backup(jpeg, password).map_err(vault_error))
}

#[wasm_bindgen]
pub fn kaskold_vault_restore_stego(jpeg: &[u8], password: &str) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let wallet = vault
            .borrow_mut()
            .add_stego_backup(jpeg, password)
            .map_err(vault_error)?;
        json(&wallet_response(wallet))
    })
}

fn wallet_response(wallet: vault_runtime::WalletSummary) -> WalletResponse {
    WalletResponse {
        index: wallet.index,
        name: wallet.name,
        active: wallet.active,
        kind: wallet_kind_label(wallet.kind),
        fingerprint: wallet.fingerprint,
        kpub: wallet.kpub,
    }
}

pub(crate) const fn wallet_kind_label(kind: vault_runtime::WalletKind) -> &'static str {
    match kind {
        vault_runtime::WalletKind::Mnemonic => "Mnemonic",
        vault_runtime::WalletKind::AccountXprv => "Account XPrv",
        vault_runtime::WalletKind::RawPrivateKey => "Raw Private Key",
    }
}

fn multisig_json(result: vault_runtime::MultisigResult) -> Result<String, JsValue> {
    let descriptor_svg = qr_svg(result.descriptor.as_bytes())?;
    multisig_json_with_descriptor(result, descriptor_svg)
}

fn multisig_json_with_descriptor(
    result: vault_runtime::MultisigResult,
    descriptor_svg: String,
) -> Result<String, JsValue> {
    let address_svg = qr_svg(result.address.as_bytes())?;
    json(&MultisigResponse {
        descriptor_svg,
        address_svg,
        descriptor: result.descriptor,
        address: result.address,
        threshold: result.threshold,
        participants: result.participants,
        chain: result.chain,
        index: result.index,
    })
}

pub(crate) fn decode_hex(text: &str) -> Result<Vec<u8>, JsValue> {
    decode_hex_bytes(text).map_err(JsValue::from_str)
}

pub(crate) fn decode_hex_bytes(text: &str) -> Result<Vec<u8>, &'static str> {
    let clean: String = text.chars().filter(|ch| !ch.is_ascii_whitespace()).collect();
    if clean.is_empty() || !clean.len().is_multiple_of(2) {
        return Err("Hex payload must contain complete bytes.");
    }
    let bytes = clean.as_bytes();
    let mut output = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = shared_signer::bytes::decode_hex_nibble(pair[0])
            .ok_or("Invalid hexadecimal payload.")?;
        let low = shared_signer::bytes::decode_hex_nibble(pair[1])
            .ok_or("Invalid hexadecimal payload.")?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

