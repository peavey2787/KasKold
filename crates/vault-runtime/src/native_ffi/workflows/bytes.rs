//! Binary/file native workflow operations.

use super::{
    debug_error, optional_text, text, u8_value, wallet_summary_json, NativeWorkflowResult,
};
use crate::native_ffi::NativeVault;
use zeroize::Zeroize;

impl NativeVault {
    pub(super) fn create_with_entropy(
        &mut self,
        input: &serde_json::Value,
        touch_transcript: &[u8],
        add: bool,
    ) -> Result<NativeWorkflowResult, String> {
        let word_count = u8_value(input, "wordCount")?;
        let mut dice_rolls = parse_dice_rolls(optional_text(input, "dice"))?;
        let passphrase = optional_text(input, "passphrase");
        let created = if add {
            self.runtime.add_wallet_with_additive_entropy(
                word_count,
                &dice_rolls,
                touch_transcript,
                passphrase,
            )
        } else {
            self.runtime
                .create_wallet_with_additive_entropy(
                    word_count,
                    &dice_rolls,
                    touch_transcript,
                    passphrase,
                )
                .map_err(crate::VaultRuntimeError::Custody)
        };
        dice_rolls.zeroize();
        let created = created.map_err(debug_error)?;
        Ok(NativeWorkflowResult::Text(
            serde_json::json!({"recoveryPhrase": created.recovery_phrase.as_str()}).to_string(),
        ))
    }

    pub(super) fn transaction_file(&mut self, data: &[u8]) -> Result<NativeWorkflowResult, String> {
        let review = self
            .runtime
            .load_transaction_file(data)
            .map_err(debug_error)?;
        Ok(NativeWorkflowResult::Text(
            serde_json::json!({
                "state": "review",
                "network": review.network,
                "inputCount": review.input_count,
                "outputCount": review.output_count,
                "inputTotal": review.input_total.to_string(),
                "outputTotal": review.output_total.to_string(),
                "externalTotal": review.external_total.to_string(),
                "changeTotal": review.change_total.to_string(),
                "ownReceiveTotal": review.own_receive_total.to_string(),
                "fee": review.fee.to_string(),
                "inputs": review.inputs.iter().map(|value| serde_json::json!({
                    "index": value.index,
                    "outpoint": value.outpoint,
                    "amount": value.amount.to_string(),
                    "scriptType": value.script_type,
                    "address": value.address,
                })).collect::<Vec<_>>(),
                "outputs": review.outputs.iter().map(|value| serde_json::json!({
                    "index": value.index,
                    "amount": value.amount.to_string(),
                    "ownership": value.ownership,
                    "address": value.address,
                })).collect::<Vec<_>>(),
            })
            .to_string(),
        ))
    }

    pub(super) fn normalize_covenant_backup(
        &self,
        data: &[u8],
    ) -> Result<NativeWorkflowResult, String> {
        crate::VaultRuntime::normalize_covenant_backup(data)
            .map(NativeWorkflowResult::Bytes)
            .map_err(debug_error)
    }

    pub(super) fn recover_material(
        &mut self,
        input: &serde_json::Value,
        data: &[u8],
    ) -> Result<NativeWorkflowResult, String> {
        let kpub = self
            .runtime
            .add_recovery_material(data, optional_text(input, "passphrase"))
            .map_err(debug_error)?;
        Ok(NativeWorkflowResult::Text(
            serde_json::json!({"kpub": kpub}).to_string(),
        ))
    }

    pub(super) fn portable_backup(
        &self,
        input: &serde_json::Value,
    ) -> Result<NativeWorkflowResult, String> {
        self.runtime
            .portable_backup(text(input, "password")?)
            .map(NativeWorkflowResult::Bytes)
            .map_err(debug_error)
    }

    pub(super) fn portable_xprv_backup(
        &self,
        input: &serde_json::Value,
    ) -> Result<NativeWorkflowResult, String> {
        self.runtime
            .portable_xprv_backup(text(input, "password")?)
            .map(NativeWorkflowResult::Bytes)
            .map_err(debug_error)
    }

    pub(super) fn restore_portable(
        &mut self,
        input: &serde_json::Value,
        data: &[u8],
    ) -> Result<NativeWorkflowResult, String> {
        let summary = self
            .runtime
            .add_portable_backup(data, text(input, "password")?)
            .map_err(debug_error)?;
        Ok(NativeWorkflowResult::Text(
            wallet_summary_json(&summary).to_string(),
        ))
    }

    pub(super) fn stego_backup(
        &self,
        input: &serde_json::Value,
        data: &[u8],
    ) -> Result<NativeWorkflowResult, String> {
        self.runtime
            .stego_backup(data, text(input, "password")?)
            .map(NativeWorkflowResult::Bytes)
            .map_err(debug_error)
    }

    pub(super) fn restore_stego(
        &mut self,
        input: &serde_json::Value,
        data: &[u8],
    ) -> Result<NativeWorkflowResult, String> {
        let summary = self
            .runtime
            .add_stego_backup(data, text(input, "password")?)
            .map_err(debug_error)?;
        Ok(NativeWorkflowResult::Text(
            wallet_summary_json(&summary).to_string(),
        ))
    }
}

fn parse_dice_rolls(text: &str) -> Result<Vec<u8>, String> {
    text.bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .map(|byte| match byte {
            b'1'..=b'6' => Ok(byte - b'0'),
            _ => Err("dice rolls must contain only digits 1 through 6".to_owned()),
        })
        .collect()
}
