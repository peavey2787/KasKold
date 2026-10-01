//! Allowlisted workflow bridge for native Android/iOS presentation shells.
//!
//! The native UI sends only operation parameters. Wallet custody, derivation,
//! backup, multisig and advanced cryptographic behavior stay inside
//! `VaultRuntime`; this module only translates JSON/byte FFI inputs to those
//! strongly owned operations and serializes presentation-safe results.

use super::{NativeVault, OK};
use crate::{MultisigResult, WalletKind, WalletSummary};

impl NativeVault {
    pub(super) fn workflow_text(&mut self, operation: &str, input_json: &str) -> i32 {
        self.clear_results();
        let input = match parse_input(input_json) {
            Ok(value) => value,
            Err(error) => return self.fail(error),
        };
        let result = self.dispatch_text_workflow(operation, &input);
        match result {
            Ok(value) => {
                self.set_text(value);
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    pub(super) fn workflow_bytes(&mut self, operation: &str, input_json: &str, data: &[u8]) -> i32 {
        self.clear_results();
        let input = match parse_input(input_json) {
            Ok(value) => value,
            Err(error) => return self.fail(error),
        };
        let result = self.dispatch_byte_workflow(operation, &input, data);
        match result {
            Ok(NativeWorkflowResult::Text(value)) => {
                self.set_text(value);
                OK
            }
            Ok(NativeWorkflowResult::Bytes(value)) => {
                self.set_bytes(value);
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn dispatch_text_workflow(
        &mut self,
        operation: &str,
        input: &serde_json::Value,
    ) -> Result<String, String> {
        match operation {
            "creation_flow" | "receive_address" | "add_create_12" | "add_create_24"
            | "add_restore" | "wallets" | "switch_wallet" | "set_wallet_name" | "delete_wallet" => {
                self.dispatch_inventory_text(operation, input)
            }
            "import_raw_key" | "import_xprv" | "multisig_kpub" | "normalize_kpub"
            | "validate_address" | "create_multisig" | "import_multisig" => {
                self.dispatch_key_text(operation, input)
            }
            "bip85"
            | "sign_message"
            | "commit_secret"
            | "decrypt_secret"
            | "set_signing_policy"
            | "clear_signing_policy" => self.dispatch_tool_text(operation, input),
            _ => Err("unsupported native Vault workflow operation".to_owned()),
        }
    }

    pub(crate) fn dispatch_inventory_text(
        &mut self,
        operation: &str,
        input: &serde_json::Value,
    ) -> Result<String, String> {
        match operation {
            "creation_flow" | "wallets" => self.dispatch_inventory_query(operation),
            _ => self.dispatch_inventory_mutation(operation, input),
        }
    }

    fn dispatch_inventory_query(&self, operation: &str) -> Result<String, String> {
        match operation {
            "creation_flow" => Ok(crate::creation_flow_json()),
            "wallets" => self.wallets(),
            _ => Err("unsupported inventory workflow operation".to_owned()),
        }
    }

    fn dispatch_inventory_mutation(
        &mut self,
        operation: &str,
        input: &serde_json::Value,
    ) -> Result<String, String> {
        match operation {
            "receive_address" => self.receive_address(input),
            "add_create_12" => self.add_create(12),
            "add_create_24" => self.add_create(24),
            "add_restore" => self.add_restore(input),
            "switch_wallet" => self.switch_wallet(input),
            "set_wallet_name" => self.set_wallet_name(input),
            "delete_wallet" => self.delete_wallet(input),
            _ => Err("unsupported inventory workflow operation".to_owned()),
        }
    }

    fn dispatch_key_text(
        &mut self,
        operation: &str,
        input: &serde_json::Value,
    ) -> Result<String, String> {
        match operation {
            "import_raw_key" => self.import_raw_key(input),
            "import_xprv" => self.import_xprv(input),
            "multisig_kpub" => self.multisig_kpub(),
            "normalize_kpub" => self.normalize_kpub(input),
            "validate_address" => self.validate_address(input),
            "create_multisig" => self.create_multisig(input),
            "import_multisig" => self.import_multisig(input),
            _ => Err("unsupported key workflow operation".to_owned()),
        }
    }

    fn dispatch_tool_text(
        &mut self,
        operation: &str,
        input: &serde_json::Value,
    ) -> Result<String, String> {
        match operation {
            "bip85" => self.bip85(input),
            "sign_message" => self.sign_message(input),
            "commit_secret" => self.commit_secret(input),
            "decrypt_secret" => self.decrypt_secret(input),
            "set_signing_policy" => self.set_signing_policy(input),
            "clear_signing_policy" => self.clear_signing_policy(),
            _ => Err("unsupported tool workflow operation".to_owned()),
        }
    }

    fn dispatch_byte_workflow(
        &mut self,
        operation: &str,
        input: &serde_json::Value,
        data: &[u8],
    ) -> Result<NativeWorkflowResult, String> {
        match operation {
            "create_with_entropy"
            | "add_create_with_entropy"
            | "recover_material"
            | "transaction_file"
            | "normalize_covenant_backup" => self.dispatch_import_bytes(operation, input, data),
            "portable_backup"
            | "portable_xprv_backup"
            | "restore_portable"
            | "stego_backup"
            | "restore_stego" => self.dispatch_backup_bytes(operation, input, data),
            _ => Err("unsupported native Vault byte workflow operation".to_owned()),
        }
    }

    fn dispatch_import_bytes(
        &mut self,
        operation: &str,
        input: &serde_json::Value,
        data: &[u8],
    ) -> Result<NativeWorkflowResult, String> {
        match operation {
            "create_with_entropy" => self.create_with_entropy(input, data, false),
            "add_create_with_entropy" => self.create_with_entropy(input, data, true),
            "recover_material" => self.recover_material(input, data),
            "transaction_file" => self.transaction_file(data),
            "normalize_covenant_backup" => self.normalize_covenant_backup(data),
            _ => Err("unsupported import byte workflow operation".to_owned()),
        }
    }

    fn dispatch_backup_bytes(
        &mut self,
        operation: &str,
        input: &serde_json::Value,
        data: &[u8],
    ) -> Result<NativeWorkflowResult, String> {
        match operation {
            "portable_backup" => self.portable_backup(input),
            "portable_xprv_backup" => self.portable_xprv_backup(input),
            "restore_portable" => self.restore_portable(input, data),
            "stego_backup" => self.stego_backup(input, data),
            "restore_stego" => self.restore_stego(input, data),
            _ => Err("unsupported backup byte workflow operation".to_owned()),
        }
    }
}

mod bytes;
mod text_ops;

enum NativeWorkflowResult {
    Text(String),
    Bytes(Vec<u8>),
}

pub(crate) fn parse_input(input_json: &str) -> Result<serde_json::Value, String> {
    if input_json.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    let value: serde_json::Value = serde_json::from_str(input_json)
        .map_err(|_| "native Vault workflow input must be valid JSON".to_owned())?;
    if !value.is_object() {
        return Err("native Vault workflow input must be a JSON object".to_owned());
    }
    Ok(value)
}

fn text<'a>(input: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    input
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{key} must be text"))
}

fn optional_text<'a>(input: &'a serde_json::Value, key: &str) -> &'a str {
    input
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

fn boolean(input: &serde_json::Value, key: &str) -> Result<bool, String> {
    input
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| format!("{key} must be a boolean"))
}

fn u64_value(input: &serde_json::Value, key: &str) -> Result<u64, String> {
    input
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}

fn u32_value(input: &serde_json::Value, key: &str) -> Result<u32, String> {
    u32::try_from(u64_value(input, key)?).map_err(|_| format!("{key} is too large"))
}

fn usize_value(input: &serde_json::Value, key: &str) -> Result<usize, String> {
    usize::try_from(u64_value(input, key)?).map_err(|_| format!("{key} is too large"))
}

fn u8_value(input: &serde_json::Value, key: &str) -> Result<u8, String> {
    u8::try_from(u64_value(input, key)?).map_err(|_| format!("{key} is too large"))
}

fn wallet_summary_json(summary: &WalletSummary) -> serde_json::Value {
    serde_json::json!({
        "index": summary.index,
        "name": summary.name,
        "active": summary.active,
        "kind": wallet_kind_label(summary.kind),
        "fingerprint": summary.fingerprint,
        "kpub": summary.kpub,
    })
}

pub(crate) fn wallet_kind_label(kind: WalletKind) -> &'static str {
    match kind {
        WalletKind::Mnemonic => "mnemonic",
        WalletKind::AccountXprv => "account-xprv",
        WalletKind::RawPrivateKey => "raw-private-key",
    }
}

fn multisig_json(result: &MultisigResult) -> serde_json::Value {
    serde_json::json!({
        "descriptor": result.descriptor,
        "address": result.address,
        "threshold": result.threshold,
        "participants": result.participants,
        "chain": result.chain,
        "index": result.index,
    })
}

fn debug_error(error: impl core::fmt::Debug) -> String {
    format!("{error:?}")
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

pub(crate) fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    let text = text.trim();
    if !text.len().is_multiple_of(2) {
        return Err("hex payload must have an even number of characters".to_owned());
    }
    let mut output = Vec::with_capacity(text.len() / 2);
    for pair in text.as_bytes().as_chunks::<2>().0 {
        let high = shared_signer::bytes::decode_hex_nibble(pair[0])
            .ok_or_else(|| "hex payload contains a non-hex character".to_owned())?;
        let low = shared_signer::bytes::decode_hex_nibble(pair[1])
            .ok_or_else(|| "hex payload contains a non-hex character".to_owned())?;
        output.push((high << 4) | low);
    }
    Ok(output)
}
