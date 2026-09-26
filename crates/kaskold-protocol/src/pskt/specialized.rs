//! Typed, template-bound witness plans for the specialized covenant families
//! that are currently exposed by Companion transaction builders.
//!
//! Proprietary metadata is never allowed to select a branch on its own. A route
//! is accepted only when (1) the redeem script exactly matches the recognized
//! template, (2) `covenantExecution` supplies every structural selector, (3)
//! those selectors choose the route's branch, (4) the already-verified
//! transaction signature is bound to that branch, and (5) route-specific proof
//! material validates against commitments embedded in the redeem script.

use blake2b_simd::Params;
use k256::schnorr::{Signature as K256Signature, VerifyingKey};
use serde_json::{Map, Value};

use super::compact::{Input, Signature, SpecializedRoute, SpecializedWitness};

const OP_0: u8 = 0x00;
const OP_1: u8 = 0x51;
const OP_IF: u8 = 0x63;
const OP_ELSE: u8 = 0x67;
const OP_ENDIF: u8 = 0x68;
const OP_VERIFY: u8 = 0x69;
const OP_DROP: u8 = 0x75;
const OP_DUP: u8 = 0x76;
const OP_SWAP: u8 = 0x7c;
const OP_CAT: u8 = 0x7e;
const OP_EQUALVERIFY: u8 = 0x88;
const OP_SUB: u8 = 0x94;
const OP_NUMEQUALVERIFY: u8 = 0x9d;
const OP_LESSTHANOREQUAL: u8 = 0xa1;
const OP_GREATERTHANOREQUAL: u8 = 0xa2;
const OP_BLAKE2B: u8 = 0xaa;
const OP_CHECKSIGVERIFY: u8 = 0xad;
const OP_CHECKLOCKTIMEVERIFY: u8 = 0xb0;
const OP_TX_INPUT_COUNT: u8 = 0xb3;
const OP_TX_OUTPUT_COUNT: u8 = 0xb4;
const OP_TX_INPUT_AMOUNT: u8 = 0xbe;
const OP_TX_OUTPUT_AMOUNT: u8 = 0xc2;
const OP_TX_OUTPUT_SPK: u8 = 0xc3;
const OP_CHECKSIGFROMSTACK: u8 = 0xd7;
const PRIVATE_SWAP_MAX_FEE_SOMPI: u64 = 500_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RouteKind {
    PrivateSwap,
    OracleV1,
    CommitReveal,
    Merkle,
}

pub(super) fn bind_specialized_witnesses(
    global: &Map<String, Value>,
    source_inputs: &[Value],
    transaction: &mut super::compact::Transaction,
) -> Result<(), String> {
    if global
        .get("covenantBranch")
        .is_some_and(|value| !value.is_null())
    {
        return Err(
            "global.covenantBranch is forbidden at the verified boundary; covenantExecution is authoritative"
                .to_string(),
        );
    }
    reject_all_specialized(global.get("proprietaries"), "global.proprietaries")?;
    if source_inputs.len() != transaction.inputs.len() {
        return Err("specialized witness input count mismatch".to_string());
    }

    for (index, (source, input)) in source_inputs
        .iter()
        .zip(transaction.inputs.iter_mut())
        .enumerate()
    {
        let object = source
            .as_object()
            .ok_or_else(|| format!("input[{index}] not object"))?;
        let proprietary = optional_object(
            object.get("proprietaries"),
            &format!("input[{index}].proprietaries"),
        )?;
        let Some(proprietary) = proprietary else {
            continue;
        };
        let route = detect_route(proprietary, index)?;
        let Some(route) = route else {
            continue;
        };
        let specialized = bind_route(index, route, proprietary, input)?;
        input.specialized_witness = Some(specialized);
    }
    Ok(())
}

fn optional_object<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> Result<Option<&'a Map<String, Value>>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(value)) => Ok(Some(value)),
        Some(_) => Err(format!("{label} must be an object or null")),
    }
}

fn detect_route(fields: &Map<String, Value>, index: usize) -> Result<Option<RouteKind>, String> {
    let routes = route_presence(fields);
    reject_mixed_routes(routes, index)?;
    reject_unsupported_route_fields(fields, index)?;
    Ok(route_from_presence(routes))
}

fn route_presence(fields: &Map<String, Value>) -> [bool; 4] {
    [
        fields.contains_key("privateSwapClaim"),
        fields.contains_key("oracleV1Claim") || fields.contains_key("oracleV1Signature"),
        fields.contains_key("commitPartA") || fields.contains_key("commitPartB"),
        fields.contains_key("merkleProof") || fields.contains_key("merkleDestSpk"),
    ]
}

fn reject_mixed_routes(routes: [bool; 4], index: usize) -> Result<(), String> {
    if routes.into_iter().filter(|value| *value).count() > 1 {
        return Err(format!(
            "input[{index}] mixes multiple specialized covenant routes"
        ));
    }
    Ok(())
}

fn reject_unsupported_route_fields(
    fields: &Map<String, Value>,
    index: usize,
) -> Result<(), String> {
    for key in fields.keys() {
        if crate::wire::pskt_schema::is_specialized_covenant_routing_field(key)
            && !crate::wire::pskt_schema::is_supported_specialized_covenant_routing_field(key)
        {
            return Err(format!(
                "input[{index}].proprietaries.{key} has no typed verified witness plan"
            ));
        }
    }
    Ok(())
}

fn route_from_presence(routes: [bool; 4]) -> Option<RouteKind> {
    let [private, oracle, commit, merkle] = routes;
    if private {
        Some(RouteKind::PrivateSwap)
    } else if oracle {
        Some(RouteKind::OracleV1)
    } else if commit {
        Some(RouteKind::CommitReveal)
    } else if merkle {
        Some(RouteKind::Merkle)
    } else {
        None
    }
}

fn reject_all_specialized(value: Option<&Value>, label: &str) -> Result<(), String> {
    let Some(object) = optional_object(value, label)? else {
        return Ok(());
    };
    if let Some(field) = object
        .keys()
        .find(|key| crate::wire::pskt_schema::is_specialized_covenant_routing_field(key))
    {
        return Err(format!(
            "{label}.{field} is input-scoped specialized covenant metadata"
        ));
    }
    Ok(())
}

fn bind_route(
    index: usize,
    route: RouteKind,
    fields: &Map<String, Value>,
    input: &Input,
) -> Result<SpecializedWitness, String> {
    let (mask, truth, signature) = validate_route_binding(index, input)?;
    let (route_id, script) = dispatch_route(index, route, fields, input, mask, truth, signature)?;
    Ok(SpecializedWitness {
        route: route_id,
        signature_script: script,
        supplied_mask: mask,
        supplied_true_mask: truth,
    })
}

fn validate_route_binding(index: usize, input: &Input) -> Result<(u16, u16, &Signature), String> {
    if input.redeem.is_empty() {
        return Err(format!(
            "input[{index}] specialized covenant is missing redeemScript"
        ));
    }
    let (mask, truth) = input.covenant_execution.ok_or_else(|| {
        format!("input[{index}] specialized covenant is missing covenantExecution")
    })?;
    let branches = shared_signer::covenant_branch::resolve_covenant_branches(&input.redeem)
        .map_err(|error| format!("input[{index}] invalid covenant branch structure: {error:?}"))?;
    validate_selector_assignment(index, mask, truth, branches.selector_mask())?;
    let signature = exactly_one_signature(index, &input.signatures)?;
    let binding = branches
        .key_at(signature.position)
        .map_err(|_| format!("input[{index}] specialized signature is not branch-bound"))?;
    if !binding.matches_selectors(mask, truth) {
        return Err(format!(
            "input[{index}] specialized signature does not belong to covenantExecution"
        ));
    }
    Ok((mask, truth, signature))
}

fn validate_selector_assignment(
    index: usize,
    mask: u16,
    truth: u16,
    expected_mask: u16,
) -> Result<(), String> {
    if truth & !mask != 0 || mask != expected_mask {
        return Err(format!(
            "input[{index}] covenantExecution is not a complete selector assignment"
        ));
    }
    Ok(())
}

fn dispatch_route(
    index: usize,
    route: RouteKind,
    fields: &Map<String, Value>,
    input: &Input,
    mask: u16,
    truth: u16,
    signature: &Signature,
) -> Result<(SpecializedRoute, Vec<u8>), String> {
    match route {
        RouteKind::PrivateSwap => bind_private_swap(index, fields, input, mask, truth, signature),
        RouteKind::OracleV1 => bind_oracle_v1(index, fields, input, mask, truth, signature),
        RouteKind::CommitReveal => bind_commit_reveal(index, fields, input, mask, truth, signature),
        RouteKind::Merkle => bind_merkle(index, fields, input, mask, truth, signature),
    }
}

fn bind_private_swap(
    index: usize,
    fields: &Map<String, Value>,
    input: &Input,
    mask: u16,
    truth: u16,
    signature: &Signature,
) -> Result<(SpecializedRoute, Vec<u8>), String> {
    require_true(fields, "privateSwapClaim", index)?;
    require_only_route_fields(fields, &["privateSwapClaim"], index)?;
    recognize_private_swap(&input.redeem)
        .map_err(|error| format!("input[{index}] private-swap template mismatch: {error}"))?;
    if mask != 0b1 || truth != 0b1 {
        return Err(format!(
            "input[{index}] private-swap claim requires covenantExecution 1/1"
        ));
    }
    let mut script = Vec::new();
    push_signature(&mut script, signature)?;
    script.push(OP_1);
    push_data(&mut script, &input.redeem)?;
    Ok((SpecializedRoute::PrivateSwapClaim, script))
}

fn bind_oracle_v1(
    index: usize,
    fields: &Map<String, Value>,
    input: &Input,
    mask: u16,
    truth: u16,
    signature: &Signature,
) -> Result<(SpecializedRoute, Vec<u8>), String> {
    require_true(fields, "oracleV1Claim", index)?;
    require_only_route_fields(fields, &["oracleV1Claim", "oracleV1Signature"], index)?;
    let oracle_signature = required_hex(fields, "oracleV1Signature", Some(64), index)?;
    let template = recognize_oracle_v1(&input.redeem)
        .map_err(|error| format!("input[{index}] oracle-v1 template mismatch: {error}"))?;
    validate_oracle_execution(index, mask, truth)?;
    let oracle_signature = verify_oracle_attestation(index, &template, &oracle_signature)?;
    let script = oracle_claim_script(&oracle_signature, signature, &input.redeem)?;
    Ok((SpecializedRoute::OracleV1Claim, script))
}

fn validate_oracle_execution(index: usize, mask: u16, truth: u16) -> Result<(), String> {
    if mask != 0b1 || truth != 0 {
        return Err(format!(
            "input[{index}] oracle-v1 claim requires covenantExecution 1/0"
        ));
    }
    Ok(())
}

fn verify_oracle_attestation(
    index: usize,
    template: &OracleTemplate,
    signature: &[u8],
) -> Result<K256Signature, String> {
    let key = VerifyingKey::from_bytes(&template.oracle_key)
        .map_err(|_| format!("input[{index}] oracle-v1 embedded oracle key is invalid"))?;
    let signature = K256Signature::try_from(signature).map_err(|_| {
        format!("input[{index}] oracle-v1 attestation signature encoding is invalid")
    })?;
    key.verify_raw(&template.commitment, &signature)
        .map_err(|_| format!("input[{index}] oracle-v1 attestation signature is invalid"))?;
    Ok(signature)
}

fn oracle_claim_script(
    oracle_signature: &K256Signature,
    signature: &Signature,
    redeem: &[u8],
) -> Result<Vec<u8>, String> {
    let mut script = Vec::new();
    push_data(&mut script, oracle_signature.to_bytes().as_slice())?;
    push_signature(&mut script, signature)?;
    script.push(OP_0);
    push_data(&mut script, redeem)?;
    Ok(script)
}

fn bind_commit_reveal(
    index: usize,
    fields: &Map<String, Value>,
    input: &Input,
    mask: u16,
    truth: u16,
    signature: &Signature,
) -> Result<(SpecializedRoute, Vec<u8>), String> {
    let (part_a, part_b) = commit_reveal_parts(fields, index)?;
    let template = recognize_commit_reveal(&input.redeem)
        .map_err(|error| format!("input[{index}] commit-reveal template mismatch: {error}"))?;
    validate_commit_reveal_claim(index, mask, truth, &part_a, &part_b, &template)?;
    let script = commit_reveal_claim_script(&part_a, &part_b, signature, &input.redeem)?;
    Ok((SpecializedRoute::CommitRevealClaim, script))
}

fn commit_reveal_parts(
    fields: &Map<String, Value>,
    index: usize,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    require_only_route_fields(fields, &["commitPartA", "commitPartB"], index)?;
    let part_a = required_hex(fields, "commitPartA", None, index)?;
    let part_b = required_hex(fields, "commitPartB", None, index)?;
    Ok((part_a, part_b))
}

fn validate_commit_reveal_claim(
    index: usize,
    mask: u16,
    truth: u16,
    part_a: &[u8],
    part_b: &[u8],
    template: &CommitTemplate,
) -> Result<(), String> {
    if mask != 0b1 || truth != 0 {
        return Err(format!(
            "input[{index}] commit-reveal claim requires covenantExecution 1/0"
        ));
    }
    let mut preimage = Vec::with_capacity(part_a.len() + part_b.len());
    preimage.extend_from_slice(part_a);
    preimage.extend_from_slice(part_b);
    if blake2b32(&preimage) != template.commitment {
        return Err(format!(
            "input[{index}] commit-reveal preimage does not match redeem commitment"
        ));
    }
    Ok(())
}

fn commit_reveal_claim_script(
    part_a: &[u8],
    part_b: &[u8],
    signature: &Signature,
    redeem: &[u8],
) -> Result<Vec<u8>, String> {
    let mut script = Vec::new();
    push_data(&mut script, part_a)?;
    push_data(&mut script, part_b)?;
    push_signature(&mut script, signature)?;
    script.push(OP_0);
    push_data(&mut script, redeem)?;
    Ok(script)
}

fn bind_merkle(
    index: usize,
    fields: &Map<String, Value>,
    input: &Input,
    mask: u16,
    truth: u16,
    signature: &Signature,
) -> Result<(SpecializedRoute, Vec<u8>), String> {
    require_only_route_fields(fields, &["merkleProof", "merkleDestSpk"], index)?;
    let dest_spk = required_hex(fields, "merkleDestSpk", None, index)?;
    let proof = parse_merkle_proof(fields.get("merkleProof"), index)?;
    let template = recognize_merkle(&input.redeem)
        .map_err(|error| format!("input[{index}] merkle template mismatch: {error}"))?;
    validate_merkle_claim(index, &proof, &template, mask, truth)?;
    verify_merkle_root(index, &dest_spk, &proof, template.root)?;
    let script = build_merkle_witness(&dest_spk, &proof, signature, &input.redeem)?;
    Ok((SpecializedRoute::MerkleClaim, script))
}

fn validate_merkle_claim(
    index: usize,
    proof: &[MerkleProofItem],
    template: &MerkleTemplate,
    mask: u16,
    truth: u16,
) -> Result<(), String> {
    if proof.len() != usize::from(template.depth) {
        return Err(format!(
            "input[{index}] merkle proof depth {} does not match redeem depth {}",
            proof.len(),
            template.depth
        ));
    }
    let expected_mask = if template.depth == 15 {
        u16::MAX
    } else {
        (1u16 << (u32::from(template.depth) + 1)) - 1
    };
    if mask != expected_mask || truth & 1 != 0 {
        return Err(format!(
            "input[{index}] merkle covenantExecution does not select the claim branch"
        ));
    }
    let expected_truth = merkle_truth_from_proof(proof)?;
    if truth != expected_truth {
        return Err(format!(
            "input[{index}] merkle proof directions do not match covenantExecution"
        ));
    }
    Ok(())
}

fn merkle_truth_from_proof(proof: &[MerkleProofItem]) -> Result<u16, String> {
    let mut expected_truth = 0u16;
    for (level, item) in proof.iter().enumerate() {
        if item.direction == 1 {
            let shift =
                u32::try_from(level + 1).map_err(|_| "merkle selector overflow".to_string())?;
            let bit = 1u16
                .checked_shl(shift)
                .ok_or_else(|| "merkle selector overflow".to_string())?;
            expected_truth |= bit;
        }
    }
    Ok(expected_truth)
}

fn verify_merkle_root(
    index: usize,
    dest_spk: &[u8],
    proof: &[MerkleProofItem],
    expected_root: [u8; 32],
) -> Result<(), String> {
    let mut current = blake2b32(dest_spk);
    for item in proof {
        current = merkle_step(current, item);
    }
    if current != expected_root {
        return Err(format!(
            "input[{index}] merkle proof does not match redeem root"
        ));
    }
    Ok(())
}

fn merkle_step(current: [u8; 32], item: &MerkleProofItem) -> [u8; 32] {
    let mut pair = [0u8; 64];
    if item.direction == 1 {
        pair[..32].copy_from_slice(&current);
        pair[32..].copy_from_slice(&item.sibling);
    } else {
        pair[..32].copy_from_slice(&item.sibling);
        pair[32..].copy_from_slice(&current);
    }
    blake2b32(&pair)
}

fn build_merkle_witness(
    dest_spk: &[u8],
    proof: &[MerkleProofItem],
    signature: &Signature,
    redeem: &[u8],
) -> Result<Vec<u8>, String> {
    let mut script = Vec::new();
    push_data(&mut script, dest_spk)?;
    for item in proof.iter().rev() {
        push_data(&mut script, &item.sibling)?;
        script.push(if item.direction == 0 { OP_0 } else { OP_1 });
    }
    push_data(&mut script, dest_spk)?;
    push_signature(&mut script, signature)?;
    script.push(OP_0);
    push_data(&mut script, redeem)?;
    Ok(script)
}

fn require_true(fields: &Map<String, Value>, key: &str, index: usize) -> Result<(), String> {
    match fields.get(key) {
        Some(Value::Bool(true)) => Ok(()),
        _ => Err(format!("input[{index}].proprietaries.{key} must be true")),
    }
}

fn require_only_route_fields(
    fields: &Map<String, Value>,
    allowed: &[&str],
    index: usize,
) -> Result<(), String> {
    for key in fields.keys() {
        if crate::wire::pskt_schema::is_specialized_covenant_routing_field(key)
            && !allowed.contains(&key.as_str())
        {
            return Err(format!(
                "input[{index}].proprietaries.{key} conflicts with the selected typed covenant route"
            ));
        }
    }
    for key in allowed {
        if !fields.contains_key(*key) {
            return Err(format!("input[{index}].proprietaries.{key} is required"));
        }
    }
    Ok(())
}

fn required_hex(
    fields: &Map<String, Value>,
    key: &str,
    exact_len: Option<usize>,
    index: usize,
) -> Result<Vec<u8>, String> {
    let text = fields
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("input[{index}].proprietaries.{key} must be lowercase hex"))?;
    let bytes = text.as_bytes();
    if bytes.len() % 2 != 0
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(format!(
            "input[{index}].proprietaries.{key} must be lowercase hex"
        ));
    }
    let decoded =
        hex::decode(text).map_err(|_| format!("input[{index}].proprietaries.{key} invalid hex"))?;
    if exact_len.is_some_and(|length| decoded.len() != length) {
        return Err(format!(
            "input[{index}].proprietaries.{key} must be {} bytes",
            exact_len.unwrap_or_default()
        ));
    }
    Ok(decoded)
}

fn exactly_one_signature(index: usize, signatures: &[Signature]) -> Result<&Signature, String> {
    if signatures.len() != 1 {
        return Err(format!(
            "input[{index}] typed specialized covenant requires exactly one verified transaction signature"
        ));
    }
    signatures
        .first()
        .ok_or_else(|| format!("input[{index}] missing signature"))
}

fn push_signature(script: &mut Vec<u8>, signature: &Signature) -> Result<(), String> {
    if signature.sighash != crate::wire::pskt_schema::SIGHASH_ALL {
        return Err("specialized witness contains unsupported sighash".to_string());
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
        _ => return Err("specialized witness item exceeds push-data limit".to_string()),
    }
    script.extend_from_slice(data);
    Ok(())
}

fn blake2b32(data: &[u8]) -> [u8; 32] {
    let hash = Params::new().hash_length(32).hash(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

struct OracleTemplate {
    commitment: [u8; 32],
    oracle_key: [u8; 32],
}

struct CommitTemplate {
    commitment: [u8; 32],
}

struct MerkleTemplate {
    root: [u8; 32],
    depth: u8,
}

struct Cursor<'a> {
    script: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(script: &'a [u8]) -> Self {
        Self { script, at: 0 }
    }

    fn op(&mut self, expected: u8) -> Result<(), String> {
        if self.script.get(self.at) != Some(&expected) {
            return Err(format!("expected opcode 0x{expected:02x} at {}", self.at));
        }
        self.at += 1;
        Ok(())
    }

    fn push(&mut self, length: usize) -> Result<&'a [u8], String> {
        if length > 75 || self.script.get(self.at).copied() != u8::try_from(length).ok() {
            return Err(format!("expected canonical PUSH{length} at {}", self.at));
        }
        let start = self.at + 1;
        let end = start
            .checked_add(length)
            .ok_or_else(|| "script position overflow".to_string())?;
        let data = self
            .script
            .get(start..end)
            .ok_or_else(|| "truncated script push".to_string())?;
        self.at = end;
        Ok(data)
    }

    fn integer(&mut self) -> Result<u64, String> {
        let opcode = *self
            .script
            .get(self.at)
            .ok_or_else(|| "missing script integer".to_string())?;
        if let Some(value) = small_script_integer(opcode) {
            self.at += 1;
            return Ok(value);
        }
        let data = self.integer_push(opcode)?;
        decode_script_integer(data)
    }

    fn integer_push(&mut self, opcode: u8) -> Result<&'a [u8], String> {
        let length = usize::from(opcode);
        if length == 0 || length > 9 || length > 75 {
            return Err("non-canonical script integer".to_string());
        }
        self.push(length)
    }

    fn finish(self) -> Result<(), String> {
        if self.at == self.script.len() {
            Ok(())
        } else {
            Err(format!("unexpected trailing script bytes at {}", self.at))
        }
    }
}

fn small_script_integer(opcode: u8) -> Option<u64> {
    if opcode == OP_0 {
        return Some(0);
    }
    (OP_1..=0x60)
        .contains(&opcode)
        .then(|| u64::from(opcode - 0x50))
}

fn decode_script_integer(data: &[u8]) -> Result<u64, String> {
    if data.last() == Some(&0) && data.len() > 1 && data[data.len() - 2] & 0x80 == 0 {
        return Err("non-minimal script integer".to_string());
    }
    if data.last().is_some_and(|byte| byte & 0x80 != 0) {
        return Err("negative script integer not permitted".to_string());
    }
    if data.len() == 9 && data[8] != 0 {
        return Err("script integer exceeds u64".to_string());
    }
    let mut bytes = [0u8; 8];
    let count = data.len().min(8);
    bytes[..count].copy_from_slice(&data[..count]);
    Ok(u64::from_le_bytes(bytes))
}

fn recognize_private_swap(script: &[u8]) -> Result<(), String> {
    let mut c = Cursor::new(script);
    let claimer = private_swap_prefix(&mut c)?;
    private_swap_claim_constraints(&mut c)?;
    private_swap_fee_constraints(&mut c)?;
    c.op(OP_1)?;
    c.op(OP_ELSE)?;
    private_swap_refund_branch(&mut c, &claimer)?;
    c.op(OP_1)?;
    c.op(OP_ENDIF)?;
    c.finish()
}

fn private_swap_prefix(c: &mut Cursor<'_>) -> Result<Vec<u8>, String> {
    let salt = c.push(16)?;
    if salt.iter().all(|byte| *byte == 0) {
        return Err("zero salt".to_string());
    }
    c.op(OP_DROP)?;
    c.op(OP_IF)?;
    let claimer = c.push(32)?.to_vec();
    c.op(OP_CHECKSIGVERIFY)?;
    Ok(claimer)
}

fn private_swap_claim_constraints(c: &mut Cursor<'_>) -> Result<(), String> {
    private_swap_shape_constraints(c)?;
    private_swap_destination_constraint(c)
}

fn private_swap_shape_constraints(c: &mut Cursor<'_>) -> Result<(), String> {
    c.op(OP_TX_INPUT_COUNT)?;
    require_script_integer(c, 1, "input-count constraint changed")?;
    c.op(OP_NUMEQUALVERIFY)?;
    c.op(OP_TX_OUTPUT_COUNT)?;
    require_script_integer(c, 1, "output-count constraint changed")?;
    c.op(OP_NUMEQUALVERIFY)
}

fn private_swap_destination_constraint(c: &mut Cursor<'_>) -> Result<(), String> {
    require_script_integer(c, 0, "destination output index changed")?;
    c.op(OP_TX_OUTPUT_SPK)?;
    let destination = private_swap_destination(c)?;
    if destination.get(0..2) != Some(&[0, 0]) {
        return Err("destination SPK version changed".to_string());
    }
    c.op(OP_EQUALVERIFY)
}

fn private_swap_destination(c: &mut Cursor<'_>) -> Result<Vec<u8>, String> {
    let opcode = *c
        .script
        .get(c.at)
        .ok_or_else(|| "missing private-swap destination".to_string())?;
    if !(5..=75).contains(&usize::from(opcode)) {
        return Err("destination SPK length changed".to_string());
    }
    Ok(c.push(usize::from(opcode))?.to_vec())
}

fn private_swap_fee_constraints(c: &mut Cursor<'_>) -> Result<(), String> {
    private_swap_nonnegative_output_constraint(c)?;
    private_swap_fee_ceiling_constraint(c)
}

fn private_swap_nonnegative_output_constraint(c: &mut Cursor<'_>) -> Result<(), String> {
    require_script_integer(c, 0, "input amount index changed")?;
    c.op(OP_TX_INPUT_AMOUNT)?;
    c.op(OP_DUP)?;
    require_script_integer(c, 0, "output amount index changed")?;
    c.op(OP_TX_OUTPUT_AMOUNT)?;
    c.op(OP_GREATERTHANOREQUAL)?;
    c.op(OP_VERIFY)
}

fn private_swap_fee_ceiling_constraint(c: &mut Cursor<'_>) -> Result<(), String> {
    require_script_integer(c, 0, "fee output index changed")?;
    c.op(OP_TX_OUTPUT_AMOUNT)?;
    c.op(OP_SUB)?;
    require_script_integer(c, PRIVATE_SWAP_MAX_FEE_SOMPI, "fee ceiling changed")?;
    c.op(OP_LESSTHANOREQUAL)?;
    c.op(OP_VERIFY)
}

fn private_swap_refund_branch(c: &mut Cursor<'_>, claimer: &[u8]) -> Result<(), String> {
    let owner = c.push(32)?;
    if owner == claimer {
        return Err("owner and claimer keys are identical".to_string());
    }
    c.op(OP_CHECKSIGVERIFY)?;
    if c.integer()? == 0 {
        return Err("refund locktime is zero".to_string());
    }
    c.op(OP_CHECKLOCKTIMEVERIFY)
}

fn require_script_integer(c: &mut Cursor<'_>, expected: u64, error: &str) -> Result<(), String> {
    if c.integer()? != expected {
        return Err(error.to_string());
    }
    Ok(())
}

fn recognize_oracle_v1(script: &[u8]) -> Result<OracleTemplate, String> {
    let mut c = Cursor::new(script);
    recognize_timelock_owner_prefix(&mut c, true)?;
    let template = oracle_claim_branch(&mut c)?;
    c.finish()?;
    Ok(template)
}

fn recognize_timelock_owner_prefix(c: &mut Cursor<'_>, salted: bool) -> Result<Vec<u8>, String> {
    consume_optional_salt(c, salted)?;
    timelock_owner_branch(c)
}

fn consume_optional_salt(c: &mut Cursor<'_>, salted: bool) -> Result<(), String> {
    if salted {
        c.push(16)?;
        c.op(OP_DROP)?;
    }
    Ok(())
}

fn timelock_owner_branch(c: &mut Cursor<'_>) -> Result<Vec<u8>, String> {
    c.op(OP_IF)?;
    let owner = c.push(32)?.to_vec();
    c.op(OP_CHECKSIGVERIFY)?;
    c.integer()?;
    c.op(OP_CHECKLOCKTIMEVERIFY)?;
    c.op(OP_1)?;
    c.op(OP_ELSE)?;
    Ok(owner)
}

fn oracle_claim_branch(c: &mut Cursor<'_>) -> Result<OracleTemplate, String> {
    c.push(32)?;
    c.op(OP_CHECKSIGVERIFY)?;
    let commitment = push32_array(c, "bad oracle commitment")?;
    let oracle_key = push32_array(c, "bad oracle key")?;
    c.op(OP_CHECKSIGFROMSTACK)?;
    c.op(OP_VERIFY)?;
    c.op(OP_1)?;
    c.op(OP_ENDIF)?;
    Ok(OracleTemplate {
        commitment,
        oracle_key,
    })
}

fn push32_array(c: &mut Cursor<'_>, error: &str) -> Result<[u8; 32], String> {
    c.push(32)?.try_into().map_err(|_| error.to_string())
}

fn recognize_commit_reveal(script: &[u8]) -> Result<CommitTemplate, String> {
    let mut c = Cursor::new(script);
    let owner = recognize_timelock_owner_prefix(&mut c, false)?;
    let commitment = recognize_commit_claim(&mut c, &owner)?;
    c.finish()?;
    Ok(CommitTemplate { commitment })
}

fn recognize_commit_claim(c: &mut Cursor<'_>, owner: &[u8]) -> Result<[u8; 32], String> {
    require_same_owner(c, owner)?;
    c.op(OP_CHECKSIGVERIFY)?;
    c.op(OP_CAT)?;
    c.op(OP_BLAKE2B)?;
    let commitment = push32_array(c, "bad commitment")?;
    c.op(OP_EQUALVERIFY)?;
    c.op(OP_1)?;
    c.op(OP_ENDIF)?;
    Ok(commitment)
}

fn recognize_merkle(script: &[u8]) -> Result<MerkleTemplate, String> {
    let mut c = Cursor::new(script);
    let owner = recognize_timelock_owner_prefix(&mut c, false)?;
    require_same_owner(&mut c, &owner)?;
    c.op(OP_CHECKSIGVERIFY)?;
    c.op(OP_BLAKE2B)?;
    let depth = recognize_merkle_layers(&mut c)?;
    let root = recognize_merkle_tail(&mut c)?;
    c.finish()?;
    Ok(MerkleTemplate { root, depth })
}

fn require_same_owner(c: &mut Cursor<'_>, owner: &[u8]) -> Result<(), String> {
    if c.push(32)? != owner {
        return Err("branch owner keys differ".to_string());
    }
    Ok(())
}

fn recognize_merkle_layers(c: &mut Cursor<'_>) -> Result<u8, String> {
    let pattern = [OP_SWAP, OP_IF, OP_SWAP, OP_ENDIF, OP_CAT, OP_BLAKE2B];
    let mut depth = 0u8;
    while c.script.get(c.at..c.at + pattern.len()) == Some(pattern.as_slice()) {
        c.at += pattern.len();
        depth = depth
            .checked_add(1)
            .ok_or_else(|| "merkle depth overflow".to_string())?;
        if depth > 15 {
            return Err("merkle depth exceeds selector capacity".to_string());
        }
    }
    Ok(depth)
}

fn recognize_merkle_tail(c: &mut Cursor<'_>) -> Result<[u8; 32], String> {
    let root = push32_array(c, "bad merkle root")?;
    c.op(OP_EQUALVERIFY)?;
    require_script_integer(c, 0, "merkle output index changed")?;
    c.op(OP_TX_OUTPUT_SPK)?;
    c.op(OP_EQUALVERIFY)?;
    c.op(OP_1)?;
    c.op(OP_ENDIF)?;
    Ok(root)
}

struct MerkleProofItem {
    sibling: [u8; 32],
    direction: u8,
}

fn parse_merkle_proof(value: Option<&Value>, index: usize) -> Result<Vec<MerkleProofItem>, String> {
    let array = value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("input[{index}].proprietaries.merkleProof must be an array"))?;
    if array.len() > 15 {
        return Err(format!("input[{index}] merkle proof exceeds 15 levels"));
    }
    array
        .iter()
        .enumerate()
        .map(|(level, value)| parse_merkle_proof_item(value, index, level))
        .collect()
}

fn parse_merkle_proof_item(
    value: &Value,
    index: usize,
    level: usize,
) -> Result<MerkleProofItem, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("input[{index}] merkle proof[{level}] must be an object"))?;
    validate_merkle_proof_item_shape(object, index, level)?;
    let sibling = parse_merkle_sibling(object, index, level)?;
    let direction = parse_merkle_direction(object, index, level)?;
    Ok(MerkleProofItem { sibling, direction })
}

fn validate_merkle_proof_item_shape(
    object: &Map<String, Value>,
    index: usize,
    level: usize,
) -> Result<(), String> {
    let valid =
        object.len() == 2 && object.contains_key("sibling") && object.contains_key("direction");
    if !valid {
        return Err(format!(
            "input[{index}] merkle proof[{level}] must contain only sibling and direction"
        ));
    }
    Ok(())
}

fn parse_merkle_sibling(
    object: &Map<String, Value>,
    index: usize,
    level: usize,
) -> Result<[u8; 32], String> {
    let sibling_text = object
        .get("sibling")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!("input[{index}] merkle proof[{level}].sibling must be lowercase hex")
        })?;
    if sibling_text.len() != 64 || !sibling_text.as_bytes().iter().all(lower_hex_byte) {
        return Err(format!(
            "input[{index}] merkle proof[{level}].sibling must be 32-byte lowercase hex"
        ));
    }
    let sibling_vec =
        hex::decode(sibling_text).map_err(|_| format!("input[{index}] invalid merkle sibling"))?;
    sibling_vec
        .as_slice()
        .try_into()
        .map_err(|_| format!("input[{index}] invalid merkle sibling length"))
}

fn lower_hex_byte(byte: &u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
}

fn parse_merkle_direction(
    object: &Map<String, Value>,
    index: usize,
    level: usize,
) -> Result<u8, String> {
    let raw = merkle_direction_u64(object.get("direction"));
    let direction = raw
        .ok_or_else(|| format!("input[{index}] merkle proof[{level}].direction must be 0 or 1"))?;
    let direction =
        u8::try_from(direction).map_err(|_| format!("input[{index}] invalid merkle direction"))?;
    if direction > 1 {
        return Err(format!(
            "input[{index}] merkle proof[{level}].direction must be 0 or 1"
        ));
    }
    Ok(direction)
}

fn merkle_direction_u64(value: Option<&Value>) -> Option<u64> {
    match value {
        Some(Value::Number(number)) => number.as_u64(),
        Some(Value::String(text)) => {
            crate::wire::pskt_schema::parse_canonical_u64_bytes(text.as_bytes()).ok()
        }
        _ => None,
    }
}
