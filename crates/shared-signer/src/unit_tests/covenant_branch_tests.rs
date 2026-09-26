use std::vec::Vec;

use crate::covenant_branch::{resolve_covenant_branches, BranchResolveError};

#[test]
fn binds_keys_to_nested_branch_paths() {
    let a = [1u8; 32];
    let b = [2u8; 32];
    let mut script = Vec::from([0x63, 0x20]);
    script.extend_from_slice(&a);
    script.extend_from_slice(&[0xad, 0x67, 0x63, 0x20]);
    script.extend_from_slice(&b);
    script.extend_from_slice(&[0xac, 0x68, 0x68]);
    let resolved = resolve_covenant_branches(&script).expect("nested covenant");
    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved.selector_mask(), 0b11);
    assert_eq!(resolved.key_at(0).expect("first key").if_mask & 1, 1);
    assert_eq!(resolved.key_at(1).expect("second key").if_mask & 1, 0);
    assert_eq!(resolved.key_at(1).expect("second key").if_mask & 2, 2);
}

#[test]
fn records_actual_op_notif_witness_truth_values() {
    let first_key = [3u8; 32];
    let second_key = [4u8; 32];
    let mut script = Vec::from([0x64, 0x20]);
    script.extend_from_slice(&first_key);
    script.extend_from_slice(&[0xac, 0x67, 0x20]);
    script.extend_from_slice(&second_key);
    script.extend_from_slice(&[0xac, 0x68]);
    let resolved = resolve_covenant_branches(&script).expect("OP_NOTIF covenant");
    let first = resolved.key_at(0).expect("first key");
    let second = resolved.key_at(1).expect("second key");
    assert!(first.matches_selectors(1, 0));
    assert!(!first.matches_selectors(1, 1));
    assert!(second.matches_selectors(1, 1));
    assert!(!second.matches_selectors(1, 0));
}

#[test]
fn duplicate_key_binding_on_the_same_branch_is_rejected() {
    let key = [9u8; 32];
    let mut script = Vec::from([0x20]);
    script.extend_from_slice(&key);
    script.push(0xac);
    script.push(0x20);
    script.extend_from_slice(&key);
    script.push(0xad);
    assert_eq!(
        resolve_covenant_branches(&script),
        Err(BranchResolveError::AmbiguousKey)
    );
}

#[test]
fn reused_key_requires_branch_disambiguation_and_malformed_flow_is_rejected() {
    let key = [7u8; 32];
    let mut script = Vec::from([0x63, 0x20]);
    script.extend_from_slice(&key);
    script.extend_from_slice(&[0xac, 0x67, 0x20]);
    script.extend_from_slice(&key);
    script.extend_from_slice(&[0xac, 0x68]);
    let resolved = resolve_covenant_branches(&script).expect("reused key covenant");
    assert_eq!(resolved.len(), 2);
    assert_eq!(
        resolved.unique_position_for_key(&key),
        Err(BranchResolveError::AmbiguousKey)
    );
    assert_eq!(resolved.position_for_key_with_selectors(&key, 1, 1), Ok(0));
    assert_eq!(resolved.position_for_key_with_selectors(&key, 1, 0), Ok(1));
    assert_eq!(
        resolve_covenant_branches(&[0x67]),
        Err(BranchResolveError::MalformedScript)
    );
    assert_eq!(
        resolve_covenant_branches(&[0x63]),
        Err(BranchResolveError::MalformedScript)
    );
}
