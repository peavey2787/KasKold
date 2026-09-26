use crate::{
    derivation::bip32::compressed_pubkey_from_raw_key,
    transaction::{
        model::{SigHashType, Transaction},
        sighash,
    },
};

use super::super::{is_fully_signed, signature_status};
use super::common::{set_p2sh_script, transaction};

fn bind_p2pk_and_sign(tx: &mut Transaction, private_key: [u8; 32]) {
    let compressed = compressed_pubkey_from_raw_key(&private_key).expect("valid private key");
    let script = &mut tx.inputs[0].utxo_entry.script_public_key;
    script.script[0] = 0x20;
    script.script[1..33].copy_from_slice(&compressed[1..33]);
    script.script[33] = 0xac;
    script.script_len = 34;
    let signature =
        sighash::sign_input(tx, 0, &private_key, SigHashType::All).expect("valid P2PK signature");
    let input = &mut tx.inputs[0];
    input.sigs[0].signature = signature.bytes;
    input.sigs[0].sighash_type = SigHashType::All.to_byte();
    input.sigs[0].pubkey_pos = 0;
    input.sigs[0].present = true;
    input.sigs[0].pubkey_compressed = compressed;
    input.sig_count = 1;
    input.sighash_type = SigHashType::All.to_byte();
}

fn bind_two_of_two(tx: &mut Transaction, first: [u8; 32], second: [u8; 32], p2sh: bool) {
    let first_pub = compressed_pubkey_from_raw_key(&first).expect("first public key");
    let second_pub = compressed_pubkey_from_raw_key(&second).expect("second public key");
    let mut redeem = [0u8; 69];
    redeem[0] = 0x52;
    redeem[1] = 0x20;
    redeem[2..34].copy_from_slice(&first_pub[1..33]);
    redeem[34] = 0x20;
    redeem[35..67].copy_from_slice(&second_pub[1..33]);
    redeem[67] = 0x52;
    redeem[68] = 0xae;
    if p2sh {
        set_p2sh_script(tx, &redeem);
    } else {
        let script = &mut tx.inputs[0].utxo_entry.script_public_key;
        script.script[..redeem.len()].copy_from_slice(&redeem);
        script.script_len = redeem.len();
    }

    for (slot, private_key) in [first, second].iter().enumerate() {
        let signature = sighash::sign_input(tx, 0, private_key, SigHashType::All)
            .expect("valid multisig signature");
        tx.inputs[0].sigs[slot].signature = signature.bytes;
        tx.inputs[0].sigs[slot].sighash_type = SigHashType::All.to_byte();
        tx.inputs[0].sigs[slot].pubkey_pos = slot as u8;
        tx.inputs[0].sigs[slot].present = true;
    }
    tx.inputs[0].sig_count = 2;
    tx.inputs[0].sighash_type = SigHashType::All.to_byte();
}

#[test]
fn empty_transaction_is_not_fully_signed() {
    let tx = Transaction::try_new().expect("transaction test allocation");
    assert!(!is_fully_signed(&tx));
    assert_eq!(signature_status(&tx), (0, 0));
}

#[test]
fn fake_signature_bytes_never_satisfy_completion() {
    let mut tx = transaction();
    tx.inputs[0].sigs[0].signature = [0x55; 64];
    tx.inputs[0].sigs[0].sighash_type = SigHashType::All.to_byte();
    tx.inputs[0].sigs[0].pubkey_pos = 0;
    tx.inputs[0].sigs[0].present = true;
    tx.inputs[0].sig_count = 1;
    tx.inputs[0].sighash_type = SigHashType::All.to_byte();
    assert_eq!(signature_status(&tx), (0, 1));
    assert!(!is_fully_signed(&tx));
}

#[test]
fn valid_p2pk_signature_is_cryptographically_complete() {
    let mut tx = transaction();
    bind_p2pk_and_sign(&mut tx, [0x11; 32]);
    assert_eq!(signature_status(&tx), (1, 1));
    assert!(is_fully_signed(&tx));

    tx.inputs[0].sigs[0].signature[0] ^= 1;
    assert_eq!(signature_status(&tx), (0, 1));
    assert!(!is_fully_signed(&tx));
}

#[test]
fn multisig_completion_requires_each_valid_bound_signature() {
    let mut direct = transaction();
    bind_two_of_two(&mut direct, [0x12; 32], [0x13; 32], false);
    assert_eq!(signature_status(&direct), (2, 2));
    assert!(is_fully_signed(&direct));

    direct.inputs[0].sigs[1].signature[7] ^= 1;
    assert_eq!(signature_status(&direct), (0, 2));
    assert!(!is_fully_signed(&direct));

    let mut p2sh = transaction();
    bind_two_of_two(&mut p2sh, [0x14; 32], [0x15; 32], true);
    assert_eq!(signature_status(&p2sh), (2, 2));
    assert!(is_fully_signed(&p2sh));
}

#[test]
fn generic_covenant_completion_requires_and_honors_execution_evidence() {
    let mut tx = transaction();
    // Canonical branch-bound key push, but generic status deliberately cannot
    // prove which execution branch the final witness selects.
    let key = compressed_pubkey_from_raw_key(&[0x16; 32]).expect("covenant public key");
    let mut redeem = [0u8; 34];
    redeem[0] = 0x20;
    redeem[1..33].copy_from_slice(&key[1..33]);
    redeem[33] = 0xac;
    set_p2sh_script(&mut tx, &redeem);
    let signature = sighash::sign_input(&tx, 0, &[0x16; 32], SigHashType::All)
        .expect("valid covenant-bound signature");
    tx.inputs[0].sigs[0].signature = signature.bytes;
    tx.inputs[0].sigs[0].sighash_type = SigHashType::All.to_byte();
    tx.inputs[0].sigs[0].pubkey_pos = 0;
    tx.inputs[0].sigs[0].present = true;
    tx.inputs[0].sig_count = 1;
    tx.inputs[0].sighash_type = SigHashType::All.to_byte();
    assert_eq!(signature_status(&tx), (0, 1));
    assert!(!is_fully_signed(&tx));

    // A branch-free covenant key is active under the empty selector assignment.
    tx.inputs[0].covenant_execution_present = true;
    tx.inputs[0].covenant_execution_mask = 0;
    tx.inputs[0].covenant_execution_true_mask = 0;
    assert_eq!(signature_status(&tx), (1, 1));
    assert!(is_fully_signed(&tx));

    // Malformed selector evidence (truth bit not supplied) cannot complete.
    tx.inputs[0].covenant_execution_mask = 0;
    tx.inputs[0].covenant_execution_true_mask = 1;
    assert_eq!(signature_status(&tx), (0, 1));
    assert!(!is_fully_signed(&tx));
}

#[test]
fn completion_rejects_wrong_key_digest_position_sighash_and_bit_corruption() {
    fn valid() -> Transaction {
        let mut tx = transaction();
        bind_p2pk_and_sign(&mut tx, [0x31; 32]);
        assert!(is_fully_signed(&tx));
        tx
    }

    let mut wrong_key = valid();
    let other = compressed_pubkey_from_raw_key(&[0x32; 32]).expect("other public key");
    wrong_key.inputs[0].utxo_entry.script_public_key.script[1..33].copy_from_slice(&other[1..33]);
    assert_eq!(signature_status(&wrong_key), (0, 1));
    assert!(!is_fully_signed(&wrong_key));

    let mut wrong_digest = valid();
    wrong_digest.locktime ^= 1;
    assert_eq!(signature_status(&wrong_digest), (0, 1));
    assert!(!is_fully_signed(&wrong_digest));

    let mut wrong_position = valid();
    wrong_position.inputs[0].sigs[0].pubkey_pos = 1;
    assert_eq!(signature_status(&wrong_position), (0, 1));
    assert!(!is_fully_signed(&wrong_position));

    let mut wrong_sighash = valid();
    wrong_sighash.inputs[0].sigs[0].sighash_type = 0;
    assert_eq!(signature_status(&wrong_sighash), (0, 1));
    assert!(!is_fully_signed(&wrong_sighash));

    let mut corrupted = valid();
    corrupted.inputs[0].sigs[0].signature[17] ^= 1;
    assert_eq!(signature_status(&corrupted), (0, 1));
    assert!(!is_fully_signed(&corrupted));
}
