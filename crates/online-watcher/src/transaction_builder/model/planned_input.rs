use crate::account::utxo::UtxoEntry;
use serde_json::Value;

/// A transaction input after coin selection and policy resolution.
#[derive(Clone, Debug)]
pub struct PlannedInput {
    pub utxo: UtxoEntry,
    pub sequence: u64,
    pub sig_op_count: u8,
    pub redeem_script: Option<Vec<u8>>,
    pub bip32_derivations: Option<Value>,
    pub derivation_hint: Option<(u8, u32)>,
}

impl PlannedInput {
    #[must_use]
    pub fn p2pk(utxo: UtxoEntry) -> Self {
        Self {
            utxo,
            sequence: 0,
            sig_op_count: 1,
            redeem_script: None,
            bip32_derivations: None,
            derivation_hint: None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn with_derivation(mut self, branch: u8, index: u32) -> Self {
        self.derivation_hint = Some((branch, index));
        self
    }
}
