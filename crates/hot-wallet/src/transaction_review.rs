//! Signer-verified transaction review data shared by software Vault shells.
//!
//! Watcher-provided derivation metadata is never trusted as a display label.
//! Hints are independently reproduced from the active wallet (or a stored
//! multisig descriptor) before an output may be labeled receive/change.

use super::{transaction_codec::parse_transaction, HotWallet, HotWalletError};
use offline_signer::{
    address::{encode_address_for_network, AddressType, MAX_ADDR_LEN},
    derivation::bip32,
    transaction::{
        kspt,
        model::{ScriptPublicKey, ScriptType, Transaction},
        std_pskt,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputOwnership {
    External,
    Change,
    Receive,
}

impl OutputOwnership {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::External => "External",
            Self::Change => "Change",
            Self::Receive => "Own receive",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionReviewInput {
    pub index: usize,
    pub outpoint: String,
    pub amount: u64,
    pub script_type: &'static str,
    pub address: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionReviewOutput {
    pub index: usize,
    pub amount: u64,
    pub ownership: OutputOwnership,
    pub address: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionReview {
    pub network: offline_signer::address::KaspaNetwork,
    pub input_count: usize,
    pub output_count: usize,
    pub input_total: u64,
    pub output_total: u64,
    pub external_total: u64,
    pub change_total: u64,
    pub own_receive_total: u64,
    pub fee: u64,
    pub inputs: Vec<TransactionReviewInput>,
    pub outputs: Vec<TransactionReviewOutput>,
}

impl HotWallet {
    pub fn review_transaction(&self, wire: &[u8]) -> Result<TransactionReview, HotWalletError> {
        let parsed = parse_transaction(wire)?;
        self.review_transaction_model(&parsed.transaction)
    }

    pub fn review_compact_kspt(&self, wire: &[u8]) -> Result<TransactionReview, HotWalletError> {
        if std_pskt::detect_tx_format(wire) != std_pskt::DetectedFormat::KsptCompact {
            return Err(HotWalletError::InvalidToolInput);
        }
        self.review_transaction(wire)
    }

    fn review_transaction_model(
        &self,
        transaction: &Transaction,
    ) -> Result<TransactionReview, HotWalletError> {
        kspt::validate_transaction_for_review(transaction)?;
        validate_generic_vault_semantics(transaction)?;
        let amounts = kspt::transaction_amounts(transaction)?;
        let ownership = self.verify_output_ownership(transaction)?;
        let (outputs, external_total, change_total, own_receive_total) =
            review_outputs(transaction, &ownership)?;
        let inputs = transaction
            .inputs()
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let (script_type, _) = kspt::analyze_input_script(transaction, index);
                TransactionReviewInput {
                    index,
                    outpoint: format!(
                        "{}:{}",
                        crate::hex_string(&input.previous_outpoint.transaction_id),
                        input.previous_outpoint.index
                    ),
                    amount: input.utxo_entry.amount,
                    script_type: script_type_label(script_type),
                    address: script_address(
                        &input.utxo_entry.script_public_key,
                        transaction.network,
                    ),
                }
            })
            .collect();
        Ok(TransactionReview {
            network: transaction.network,
            input_count: transaction.num_inputs,
            output_count: transaction.num_outputs,
            input_total: amounts.input_total,
            output_total: amounts.output_total,
            external_total,
            change_total,
            own_receive_total,
            fee: amounts.fee,
            inputs,
            outputs,
        })
    }

    fn verify_output_ownership(
        &self,
        transaction: &Transaction,
    ) -> Result<Vec<OutputOwnership>, HotWalletError> {
        let mut ownership = vec![OutputOwnership::External; transaction.num_outputs];
        if let Some(raw_key) = self.raw_key_bytes() {
            verify_raw_output_hints(transaction, raw_key, &mut ownership)?;
        } else {
            self.verify_account_output_hints(transaction, &mut ownership)?;
        }
        verify_multisig_output_hints(transaction, self.multisig_configs(), &mut ownership)?;
        Ok(ownership)
    }

    fn verify_account_output_hints(
        &self,
        transaction: &Transaction,
        ownership: &mut [OutputOwnership],
    ) -> Result<(), HotWalletError> {
        let mut derived_account = if self.account_key().is_none() {
            Some(bip32::derive_account_key(self.seed_bytes()?)?)
        } else {
            None
        };
        let account = self
            .account_key()
            .or(derived_account.as_ref())
            .ok_or(HotWalletError::InvalidToolInput)?;
        let result = verify_account_hints(transaction, account, ownership);
        if let Some(account) = derived_account.as_mut() {
            account.zeroize();
        }
        result
    }
}

/// Generic Vault payment review deliberately supports only the semantics the
/// payment review UI can fully explain. Advanced transaction semantics must
/// use their dedicated typed workflow instead of being blind-signed here.
pub(crate) fn validate_generic_vault_semantics(
    transaction: &Transaction,
) -> Result<(), HotWalletError> {
    validate_generic_transaction_header(transaction)?;
    validate_generic_inputs(transaction)?;
    validate_generic_outputs(transaction)
}

fn validate_generic_transaction_header(transaction: &Transaction) -> Result<(), HotWalletError> {
    use offline_signer::transaction::model::SUBNETWORK_ID_NATIVE;
    if transaction.version != 1
        || transaction.locktime != 0
        || transaction.subnetwork_id != SUBNETWORK_ID_NATIVE
        || transaction.gas != 0
        || !transaction.payload.is_empty()
    {
        return Err(HotWalletError::InvalidToolInput);
    }
    Ok(())
}

fn validate_generic_inputs(transaction: &Transaction) -> Result<(), HotWalletError> {
    for (index, input) in transaction.inputs().iter().enumerate() {
        validate_generic_input_fields(input)?;
        let (script_type, _) = kspt::analyze_input_script(transaction, index);
        if !matches!(script_type, ScriptType::P2PK | ScriptType::Multisig) {
            return Err(HotWalletError::InvalidToolInput);
        }
    }
    Ok(())
}

fn validate_generic_input_fields(
    input: &offline_signer::transaction::model::TransactionInput,
) -> Result<(), HotWalletError> {
    use offline_signer::transaction::model::SigHashType;
    if input.sequence != u64::MAX || input.sig_op_count != 1 {
        return Err(HotWalletError::InvalidToolInput);
    }
    if input.sighash_type != 0 && input.sighash_type != SigHashType::All.to_byte() {
        return Err(HotWalletError::InvalidToolInput);
    }
    Ok(())
}

fn validate_generic_outputs(transaction: &Transaction) -> Result<(), HotWalletError> {
    for output in transaction.outputs() {
        if output.has_covenant || !generic_output_script_supported(output) {
            return Err(HotWalletError::InvalidToolInput);
        }
    }
    Ok(())
}

fn generic_output_script_supported(
    output: &offline_signer::transaction::model::TransactionOutput,
) -> bool {
    matches!(
        offline_signer::transaction::model::detect_script_type(
            output.script_public_key.script_bytes(),
            output.script_public_key.script_len,
        ),
        ScriptType::P2PK | ScriptType::P2SH
    )
}

fn review_outputs(
    transaction: &Transaction,
    ownership: &[OutputOwnership],
) -> Result<(Vec<TransactionReviewOutput>, u64, u64, u64), HotWalletError> {
    let mut external_total = 0u64;
    let mut change_total = 0u64;
    let mut own_receive_total = 0u64;
    let mut outputs = Vec::with_capacity(transaction.num_outputs);
    for (index, output) in transaction.outputs().iter().enumerate() {
        let target = match ownership[index] {
            OutputOwnership::External => &mut external_total,
            OutputOwnership::Change => &mut change_total,
            OutputOwnership::Receive => &mut own_receive_total,
        };
        *target = target
            .checked_add(output.value)
            .ok_or(HotWalletError::Kspt(kspt::PsktError::OutputAmountOverflow))?;
        outputs.push(TransactionReviewOutput {
            index,
            amount: output.value,
            ownership: ownership[index],
            address: script_address(&output.script_public_key, transaction.network),
        });
    }
    Ok((outputs, external_total, change_total, own_receive_total))
}

fn verify_raw_output_hints(
    transaction: &Transaction,
    raw_key: &[u8; 32],
    ownership: &mut [OutputOwnership],
) -> Result<(), HotWalletError> {
    let xonly = bip32::pubkey_from_raw_key(raw_key)?;
    for (index, output) in transaction.outputs().iter().enumerate() {
        if !output.has_derivation_hint {
            continue;
        }
        if output.derivation_branch == 0
            && output.derivation_index == 0
            && p2pk_matches(&output.script_public_key, &xonly)
        {
            ownership[index] = OutputOwnership::Receive;
        }
    }
    Ok(())
}

fn verify_account_hints(
    transaction: &Transaction,
    account: &bip32::ExtendedPrivKey,
    ownership: &mut [OutputOwnership],
) -> Result<(), HotWalletError> {
    for (index, output) in transaction.outputs().iter().enumerate() {
        if !output.has_derivation_hint {
            continue;
        }
        if output.derivation_branch <= 1 {
            if let Ok(xonly) = hinted_output_xonly(account, output) {
                if p2pk_matches(&output.script_public_key, &xonly) {
                    ownership[index] = ownership_from_branch(output.derivation_branch);
                    continue;
                }
            }
        }
        // Derivation metadata is host-provided and therefore untrusted. If the
        // hint is stale or forged, independently derive both wallet branches.
        // A non-matching output remains External rather than aborting review.
        let Some(target) = p2pk_xonly(&output.script_public_key) else {
            continue;
        };
        if let Some(branch) = find_owned_output(account, &target, output.derivation_branch) {
            ownership[index] = ownership_from_branch(branch);
        }
    }
    Ok(())
}

fn verify_multisig_output_hints(
    transaction: &Transaction,
    configs: &[offline_signer::transaction::model::MultisigConfig],
    ownership: &mut [OutputOwnership],
) -> Result<(), HotWalletError> {
    if offline_signer::transaction::model::find_forged_change(transaction, configs).is_some() {
        return Err(HotWalletError::InvalidToolInput);
    }
    for (index, slot) in ownership.iter_mut().enumerate() {
        let Some(chain) = offline_signer::transaction::model::trusted_multisig_output_chain(
            transaction,
            configs,
            index,
        ) else {
            continue;
        };
        *slot = ownership_from_branch(chain);
    }
    Ok(())
}

fn hinted_output_xonly(
    account: &bip32::ExtendedPrivKey,
    output: &offline_signer::transaction::model::TransactionOutput,
) -> Result<[u8; 32], HotWalletError> {
    let child = if output.derivation_branch == 1 {
        bip32::derive_change_key(account, output.derivation_index)?
    } else {
        bip32::derive_address_key(account, output.derivation_index)?
    };
    child.public_key_x_only().map_err(HotWalletError::Bip32)
}

fn find_owned_output(
    account: &bip32::ExtendedPrivKey,
    target_xonly: &[u8; 32],
    preferred_branch: u8,
) -> Option<u8> {
    let branches = if preferred_branch == 1 {
        [1u8, 0u8]
    } else {
        [0u8, 1u8]
    };
    for branch in branches {
        for index in 0..bip32::ADDR_SCAN_DEPTH {
            let child = if branch == 1 {
                bip32::derive_change_key(account, u32::from(index))
            } else {
                bip32::derive_address_key(account, u32::from(index))
            };
            let Ok(child) = child else { continue };
            if child.public_key_x_only().ok().as_ref() == Some(target_xonly) {
                return Some(branch);
            }
        }
    }
    None
}

const fn ownership_from_branch(branch: u8) -> OutputOwnership {
    if branch == 1 {
        OutputOwnership::Change
    } else {
        OutputOwnership::Receive
    }
}

fn p2pk_xonly(script: &ScriptPublicKey) -> Option<[u8; 32]> {
    if script.script_len != 34 || script.script[0] != 0x20 || script.script[33] != 0xac {
        return None;
    }
    let mut xonly = [0u8; 32];
    xonly.copy_from_slice(&script.script[1..33]);
    Some(xonly)
}

fn p2pk_matches(script: &ScriptPublicKey, xonly: &[u8; 32]) -> bool {
    p2pk_xonly(script).is_some_and(|candidate| candidate == *xonly)
}

fn script_address(
    script: &ScriptPublicKey,
    network: offline_signer::address::KaspaNetwork,
) -> Option<String> {
    if network == offline_signer::address::KaspaNetwork::Unknown {
        return None;
    }
    let (address_type, start) = script_address_shape(script.script_bytes())?;
    let mut material = [0u8; 32];
    material.copy_from_slice(&script.script_bytes()[start..start + 32]);
    encode_script_address(&material, address_type, network)
}

fn script_address_shape(value: &[u8]) -> Option<(AddressType, usize)> {
    match (
        value.len(),
        value.first().copied(),
        value.get(1).copied(),
        value.last().copied(),
    ) {
        (34, Some(0x20), _, Some(0xac)) => Some((AddressType::P2pk, 1)),
        (35, Some(0xaa), Some(0x20), Some(0x87)) => Some((AddressType::P2sh, 2)),
        _ => None,
    }
}

fn encode_script_address(
    material: &[u8; 32],
    address_type: AddressType,
    network: offline_signer::address::KaspaNetwork,
) -> Option<String> {
    let mut encoded = [0u8; MAX_ADDR_LEN];
    let length = encode_address_for_network(material, address_type, network, &mut encoded);
    core::str::from_utf8(&encoded[..length])
        .ok()
        .map(str::to_owned)
}

const fn script_type_label(script_type: ScriptType) -> &'static str {
    match script_type {
        ScriptType::P2PK => "P2PK",
        ScriptType::P2SH => "P2SH",
        ScriptType::Multisig => "Multisig",
        ScriptType::Unknown => "Unknown",
    }
}
