//! Text/JSON native workflow operations.

use zeroize::Zeroize;

use super::{
    boolean, debug_error, hex_decode, hex_lower, multisig_json, text, u32_value, u8_value,
    usize_value, wallet_summary_json,
};
use crate::native_ffi::NativeVault;

impl NativeVault {
    pub(super) fn add_create(&mut self, words: u8) -> Result<String, String> {
        let created = match words {
            12 => self.runtime.add_wallet_12(),
            24 => self.runtime.add_wallet_24(),
            _ => return Err("wallet word count must be 12 or 24".to_owned()),
        }
        .map_err(debug_error)?;
        Ok(serde_json::json!({"recoveryPhrase": created.recovery_phrase.as_str()}).to_string())
    }

    pub(super) fn add_restore(&mut self, input: &serde_json::Value) -> Result<String, String> {
        let kpub = self
            .runtime
            .add_restored_wallet(text(input, "phrase")?, text(input, "passphrase")?)
            .map_err(debug_error)?;
        Ok(serde_json::json!({"kpub": kpub}).to_string())
    }

    pub(super) fn receive_address(&self, input: &serde_json::Value) -> Result<String, String> {
        let network = text(input, "network")?;
        let change = boolean(input, "change")?;
        let index = u32_value(input, "index")?;
        let address = self
            .runtime
            .derive_receive_address(network, change, index)
            .map_err(debug_error)?;
        Ok(serde_json::json!({"address": address}).to_string())
    }

    pub(super) fn wallets(&self) -> Result<String, String> {
        let summaries = self.runtime.wallet_summaries().map_err(debug_error)?;
        Ok(serde_json::json!({
            "wallets": summaries.iter().map(wallet_summary_json).collect::<Vec<_>>(),
        })
        .to_string())
    }

    pub(super) fn switch_wallet(&mut self, input: &serde_json::Value) -> Result<String, String> {
        let index = usize_value(input, "index")?;
        let kpub = self.runtime.switch_wallet(index).map_err(debug_error)?;
        Ok(serde_json::json!({"index": index, "kpub": kpub}).to_string())
    }

    pub(super) fn set_wallet_name(&mut self, input: &serde_json::Value) -> Result<String, String> {
        let index = usize_value(input, "index")?;
        self.runtime
            .set_wallet_name(index, text(input, "name")?)
            .map_err(debug_error)?;
        Ok(serde_json::json!({"index": index}).to_string())
    }

    pub(super) fn delete_wallet(&mut self, input: &serde_json::Value) -> Result<String, String> {
        let index = usize_value(input, "index")?;
        self.runtime.delete_wallet(index).map_err(debug_error)?;
        Ok(serde_json::json!({"deleted": index}).to_string())
    }

    pub(super) fn import_raw_key(&mut self, input: &serde_json::Value) -> Result<String, String> {
        let summary = self
            .runtime
            .add_raw_private_key(text(input, "privateKey")?)
            .map_err(debug_error)?;
        Ok(wallet_summary_json(&summary).to_string())
    }

    pub(super) fn import_xprv(&mut self, input: &serde_json::Value) -> Result<String, String> {
        let summary = self
            .runtime
            .add_account_xprv(text(input, "xprv")?)
            .map_err(debug_error)?;
        Ok(wallet_summary_json(&summary).to_string())
    }

    pub(super) fn multisig_kpub(&self) -> Result<String, String> {
        let kpub = self.runtime.export_multisig_kpub().map_err(debug_error)?;
        Ok(serde_json::json!({"kpub": kpub}).to_string())
    }

    pub(super) fn normalize_kpub(&self, input: &serde_json::Value) -> Result<String, String> {
        let value = crate::VaultRuntime::normalize_watch_kpub(text(input, "value")?.as_bytes())
            .map_err(debug_error)?;
        Ok(serde_json::json!({"value": value}).to_string())
    }

    pub(super) fn validate_address(&self, input: &serde_json::Value) -> Result<String, String> {
        let value = crate::VaultRuntime::validate_multisig_address(text(input, "value")?)
            .map_err(debug_error)?;
        Ok(serde_json::json!({"value": value}).to_string())
    }

    pub(super) fn create_multisig(&mut self, input: &serde_json::Value) -> Result<String, String> {
        let threshold = u8_value(input, "threshold")?;
        let cosigners = input
            .get("cosigners")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "cosigners must be an array".to_owned())?;
        let owned = cosigners
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "each cosigner must be text".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
        let result = self
            .runtime
            .create_multisig(
                threshold,
                &refs,
                text(input, "network")?,
                u8_value(input, "chain")?,
                u32_value(input, "index")?,
            )
            .map_err(debug_error)?;
        Ok(multisig_json(&result).to_string())
    }

    pub(super) fn import_multisig(&mut self, input: &serde_json::Value) -> Result<String, String> {
        let result = self
            .runtime
            .import_multisig_descriptor(
                text(input, "descriptor")?,
                text(input, "network")?,
                u8_value(input, "chain")?,
                u32_value(input, "index")?,
            )
            .map_err(debug_error)?;
        Ok(multisig_json(&result).to_string())
    }

    pub(super) fn bip85(&self, input: &serde_json::Value) -> Result<String, String> {
        let phrase = self
            .runtime
            .derive_bip85_phrase(u8_value(input, "wordCount")?, u32_value(input, "index")?)
            .map_err(debug_error)?;
        Ok(serde_json::json!({"phrase": phrase.as_str()}).to_string())
    }

    pub(super) fn sign_message(&self, input: &serde_json::Value) -> Result<String, String> {
        let signed = self
            .runtime
            .sign_message(text(input, "message")?.as_bytes())
            .map_err(debug_error)?;
        Ok(serde_json::json!({
            "digestHex": hex_lower(&signed.digest),
            "signatureHex": hex_lower(&signed.signature),
        })
        .to_string())
    }

    pub(super) fn commit_secret(&self, input: &serde_json::Value) -> Result<String, String> {
        let committed = self
            .runtime
            .commit_secret(text(input, "secret")?.as_bytes())
            .map_err(debug_error)?;
        Ok(serde_json::json!({
            "payloadHex": hex_lower(committed.payload.as_slice()),
            "commitmentHex": hex_lower(&committed.commitment),
        })
        .to_string())
    }

    pub(super) fn decrypt_secret(&self, input: &serde_json::Value) -> Result<String, String> {
        let mut payload = hex_decode(text(input, "payloadHex")?)?;
        let result = self
            .runtime
            .decrypt_secret(&payload)
            .map_err(debug_error)
            .and_then(|plain| {
                String::from_utf8(plain.to_vec())
                    .map_err(|_| "decrypted secret is not UTF-8".to_owned())
            });
        payload.zeroize();
        result.map(|secret| serde_json::json!({"secret": secret}).to_string())
    }

    pub(super) fn set_signing_policy(
        &mut self,
        input: &serde_json::Value,
    ) -> Result<String, String> {
        self.runtime
            .set_session_signing_policy(text(input, "notBeforeUtc")?, text(input, "weeklyWindows")?)
            .map_err(debug_error)?;
        Ok("{}".to_owned())
    }

    pub(super) fn clear_signing_policy(&mut self) -> Result<String, String> {
        self.runtime.clear_session_signing_policy();
        Ok("{}".to_owned())
    }
}
