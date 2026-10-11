use super::*;
use offline_signer::{
    address::{encode_p2pk_address, encode_p2sh_address, KaspaNetwork},
    derivation::bip32::{
        derive_account_key, derive_address_key, derive_change_key, ADDR_SCAN_DEPTH,
    },
    transaction::model::{SigHashType, TransactionInput, TransactionOutput},
};

const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn account() -> bip32::ExtendedPrivKey {
    let wallet = HotWallet::restore(MNEMONIC, "").expect("known mnemonic restores");
    derive_account_key(&wallet.seed.bytes).expect("account derivation")
}

fn script(bytes: &[u8]) -> ScriptPublicKey {
    let mut script = ScriptPublicKey::default();
    script.script[..bytes.len()].copy_from_slice(bytes);
    script.script_len = bytes.len();
    script
}

fn p2pk(xonly: &[u8; 32]) -> Vec<u8> {
    let mut bytes = vec![0x20];
    bytes.extend_from_slice(xonly);
    bytes.push(0xac);
    bytes
}

fn p2sh(hash: &[u8; 32]) -> Vec<u8> {
    let mut bytes = vec![0xaa, 0x20];
    bytes.extend_from_slice(hash);
    bytes.push(0x87);
    bytes
}

fn hinted_output(branch: u8, index: u32, xonly: &[u8; 32]) -> TransactionOutput {
    let mut output = TransactionOutput::empty();
    output.has_derivation_hint = true;
    output.derivation_branch = branch;
    output.derivation_index = index;
    output.script_public_key = script(&p2pk(xonly));
    output
}

#[test]
fn ownership_and_script_type_labels_are_exact() {
    assert_eq!(OutputOwnership::External.label(), "External");
    assert_eq!(OutputOwnership::Change.label(), "Change");
    assert_eq!(OutputOwnership::Receive.label(), "Own receive");
    assert_eq!(script_type_label(ScriptType::P2PK), "P2PK");
    assert_eq!(script_type_label(ScriptType::P2SH), "P2SH");
    assert_eq!(script_type_label(ScriptType::Multisig), "Multisig");
    assert_eq!(script_type_label(ScriptType::Unknown), "Unknown");
}

#[test]
fn script_addresses_are_encoded_only_for_known_networks_and_shapes() {
    let key = [0x11; 32];
    let hash = [0x22; 32];
    assert_eq!(
        script_address_shape(&p2pk(&key)),
        Some((AddressType::P2pk, 1))
    );
    assert_eq!(
        script_address_shape(&p2sh(&hash)),
        Some((AddressType::P2sh, 2))
    );
    assert_eq!(script_address_shape(&[0x51]), None);

    assert_eq!(
        script_address(&script(&p2pk(&key)), KaspaNetwork::Mainnet),
        Some(encode_p2pk_address(&key, "kaspa"))
    );
    assert_eq!(
        script_address(&script(&p2sh(&hash)), KaspaNetwork::Testnet),
        Some(encode_p2sh_address(&hash, "kaspatest"))
    );
    assert_eq!(
        script_address(&script(&p2pk(&key)), KaspaNetwork::Unknown),
        None
    );
    assert_eq!(
        script_address(&script(&[0x51]), KaspaNetwork::Mainnet),
        None
    );
    assert_eq!(
        encode_script_address(&key, AddressType::P2pk, KaspaNetwork::Mainnet),
        Some(encode_p2pk_address(&key, "kaspa"))
    );
}

#[test]
fn p2pk_extraction_requires_length_push_and_checksig() {
    let key = [0x33; 32];
    assert_eq!(p2pk_xonly(&script(&p2pk(&key))), Some(key));

    let mut wrong_push = p2pk(&key);
    wrong_push[0] = 0x21;
    assert_eq!(p2pk_xonly(&script(&wrong_push)), None);

    let mut wrong_opcode = p2pk(&key);
    wrong_opcode[33] = 0xad;
    assert_eq!(p2pk_xonly(&script(&wrong_opcode)), None);

    let mut long = p2pk(&key);
    long.push(0x00);
    assert_eq!(p2pk_xonly(&script(&long)), None);
}

#[test]
fn hinted_keys_follow_the_hinted_branch() {
    let account = account();
    let receive = derive_address_key(&account, 3)
        .unwrap()
        .public_key_x_only()
        .unwrap();
    let change = derive_change_key(&account, 3)
        .unwrap()
        .public_key_x_only()
        .unwrap();
    assert_ne!(receive, change);
    assert_eq!(
        hinted_output_xonly(&account, &hinted_output(0, 3, &receive)).unwrap(),
        receive
    );
    assert_eq!(
        hinted_output_xonly(&account, &hinted_output(1, 3, &change)).unwrap(),
        change
    );

    assert_eq!(find_owned_output(&account, &change, 0), Some(1));
    assert_eq!(find_owned_output(&account, &receive, 1), Some(0));
    assert_eq!(find_owned_output(&account, &[0x77; 32], 0), None);
}

#[test]
fn verified_hints_beyond_the_scan_window_still_label_owned_outputs() {
    let account = account();
    let index = u32::from(ADDR_SCAN_DEPTH) + 5;
    let receive = derive_address_key(&account, index)
        .unwrap()
        .public_key_x_only()
        .unwrap();
    let mut transaction = Transaction::try_new().expect("transaction allocation");
    transaction.num_outputs = 1;
    transaction.outputs[0] = hinted_output(0, index, &receive);
    let mut ownership = [OutputOwnership::External];
    verify_account_hints(&transaction, &account, &mut ownership).unwrap();
    assert_eq!(ownership, [OutputOwnership::Receive]);

    // A hint naming a non-wallet branch is not trusted, and the scan window
    // does not reach this index, so the output stays external.
    transaction.outputs[0].derivation_branch = 2;
    let mut ownership = [OutputOwnership::External];
    verify_account_hints(&transaction, &account, &mut ownership).unwrap();
    assert_eq!(ownership, [OutputOwnership::External]);
}

#[test]
fn generic_review_accepts_only_explainable_inputs_and_outputs() {
    let mut input = TransactionInput::empty();
    input.sequence = u64::MAX;
    input.sig_op_count = 1;
    input.sighash_type = 0;
    assert!(validate_generic_input_fields(&input).is_ok());
    input.sighash_type = SigHashType::All.to_byte();
    assert!(validate_generic_input_fields(&input).is_ok());
    input.sighash_type = 0x02;
    assert!(validate_generic_input_fields(&input).is_err());

    let mut output = TransactionOutput::empty();
    output.script_public_key = script(&p2pk(&[0x44; 32]));
    assert!(generic_output_script_supported(&output));
    output.script_public_key = script(&p2sh(&[0x45; 32]));
    assert!(generic_output_script_supported(&output));
    output.script_public_key = script(&[0x51]);
    assert!(!generic_output_script_supported(&output));
}
