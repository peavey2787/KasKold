use alloc::vec::Vec;

use super::*;
use crate::transaction::model::{Transaction, OP_1, OP_CHECKMULTISIG, OP_CHECKSIG, OP_DATA_32};

fn compressed(key: [u8; 32]) -> [u8; 33] {
    let mut out = [0u8; 33];
    out[0] = 0x02;
    out[1..].copy_from_slice(&key);
    out
}

fn one_input() -> Transaction {
    let mut tx = Transaction::try_new().expect("transaction storage");
    tx.num_inputs = 1;
    tx
}

#[test]
fn pskt_key_position_covers_p2pk_multisig_unknown_and_binding_failures() {
    let key = [0x31; 32];
    let mut tx = one_input();
    let script = &mut tx.inputs[0].utxo_entry.script_public_key;
    script.script[0] = OP_DATA_32;
    script.script[1..33].copy_from_slice(&key);
    script.script[33] = OP_CHECKSIG;
    script.script_len = 34;

    assert_eq!(position_for_pskt_key(&tx, 0, &compressed(key)), Ok(0));
    assert_eq!(
        position_for_pskt_key(&tx, 0, &compressed([0x32; 32])),
        Err(SignatureValidationError::InvalidBinding)
    );
    let mut invalid_prefix = compressed(key);
    invalid_prefix[0] = 0x04;
    assert_eq!(
        position_for_pskt_key(&tx, 0, &invalid_prefix),
        Err(SignatureValidationError::InvalidBinding)
    );

    let mut multisig = one_input();
    let script = &mut multisig.inputs[0].utxo_entry.script_public_key;
    script.script[0] = OP_1;
    script.script[1] = OP_DATA_32;
    script.script[2..34].copy_from_slice(&key);
    script.script[34] = OP_1;
    script.script[35] = OP_CHECKMULTISIG;
    script.script_len = 36;
    assert_eq!(position_for_pskt_key(&multisig, 0, &compressed(key)), Ok(0));
    assert_eq!(
        position_for_pskt_key(&multisig, 0, &compressed([0x33; 32])),
        Err(SignatureValidationError::InvalidBinding)
    );

    let mut unknown = one_input();
    unknown.inputs[0].utxo_entry.script_public_key.script[0] = 0xff;
    unknown.inputs[0].utxo_entry.script_public_key.script_len = 1;
    assert_eq!(
        position_for_pskt_key(&unknown, 0, &compressed(key)),
        Err(SignatureValidationError::InvalidBinding)
    );
}

fn covenant_input(owner: [u8; 32], beneficiary: [u8; 32]) -> Transaction {
    let mut tx = one_input();
    let script = &mut tx.inputs[0].utxo_entry.script_public_key;
    script.script[0] = 0xaa;
    script.script[1] = OP_DATA_32;
    script.script[2..34].fill(0x55);
    script.script[34] = 0x87;
    script.script_len = 35;

    let mut redeem = Vec::new();
    redeem.extend_from_slice(&[0x63, OP_DATA_32]);
    redeem.extend_from_slice(&owner);
    redeem.extend_from_slice(&[OP_CHECKSIG, 0x67, OP_DATA_32]);
    redeem.extend_from_slice(&beneficiary);
    redeem.extend_from_slice(&[OP_CHECKSIG, 0x68]);
    tx.inputs[0].redeem_script[..redeem.len()].copy_from_slice(&redeem);
    tx.inputs[0].redeem_script_len = redeem.len();
    tx.inputs[0].covenant_execution_present = true;
    tx.inputs[0].covenant_execution_mask = 1;
    tx.inputs[0].covenant_execution_true_mask = 1;
    tx
}

#[test]
fn covenant_pskt_key_position_and_execution_binding_cover_all_fail_closed_paths() {
    let owner = [0x41; 32];
    let beneficiary = [0x42; 32];
    let mut tx = covenant_input(owner, beneficiary);

    assert_eq!(position_for_pskt_key(&tx, 0, &compressed(owner)), Ok(0));
    assert_eq!(
        position_for_pskt_key(&tx, 0, &compressed(beneficiary)),
        Err(SignatureValidationError::InvalidBinding)
    );
    assert_eq!(
        validate_covenant_execution_binding(&tx.inputs[0], 1),
        Ok(())
    );

    tx.inputs[0].covenant_execution_true_mask = 0;
    assert_eq!(
        position_for_pskt_key(&tx, 0, &compressed(beneficiary)),
        Ok(1)
    );

    tx.inputs[0].covenant_execution_present = false;
    assert_eq!(
        validate_covenant_execution_binding(&tx.inputs[0], 1),
        Err(SignatureValidationError::InvalidBinding)
    );

    tx.inputs[0].covenant_execution_present = true;
    tx.inputs[0].covenant_execution_mask = 1;
    tx.inputs[0].covenant_execution_true_mask = 2;
    assert_eq!(
        validate_covenant_execution_binding(&tx.inputs[0], 1),
        Err(SignatureValidationError::InvalidBinding)
    );

    tx.inputs[0].covenant_execution_true_mask = 0;
    tx.inputs[0].covenant_execution_mask = 3;
    assert_eq!(
        validate_covenant_execution_binding(&tx.inputs[0], 1),
        Err(SignatureValidationError::InvalidBinding)
    );

    assert_eq!(
        position_for_covenant_key(&tx, 1, &owner),
        Err(SignatureValidationError::InvalidModel)
    );

    tx.inputs[0].covenant_execution_mask = 1;
    tx.inputs[0].redeem_script[0] = 0x63;
    tx.inputs[0].redeem_script_len = 1;
    assert_eq!(
        position_for_covenant_key(&tx, 0, &owner),
        Err(SignatureValidationError::InvalidBinding)
    );
}
