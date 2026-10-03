use serde_json::json;

fn signature_map() -> serde_json::Value {
    let mut signatures = serde_json::Map::new();
    signatures.insert(
        format!("02{}", "11".repeat(32)),
        json!({ "schnorr": "22".repeat(64) }),
    );
    serde_json::Value::Object(signatures)
}

fn p2pk_input() -> serde_json::Value {
    json!({
        "utxoEntry": {
            "amount": 100_000_000u64,
            "scriptPublicKey": format!("000020{}ac", "33".repeat(32))
        },
        "previousOutpoint": {
            "transactionId": "44".repeat(32),
            "index": 7
        },
        "sequence": 9,
        "sigOpCount": 2,
        "redeemScript": null,
        "partialSigs": signature_map()
    })
}

fn encoded_pskb(document: serde_json::Value) -> String {
    crate::transaction_builder::pskb::encode_pskt_value(super::canonical_test_pskt(document))
        .expect("PSKB wire")
}

#[test]
fn production_finalizer_rejects_placeholder_signatures_before_script_assembly() {
    use super::super::consensus::finalize_to_consensus;

    let wire = encoded_pskb(json!({
        "global": {"txVersion": 0},
        "inputs": [p2pk_input()],
        "outputs": [{
            "amount": 5,
            "scriptPublicKey": "000051",
            "covenantBinding": null
        }]
    }));
    let error = finalize_to_consensus(&wire).expect_err("placeholder signature must not finalize");
    assert!(
        error.contains("cryptographic") || error.contains("complete"),
        "unexpected verification error: {error}"
    );
}

#[test]
fn verified_materializer_covers_authorized_input_output_and_storage_mass_paths() {
    use crate::protocol::transaction::consensus::InputEncoding;

    let signer = 0x19;
    let document = json!([{
        "global": {
            "txVersion": 1,
            "fallbackLockTime": "42",
            "subnetworkId": "ab".repeat(20),
            "gas": "7",
            "txPayload": "cafe"
        },
        "inputs": [{
            "utxoEntry": {
                "amount": "1000000000",
                "scriptPublicKey": format!(
                    "000020{}ac",
                    hex::encode(super::kspt_bridge::test_xonly(signer))
                ),
                "covenantId": "11".repeat(32)
            },
            "previousOutpoint": {
                "transactionId": "44".repeat(32),
                "index": 7
            },
            "sequence": "9",
            "sigOpCount": 1,
            "redeemScript": null,
            "partialSigs": {}
        }],
        "outputs": [{
            "amount": "100000000",
            "scriptPublicKey": format!("000020{}ac", "55".repeat(32)),
            "covenantBinding": {
                "authorizingInput": 0,
                "covenantId": "22".repeat(32)
            }
        }]
    }]);
    let (_, signed, _) = super::kspt_bridge::sign_first_input_document(document, &[signer]);

    let finalized = super::super::consensus::finalize_to_consensus(&signed)
        .expect("verified consensus finalization");
    assert!(finalized.storage_mass > 0);
    let transaction = finalized.into_consensus_transaction();
    assert_eq!(transaction.tx_version, 1);
    assert_eq!(transaction.input_encoding, InputEncoding::Budgeted);
    assert_eq!(transaction.inputs.len(), 1);
    assert_eq!(transaction.outputs.len(), 1);
    assert_eq!(transaction.inputs[0].prev_tx_id, [0x44; 32]);
    assert_eq!(transaction.inputs[0].prev_index, 7);
    assert_eq!(transaction.inputs[0].sequence, 9);
    assert_eq!(transaction.inputs[0].sig_op_count, 1);
    assert_eq!(transaction.inputs[0].sig_script.len(), 66);
    assert_eq!(transaction.outputs[0].value, 100_000_000);
    assert_eq!(transaction.outputs[0].covenant, Some((0, [0x22; 32])));
    assert_eq!(transaction.locktime, 42);
    assert_eq!(transaction.subnetwork_id, [0xab; 20]);
    assert_eq!(transaction.gas, 7);
    assert_eq!(transaction.payload, vec![0xca, 0xfe]);
    assert!(transaction.storage_mass > 0);

    let signed_kspt = crate::protocol::pskt::relay_pskb_as_kspt_hex_for_network(&signed, "mainnet")
        .expect("signed compact relay");
    let compact = crate::protocol::transaction::signed_kspt::decode_signed_kspt(&signed_kspt)
        .expect("verified compact transaction materialization");
    assert_eq!(compact.input_encoding, InputEncoding::Compact);
    assert_eq!(compact.tx_version, transaction.tx_version);
    assert_eq!(compact.inputs.len(), transaction.inputs.len());
    assert_eq!(compact.outputs.len(), transaction.outputs.len());
    assert_eq!(compact.payload, transaction.payload);
    assert!(compact.storage_mass > 0);
}
