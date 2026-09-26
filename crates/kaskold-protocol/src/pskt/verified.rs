//! Typed authorization result used by consumer finalizers.
//!
//! A `VerifiedTransaction` is created only after canonical parsing, semantic
//! validation, branch binding and Schnorr verification have all succeeded.
//! Finalizers must materialize consensus bytes exclusively from this object;
//! reparsing the original PSKT/KSPT after authorization is forbidden.

use super::{compact, relay_fields::parse_multisig_redeem};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedSignature {
    position: u8,
    sighash: u8,
    bytes: [u8; 64],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifiedCovenantRoute {
    PrivateSwapClaim,
    OracleV1Claim,
    CommitRevealClaim,
    MerkleClaim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifiedWitnessPlan {
    P2pk {
        signature: VerifiedSignature,
    },
    Multisig {
        threshold: u8,
        signatures: Vec<VerifiedSignature>,
        redeem_script: Vec<u8>,
    },
    /// Generic branch-aware covenant witness. Consumer finalization deliberately
    /// supports one canonical outer selector only. More complex/specialized
    /// covenants must define a typed execution plan before they can cross this
    /// boundary; host routing hints are never consulted after authorization.
    Covenant {
        signature: VerifiedSignature,
        redeem_script: Vec<u8>,
        supplied_mask: u16,
        supplied_true_mask: u16,
    },
    /// Specialized covenant witness whose template, branch selectors and route
    /// metadata were validated before authorization. The exact script is frozen
    /// into the verified model so materialization performs no second parse.
    SpecializedCovenant {
        route: VerifiedCovenantRoute,
        signature_script: Vec<u8>,
        supplied_mask: u16,
        supplied_true_mask: u16,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedInput {
    previous_tx_id: [u8; 32],
    previous_index: u32,
    amount: u64,
    sequence: u64,
    sig_op_count: u8,
    script_version: u16,
    script_public_key: Vec<u8>,
    has_covenant_id: bool,
    witness: VerifiedWitnessPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedOutput {
    amount: u64,
    script_version: u16,
    script_public_key: Vec<u8>,
    covenant: Option<(u16, [u8; 32])>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedTransaction {
    version: u16,
    locktime: u64,
    subnetwork_id: [u8; 20],
    gas: u64,
    payload: Vec<u8>,
    inputs: Vec<VerifiedInput>,
    outputs: Vec<VerifiedOutput>,
}

impl VerifiedSignature {
    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn position(&self) -> u8 {
        self.position
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn sighash(&self) -> u8 {
        self.sighash
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn bytes(&self) -> &[u8; 64] {
        &self.bytes
    }
}

impl VerifiedInput {
    #[must_use]
    pub const fn previous_tx_id(&self) -> &[u8; 32] {
        &self.previous_tx_id
    }

    #[must_use]
    pub const fn previous_index(&self) -> u32 {
        self.previous_index
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn amount(&self) -> u64 {
        self.amount
    }

    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub const fn sig_op_count(&self) -> u8 {
        self.sig_op_count
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn script_version(&self) -> u16 {
        self.script_version
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub fn script_public_key(&self) -> &[u8] {
        &self.script_public_key
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn has_covenant_id(&self) -> bool {
        self.has_covenant_id
    }

    #[must_use]
    pub const fn witness(&self) -> &VerifiedWitnessPlan {
        &self.witness
    }
}

impl VerifiedOutput {
    #[must_use]
    pub const fn amount(&self) -> u64 {
        self.amount
    }

    #[must_use]
    pub const fn script_version(&self) -> u16 {
        self.script_version
    }

    #[must_use]
    pub fn script_public_key(&self) -> &[u8] {
        &self.script_public_key
    }

    #[must_use]
    pub const fn covenant(&self) -> Option<(u16, [u8; 32])> {
        self.covenant
    }
}

impl VerifiedTransaction {
    #[must_use]
    pub const fn version(&self) -> u16 {
        self.version
    }

    #[must_use]
    pub const fn locktime(&self) -> u64 {
        self.locktime
    }

    #[must_use]
    pub const fn subnetwork_id(&self) -> &[u8; 20] {
        &self.subnetwork_id
    }

    #[must_use]
    pub const fn gas(&self) -> u64 {
        self.gas
    }

    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    #[must_use]
    pub fn inputs(&self) -> &[VerifiedInput] {
        &self.inputs
    }

    #[must_use]
    pub fn outputs(&self) -> &[VerifiedOutput] {
        &self.outputs
    }
}

impl VerifiedWitnessPlan {
    /// Serialize exactly the witness plan that was authorized. No source JSON or
    /// KSPT bytes are consulted here.
    pub fn materialize_signature_script(&self) -> Result<Vec<u8>, String> {
        match self {
            Self::P2pk { signature } => materialize_p2pk(signature),
            Self::Multisig {
                threshold,
                signatures,
                redeem_script,
            } => materialize_multisig(*threshold, signatures, redeem_script),
            Self::Covenant {
                signature,
                redeem_script,
                supplied_mask,
                supplied_true_mask,
            } => materialize_covenant(
                signature,
                redeem_script,
                *supplied_mask,
                *supplied_true_mask,
            ),
            Self::SpecializedCovenant {
                signature_script,
                supplied_mask,
                supplied_true_mask,
                ..
            } => materialize_specialized(signature_script, *supplied_mask, *supplied_true_mask),
        }
    }
}

fn materialize_p2pk(signature: &VerifiedSignature) -> Result<Vec<u8>, String> {
    let mut script = Vec::with_capacity(66);
    push_signature(&mut script, signature)?;
    Ok(script)
}

fn materialize_multisig(
    threshold: u8,
    signatures: &[VerifiedSignature],
    redeem_script: &[u8],
) -> Result<Vec<u8>, String> {
    if signatures.len() != usize::from(threshold) {
        return Err("verified multisig witness cardinality changed".to_string());
    }
    let mut script = Vec::new();
    for signature in signatures {
        push_signature(&mut script, signature)?;
    }
    push_data(&mut script, redeem_script)?;
    Ok(script)
}

fn materialize_covenant(
    signature: &VerifiedSignature,
    redeem_script: &[u8],
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<Vec<u8>, String> {
    if supplied_mask != 0b1 || supplied_true_mask & !supplied_mask != 0 {
        return Err("verified generic covenant selector plan is invalid".to_string());
    }
    let mut script = Vec::new();
    push_signature(&mut script, signature)?;
    script.push(if supplied_true_mask & 1 != 0 {
        0x51
    } else {
        0x00
    });
    push_data(&mut script, redeem_script)?;
    Ok(script)
}

fn materialize_specialized(
    signature_script: &[u8],
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<Vec<u8>, String> {
    if supplied_true_mask & !supplied_mask != 0 {
        return Err("verified specialized covenant selector plan is invalid".to_string());
    }
    Ok(signature_script.to_vec())
}

fn push_signature(script: &mut Vec<u8>, signature: &VerifiedSignature) -> Result<(), String> {
    if signature.sighash != crate::wire::pskt_schema::SIGHASH_ALL {
        return Err("verified witness contains unsupported sighash".to_string());
    }
    script.push(65);
    script.extend_from_slice(&signature.bytes);
    script.push(signature.sighash);
    Ok(())
}

fn push_data(script: &mut Vec<u8>, data: &[u8]) -> Result<(), String> {
    match data.len() {
        0..=75 => script.push(data.len() as u8),
        76..=255 => script.extend_from_slice(&[0x4c, data.len() as u8]),
        256..=65535 => {
            script.push(0x4d);
            script.extend_from_slice(&(data.len() as u16).to_le_bytes());
        }
        _ => return Err("verified redeem script exceeds push-data limit".to_string()),
    }
    script.extend_from_slice(data);
    Ok(())
}

pub(super) fn from_compact(
    transaction: compact::Transaction,
) -> Result<VerifiedTransaction, String> {
    let mut inputs = Vec::new();
    inputs
        .try_reserve(transaction.inputs.len())
        .map_err(|_| "verified input allocation failed".to_string())?;
    for (index, input) in transaction.inputs.into_iter().enumerate() {
        let witness = witness_plan(index, &input)?;
        inputs.push(VerifiedInput {
            previous_tx_id: input.tx_id,
            previous_index: input.index,
            amount: input.amount,
            sequence: input.sequence,
            sig_op_count: input.sig_op_count,
            script_version: input.script_version,
            script_public_key: input.script,
            has_covenant_id: input.has_covenant_id,
            witness,
        });
    }

    let outputs = transaction
        .outputs
        .into_iter()
        .map(|output| VerifiedOutput {
            amount: output.amount,
            script_version: output.script_version,
            script_public_key: output.script,
            covenant: output.covenant,
        })
        .collect();

    Ok(VerifiedTransaction {
        version: transaction.version,
        locktime: transaction.locktime,
        subnetwork_id: transaction.subnetwork,
        gas: transaction.gas,
        payload: transaction.payload,
        inputs,
        outputs,
    })
}

fn witness_plan(index: usize, input: &compact::Input) -> Result<VerifiedWitnessPlan, String> {
    if input.redeem.is_empty() {
        return p2pk_witness_plan(index, input);
    }
    require_p2sh_redeem_binding(index, &input.script, &input.redeem)?;
    if let Some(specialized) = &input.specialized_witness {
        return specialized_witness_plan(specialized);
    }
    if let Some((threshold, _)) = parse_multisig_redeem(&input.redeem) {
        return multisig_witness_plan(index, input, threshold);
    }
    covenant_witness_plan(index, input)
}

fn p2pk_witness_plan(index: usize, input: &compact::Input) -> Result<VerifiedWitnessPlan, String> {
    require_p2pk_script(index, &input.script)?;
    let signature = exactly_one_signature(index, &input.signatures)?;
    if signature.position != 0 {
        return Err(format!(
            "input[{index}] P2PK signature position must be zero"
        ));
    }
    Ok(VerifiedWitnessPlan::P2pk {
        signature: copy_signature(signature),
    })
}

fn specialized_witness_plan(
    specialized: &compact::SpecializedWitness,
) -> Result<VerifiedWitnessPlan, String> {
    let route = match specialized.route {
        compact::SpecializedRoute::PrivateSwapClaim => VerifiedCovenantRoute::PrivateSwapClaim,
        compact::SpecializedRoute::OracleV1Claim => VerifiedCovenantRoute::OracleV1Claim,
        compact::SpecializedRoute::CommitRevealClaim => VerifiedCovenantRoute::CommitRevealClaim,
        compact::SpecializedRoute::MerkleClaim => VerifiedCovenantRoute::MerkleClaim,
    };
    Ok(VerifiedWitnessPlan::SpecializedCovenant {
        route,
        signature_script: specialized.signature_script.clone(),
        supplied_mask: specialized.supplied_mask,
        supplied_true_mask: specialized.supplied_true_mask,
    })
}

fn multisig_witness_plan(
    index: usize,
    input: &compact::Input,
    threshold: u8,
) -> Result<VerifiedWitnessPlan, String> {
    if input.signatures.len() != usize::from(threshold) {
        return Err(format!(
            "input[{index}] verified multisig witness has {} signatures, expected {threshold}",
            input.signatures.len()
        ));
    }
    let mut signatures = input
        .signatures
        .iter()
        .map(copy_signature)
        .collect::<Vec<_>>();
    signatures.sort_by_key(|signature| signature.position);
    Ok(VerifiedWitnessPlan::Multisig {
        threshold,
        signatures,
        redeem_script: input.redeem.clone(),
    })
}

fn covenant_witness_plan(
    index: usize,
    input: &compact::Input,
) -> Result<VerifiedWitnessPlan, String> {
    let (supplied_mask, supplied_true_mask) = input
        .covenant_execution
        .ok_or_else(|| format!("input[{index}] covenant is missing covenantExecution"))?;
    let branches = shared_signer::covenant_branch::resolve_covenant_branches(&input.redeem)
        .map_err(|error| format!("input[{index}] invalid covenant branch structure: {error:?}"))?;
    validate_covenant_selector_plan(index, &branches, supplied_mask, supplied_true_mask)?;
    let signature = exactly_one_signature(index, &input.signatures)?;
    validate_covenant_signature(
        index,
        &branches,
        signature,
        supplied_mask,
        supplied_true_mask,
    )?;
    Ok(VerifiedWitnessPlan::Covenant {
        signature: copy_signature(signature),
        redeem_script: input.redeem.clone(),
        supplied_mask,
        supplied_true_mask,
    })
}

fn validate_covenant_selector_plan(
    index: usize,
    branches: &shared_signer::covenant_branch::BranchResolution,
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<(), String> {
    if supplied_true_mask & !supplied_mask != 0 || supplied_mask != branches.selector_mask() {
        return Err(format!(
            "input[{index}] covenantExecution is not a complete selector assignment"
        ));
    }
    if branches.selector_mask() != 0b1 {
        return Err(format!(
            "input[{index}] covenant selector topology requires a typed specialized witness plan"
        ));
    }
    Ok(())
}

fn validate_covenant_signature(
    index: usize,
    branches: &shared_signer::covenant_branch::BranchResolution,
    signature: &compact::Signature,
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<(), String> {
    let binding = branches
        .key_at(signature.position)
        .map_err(|_| format!("input[{index}] covenant signature position is not branch-bound"))?;
    if !binding.matches_selectors(supplied_mask, supplied_true_mask) {
        return Err(format!(
            "input[{index}] covenant signature is outside the authorized execution branch"
        ));
    }
    if binding.decision_mask & 1 == 0 {
        return Err(format!(
            "input[{index}] covenant signer is not protected by the outer selector"
        ));
    }
    Ok(())
}

fn exactly_one_signature(
    index: usize,
    signatures: &[compact::Signature],
) -> Result<&compact::Signature, String> {
    if signatures.len() != 1 {
        return Err(format!(
            "input[{index}] witness plan requires exactly one verified signature, got {}",
            signatures.len()
        ));
    }
    signatures
        .first()
        .ok_or_else(|| format!("input[{index}] verified signature is missing"))
}

fn copy_signature(signature: &compact::Signature) -> VerifiedSignature {
    VerifiedSignature {
        position: signature.position,
        sighash: signature.sighash,
        bytes: signature.bytes,
    }
}

fn require_p2pk_script(index: usize, script: &[u8]) -> Result<(), String> {
    if script.len() == 34 && script.first() == Some(&0x20) && script.get(33) == Some(&0xac) {
        Ok(())
    } else {
        Err(format!(
            "input[{index}] signature without redeemScript is not canonical P2PK"
        ))
    }
}

fn require_p2sh_redeem_binding(index: usize, script: &[u8], redeem: &[u8]) -> Result<(), String> {
    if script.len() != 35
        || script.first() != Some(&0xaa)
        || script.get(1) != Some(&0x20)
        || script.get(34) != Some(&0x87)
    {
        return Err(format!(
            "input[{index}] redeemScript is present but scriptPublicKey is not canonical P2SH"
        ));
    }
    let hash = blake2b_simd::Params::new().hash_length(32).hash(redeem);
    if script.get(2..34) != Some(hash.as_bytes()) {
        return Err(format!(
            "input[{index}] redeemScript hash does not match scriptPublicKey"
        ));
    }
    Ok(())
}
