use crate::transaction::{
    model::{Transaction, MAX_SIGS_PER_INPUT},
    std_pskt::{move_ksp_sigs_to_pskt, pskt_signature_status, PskError},
};

fn p2pk_transaction() -> Transaction {
    let mut transaction = Transaction::try_new().expect("transaction test allocation");
    transaction.num_inputs = 1;
    let script = &mut transaction.inputs[0].utxo_entry.script_public_key;
    script.script[0] = 0x20;
    script.script[1..33].fill(0x11);
    script.script[33] = 0xac;
    script.script_len = 34;
    transaction
}

#[test]
fn pskt_signature_merge_preserves_existing_entries_deduplicates_and_sorts() {
    let mut transaction = p2pk_transaction();
    let input = &mut transaction.inputs[0];

    input.incoming_partial_sigs[0].present = true;
    input.incoming_partial_sigs[0].pubkey = [0x03; 33];
    input.incoming_partial_sigs[0].signature = [0x33; 64];
    input.incoming_partial_sigs_count = 1;

    input.sig_count = 5;
    input.sigs[0].present = false;
    input.sigs[1].present = true;
    input.sigs[1].pubkey_compressed = [0u8; 33];
    input.sigs[2].present = true;
    input.sigs[2].pubkey_compressed = [0x03; 33];
    input.sigs[2].signature = [0x33; 64];
    input.sigs[3].present = true;
    input.sigs[3].pubkey_compressed = [0x02; 33];
    input.sigs[3].signature = [0x22; 64];
    input.sigs[4].present = true;
    input.sigs[4].pubkey_compressed = [0x04; 33];
    input.sigs[4].signature = [0x44; 64];

    move_ksp_sigs_to_pskt(&mut transaction).expect("merge signatures");

    let input = &transaction.inputs[0];
    assert_eq!(input.incoming_partial_sigs_count, 3);
    assert_eq!(input.incoming_partial_sigs[0].pubkey, [0x02; 33]);
    assert_eq!(input.incoming_partial_sigs[1].pubkey, [0x03; 33]);
    assert_eq!(input.incoming_partial_sigs[2].pubkey, [0x04; 33]);
    assert_eq!(input.incoming_partial_sigs[1].signature, [0x33; 64]);

    move_ksp_sigs_to_pskt(&mut transaction).expect("merge signatures");
    assert_eq!(transaction.inputs[0].incoming_partial_sigs_count, 3);
}

#[test]
fn pskt_signature_merge_rejects_conflicting_existing_signature() {
    let mut transaction = p2pk_transaction();
    let input = &mut transaction.inputs[0];
    input.incoming_partial_sigs_count = 1;
    input.incoming_partial_sigs[0].present = true;
    input.incoming_partial_sigs[0].pubkey = [0x03; 33];
    input.incoming_partial_sigs[0].signature = [0x11; 64];
    input.sig_count = 1;
    input.sigs[0].present = true;
    input.sigs[0].pubkey_compressed = [0x03; 33];
    input.sigs[0].signature = [0x22; 64];

    assert_eq!(
        move_ksp_sigs_to_pskt(&mut transaction),
        Err(PskError::SignatureConflict)
    );
    assert_eq!(
        transaction.inputs[0].incoming_partial_sigs[0].signature,
        [0x11; 64]
    );
}

#[test]
fn pskt_signature_merge_stops_at_fixed_capacity() {
    let mut transaction = p2pk_transaction();
    let input = &mut transaction.inputs[0];
    input.incoming_partial_sigs_count = MAX_SIGS_PER_INPUT as u8;
    for index in 0..MAX_SIGS_PER_INPUT {
        input.incoming_partial_sigs[index].present = true;
        input.incoming_partial_sigs[index].pubkey = [index as u8 + 1; 33];
    }
    input.sig_count = 1;
    input.sigs[0].present = true;
    input.sigs[0].pubkey_compressed = [0xff; 33];

    assert_eq!(
        move_ksp_sigs_to_pskt(&mut transaction),
        Err(PskError::TooManyPartialSigs)
    );
    assert_eq!(
        transaction.inputs[0].incoming_partial_sigs_count,
        MAX_SIGS_PER_INPUT as u8
    );
}

#[test]
fn pskt_signature_status_counts_only_valid_bound_signatures() {
    use crate::{
        derivation::bip32::compressed_pubkey_from_raw_key,
        transaction::{model::SigHashType, sighash},
    };

    let private_key = [0x21; 32];
    let compressed = compressed_pubkey_from_raw_key(&private_key).expect("public key");
    let mut p2pk = p2pk_transaction();
    p2pk.inputs[0].utxo_entry.script_public_key.script[1..33].copy_from_slice(&compressed[1..33]);
    p2pk.inputs[0].sighash_type = SigHashType::All.to_byte();
    let signature =
        sighash::sign_input(&p2pk, 0, &private_key, SigHashType::All).expect("valid signature");
    p2pk.inputs[0].incoming_partial_sigs[0].present = true;
    p2pk.inputs[0].incoming_partial_sigs[0].pubkey = compressed;
    p2pk.inputs[0].incoming_partial_sigs[0].signature = signature.bytes;
    p2pk.inputs[0].incoming_partial_sigs_count = 1;
    assert_eq!(pskt_signature_status(&p2pk), (1, 1));

    p2pk.inputs[0].incoming_partial_sigs[0].signature[0] ^= 1;
    assert_eq!(pskt_signature_status(&p2pk), (0, 1));
}

#[test]
fn pskt_signature_status_rejects_unbound_or_generic_covenant_completion() {
    let mut unknown = Transaction::try_new().expect("transaction test allocation");
    unknown.num_inputs = 1;
    assert_eq!(pskt_signature_status(&unknown), (0, 1));

    let mut p2sh = Transaction::try_new().expect("transaction test allocation");
    p2sh.num_inputs = 1;
    let script = &mut p2sh.inputs[0].utxo_entry.script_public_key;
    script.script[0] = 0xaa;
    script.script[1] = 0x20;
    script.script[2..34].fill(0x44);
    script.script[34] = 0x87;
    script.script_len = 35;
    assert_eq!(pskt_signature_status(&p2sh), (0, 1));
}
