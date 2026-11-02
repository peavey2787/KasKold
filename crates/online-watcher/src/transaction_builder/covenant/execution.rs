//! The witness selectors each covenant spend path supplies.
//!
//! Signers and finalizers act only on a complete selector assignment, so every
//! covenant input carries one, proven here against the script it will run.

use kaspa_portal::contract::covenant::{
    branch::{resolve_covenant_branches, BranchResolution},
    execution::{trace_witness, ExecutionError, WitnessItem},
};

use crate::transaction_builder::pskb::CovenantExecution;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SpendPath {
    /// The first key with every selector its path leaves open true: the
    /// owner's immediate branch rather than a time-locked alternative.
    Owner,
    /// The key at this position with every selector its path leaves open false.
    Key(u8),
    /// An exact assignment, such as the one a Merkle proof implies.
    Exact(u16),
}

/// A script family's named paths, usable only on a script with its selectors.
struct NamedPaths {
    selector_mask: u16,
    paths: &'static [(&'static str, u16)],
}

const NAMED_PATHS: [NamedPaths; 2] = [
    // Alice, Bob, arbiter, the arbiter's direction, and the dispute selector.
    NamedPaths {
        selector_mask: 0b1_1111,
        paths: &[
            ("buyer-release", 0b0_0001),
            ("seller-refund", 0b0_0010),
            ("arbiter-award-seller", 0b0_1100),
            ("arbiter-refund-buyer", 0b0_0100),
            ("buyer-dispute", 0b1_0000),
            ("seller-dispute", 0b0_0000),
        ],
    },
    // Script state 0, pickup, refund, script state 1, outer, buyer, arbiter.
    NamedPaths {
        selector_mask: 0b111_1111,
        paths: &[
            ("pickup", 0b000_0011),
            ("state0-arb-refund", 0b000_0101),
            ("state0-timeout", 0b000_0001),
            ("delivery", 0b011_1000),
            ("state1-arb-award", 0b101_1000),
            ("state1-timeout", 0b001_1000),
            ("state1-arb-refund", 0b000_1000),
        ],
    },
];

fn branches_of(redeem_script: &[u8]) -> Result<BranchResolution, String> {
    resolve_covenant_branches(redeem_script)
        .map_err(|error| format!("Covenant script has no usable branches: {error:?}"))
}

fn truth_for(branches: &BranchResolution, path: SpendPath) -> Result<u16, String> {
    let key_error = |error| format!("Covenant script has no such key: {error:?}");
    match path {
        SpendPath::Owner => {
            let key = branches.key_at(0).map_err(key_error)?;
            let reaching = branches.truth_reaching_key(0).map_err(key_error)?;
            Ok((branches.selector_mask() & !key.decision_mask) | reaching)
        }
        SpendPath::Key(position) => branches.truth_reaching_key(position).map_err(key_error),
        SpendPath::Exact(truth) => Ok(truth),
    }
}

fn proven(
    redeem_script: &[u8],
    branches: &BranchResolution,
    truth: u16,
) -> Result<CovenantExecution, String> {
    let supplied_mask = branches.selector_mask();
    match trace_witness(redeem_script, supplied_mask, truth) {
        // A typed witness route supplies these, and its finalizer proves them.
        Ok(_)
        | Err(ExecutionError::WitnessDataRequired | ExecutionError::UnsupportedSignatureCheck) => {}
        Err(error) => return Err(format!("Covenant script cannot run that path: {error:?}")),
    }
    Ok(CovenantExecution {
        supplied_mask,
        supplied_true_mask: truth,
    })
}

pub(crate) fn execution_for(
    redeem_script: &[u8],
    path: SpendPath,
) -> Result<CovenantExecution, String> {
    let branches = branches_of(redeem_script)?;
    proven(redeem_script, &branches, truth_for(&branches, path)?)
}

fn named_truth(selector_mask: u16, path_name: &str) -> Result<u16, String> {
    NAMED_PATHS
        .iter()
        .filter(|family| family.selector_mask == selector_mask)
        .flat_map(|family| family.paths)
        .find_map(|(name, truth)| (*name == path_name).then_some(*truth))
        .ok_or_else(|| format!("This covenant has no {path_name} path"))
}

/// The assignment for a path as the Companion names it.
pub(crate) fn execution_named(
    redeem_script: &[u8],
    path_name: &str,
) -> Result<CovenantExecution, String> {
    match path_name {
        "" | "owner" => execution_for(redeem_script, SpendPath::Owner),
        "owner-time" => execution_for(redeem_script, SpendPath::Key(0)),
        "beneficiary" => execution_for(redeem_script, SpendPath::Key(1)),
        _ => {
            let branches = branches_of(redeem_script)?;
            let truth = named_truth(branches.selector_mask(), path_name)?;
            proven(redeem_script, &branches, truth)
        }
    }
}

/// How many signatures a path checks. A typed witness route's data cannot be
/// traced ahead of time, and each of those routes checks one signature.
pub(crate) fn signatures_required(redeem_script: &[u8], execution: &CovenantExecution) -> u8 {
    trace_witness(
        redeem_script,
        execution.supplied_mask,
        execution.supplied_true_mask,
    )
    .map_or(1, |witness| {
        let signatures = witness
            .iter()
            .filter(|item| matches!(item, WitnessItem::Signature { .. }))
            .count();
        u8::try_from(signatures).unwrap_or(u8::MAX)
    })
}
