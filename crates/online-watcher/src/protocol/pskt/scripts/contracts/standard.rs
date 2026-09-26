// KasKold Companion Web — organized PSKT subsystem
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use serde_json::Value;

use crate::protocol::pskt::scripts::{
    first_schnorr_signature, push_data_sigscript, push_redeem_script,
};

use crate::protocol::pskt::scripts::common::strip_optional_covenant_salt;

pub(crate) fn compute_genesis_covenant_id(
    prev_tx_id: &[u8; 32],
    prev_index: u32,
    output_index: u32,
    output_value: u64,
    spk_version: u16,
    spk_script: &[u8],
) -> [u8; 32] {
    let h = blake2b_simd::Params::new()
        .hash_length(32)
        .key(b"CovenantID")
        .to_state()
        .update(prev_tx_id)
        .update(&prev_index.to_le_bytes())
        .update(&1u64.to_le_bytes())
        .update(&output_index.to_le_bytes())
        .update(&output_value.to_le_bytes())
        .update(&spk_version.to_le_bytes())
        .update(&(spk_script.len() as u64).to_le_bytes())
        .update(spk_script)
        .finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(h.as_bytes());
    out
}

pub(crate) fn build_p2sh_covenant_sig_script_with_execution(
    redeem: &[u8],
    partial_map: &serde_json::Map<String, Value>,
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<Vec<u8>, String> {
    let (pk_hex, sig_bytes) = first_schnorr_signature(
        partial_map,
        "Covenant input has no signature".to_string(),
        "partial sig missing schnorr variant".to_string(),
        None,
        "sig hex",
    )?;
    let body = covenant_body(redeem);
    let resolution = resolve_complete_execution(body, supplied_mask, supplied_true_mask)?;
    let signer_binding = signer_branch_binding(body, pk_hex)?;
    validate_execution_signer(&signer_binding, supplied_mask, supplied_true_mask)?;
    let nested = covenant_has_nested_if(body);
    validate_generic_execution_topology(&resolution, nested)?;
    build_execution_sig_script(redeem, &sig_bytes, supplied_true_mask, nested)
}

fn resolve_complete_execution(
    body: &[u8],
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<shared_signer::covenant_branch::BranchResolution, String> {
    let resolution = shared_signer::covenant_branch::resolve_covenant_branches(body)
        .map_err(|error| format!("invalid covenant branch structure: {error:?}"))?;
    if supplied_true_mask & !supplied_mask != 0 || supplied_mask != resolution.selector_mask() {
        return Err(
            "covenantExecution is not a complete selector assignment for this redeem script".into(),
        );
    }
    Ok(resolution)
}

fn validate_execution_signer(
    signer_binding: &shared_signer::covenant_branch::BranchKey,
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<(), String> {
    if !signer_binding.matches_selectors(supplied_mask, supplied_true_mask) {
        return Err("covenant signature is not authorized by covenantExecution".into());
    }
    if signer_binding.decision_mask & 1 == 0 {
        return Err("covenant signer is not protected by the outer branch selector".into());
    }
    Ok(())
}

fn validate_generic_execution_topology(
    resolution: &shared_signer::covenant_branch::BranchResolution,
    nested: bool,
) -> Result<(), String> {
    let supported_mask = if nested { 0b11 } else { 0b1 };
    if resolution.selector_mask() != supported_mask {
        return Err(
            "generic covenant finalizer cannot prove this selector topology; use a specialized finalizer"
                .into(),
        );
    }
    Ok(())
}

fn build_execution_sig_script(
    redeem: &[u8],
    sig_bytes: &[u8],
    supplied_true_mask: u16,
    nested: bool,
) -> Result<Vec<u8>, String> {
    let outer_selector_true = supplied_true_mask & 1 != 0;
    let nested_selector_true = nested.then_some(supplied_true_mask & 2 != 0);
    let mut sig_script = Vec::with_capacity(sig_bytes.len() + redeem.len() + 10);
    append_covenant_branch_from_execution(
        &mut sig_script,
        sig_bytes,
        outer_selector_true,
        nested_selector_true,
    );
    push_redeem_script(&mut sig_script, redeem)?;
    Ok(sig_script)
}

pub(crate) fn build_p2sh_covenant_sig_script(
    redeem: &[u8],
    partial_map: &serde_json::Map<String, Value>,
    force_time_path: bool,
) -> Result<Vec<u8>, String> {
    let (pk_hex, sig_bytes) = first_schnorr_signature(
        partial_map,
        "Covenant input has no signature".to_string(),
        "partial sig missing schnorr variant".to_string(),
        None,
        "sig hex",
    )?;
    let body = covenant_body(redeem);
    let signer_binding = signer_branch_binding(body, pk_hex)?;
    if signer_binding.decision_mask & 1 == 0 {
        return Err("covenant signer is not protected by the outer branch selector".into());
    }
    let outer_selector_true = signer_binding.if_mask & 1 != 0;
    let nested = covenant_has_nested_if(body);
    let mut sig_script = Vec::with_capacity(sig_bytes.len() + redeem.len() + 10);
    append_covenant_branch(
        &mut sig_script,
        &sig_bytes,
        outer_selector_true,
        nested,
        force_time_path,
    );
    push_redeem_script(&mut sig_script, redeem)?;
    Ok(sig_script)
}

fn covenant_body(redeem: &[u8]) -> &[u8] {
    strip_optional_covenant_salt(redeem)
}

fn signer_branch_binding(
    body: &[u8],
    pk_hex: &str,
) -> Result<shared_signer::covenant_branch::BranchKey, String> {
    let bytes = pk_hex.as_bytes();
    if bytes.len() != 66
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        || !(bytes.starts_with(b"02") || bytes.starts_with(b"03"))
    {
        return Err("covenant signer public key must be canonical compressed SEC1 hex".into());
    }
    let encoded = &bytes[2..];
    let signer: [u8; 32] = hex::decode(encoded)
        .map_err(|_| "invalid covenant signer public key hex")?
        .try_into()
        .map_err(|_| "covenant signer public key must be 32 bytes")?;
    let resolution = shared_signer::covenant_branch::resolve_covenant_branches(body)
        .map_err(|error| format!("invalid covenant branch structure: {error:?}"))?;
    let position = resolution.unique_position_for_key(&signer).map_err(|error| match error {
        shared_signer::covenant_branch::BranchResolveError::AmbiguousKey => {
            "covenant signer public key is branch-ambiguous and requires an explicit specialized branch policy".to_string()
        }
        _ => "covenant signer public key is not bound to a canonical branch".to_string(),
    })?;
    resolution
        .key_at(position)
        .map_err(|_| "covenant signer branch position is invalid".to_string())
}

fn covenant_has_nested_if(body: &[u8]) -> bool {
    matches!((body.get(34), body.get(35)), (Some(0xad), Some(0x63)))
}

fn append_covenant_branch_from_execution(
    sig_script: &mut Vec<u8>,
    sig_bytes: &[u8],
    outer_selector_true: bool,
    nested_selector_true: Option<bool>,
) {
    if let Some(nested_true) = nested_selector_true {
        sig_script.push(if nested_true { 0x51 } else { 0x00 });
    }
    sig_script.push(sig_bytes.len() as u8);
    sig_script.extend_from_slice(sig_bytes);
    sig_script.push(if outer_selector_true { 0x51 } else { 0x00 });
}

fn append_covenant_branch(
    sig_script: &mut Vec<u8>,
    sig_bytes: &[u8],
    outer_selector_true: bool,
    nested: bool,
    force_time_path: bool,
) {
    if outer_selector_true && nested {
        sig_script.push(if force_time_path { 0x00 } else { 0x51 });
    }
    sig_script.push(sig_bytes.len() as u8);
    sig_script.extend_from_slice(sig_bytes);
    sig_script.push(if outer_selector_true { 0x51 } else { 0x00 });
}

pub(crate) fn build_p2sh_covenant_borrower_sig_script_with_execution(
    redeem: &[u8],
    partial_map: &serde_json::Map<String, Value>,
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<Vec<u8>, String> {
    let (pk_hex, sig_bytes) = first_schnorr_signature(
        partial_map,
        "Beneficiary input has no signature".to_string(),
        "partial sig missing schnorr variant".to_string(),
        Some("bad sig length"),
        "sig hex",
    )?;
    let body = covenant_body(redeem);
    let resolution = resolve_complete_execution(body, supplied_mask, supplied_true_mask)?;
    validate_beneficiary_topology(&resolution)?;
    let binding = signer_branch_binding(body, pk_hex)?;
    validate_beneficiary_execution(&binding, supplied_mask, supplied_true_mask)?;
    build_beneficiary_execution_sig_script(redeem, &sig_bytes, supplied_true_mask)
}

fn validate_beneficiary_topology(
    resolution: &shared_signer::covenant_branch::BranchResolution,
) -> Result<(), String> {
    if resolution.selector_mask() != 0b1 {
        return Err(
            "generic beneficiary finalizer supports only a single proven outer selector; use a specialized finalizer"
                .into(),
        );
    }
    Ok(())
}

fn validate_beneficiary_execution(
    binding: &shared_signer::covenant_branch::BranchKey,
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<(), String> {
    if !binding.matches_selectors(supplied_mask, supplied_true_mask) {
        return Err("beneficiary signature is not authorized by covenantExecution".into());
    }
    if binding.decision_mask & 1 == 0 || binding.if_mask & 1 != 0 {
        return Err("beneficiary signer does not resolve to the proven outer ELSE branch".into());
    }
    Ok(())
}

fn build_beneficiary_execution_sig_script(
    redeem: &[u8],
    sig_bytes: &[u8],
    supplied_true_mask: u16,
) -> Result<Vec<u8>, String> {
    let mut sig_script = Vec::with_capacity(sig_bytes.len() + redeem.len() + 6);
    sig_script.push(sig_bytes.len() as u8);
    sig_script.extend_from_slice(sig_bytes);
    sig_script.push(if supplied_true_mask & 1 != 0 {
        0x51
    } else {
        0x00
    });
    push_redeem_script(&mut sig_script, redeem)?;
    Ok(sig_script)
}

pub(crate) fn build_p2sh_covenant_borrower_sig_script(
    redeem: &[u8],
    partial_map: &serde_json::Map<String, Value>,
) -> Result<Vec<u8>, String> {
    // Generic beneficiary routing is intentionally limited to a single,
    // structurally proven outer ELSE selector. Any signer key that is also
    // guarded by a nested conditional requires a named specialized covenant
    // finalizer; guessing inner selector bytes here would be unsafe.
    let (pk_hex, sig_bytes) = first_schnorr_signature(
        partial_map,
        "Beneficiary input has no signature".to_string(),
        "partial sig missing schnorr variant".to_string(),
        Some("bad sig length"),
        "sig hex",
    )?;
    let binding = signer_branch_binding(covenant_body(redeem), pk_hex)?;
    if binding.decision_mask & 1 == 0 {
        return Err("beneficiary signer is not protected by the outer branch selector".into());
    }
    if binding.if_mask & 1 != 0 {
        return Err("beneficiary signer resolves to the owner/IF branch".into());
    }
    if binding.decision_mask & !1 != 0 {
        return Err(
            "nested beneficiary covenant requires an explicit specialized branch policy".into(),
        );
    }

    let mut sig_script = Vec::with_capacity(sig_bytes.len() + redeem.len() + 6);
    sig_script.push(sig_bytes.len() as u8);
    sig_script.extend_from_slice(&sig_bytes);
    sig_script.push(0x00); // exact outer OP_ELSE selector
    push_redeem_script(&mut sig_script, redeem)?;
    Ok(sig_script)
}

pub(crate) fn build_p2sh_treasury_sig_script(
    redeem: &[u8],
    partial_map: &serde_json::Map<String, Value>,
) -> Result<Vec<u8>, String> {
    // Get the single signature
    let (_pk_hex, sig_bytes) = first_schnorr_signature(
        partial_map,
        "Treasury input has no signature".to_string(),
        "Treasury sig missing schnorr field".to_string(),
        None,
        "bad treasury sig hex",
    )?;

    let mut sig_script: Vec<u8> = Vec::with_capacity(sig_bytes.len() + 2 + redeem.len() + 2);

    // Push signature (64 bytes sig + 1 byte sighash type = 65)
    sig_script.push(sig_bytes.len() as u8);
    sig_script.extend_from_slice(&sig_bytes);

    // Push redeem script
    push_redeem_script(&mut sig_script, redeem)?;

    Ok(sig_script)
}

pub(crate) fn build_p2sh_token_conservation_sig_script(redeem: &[u8]) -> Result<Vec<u8>, String> {
    let mut sig_script: Vec<u8> = Vec::with_capacity(redeem.len() + 4);
    push_redeem_script(&mut sig_script, redeem)?;
    Ok(sig_script)
}

pub(crate) fn build_p2sh_covenant_nosig_script(redeem: &[u8]) -> Result<Vec<u8>, String> {
    // Single OP_FALSE selects the ELSE branch for borrower/no-sig covenants.
    // Nested scripts (escrow) use different builders with explicit branch selectors.
    let false_count = 1;

    let mut sig_script: Vec<u8> = Vec::with_capacity(redeem.len() + 5 + false_count);

    // Push OP_FALSE(s) to select ELSE branch(es)
    sig_script.resize(sig_script.len() + false_count, 0x00u8);

    // Push redeem script
    push_redeem_script(&mut sig_script, redeem)?;

    Ok(sig_script)
}

pub(crate) fn build_p2sh_private_swap_claim_sig_script(
    redeem: &[u8],
    partial_map: &serde_json::Map<String, Value>,
) -> Result<Vec<u8>, String> {
    let (_pk, sig_bytes) = first_schnorr_signature(
        partial_map,
        "Private Swap claim missing completed signature".to_string(),
        "Private Swap signature missing schnorr variant".to_string(),
        Some("Private Swap signature length"),
        "Private Swap signature hex",
    )?;
    let mut script = Vec::with_capacity(sig_bytes.len() + redeem.len() + 6);
    push_data_sigscript(&mut script, &sig_bytes);
    script.push(0x51); // OP_TRUE selects adaptor-claim branch.
    push_redeem_script(&mut script, redeem)?;
    Ok(script)
}
