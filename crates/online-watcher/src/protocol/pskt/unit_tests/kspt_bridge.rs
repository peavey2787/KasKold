use serde_json::{json, Map, Value};

use super::super::{merge_signed_kspt_into_pskb, relay_pskb_as_kspt_hex_for_network};

const RELAY_KSPT_HEX: &str = "4b53505401000000010000000100000000000000000000000000000000000000000000000000000000000000000000000000001111111111111111111111111111111111111111111111111111111111111111010000006400000000000000000000000000000001000022204444444444444444444444444444444444444444444444444444444444444444ac0000005a00000000000000000022205555555555555555555555555555555555555555555555555555555555555555ac4e01";

pub(super) fn test_secret(seed: u8) -> [u8; 32] {
    let mut secret = [0u8; 32];
    secret[31] = seed.max(1);
    secret
}

pub(super) fn test_xonly(seed: u8) -> [u8; 32] {
    let signing = k256::schnorr::SigningKey::from_bytes(&test_secret(seed))
        .expect("deterministic test signing key");
    let bytes = signing.verifying_key().to_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes[..]);
    out
}

pub(super) fn test_compressed_key(seed: u8) -> String {
    format!("02{}", hex::encode(test_xonly(seed)))
}

fn encode_document(document: &Value) -> String {
    let document = super::canonical_test_pskt(document.clone());
    let json = serde_json::to_vec(&document).expect("PSKB JSON");
    let mut wire = b"PSKB".to_vec();
    wire.extend_from_slice(hex::encode(json).as_bytes());
    hex::encode(wire)
}

pub(super) fn sign_first_input_document(
    mut document: Value,
    seeds: &[u8],
) -> (String, String, Vec<String>) {
    document = super::canonical_test_pskt(document);
    document[0]["inputs"][0]["partialSigs"] = json!({});
    let unsigned = encode_document(&document);
    let unsigned_kspt =
        relay_pskb_as_kspt_hex_for_network(&unsigned, "mainnet").expect("unsigned relay");
    let digest = crate::protocol::pskt::compact_kspt_sighash_wire(
        &hex::decode(unsigned_kspt).expect("unsigned KSPT hex"),
    )
    .expect("SIGHASH_ALL digest");

    let partials = document[0]["inputs"][0]["partialSigs"]
        .as_object_mut()
        .expect("partial signatures object");
    let mut signatures = Vec::with_capacity(seeds.len());
    for seed in seeds {
        let signature = offline_signer::crypto::schnorr::schnorr_sign(&test_secret(*seed), &digest)
            .expect("deterministic valid Schnorr signature");
        let signature_hex = hex::encode(signature.bytes);
        partials.insert(
            test_compressed_key(*seed),
            json!({"schnorr": signature_hex.clone()}),
        );
        signatures.push(signature_hex);
    }
    (unsigned, encode_document(&document), signatures)
}

fn valid_signed_p2pk_fixture() -> (String, String, String) {
    let mut document = decode_pskb_document(&pskb_wire(false));
    document[0]["inputs"][0]["utxoEntry"]["scriptPublicKey"] =
        json!(format!("000020{}ac", hex::encode(test_xonly(1))));
    let (unsigned, signed, signatures) = sign_first_input_document(document, &[1]);
    (unsigned, signed, signatures.into_iter().next().unwrap())
}

#[test]
fn unsigned_p2pk_relay_vector_is_byte_stable() {
    assert_eq!(
        relay_pskb_as_kspt_hex_for_network(&pskb_wire(false), "mainnet").unwrap(),
        RELAY_KSPT_HEX
    );
}

#[test]
fn signed_relay_flags_match_hardware_import_completeness() {
    let (_, signed_p2pk_pskb, _) = valid_signed_p2pk_fixture();
    let signed_p2pk = relay_pskb_as_kspt_hex_for_network(&signed_p2pk_pskb, "mainnet")
        .expect("signed P2PK relay");
    let signed_p2pk_bytes = hex::decode(&signed_p2pk).expect("signed P2PK hex");
    assert_eq!(
        signed_p2pk_bytes[5],
        kaskold_protocol::wire::kspt::FLAG_SIGNED_OR_COMPLETE
    );
    let mut p2pk_hardware =
        offline_signer::transaction::model::Transaction::try_new().expect("hardware transaction");
    offline_signer::transaction::kspt::parse_compact_kspt(&signed_p2pk_bytes, &mut p2pk_hardware)
        .expect("fully signed host relay must import on KasKold");
    assert_eq!(p2pk_hardware.inputs[0].sig_count, 1);

    let first = test_compressed_key(0x31);
    let second = test_compressed_key(0x32);
    let mut redeem = vec![0x52, 0x20];
    redeem.extend_from_slice(&test_xonly(0x31));
    redeem.push(0x20);
    redeem.extend_from_slice(&test_xonly(0x32));
    redeem.extend_from_slice(&[0x52, 0xae]);
    let p2sh = format!("0000aa20{}87", "77".repeat(32));
    let derivations = json!({
        first.clone(): {"keyFingerprint": "01020304", "derivationPath": "m/45'/111111'/0'/0/0/0"},
        second.clone(): {"keyFingerprint": "05060708", "derivationPath": "m/45'/111111'/0'/0/0/0"}
    });
    let document = json!([{
        "global": {"txVersion": 0, "subnetworkId": "00".repeat(20), "gas": 0},
        "inputs": [{
            "utxoEntry": {"amount": 100000000u64, "scriptPublicKey": p2sh},
            "previousOutpoint": {"transactionId": "66".repeat(32), "index": 0},
            "sequence": u64::MAX.to_string(),
            "partialSigs": {},
            "redeemScript": hex::encode(&redeem),
            "sigOpCount": 2,
            "bip32Derivations": derivations
        }],
        "outputs": [{
            "amount": 99000000u64,
            "scriptPublicKey": format!("000020{}ac", "55".repeat(32))
        }]
    }]);

    let (_, partial_pskb, _) = sign_first_input_document(document.clone(), &[0x31]);
    let partial_relay = relay_pskb_as_kspt_hex_for_network(&partial_pskb, "mainnet")
        .expect("partial multisig relay");
    let partial_bytes = hex::decode(&partial_relay).expect("partial relay hex");
    assert_eq!(partial_bytes[5], 0, "1-of-2 relay must remain partial");
    let mut partial_hardware =
        offline_signer::transaction::model::Transaction::try_new().expect("hardware transaction");
    offline_signer::transaction::kspt::parse_compact_kspt(&partial_bytes, &mut partial_hardware)
        .expect("partially signed host relay must import on KasKold");
    assert_eq!(partial_hardware.inputs[0].sig_count, 1);
    assert!(partial_hardware.inputs[0].ms45_hint.present);

    let (_, complete_pskb, _) = sign_first_input_document(document, &[0x31, 0x32]);
    let complete_relay = relay_pskb_as_kspt_hex_for_network(&complete_pskb, "mainnet")
        .expect("complete multisig relay");
    let complete_bytes = hex::decode(&complete_relay).expect("complete relay hex");
    assert_eq!(
        complete_bytes[5],
        kaskold_protocol::wire::kspt::FLAG_SIGNED_OR_COMPLETE
    );
    let mut complete_hardware =
        offline_signer::transaction::model::Transaction::try_new().expect("hardware transaction");
    offline_signer::transaction::kspt::parse_compact_kspt(&complete_bytes, &mut complete_hardware)
        .expect("complete host multisig relay must import on KasKold");
    assert_eq!(complete_hardware.inputs[0].sig_count, 2);
}

fn pskb_wire(signed: bool) -> String {
    let input_public_key = "44".repeat(32);
    let output_public_key = "55".repeat(32);
    let mut partial_signatures = Map::new();
    if signed {
        partial_signatures.insert(
            format!("02{}", input_public_key),
            json!({"schnorr": "22".repeat(64)}),
        );
    }
    let document = json!([{
        "global": {
            "txVersion": 0,
            "fallbackLockTime": null,
            "subnetworkId": "00".repeat(20),
            "gas": 0
        },
        "inputs": [{
            "utxoEntry": {
                "amount": 100,
                "scriptPublicKey": format!("000020{}ac", input_public_key)
            },
            "previousOutpoint": {
                "transactionId": "11".repeat(32),
                "index": 1
            },
            "sequence": 0,
            "partialSigs": Value::Object(partial_signatures),
            "redeemScript": null,
            "sigOpCount": 1
        }],
        "outputs": [{
            "amount": 90,
            "scriptPublicKey": format!("000020{}ac", output_public_key)
        }]
    }]);
    let document = super::canonical_test_pskt(document);
    let json = serde_json::to_vec(&document).unwrap();
    let mut wire = b"PSKB".to_vec();
    wire.extend_from_slice(hex::encode(json).as_bytes());
    hex::encode(wire)
}

#[test]
fn relay_v4_encodes_explicit_networks_and_derivation_hints() {
    let mut document = decode_pskb_document(&pskb_wire(false));
    document[0]["inputs"][0]["proprietaries"] = json!({
        "kassignerDerivation": {"branch": 0, "index": "500"}
    });
    document[0]["outputs"][0]["proprietaries"] = json!({
        "kassignerDerivation": {"branch": 1, "index": "37"}
    });
    let wire = encode_document(&document);

    for (network, code) in [
        ("mainnet", 1u8),
        ("testnet-10", 2),
        ("devnet", 3),
        ("simnet", 4),
    ] {
        let bytes = hex::decode(
            relay_pskb_as_kspt_hex_for_network(&wire, network).expect("network-aware relay"),
        )
        .expect("relay hex");
        assert_eq!(bytes[4], kaskold_protocol::wire::kspt::KSPT_VERSION);
        assert!(
            bytes.ends_with(&[b'N', code, b'A', 0, 0, 0xf4, 0x01, 0, 0, b'D', 0, 1, 37, 0, 0, 0,])
        );
    }

    assert_eq!(
        relay_pskb_as_kspt_hex_for_network(&wire, "unknown").unwrap_err(),
        "WrongNetwork: unsupported Kaspa network: unknown",
    );

    document[0]["outputs"][0]["proprietaries"]["kassignerDerivation"] =
        json!({"branch": 0, "index": 11});
    let numeric = hex::decode(
        relay_pskb_as_kspt_hex_for_network(&encode_document(&document), "mainnet")
            .expect("numeric derivation hint"),
    )
    .expect("relay hex");
    assert!(numeric.ends_with(&[b'N', 1, b'A', 0, 0, 0xf4, 0x01, 0, 0, b'D', 0, 0, 11, 0, 0, 0,]));

    document[0]["outputs"][0]["proprietaries"]["kassignerDerivation"] =
        json!({"branch": 2, "index": 11});
    assert!(
        relay_pskb_as_kspt_hex_for_network(&encode_document(&document), "mainnet")
            .unwrap_err()
            .contains("branch must be 0 or 1")
    );
}

fn decode_pskb_document(wire: &str) -> Value {
    let bytes = hex::decode(wire).expect("wire hex");
    assert_eq!(&bytes[..4], b"PSKB");
    let json_hex = core::str::from_utf8(&bytes[4..]).expect("JSON hex");
    serde_json::from_slice(&hex::decode(json_hex).expect("JSON bytes")).expect("PSKB JSON")
}

#[test]
fn signed_kspt_merge_populates_p2pk_signatures_and_rejects_bad_envelopes() {
    let (unsigned_pskb, signed_pskb, signature_hex) = valid_signed_p2pk_fixture();
    let signed_kspt =
        relay_pskb_as_kspt_hex_for_network(&signed_pskb, "mainnet").expect("valid signed KSPT");

    let merged =
        merge_signed_kspt_into_pskb(&signed_kspt, &unsigned_pskb).expect("merge signed KSPT");
    let document = decode_pskb_document(&merged);
    let signatures = document[0]["inputs"][0]["partialSigs"]
        .as_object()
        .expect("partial signatures");
    assert_eq!(signatures.len(), 1);
    let signature = signatures.values().next().expect("signature");
    assert_eq!(signature["schnorr"], signature_hex);

    assert!(merge_signed_kspt_into_pskb("zz", &unsigned_pskb).is_err());
    assert!(merge_signed_kspt_into_pskb(&hex::encode(b"KSPT"), &unsigned_pskb).is_err());

    for network_code in 1u8..=4 {
        let mut networked = hex::decode(&signed_kspt).unwrap();
        assert_eq!(networked[networked.len() - 2], b'N');
        let last = networked.len() - 1;
        networked[last] = network_code;
        merge_signed_kspt_into_pskb(&hex::encode(networked), &unsigned_pskb)
            .expect("canonical signed network code");
    }
    let mut invalid_network = hex::decode(&signed_kspt).unwrap();
    let last = invalid_network.len() - 1;
    invalid_network[last] = 5;
    assert!(
        merge_signed_kspt_into_pskb(&hex::encode(invalid_network), &unsigned_pskb)
            .unwrap_err()
            .contains("network")
    );

    let mut unsupported = hex::decode(&signed_kspt).unwrap();
    unsupported[4] = 3;
    assert!(
        merge_signed_kspt_into_pskb(&hex::encode(unsupported), &unsigned_pskb)
            .unwrap_err()
            .contains("unsupported KSPT version")
    );

    let signature_bytes = hex::decode(&signature_hex).expect("signature hex");
    let mut prefix = vec![0u8, 1u8];
    prefix.extend_from_slice(&signature_bytes[..4]);
    let mut changed_sighash = hex::decode(&signed_kspt).unwrap();
    let signature_offset = changed_sighash
        .windows(prefix.len())
        .position(|window| window == prefix)
        .expect("signature record prefix");
    changed_sighash[signature_offset + 1] = 0x02;
    assert_eq!(
        merge_signed_kspt_into_pskb(&hex::encode(changed_sighash), &unsigned_pskb).unwrap_err(),
        "TransactionMismatch: input[0] signed KSPT changed sighash type to 0x02",
    );
}

#[test]
fn relay_outpoint_parser_covers_valid_zero_and_malformed_shapes() {
    use super::super::kspt_bridge::first_outpoint;

    let valid = json!([{
        "previousOutpoint": {
            "transactionId": "ab".repeat(32),
            "index": 7
        }
    }]);
    assert_eq!(
        first_outpoint(valid.as_array().unwrap()),
        Some(([0xab; 32], 7))
    );

    let zero_index = json!([{
        "previousOutpoint": {
            "transactionId": "cd".repeat(32),
            "index": 0
        }
    }]);
    assert_eq!(
        first_outpoint(zero_index.as_array().unwrap()),
        Some(([0xcd; 32], 0)),
    );

    for malformed in [
        json!([]),
        json!([null]),
        json!([{}]),
        json!([{"previousOutpoint": null}]),
        json!([{"previousOutpoint": {}}]),
        json!([{"previousOutpoint": {"transactionId": "cd".repeat(32)}}]),
        json!([{"previousOutpoint": {"transactionId": 1}}]),
        json!([{"previousOutpoint": {"transactionId": "zz"}}]),
        json!([{"previousOutpoint": {"transactionId": "00"}}]),
    ] {
        assert!(first_outpoint(malformed.as_array().unwrap()).is_none());
    }
}

#[test]
fn relay_appends_stealth_and_persistent_covenant_trailers() {
    let mut document = decode_pskb_document(&pskb_wire(false));
    document[0]["inputs"][0]["proprietaries"] = json!({
        "stealthTweak": "aa".repeat(32),
        "persistentVault": true
    });
    document[0]["outputs"][0]["covenantBinding"] = Value::Null;
    let json = serde_json::to_vec(&document).unwrap();
    let mut wire = b"PSKB".to_vec();
    wire.extend_from_slice(hex::encode(json).as_bytes());

    let relay =
        relay_pskb_as_kspt_hex_for_network(&hex::encode(wire), "mainnet").expect("relay trailers");
    let bytes = hex::decode(relay).unwrap();
    let covenant = bytes.len() - 36;
    let stealth = covenant - 33;
    assert_eq!(bytes[stealth], b'S');
    assert_eq!(&bytes[stealth + 1..covenant], &[0xaa; 32]);
    assert_eq!(bytes[covenant], b'C');
    assert_eq!(bytes[covenant + 1], 0);
    assert_eq!(&bytes[covenant + 2..covenant + 4], &[0, 0]);

    let mut output_script = vec![0x20];
    output_script.extend_from_slice(&[0x55; 32]);
    output_script.push(0xac);
    let expected = crate::protocol::pskt::scripts::compute_genesis_covenant_id(
        &[0x11; 32],
        1,
        0,
        90,
        0,
        &output_script,
    );
    assert_eq!(&bytes[covenant + 4..covenant + 36], &expected[..]);
}

#[test]
fn relay_preserves_explicit_covenant_bindings_and_count_boundaries() {
    let mut document = decode_pskb_document(&pskb_wire(false));
    let valid_input = document[0]["inputs"][0].clone();
    document[0]["inputs"] = Value::Array(vec![valid_input; 8]);
    document[0]["global"]["inputCount"] = json!(8);
    document[0]["outputs"][0]["covenantBinding"] = json!({
        "authorizingInput": 7,
        "covenantId": "ab".repeat(32)
    });
    let json = serde_json::to_vec(&document).unwrap();
    let mut wire = b"PSKB".to_vec();
    wire.extend_from_slice(hex::encode(json).as_bytes());
    let relay = hex::decode(
        relay_pskb_as_kspt_hex_for_network(&hex::encode(wire), "mainnet")
            .expect("explicit binding"),
    )
    .unwrap();
    assert_eq!(relay[relay.len() - 36], b'C');
    assert_eq!(relay[relay.len() - 35], 0);
    assert_eq!(
        &relay[relay.len() - 34..relay.len() - 32],
        &7u16.to_le_bytes()
    );
    assert_eq!(&relay[relay.len() - 32..], &[0xab; 32]);

    document[0]["outputs"][0]["covenantBinding"]["covenantId"] = Value::String("ab".repeat(31));
    let json = serde_json::to_vec(&document).unwrap();
    let mut wire = b"PSKB".to_vec();
    wire.extend_from_slice(hex::encode(json).as_bytes());
    assert_eq!(
        relay_pskb_as_kspt_hex_for_network(&hex::encode(wire), "mainnet").unwrap_err(),
        "Encoding: output[0] covenant id must be 32 bytes",
    );

    fn unchecked(document: Value) -> String {
        let json = serde_json::to_vec(&vec![document]).unwrap();
        let mut wire = b"PSKB".to_vec();
        wire.extend_from_slice(hex::encode(json).as_bytes());
        hex::encode(wire)
    }

    let source = decode_pskb_document(&pskb_wire(false));
    let valid_input = source[0]["inputs"][0].clone();
    let mut global_33 = source[0]["global"].clone();
    global_33["inputCount"] = json!(33);
    let inputs_33 = json!({
        "global": global_33,
        "inputs": vec![valid_input; 33],
        "outputs": source[0]["outputs"].clone()
    });
    assert_eq!(
        relay_pskb_as_kspt_hex_for_network(&unchecked(inputs_33), "mainnet").unwrap_err(),
        "Encoding: too many inputs for signer capabilities: 33 > 32",
    );

    let mut valid_output = source[0]["outputs"][0].clone();
    valid_output["amount"] = json!(1);
    let mut global_8 = source[0]["global"].clone();
    global_8["outputCount"] = json!(8);
    let outputs_8 = json!({
        "global": global_8,
        "inputs": source[0]["inputs"].clone(),
        "outputs": vec![valid_output.clone(); 8]
    });
    assert!(
        relay_pskb_as_kspt_hex_for_network(&unchecked(outputs_8), "mainnet").is_ok(),
        "exact signer output capacity must remain relayable",
    );

    let mut global_9 = source[0]["global"].clone();
    global_9["outputCount"] = json!(9);
    let outputs_9 = json!({
        "global": global_9,
        "inputs": source[0]["inputs"].clone(),
        "outputs": vec![valid_output; 9]
    });
    assert_eq!(
        relay_pskb_as_kspt_hex_for_network(&unchecked(outputs_9), "mainnet").unwrap_err(),
        "Encoding: too many outputs for signer capabilities: 9 > 8",
    );
}

#[test]
fn compact_private_swap_sighash_wire_covers_current_single_input_relay() {
    let wire = hex::decode(RELAY_KSPT_HEX).expect("relay KSPT");
    let digest =
        crate::protocol::pskt::compact_kspt_sighash_wire(&wire).expect("SIGHASH_ALL digest");
    assert_ne!(digest, [0u8; 32]);
    assert!(crate::protocol::pskt::compact_kspt_sighash_wire(b"KSPT").is_err());
}

#[test]
fn hd45_relay_and_signature_merge_preserve_derivation_metadata_end_to_end() {
    let mut unsigned_document = decode_pskb_document(&pskb_wire(false));
    unsigned_document[0]["inputs"][0]["utxoEntry"]["scriptPublicKey"] =
        json!(format!("000020{}ac", hex::encode(test_xonly(0x44))));
    let input_derivations = json!({
        test_compressed_key(0x44): {
            "keyFingerprint": "a1b2c3d4",
            "derivationPath": "m/45'/111111'/0'/2/0/17"
        }
    });
    let output_derivations = json!({
        format!("02{}", "55".repeat(32)): {
            "keyFingerprint": "11223344",
            "derivationPath": "m/45'/111111'/0'/2/1/9"
        }
    });
    unsigned_document[0]["inputs"][0]["bip32Derivations"] = input_derivations.clone();
    unsigned_document[0]["outputs"][0]["bip32Derivations"] = output_derivations.clone();
    let unsigned_wire = encode_document(&unsigned_document);

    let relay = hex::decode(
        relay_pskb_as_kspt_hex_for_network(&unsigned_wire, "mainnet").expect("45' relay"),
    )
    .expect("relay hex");
    assert!(relay.windows(14).any(|record| {
        record[0] == b'I'
            && record[1] == 0
            && u32::from_le_bytes(record[2..6].try_into().unwrap()) == 2
            && u32::from_le_bytes(record[6..10].try_into().unwrap()) == 0
            && u32::from_le_bytes(record[10..14].try_into().unwrap()) == 17
    }));
    assert!(relay.windows(14).any(|record| {
        record[0] == b'O'
            && record[1] == 0
            && u32::from_le_bytes(record[2..6].try_into().unwrap()) == 2
            && u32::from_le_bytes(record[6..10].try_into().unwrap()) == 1
            && u32::from_le_bytes(record[10..14].try_into().unwrap()) == 9
    }));

    let (_, signed_document, _) = sign_first_input_document(unsigned_document.clone(), &[0x44]);
    let signed_relay =
        relay_pskb_as_kspt_hex_for_network(&signed_document, "mainnet").expect("signed 45' relay");

    let merged =
        merge_signed_kspt_into_pskb(&signed_relay, &unsigned_wire).expect("merge signed 45' relay");
    let merged_document = decode_pskb_document(&merged);
    assert_eq!(
        merged_document[0]["inputs"][0]["bip32Derivations"],
        input_derivations
    );
    assert_eq!(
        merged_document[0]["outputs"][0]["bip32Derivations"],
        output_derivations
    );
    assert_eq!(
        merged_document[0]["inputs"][0]["partialSigs"]
            .as_object()
            .expect("partial signatures")
            .len(),
        1
    );
}
