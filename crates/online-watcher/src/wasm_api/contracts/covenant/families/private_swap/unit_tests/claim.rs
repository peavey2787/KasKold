//! Claim-PSKT preparation and completed-signature insertion.

use super::*;

fn unsigned_pskt(input_count: usize, partial_sigs: Value) -> String {
    let inputs = (0..input_count)
        .map(|index| {
            json!({
                "utxoEntry": {
                    "amount": 1000,
                    "scriptPublicKey": format!("000020{}ac", "11".repeat(32))
                },
                "previousOutpoint": {
                    "transactionId": format!("{:02x}", u8::try_from(index).unwrap().wrapping_add(1)).repeat(32),
                    "index": index
                },
                "sighashType": 1,
                "partialSigs": partial_sigs.clone(),
                "proprietaries": {}
            })
        })
        .collect::<Vec<_>>();
    let document = json!({
        "global": {
            "version": 0,
            "txVersion": 0,
            "inputCount": input_count,
            "outputCount": 0
        },
        "inputs": inputs,
        "outputs": []
    });
    raw_pskt(&document)
}

fn raw_pskt(document: &Value) -> String {
    let mut wire = b"PSKT".to_vec();
    wire.extend_from_slice(hex::encode(serde_json::to_vec(document).unwrap()).as_bytes());
    hex::encode(wire)
}

#[test]
fn completed_signature_insertion_requires_one_unsigned_input_and_preserves_sighash_all() {
    let wire = unsigned_pskt(1, json!({}));
    let signed = private_swap_insert_completed_signature_string(&wire, &h(0x22, 32), &h(0x33, 64))
        .expect("insert completed signature");
    let signed = hex::decode(signed).unwrap();
    assert_eq!(&signed[..4], b"PSKT");
    let root: Value = serde_json::from_slice(&hex::decode(&signed[4..]).unwrap()).unwrap();
    let partial = root["inputs"][0]["partialSigs"].as_object().unwrap();
    let entry = partial.get(&format!("02{}", h(0x22, 32))).unwrap();
    assert!(entry.get("sighashType").is_none());
    assert_eq!(entry["schnorr"], h(0x33, 64));

    assert_eq!(
        private_swap_insert_completed_signature_string(&wire, "00", &h(0x33, 64))
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("claim pubkey must be 32 bytes")
    );
    assert_eq!(
        private_swap_insert_completed_signature_string(&wire, &h(0x22, 32), "00")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Private Swap completed signature must be 64 bytes")
    );
    assert_eq!(
        insert_completed_signature(&unsigned_pskt(0, json!({})), &[1; 32], &[2; 64])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("a sole signature needs exactly one input, got 0")
    );
    assert_eq!(
        insert_completed_signature(&unsigned_pskt(2, json!({})), &[1; 32], &[2; 64])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("a sole signature needs exactly one input, got 2")
    );
    assert_eq!(
        insert_completed_signature(
            &unsigned_pskt(
                1,
                json!({
                    format!("02{}", h(0x11, 32)): {"schnorr": h(0x22, 64)}
                }),
            ),
            &[1; 32],
            &[2; 64]
        )
        .err()
        .map(|error| error.to_string())
        .as_deref(),
        Some("input already carries a signature")
    );
    let missing = raw_pskt(&json!({"inputs": [{}]}));
    assert_eq!(
        insert_completed_signature(&missing, &[1; 32], &[2; 64])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("inputs[0].partialSigs must be an object")
    );
}

#[test]
fn private_swap_claim_preparation_is_host_testable_without_network_or_wasm_values() {
    let destination = crate::account::address::encode_p2pk_address(&[0x22; 32], "kaspa");
    let script = crate::contracts::covenant::script::build_private_swap_script(
        &[0x11; 32],
        &[0x12; 32],
        &crate::account::address::address_to_script_pubkey(&destination).expect("destination"),
        100,
        &[0x13; 16],
    )
    .expect("swap script");
    let source = crate::wasm_api::test_support::covenant_address(&script);
    let script_hex = hex::encode(&script);
    let one = serde_json::json!([{
        "tx_id": "33".repeat(32), "index": 0, "amount": "1000000"
    }])
    .to_string();
    let (prepared, redeem) =
        prepare_private_swap_claim(&source, &destination, &script_hex, &one, 1000)
            .expect("single selected UTXO");
    let pskb = ready(create_private_swap_claim(
        &source,
        &destination,
        &script_hex,
        &one,
        1000,
    ))
    .expect("claim PSKB wrapper");
    assert!(!pskb.is_empty());
    assert_eq!(prepared.utxos.len(), 1);
    assert_eq!(redeem, script);

    let two = serde_json::json!([
        {"tx_id": "33".repeat(32), "index": 0, "amount": "1000000"},
        {"tx_id": "44".repeat(32), "index": 1, "amount": "1000000"}
    ])
    .to_string();
    assert_eq!(
        prepare_private_swap_claim(&source, &destination, &script_hex, &two, 1000)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Private Swap claim requires exactly one funding UTXO")
    );
    assert_eq!(
        prepare_private_swap_claim(&source, &destination, "zz", &one, 1000)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Bad redeem hex: Invalid character 'z' at position 0")
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn private_swap_claim_sighash_wasm_boundary_rejects_malformed_wire() {
    assert_eq!(
        private_swap_claim_sighash("zz")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad KSPT: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        private_swap_claim_sighash("00")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Private Swap claim: BufferTooShort"))"#)
    );
}
