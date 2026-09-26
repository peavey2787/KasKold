use blake2b_simd::Params;
use k256::schnorr::{Signature as K256Signature, VerifyingKey};
use serde_json::{Map, Value};

use super::{relay_fields::find_pubkey_position, wire};
use crate::{wire::kspt as kspt_wire, Network};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Signature {
    pub(super) position: u8,
    pub(super) sighash: u8,
    pub(super) bytes: [u8; 64],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)] // every specialized route is a covenant claim
pub(super) enum SpecializedRoute {
    PrivateSwapClaim,
    OracleV1Claim,
    CommitRevealClaim,
    MerkleClaim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SpecializedWitness {
    pub(super) route: SpecializedRoute,
    pub(super) signature_script: Vec<u8>,
    pub(super) supplied_mask: u16,
    pub(super) supplied_true_mask: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Input {
    pub(super) tx_id: [u8; 32],
    pub(super) index: u32,
    pub(super) amount: u64,
    pub(super) sequence: u64,
    pub(super) sig_op_count: u8,
    pub(super) script_version: u16,
    pub(super) script: Vec<u8>,
    /// Previous UTXO carries a covenant id; needed for exact KIP-9 storage mass.
    /// Compact KSPT v1 does not encode this metadata, so decoded KSPT defaults
    /// fail-conservatively to false while standard PSKT preserves it.
    pub(super) has_covenant_id: bool,
    pub(super) signatures: Vec<Signature>,
    pub(super) redeem: Vec<u8>,
    pub(super) derivation: Option<(u8, u32)>,
    pub(super) ms45: Option<(u32, u32, u32)>,
    pub(super) covenant_execution: Option<(u16, u16)>,
    /// Host-only typed witness plan for specialized PSKT covenant routes. This
    /// is deliberately not part of compact KSPT v1; raw KSPT therefore supports
    /// only the generic covenant witness grammar carried by covenantExecution.
    pub(super) specialized_witness: Option<SpecializedWitness>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Output {
    pub(super) amount: u64,
    pub(super) script_version: u16,
    pub(super) script: Vec<u8>,
    pub(super) derivation: Option<(u8, u32)>,
    pub(super) ms45: Option<(u32, u32, u32)>,
    pub(super) covenant: Option<(u16, [u8; 32])>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Transaction {
    pub(super) flags: u8,
    pub(super) version: u16,
    pub(super) locktime: u64,
    pub(super) subnetwork: [u8; 20],
    pub(super) gas: u64,
    pub(super) payload: Vec<u8>,
    pub(super) network: u8,
    pub(super) inputs: Vec<Input>,
    pub(super) outputs: Vec<Output>,
    pub(super) stealth: Option<[u8; 32]>,
}

impl kspt_wire::EncodeSource for Transaction {
    fn global(&self) -> kspt_wire::Global<'_> {
        kspt_wire::Global {
            flags: self.flags,
            version: self.version,
            input_count: self.inputs.len() as u32,
            output_count: self.outputs.len() as u8,
            locktime: self.locktime,
            subnetwork_id: self.subnetwork,
            gas: self.gas,
            payload: &self.payload,
        }
    }

    fn input(&self, index: usize) -> kspt_wire::Input<'_> {
        let input = &self.inputs[index];
        kspt_wire::Input {
            previous_tx_id: input.tx_id,
            previous_index: input.index,
            amount: input.amount,
            sequence: input.sequence,
            sig_op_count: input.sig_op_count,
            script_version: input.script_version,
            script: &input.script,
        }
    }

    fn signature_count(&self, input: usize) -> usize {
        self.inputs[input].signatures.len()
    }
    fn signature(&self, input: usize, slot: usize) -> kspt_wire::Signature {
        let signature = &self.inputs[input].signatures[slot];
        kspt_wire::Signature {
            position: signature.position,
            sighash: signature.sighash,
            bytes: signature.bytes,
        }
    }
    fn redeem(&self, input: usize) -> &[u8] {
        &self.inputs[input].redeem
    }
    fn output(&self, index: usize) -> kspt_wire::Output<'_> {
        let output = &self.outputs[index];
        kspt_wire::Output {
            amount: output.amount,
            script_version: output.script_version,
            script: &output.script,
        }
    }
    fn network(&self) -> u8 {
        self.network
    }
    fn stealth(&self) -> Option<[u8; 32]> {
        self.stealth
    }
    fn input_derivation(&self, index: usize) -> Option<kspt_wire::Derivation> {
        self.inputs[index]
            .derivation
            .map(|(branch, index)| kspt_wire::Derivation { branch, index })
    }
    fn output_derivation(&self, index: usize) -> Option<kspt_wire::Derivation> {
        self.outputs[index]
            .derivation
            .map(|(branch, index)| kspt_wire::Derivation { branch, index })
    }
    fn input_ms45(&self, index: usize) -> Option<kspt_wire::Ms45Derivation> {
        self.inputs[index]
            .ms45
            .map(|(cosigner, chain, index)| kspt_wire::Ms45Derivation {
                cosigner,
                chain,
                index,
            })
    }
    fn covenant_execution(&self, index: usize) -> Option<kspt_wire::CovenantExecution> {
        self.inputs[index]
            .covenant_execution
            .map(
                |(supplied_mask, supplied_true_mask)| kspt_wire::CovenantExecution {
                    supplied_mask,
                    supplied_true_mask,
                },
            )
    }
    fn output_ms45(&self, index: usize) -> Option<kspt_wire::Ms45Derivation> {
        self.outputs[index]
            .ms45
            .map(|(cosigner, chain, index)| kspt_wire::Ms45Derivation {
                cosigner,
                chain,
                index,
            })
    }
    fn covenant(&self, index: usize) -> Option<kspt_wire::Covenant> {
        self.outputs[index]
            .covenant
            .map(|(authorizing_input, id)| kspt_wire::Covenant {
                authorizing_input,
                id,
            })
    }
}

pub(crate) fn validate_and_merge(
    original_pskt_hex: &str,
    signed_kspt: &[u8],
    network: Network,
) -> Result<String, String> {
    let unsigned = super::relay::encode_pskt(original_pskt_hex, network)?;
    let expected = parse(&unsigned)?;
    let signed = parse(signed_kspt)?;
    validate_same_transaction(&expected, &signed, network)?;
    verify_all_signatures(&expected)?;
    verify_all_signatures(&signed)?;
    merge_signatures(original_pskt_hex, &signed)
}

fn validate_same_transaction(
    expected: &Transaction,
    signed: &Transaction,
    network: Network,
) -> Result<(), String> {
    if signed.network != network.kspt_code() || expected.network != network.kspt_code() {
        return Err("signed KSPT network does not match requested network".to_string());
    }
    let mut expected_body = expected.clone();
    let mut signed_body = signed.clone();
    expected_body.flags = 0;
    signed_body.flags = 0;
    for input in &mut expected_body.inputs {
        input.signatures.clear();
    }
    for input in &mut signed_body.inputs {
        input.signatures.clear();
    }
    if expected_body != signed_body {
        return Err("signed KSPT transaction body does not match the wallet PSKT".to_string());
    }
    if signed
        .inputs
        .iter()
        .all(|input| input.signatures.is_empty())
    {
        return Err("signed KSPT contains no signatures".to_string());
    }
    Ok(())
}

fn merge_signatures(original_pskt_hex: &str, transaction: &Transaction) -> Result<String, String> {
    let (format, mut root) = wire::decode(original_pskt_hex)?;
    let document = wire::document_mut(&mut root, format)?
        .as_object_mut()
        .ok_or_else(|| "PSKT not object".to_string())?;
    let inputs = document
        .get_mut("inputs")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "missing inputs".to_string())?;
    if inputs.len() != transaction.inputs.len() {
        return Err("signed KSPT input count does not match PSKT".to_string());
    }
    for (position, signed_input) in transaction.inputs.iter().enumerate() {
        merge_input(&mut inputs[position], signed_input, position)?;
    }
    wire::encode(format, &root)
}

fn merge_input(value: &mut Value, signed: &Input, index: usize) -> Result<(), String> {
    if signed.signatures.is_empty() {
        return Ok(());
    }
    let input = value
        .as_object_mut()
        .ok_or_else(|| format!("input[{index}] not object"))?;
    let redeem_hex = redeem_script_hex(input, index)?;
    let partials = partial_signatures_mut(input)?;
    merge_input_route(partials, redeem_hex.as_deref(), signed, index)
}

fn redeem_script_hex(input: &Map<String, Value>, index: usize) -> Result<Option<String>, String> {
    match input.get("redeemScript") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!(
            "input[{index}] redeemScript must be a hex string or null"
        )),
    }
}

fn merge_input_route(
    partials: &mut Map<String, Value>,
    redeem_hex: Option<&str>,
    signed: &Input,
    index: usize,
) -> Result<(), String> {
    let Some(redeem_hex) = redeem_hex else {
        return merge_p2pk(partials, signed, index);
    };
    let redeem = wire::decode_lower_hex(redeem_hex, &format!("input[{index}] redeemScript"))?;
    merge_redeem_signatures(partials, &redeem, &signed.signatures, index)
}

fn merge_redeem_signatures(
    partials: &mut Map<String, Value>,
    redeem: &[u8],
    signatures: &[Signature],
    index: usize,
) -> Result<(), String> {
    if super::relay_fields::parse_multisig_redeem(redeem).is_some() {
        return merge_multisig(partials, redeem, signatures, index);
    }
    merge_covenant(partials, redeem, signatures, index)
}

fn merge_p2pk(
    partials: &mut Map<String, Value>,
    signed: &Input,
    index: usize,
) -> Result<(), String> {
    if signed.script.len() != 34 || signed.script[0] != 0x20 || signed.script[33] != 0xac {
        return Err(format!("input[{index}] is not a standard P2PK input"));
    }
    if signed.signatures.len() != 1 {
        return Err(format!(
            "input[{index}] standard P2PK input must contain exactly one signature"
        ));
    }
    let signature = signed
        .signatures
        .first()
        .ok_or_else(|| format!("input[{index}] has no signature"))?;
    if signature.position != 0 {
        return Err(format!(
            "input[{index}] P2PK signature position must be zero"
        ));
    }
    require_sighash_all(signature, index)?;
    let public_key = format!("02{}", hex::encode(&signed.script[1..33]));
    insert_signature(partials, public_key, &signature.bytes, index)
}

fn merge_multisig(
    partials: &mut Map<String, Value>,
    redeem: &[u8],
    signatures: &[Signature],
    input_index: usize,
) -> Result<(), String> {
    for signature in signatures {
        require_sighash_all(signature, input_index)?;
        let key = multisig_xonly(redeem, signature.position).ok_or_else(|| {
            format!(
                "input[{input_index}] signature position {} is invalid",
                signature.position
            )
        })?;
        let public_key = format!("02{}", hex::encode(key));
        insert_signature(partials, public_key, &signature.bytes, input_index)?;
    }
    Ok(())
}

fn merge_covenant(
    partials: &mut Map<String, Value>,
    redeem: &[u8],
    signatures: &[Signature],
    input_index: usize,
) -> Result<(), String> {
    let resolution =
        shared_signer::covenant_branch::resolve_covenant_branches(redeem).map_err(|error| {
            format!("input[{input_index}] invalid covenant branch structure: {error:?}")
        })?;
    for signature in signatures {
        require_sighash_all(signature, input_index)?;
        let binding = resolution.key_at(signature.position).map_err(|_| {
            format!(
                "input[{input_index}] covenant signature position {} is invalid",
                signature.position
            )
        })?;
        let public_key = format!("02{}", hex::encode(binding.key));
        insert_signature(partials, public_key, &signature.bytes, input_index)?;
    }
    Ok(())
}

fn require_sighash_all(signature: &Signature, input_index: usize) -> Result<(), String> {
    if signature.sighash == 0x01 {
        Ok(())
    } else {
        Err(format!(
            "input[{input_index}] signed KSPT changed sighash type to 0x{:02x}",
            signature.sighash
        ))
    }
}

fn partial_signatures_mut(
    input: &mut Map<String, Value>,
) -> Result<&mut Map<String, Value>, String> {
    match input.get("partialSigs") {
        None | Some(Value::Null) => {
            input.insert("partialSigs".to_string(), Value::Object(Map::new()));
        }
        Some(Value::Object(_)) => {}
        Some(_) => return Err("partialSigs must be an object".to_string()),
    }
    input
        .get_mut("partialSigs")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "partialSigs normalization failed".to_string())
}

fn insert_signature(
    partials: &mut Map<String, Value>,
    public_key: String,
    signature: &[u8; 64],
    input_index: usize,
) -> Result<(), String> {
    let incoming = hex::encode(signature);
    if let Some(existing) = partials.get(&public_key) {
        let existing = existing
            .get("schnorr")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!("input[{input_index}] existing partial signature is malformed")
            })?;
        if existing != incoming {
            return Err(format!(
                "input[{input_index}] conflicting signature for public key {public_key}"
            ));
        }
        return Ok(());
    }
    if partials.len() >= usize::from(crate::SIGNER_CAPABILITIES.max_signatures_per_input) {
        return Err(format!(
            "input[{input_index}] partial signature capacity exceeded"
        ));
    }
    let mut value = Map::new();
    value.insert("schnorr".to_string(), Value::String(incoming));
    partials.insert(public_key, Value::Object(value));
    Ok(())
}

fn multisig_xonly(redeem: &[u8], position: u8) -> Option<[u8; 32]> {
    let public_key = (0u8..=u8::MAX).find_map(|candidate| {
        let start = 2usize.checked_add(usize::from(candidate).checked_mul(33)?)?;
        let key = redeem.get(start..start + 32)?;
        let mut prefixed = String::from("02");
        prefixed.push_str(&hex::encode(key));
        (find_pubkey_position(redeem, &prefixed) == Some(position)).then_some(key)
    })?;
    public_key.try_into().ok()
}

fn covenant_active_positions(
    branches: &shared_signer::covenant_branch::BranchResolution,
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Vec<u8> {
    (0..branches.len())
        .filter_map(|position| {
            let position = u8::try_from(position).ok()?;
            branches
                .key_at(position)
                .ok()?
                .matches_selectors(supplied_mask, supplied_true_mask)
                .then_some(position)
        })
        .collect()
}

pub(super) fn verified_complete(transaction: &Transaction) -> Result<bool, String> {
    if transaction.inputs.is_empty() {
        return Ok(false);
    }
    for (index, input) in transaction.inputs.iter().enumerate() {
        let Some(required) = required_signature_count(index, input)? else {
            return Ok(false);
        };
        if input.signatures.len() != required {
            return Ok(false);
        }
        verify_input_signatures(transaction, index)?;
    }
    Ok(true)
}

fn required_signature_count(index: usize, input: &Input) -> Result<Option<usize>, String> {
    if input.redeem.is_empty() {
        return Ok(canonical_p2pk(input).then_some(1));
    }
    if let Some((threshold, _)) = super::relay_fields::parse_multisig_redeem(&input.redeem) {
        return Ok(Some(usize::from(threshold)));
    }
    if input.specialized_witness.is_some() {
        return Ok((input.signatures.len() == 1).then_some(1));
    }
    generic_covenant_required(index, input)
}

fn canonical_p2pk(input: &Input) -> bool {
    input.script.len() == 34
        && input.script.first() == Some(&0x20)
        && input.script.get(33) == Some(&0xac)
}

fn generic_covenant_required(index: usize, input: &Input) -> Result<Option<usize>, String> {
    let Some((mask, truth)) = input.covenant_execution else {
        return Ok(None);
    };
    let branches = shared_signer::covenant_branch::resolve_covenant_branches(&input.redeem)
        .map_err(|error| format!("input[{index}] invalid covenant branch structure: {error:?}"))?;
    if mask != branches.selector_mask() || truth & !mask != 0 || branches.selector_mask() != 0b1 {
        return Ok(None);
    }
    let active = covenant_active_positions(&branches, mask, truth);
    if active.len() != 1 || input.signatures.len() != 1 {
        return Ok(None);
    }
    let Some(signature) = input.signatures.first() else {
        return Ok(None);
    };
    if signature.position != active[0] {
        return Ok(None);
    }
    let Ok(binding) = branches.key_at(active[0]) else {
        return Ok(None);
    };
    Ok((binding.decision_mask & 0b1 != 0).then_some(1))
}

fn verify_all_signatures(transaction: &Transaction) -> Result<(), String> {
    for index in 0..transaction.inputs.len() {
        verify_input_signatures(transaction, index)?;
    }
    Ok(())
}

pub(super) fn verified_signature_count_for_input(
    transaction: &Transaction,
    input_index: usize,
) -> Result<usize, String> {
    let input = transaction
        .inputs
        .get(input_index)
        .ok_or_else(|| "signature input index out of range".to_string())?;
    verify_input_signatures(transaction, input_index)?;
    if input.redeem.is_empty()
        || super::relay_fields::parse_multisig_redeem(&input.redeem).is_some()
    {
        return Ok(input.signatures.len());
    }
    if input.specialized_witness.is_some() {
        return specialized_signature_count(input, input_index);
    }
    generic_covenant_signature_count(input, input_index)
}

fn specialized_signature_count(input: &Input, input_index: usize) -> Result<usize, String> {
    if input.signatures.len() != 1 {
        return Err(format!(
            "input[{input_index}] typed specialized covenant requires exactly one verified signature"
        ));
    }
    Ok(1)
}

fn generic_covenant_signature_count(input: &Input, input_index: usize) -> Result<usize, String> {
    let (mask, truth) = input
        .covenant_execution
        .ok_or_else(|| format!("input[{input_index}] covenant is missing covenantExecution"))?;
    let branches = shared_signer::covenant_branch::resolve_covenant_branches(&input.redeem)
        .map_err(|error| {
            format!("input[{input_index}] invalid covenant branch structure: {error:?}")
        })?;
    validate_generic_covenant_count_binding(input, input_index, &branches, mask, truth)?;
    Ok(input.signatures.len())
}

fn validate_generic_covenant_count_binding(
    input: &Input,
    input_index: usize,
    branches: &shared_signer::covenant_branch::BranchResolution,
    mask: u16,
    truth: u16,
) -> Result<(), String> {
    if truth & !mask != 0 || mask != branches.selector_mask() {
        return Err(format!(
            "input[{input_index}] covenantExecution is not a complete selector assignment"
        ));
    }
    if branches.selector_mask() != 0b1 {
        return Err(format!(
            "input[{input_index}] generic covenant selector topology requires a typed specialized witness plan"
        ));
    }
    let active = covenant_active_positions(branches, mask, truth);
    if active.len() != 1 {
        return Err(format!(
            "input[{input_index}] generic covenant must have exactly one active signer"
        ));
    }
    if input
        .signatures
        .iter()
        .any(|signature| signature.position != active[0])
    {
        return Err(format!(
            "input[{input_index}] contains a signature outside the active covenant branch"
        ));
    }
    Ok(())
}

fn verify_input_signatures(transaction: &Transaction, input_index: usize) -> Result<(), String> {
    let input = transaction
        .inputs
        .get(input_index)
        .ok_or_else(|| "signature input index out of range".to_string())?;
    if input.signatures.is_empty() {
        return Ok(());
    }
    let message = sighash_all(transaction, input_index)?;
    let mut seen = [false; 256];
    for signature in &input.signatures {
        verify_one_input_signature(input, input_index, signature, &message, &mut seen)?;
    }
    Ok(())
}

fn verify_one_input_signature(
    input: &Input,
    input_index: usize,
    signature: &Signature,
    message: &[u8; 32],
    seen: &mut [bool; 256],
) -> Result<(), String> {
    require_sighash_all(signature, input_index)?;
    let slot = usize::from(signature.position);
    if seen[slot] {
        return Err(format!(
            "input[{input_index}] contains duplicate signature position {}",
            signature.position
        ));
    }
    seen[slot] = true;
    let public_key = bound_public_key(input, input_index, signature)?;
    verify_bip340(&public_key, message, &signature.bytes).map_err(|error| {
        format!(
            "input[{input_index}] signature position {}: {error}",
            signature.position
        )
    })
}

fn bound_public_key(
    input: &Input,
    input_index: usize,
    signature: &Signature,
) -> Result<[u8; 32], String> {
    if input.redeem.is_empty() {
        return p2pk_bound_key(input, input_index, signature.position);
    }
    if super::relay_fields::parse_multisig_redeem(&input.redeem).is_some() {
        return multisig_xonly(&input.redeem, signature.position).ok_or_else(|| {
            format!(
                "input[{input_index}] signature position {} has no bound public key",
                signature.position
            )
        });
    }
    covenant_bound_key(input, input_index, signature.position)
}

fn p2pk_bound_key(input: &Input, input_index: usize, position: u8) -> Result<[u8; 32], String> {
    if position != 0 || !canonical_p2pk(input) {
        return Err(format!(
            "input[{input_index}] signature is not bound to a canonical P2PK key"
        ));
    }
    input.script[1..33].try_into().map_err(|_| {
        format!("input[{input_index}] signature position {position} has no bound public key")
    })
}

fn covenant_bound_key(input: &Input, input_index: usize, position: u8) -> Result<[u8; 32], String> {
    let branches = shared_signer::covenant_branch::resolve_covenant_branches(&input.redeem)
        .map_err(|error| {
            format!("input[{input_index}] invalid covenant branch structure: {error:?}")
        })?;
    let binding = branches.key_at(position).map_err(|_| {
        format!(
            "input[{input_index}] covenant signature position {position} has no branch-bound key"
        )
    })?;
    if let Some((mask, truth)) = input.covenant_execution {
        if !binding.matches_selectors(mask, truth) {
            return Err(format!(
                "input[{input_index}] covenant signature position {position} is outside the proven execution branch"
            ));
        }
    }
    Ok(binding.key)
}

fn verify_bip340(
    public_key: &[u8; 32],
    message: &[u8; 32],
    signature: &[u8; 64],
) -> Result<(), String> {
    let key = VerifyingKey::from_bytes(public_key)
        .map_err(|_| "invalid Schnorr public key".to_string())?;
    let signature = K256Signature::try_from(signature.as_slice())
        .map_err(|_| "invalid Schnorr signature encoding".to_string())?;
    key.verify_raw(message, &signature)
        .map_err(|_| "Schnorr signature verification failed".to_string())
}

fn signing_hash_state() -> blake2b_simd::State {
    Params::new()
        .hash_length(32)
        .key(b"TransactionSigningHash")
        .to_state()
}

fn finish_hash(state: blake2b_simd::State) -> [u8; 32] {
    let hash = state.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

fn hash_previous_outputs(transaction: &Transaction) -> [u8; 32] {
    let mut state = signing_hash_state();
    for input in &transaction.inputs {
        state.update(&input.tx_id);
        state.update(&input.index.to_le_bytes());
    }
    finish_hash(state)
}

fn hash_sequences(transaction: &Transaction) -> [u8; 32] {
    let mut state = signing_hash_state();
    for input in &transaction.inputs {
        state.update(&input.sequence.to_le_bytes());
    }
    finish_hash(state)
}

fn hash_sig_op_counts(transaction: &Transaction) -> [u8; 32] {
    let mut state = signing_hash_state();
    for input in &transaction.inputs {
        state.update(&[input.sig_op_count]);
    }
    finish_hash(state)
}

fn hash_outputs(transaction: &Transaction) -> [u8; 32] {
    let mut state = signing_hash_state();
    for output in &transaction.outputs {
        state.update(&output.amount.to_le_bytes());
        state.update(&output.script_version.to_le_bytes());
        state.update(&(output.script.len() as u64).to_le_bytes());
        state.update(&output.script);
        if transaction.version >= 1 {
            match output.covenant {
                Some((authorizing_input, id)) => {
                    state.update(&[1]);
                    state.update(&authorizing_input.to_le_bytes());
                    state.update(&id);
                }
                None => {
                    state.update(&[0]);
                }
            }
        }
    }
    finish_hash(state)
}

fn hash_payload(transaction: &Transaction) -> [u8; 32] {
    if transaction.subnetwork == [0u8; 20] && transaction.payload.is_empty() {
        return [0u8; 32];
    }
    let mut state = signing_hash_state();
    state.update(&(transaction.payload.len() as u64).to_le_bytes());
    state.update(&transaction.payload);
    finish_hash(state)
}

fn sighash_all(transaction: &Transaction, input_index: usize) -> Result<[u8; 32], String> {
    let input = transaction
        .inputs
        .get(input_index)
        .ok_or_else(|| "sighash input index out of range".to_string())?;
    let mut state = signing_hash_state();
    state.update(&transaction.version.to_le_bytes());
    state.update(&hash_previous_outputs(transaction));
    state.update(&hash_sequences(transaction));
    if transaction.version == 0 {
        state.update(&hash_sig_op_counts(transaction));
    }
    state.update(&input.tx_id);
    state.update(&input.index.to_le_bytes());
    state.update(&input.script_version.to_le_bytes());
    state.update(&(input.script.len() as u64).to_le_bytes());
    state.update(&input.script);
    state.update(&input.amount.to_le_bytes());
    state.update(&input.sequence.to_le_bytes());
    if transaction.version == 0 {
        state.update(&[input.sig_op_count]);
    }
    state.update(&hash_outputs(transaction));
    state.update(&transaction.locktime.to_le_bytes());
    state.update(&transaction.subnetwork);
    state.update(&transaction.gas.to_le_bytes());
    state.update(&hash_payload(transaction));
    state.update(&[0x01]);
    Ok(finish_hash(state))
}

mod parser;

pub(super) fn parse(data: &[u8]) -> Result<Transaction, String> {
    parser::parse(data)
}

#[cfg(test)]
pub(crate) fn test_sighash_all_for_pskt(
    pskt_hex: &str,
    network: Network,
    input_index: usize,
) -> Result<[u8; 32], String> {
    let compact = super::relay::encode_pskt(pskt_hex, network)?;
    let transaction = parse(&compact)?;
    sighash_all(&transaction, input_index)
}
