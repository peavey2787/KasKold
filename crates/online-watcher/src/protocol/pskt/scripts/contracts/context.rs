// KasKold Companion Web — typed covenant routing context
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use serde_json::{Map, Value};

pub(crate) struct SignerBranch {
    pub(crate) is_owner: bool,
    pub(crate) is_beneficiary: bool,
}

impl SignerBranch {
    pub(crate) fn detect(
        redeem_body: &[u8],
        partial_signatures: &Map<String, Value>,
        force_beneficiary: bool,
    ) -> Result<Self, String> {
        if partial_signatures.is_empty() {
            return Ok(Self {
                is_owner: false,
                is_beneficiary: false,
            });
        }
        let resolution = resolve_outer_branches(redeem_body)?;
        let selector = detect_outer_selector(&resolution, partial_signatures)?;
        let branch = Self {
            is_owner: selector == Some(true),
            is_beneficiary: selector == Some(false),
        };
        if force_beneficiary && !branch.is_beneficiary {
            return Err(
                "forced beneficiary routing does not match the branch-bound signer".to_string(),
            );
        }
        Ok(branch)
    }
}

fn resolve_outer_branches(
    redeem_body: &[u8],
) -> Result<shared_signer::covenant_branch::BranchResolution, String> {
    if redeem_body.first() != Some(&0x63) {
        return Err("IF/ELSE covenant routing requires a canonical outer OP_IF".to_string());
    }
    shared_signer::covenant_branch::resolve_covenant_branches(redeem_body)
        .map_err(|error| format!("invalid covenant branch structure: {error:?}"))
}

fn detect_outer_selector(
    resolution: &shared_signer::covenant_branch::BranchResolution,
    partial_signatures: &Map<String, Value>,
) -> Result<Option<bool>, String> {
    let mut outer_selector = None;
    for public_key in partial_signatures.keys() {
        let selector = selector_for_key(resolution, public_key)?;
        merge_outer_selector(&mut outer_selector, selector)?;
    }
    Ok(outer_selector)
}

fn selector_for_key(
    resolution: &shared_signer::covenant_branch::BranchResolution,
    public_key: &str,
) -> Result<bool, String> {
    let key = canonical_xonly(public_key)?;
    let position = resolution
        .unique_position_for_key(&key)
        .map_err(|error| branch_key_error(error, public_key))?;
    let binding = resolution
        .key_at(position)
        .map_err(|_| format!("covenant signature key position is invalid: {public_key}"))?;
    if binding.decision_mask & 1 == 0 {
        return Err(
            "covenant signature key is not protected by the outer branch selector".to_string(),
        );
    }
    Ok(binding.if_mask & 1 != 0)
}

fn branch_key_error(
    error: shared_signer::covenant_branch::BranchResolveError,
    public_key: &str,
) -> String {
    match error {
        shared_signer::covenant_branch::BranchResolveError::AmbiguousKey => format!(
            "covenant signature key is present on multiple branches and requires an explicit branch policy: {public_key}"
        ),
        _ => format!("covenant signature key is not branch-bound: {public_key}"),
    }
}

fn merge_outer_selector(slot: &mut Option<bool>, selector: bool) -> Result<(), String> {
    match *slot {
        None => {
            *slot = Some(selector);
            Ok(())
        }
        Some(existing) if existing == selector => Ok(()),
        Some(_) => Err("covenant signatures span conflicting outer branches".to_string()),
    }
}

fn canonical_xonly(public_key: &str) -> Result<[u8; 32], String> {
    let bytes = public_key.as_bytes();
    if bytes.len() != 66
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        || !(bytes.starts_with(b"02") || bytes.starts_with(b"03"))
    {
        return Err("covenant signer public key must be canonical compressed SEC1 hex".to_string());
    }
    let encoded = &bytes[2..];
    hex::decode(encoded)
        .map_err(|_| "invalid covenant signer public key hex".to_string())?
        .try_into()
        .map_err(|_| "covenant signer public key must be 32 bytes".to_string())
}

pub(crate) struct SignaturePolicy {
    pub(crate) minimum_signatures: u64,
}

pub(crate) struct PrivateSwapContext {
    pub(crate) claim: bool,
}

pub(crate) struct OracleContext {
    pub(crate) v1: OracleV1Context,
    pub(crate) model_b: OracleModelBContext,
}

pub(crate) struct OracleV1Context {
    pub(crate) claim: bool,
    pub(crate) signature: Option<Vec<u8>>,
}

pub(crate) struct OracleModelBContext {
    pub(crate) risc0: bool,
    pub(crate) passthrough: bool,
    pub(crate) heartbeat: bool,
    pub(crate) consumer: bool,
}

pub(crate) struct ProofContext {
    pub(crate) zk_proof: Option<Vec<u8>>,
    pub(crate) zk_public_inputs: Option<Vec<Vec<u8>>>,
    pub(crate) zk_verification_key: Option<Vec<u8>>,
    pub(crate) risc0_seal: Option<Vec<u8>>,
    pub(crate) risc0_fields: Option<Map<String, Value>>,
    pub(crate) risc0_bridge: bool,
    pub(crate) groth16_bridge: bool,
}

pub(crate) struct CommitRevealContext {
    pub(crate) part_a: Option<Vec<u8>>,
    pub(crate) part_b: Option<Vec<u8>>,
    pub(crate) preimage: Option<Vec<u8>>,
}

pub(crate) struct MerkleContext {
    pub(crate) proof: Option<String>,
    pub(crate) destination_script: Option<Vec<u8>>,
}

pub(crate) struct BridgeContext {
    pub(crate) withdrawal_script: Option<Vec<u8>>,
}

pub(crate) struct RollupContext {
    pub(crate) state_advance: bool,
    pub(crate) state_refund: bool,
    pub(crate) proof: Option<Vec<u8>>,
    pub(crate) prefix: Option<Vec<u8>>,
    pub(crate) suffix: Option<Vec<u8>>,
    pub(crate) deposit_advance: bool,
    pub(crate) unified_advance: bool,
    pub(crate) forced_exit: bool,
    pub(crate) deposit_holding_credit: bool,
    pub(crate) deposit_holding_refund: bool,
}

pub(crate) struct CovenantContext {
    pub(crate) signatures: SignaturePolicy,
    pub(crate) private_swap: PrivateSwapContext,
    pub(crate) oracle: OracleContext,
    pub(crate) proofs: ProofContext,
    pub(crate) commit_reveal: CommitRevealContext,
    pub(crate) merkle: MerkleContext,
    pub(crate) bridge: BridgeContext,
    pub(crate) rollup: RollupContext,
}

impl CovenantContext {
    pub(crate) fn parse(input: &Map<String, Value>) -> Result<Self, String> {
        let proprietary = proprietary_fields(input)?;
        let minimum_signatures = minimum_signatures(input)?;
        Ok(Self {
            signatures: SignaturePolicy { minimum_signatures },
            private_swap: parse_private_swap(proprietary)?,
            oracle: parse_oracle(proprietary)?,
            proofs: parse_proofs(proprietary)?,
            commit_reveal: parse_commit_reveal(proprietary)?,
            merkle: parse_merkle(proprietary)?,
            bridge: parse_bridge(proprietary)?,
            rollup: parse_rollup(proprietary)?,
        })
    }
}

fn proprietary_fields(input: &Map<String, Value>) -> Result<Option<&Map<String, Value>>, String> {
    match input.get("proprietaries") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(values)) => Ok(Some(values)),
        Some(_) => Err("proprietaries must be an object or null".to_string()),
    }
}

fn minimum_signatures(input: &Map<String, Value>) -> Result<u64, String> {
    match input.get("minimumSignatures") {
        None | Some(Value::Null) => Ok(1),
        Some(Value::Number(value)) => value
            .as_u64()
            .ok_or_else(|| "minimumSignatures must be an unsigned integer".to_string()),
        Some(Value::String(value)) => parse_canonical_u64(value, "minimumSignatures"),
        Some(_) => {
            Err("minimumSignatures must be an unsigned integer or decimal string".to_string())
        }
    }
}

fn parse_private_swap(
    proprietary: Option<&Map<String, Value>>,
) -> Result<PrivateSwapContext, String> {
    Ok(PrivateSwapContext {
        claim: bool_value(proprietary, "privateSwapClaim")?,
    })
}

fn parse_oracle(proprietary: Option<&Map<String, Value>>) -> Result<OracleContext, String> {
    Ok(OracleContext {
        v1: OracleV1Context {
            claim: bool_value(proprietary, "oracleV1Claim")?,
            signature: decode_hex(proprietary, "oracleV1Signature")?,
        },
        model_b: OracleModelBContext {
            risc0: bool_value(proprietary, "risc0OracleMb")?,
            passthrough: bool_value(proprietary, "oracleMbPassthrough")?,
            heartbeat: bool_value(proprietary, "oracleMbHeartbeat")?,
            consumer: bool_value(proprietary, "oracleMbConsumer")?,
        },
    })
}

fn parse_proofs(proprietary: Option<&Map<String, Value>>) -> Result<ProofContext, String> {
    Ok(ProofContext {
        zk_proof: decode_hex(proprietary, "zkProof")?,
        zk_public_inputs: decode_hex_array(proprietary, "zkPublicInputs")?,
        zk_verification_key: decode_hex(proprietary, "zkVk")?,
        risc0_seal: decode_hex(proprietary, "risc0Seal")?,
        risc0_fields: object_value(proprietary, "risc0Fields")?,
        risc0_bridge: bool_value(proprietary, "risc0Bridge")?,
        groth16_bridge: bool_value(proprietary, "groth16Bridge")?,
    })
}

fn parse_commit_reveal(
    proprietary: Option<&Map<String, Value>>,
) -> Result<CommitRevealContext, String> {
    Ok(CommitRevealContext {
        part_a: decode_hex(proprietary, "commitPartA")?,
        part_b: decode_hex(proprietary, "commitPartB")?,
        preimage: decode_hex(proprietary, "commitPreimage")?,
    })
}

fn parse_merkle(proprietary: Option<&Map<String, Value>>) -> Result<MerkleContext, String> {
    Ok(MerkleContext {
        proof: string_value(proprietary, "merkleProof")?,
        destination_script: decode_hex(proprietary, "merkleDestSpk")?,
    })
}

fn parse_bridge(proprietary: Option<&Map<String, Value>>) -> Result<BridgeContext, String> {
    Ok(BridgeContext {
        withdrawal_script: decode_hex(proprietary, "withdrawalSpk")?,
    })
}

fn parse_rollup(proprietary: Option<&Map<String, Value>>) -> Result<RollupContext, String> {
    let (state_advance, state_refund, deposit_advance, unified_advance) =
        rollup_primary_flags(proprietary)?;
    let (forced_exit, deposit_holding_credit, deposit_holding_refund) =
        rollup_exit_flags(proprietary)?;
    let (proof, prefix, suffix) = rollup_payloads(proprietary)?;
    Ok(RollupContext {
        state_advance,
        state_refund,
        proof,
        prefix,
        suffix,
        deposit_advance,
        unified_advance,
        forced_exit,
        deposit_holding_credit,
        deposit_holding_refund,
    })
}

fn rollup_primary_flags(
    proprietary: Option<&Map<String, Value>>,
) -> Result<(bool, bool, bool, bool), String> {
    Ok((
        bool_value(proprietary, "rollupStateAdvance")?,
        bool_value(proprietary, "rollupStateRefund")?,
        bool_value(proprietary, "rollupDepositAdvance")?,
        bool_value(proprietary, "rollupUnifiedAdvance")?,
    ))
}

fn rollup_exit_flags(
    proprietary: Option<&Map<String, Value>>,
) -> Result<(bool, bool, bool), String> {
    Ok((
        bool_value(proprietary, "rollupForcedExit")?,
        bool_value(proprietary, "depositHoldingCredit")?,
        bool_value(proprietary, "depositHoldingRefund")?,
    ))
}

/// Optional rollup proof, prefix and suffix payloads.
type RollupPayloads = (Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>);

fn rollup_payloads(proprietary: Option<&Map<String, Value>>) -> Result<RollupPayloads, String> {
    Ok((
        decode_hex(proprietary, "rollupProof")?,
        decode_hex(proprietary, "rollupPrefix")?,
        decode_hex(proprietary, "rollupSuffix")?,
    ))
}

fn field<'a>(proprietary: Option<&'a Map<String, Value>>, key: &str) -> Option<&'a Value> {
    proprietary.and_then(|fields| fields.get(key))
}

fn parse_canonical_u64(value: &str, key: &str) -> Result<u64, String> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(format!("{key} must be a canonical unsigned decimal string"));
    }
    value
        .parse::<u64>()
        .map_err(|_| format!("{key} exceeds u64"))
}

fn decode_hex(
    proprietary: Option<&Map<String, Value>>,
    key: &str,
) -> Result<Option<Vec<u8>>, String> {
    let Some(value) = field(proprietary, key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let text = value
        .as_str()
        .ok_or_else(|| format!("{key} must be a hex string or null"))?;
    let bytes = text.as_bytes();
    if bytes.len() % 2 != 0
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(format!("{key} must be even-length lowercase hexadecimal"));
    }
    hex::decode(text)
        .map(Some)
        .map_err(|_| format!("{key} contains invalid hex"))
}

fn decode_hex_array(
    proprietary: Option<&Map<String, Value>>,
    key: &str,
) -> Result<Option<Vec<Vec<u8>>>, String> {
    let Some(value) = field(proprietary, key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let values = value
        .as_array()
        .ok_or_else(|| format!("{key} must be an array of hex strings or null"))?;
    let mut decoded = Vec::with_capacity(values.len());
    for (index, item) in values.iter().enumerate() {
        let text = item
            .as_str()
            .ok_or_else(|| format!("{key}[{index}] must be a hex string"))?;
        let bytes = text.as_bytes();
        if bytes.len() % 2 != 0
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        {
            return Err(format!(
                "{key}[{index}] must be even-length lowercase hexadecimal"
            ));
        }
        decoded
            .push(hex::decode(text).map_err(|_| format!("{key}[{index}] contains invalid hex"))?);
    }
    Ok(Some(decoded))
}

fn string_value(
    proprietary: Option<&Map<String, Value>>,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(value) = field(proprietary, key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(|value| Some(value.to_owned()))
        .ok_or_else(|| format!("{key} must be a string or null"))
}

fn object_value(
    proprietary: Option<&Map<String, Value>>,
    key: &str,
) -> Result<Option<Map<String, Value>>, String> {
    let Some(value) = field(proprietary, key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_object()
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("{key} must be an object or null"))
}

fn bool_value(proprietary: Option<&Map<String, Value>>, key: &str) -> Result<bool, String> {
    let Some(value) = field(proprietary, key) else {
        return Ok(false);
    };
    if value.is_null() {
        return Ok(false);
    }
    value
        .as_bool()
        .ok_or_else(|| format!("{key} must be boolean or null"))
}
