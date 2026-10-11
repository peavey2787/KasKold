//! Session entry points enforce their bounds and report idle state honestly.

use super::MNEMONIC;
use crate::{VaultRuntime, VaultRuntimeError};

const MAX_LOCAL_TRANSACTION_BYTES: usize = 1_048_576;

#[test]
fn local_transaction_files_must_be_non_empty_and_bounded() {
    let mut runtime = VaultRuntime::new();
    assert!(matches!(
        runtime.load_transaction_file(b"x"),
        Err(VaultRuntimeError::Locked)
    ));
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    for size in [0, MAX_LOCAL_TRANSACTION_BYTES + 1] {
        assert!(matches!(
            runtime.load_transaction_file(&vec![0u8; size]),
            Err(VaultRuntimeError::InvalidLocalTransaction)
        ));
    }
    // The largest permitted file reaches the transaction parser.
    for size in [1, MAX_LOCAL_TRANSACTION_BYTES] {
        assert!(matches!(
            runtime.load_transaction_file(&vec![0u8; size]),
            Err(VaultRuntimeError::Custody(_))
        ));
    }
}

#[test]
fn idle_sessions_are_not_awaiting_any_reveal() {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    assert!(!runtime.anti_klepto_awaiting_reveal());
    assert!(!runtime.private_swap_awaiting_reveal());
    runtime.private_swap_cancel();
    assert!(!runtime.private_swap_awaiting_reveal());
}

#[test]
fn session_signing_policy_is_validated_enforced_and_clearable() {
    let mut runtime = VaultRuntime::new();
    assert!(matches!(
        runtime.set_session_signing_policy("not-a-date", ""),
        Err(VaultRuntimeError::InvalidSigningPolicy)
    ));
    assert!(matches!(
        runtime.set_session_signing_policy("", "not-a-window"),
        Err(VaultRuntimeError::InvalidSigningPolicy)
    ));
    runtime
        .set_session_signing_policy("209901010000", "")
        .unwrap();
    assert!(matches!(
        runtime.check_session_signing_policy(1_800_000_000),
        Err(VaultRuntimeError::SigningBlockedNotBefore)
    ));
    runtime.clear_session_signing_policy();
    assert!(runtime.check_session_signing_policy(1_800_000_000).is_ok());
}

#[test]
fn creation_flow_json_publishes_the_shared_stage_contract() {
    let flow: serde_json::Value = serde_json::from_str(&crate::creation_flow_json()).unwrap();
    let stages = flow["stages"].as_array().unwrap();
    assert_eq!(
        stages.len(),
        shared_signer::creation_flow::CREATION_STAGE_ORDER.len()
    );
    assert_eq!(
        stages[0],
        shared_signer::creation_flow::CREATION_STAGE_ORDER[0].name()
    );
    assert_eq!(
        flow["touchEntropyTarget"],
        shared_signer::creation_flow::TOUCH_ENTROPY_TARGET
    );
}
