use crate::protocol::pskt::{
    error::PsktWireError,
    wire::{decode_root, decode_root_for_review, format_wire_error, ErrorStyle},
};
use serde_json::{json, Value};

fn raw_pskb(root: &Value) -> String {
    let mut wire = b"PSKB".to_vec();
    wire.extend_from_slice(hex::encode(serde_json::to_vec(root).unwrap()).as_bytes());
    hex::encode(wire)
}

fn assert_strict_rejects(mut root: Value, mutate: impl FnOnce(&mut Value)) {
    mutate(&mut root);
    assert!(decode_root(&raw_pskb(&root)).is_err());
}

fn clone_error(error: &PsktWireError) -> PsktWireError {
    match error {
        PsktWireError::UnknownFormat => PsktWireError::UnknownFormat,
        PsktWireError::OuterHex(message) => PsktWireError::OuterHex(message.clone()),
        PsktWireError::TooShort => PsktWireError::TooShort,
        PsktWireError::MagicMismatch => PsktWireError::MagicMismatch,
        PsktWireError::InnerHex(message) => PsktWireError::InnerHex(message.clone()),
        PsktWireError::Json(message) => PsktWireError::Json(message.clone()),
    }
}

#[test]
fn wire_error_formatting_covers_standard_and_review_styles() {
    let cases = [
        (
            PsktWireError::UnknownFormat,
            "Not a PSKT/PSKB payload",
            "Not a PSKT/PSKB payload",
        ),
        (
            PsktWireError::OuterHex("bad".into()),
            "outer hex: bad",
            "Bad outer hex: bad",
        ),
        (
            PsktWireError::TooShort,
            "payload too short",
            "Payload too short",
        ),
        (
            PsktWireError::MagicMismatch,
            "wire magic does not match detected format",
            "wire magic does not match detected format",
        ),
        (
            PsktWireError::InnerHex("bad".into()),
            "inner hex: bad",
            "Bad inner hex: bad",
        ),
        (
            PsktWireError::Json("bad".into()),
            "JSON parse: bad",
            "JSON parse: bad",
        ),
    ];

    for (error, standard, review) in cases {
        assert_eq!(
            format_wire_error(clone_error(&error), ErrorStyle::Standard),
            standard,
        );
        assert_eq!(format_wire_error(error, ErrorStyle::Review), review);
    }
}

#[test]
fn exact_four_byte_magic_is_not_misclassified_as_a_short_outer_envelope() {
    let standard = decode_root("50534b54").unwrap_err();
    assert_ne!(standard, "payload too short");
    assert!(standard.starts_with("JSON parse:") || standard.starts_with("inner hex:"));

    let review = decode_root_for_review("50534b42").unwrap_err();
    assert_ne!(review, "Payload too short");
    assert!(review.starts_with("JSON parse:") || review.starts_with("Bad inner hex:"));
}

#[test]
fn pskt_shape_and_output_optional_boundaries_are_exercised_through_strict_decode() {
    use crate::protocol::pskt::{wire::pskt_from_root_for_review, PsktFormat};

    assert!(pskt_from_root_for_review(&json!({}), PsktFormat::Pskb)
        .unwrap_err()
        .contains("PSKB body is not an array"));
    assert!(
        pskt_from_root_for_review(&json!([{}, {}]), PsktFormat::Pskb)
            .unwrap_err()
            .contains("exactly 1 PSKT")
    );

    let base = || {
        json!([{
            "global": {},
            "inputs": [{
                "utxoEntry": {"amount": 2, "scriptPublicKey": "0000"},
                "previousOutpoint": {
                    "transactionId": "00".repeat(32),
                    "index": 0
                },
                "sighashType": 1,
                "proprietaries": {}
            }],
            "outputs": [{
                "amount": 1,
                "scriptPublicKey": "0000",
                "proprietaries": {}
            }]
        }])
    };

    let valid = super::pskb_wire(base());
    decode_root(&valid).expect("valid output optionals");

    let mut with_null_redeem = base();
    with_null_redeem[0]["outputs"][0]["redeemScript"] = serde_json::Value::Null;
    decode_root(&super::pskb_wire(with_null_redeem)).expect("null redeem script");

    for (field, value) in [
        ("redeemScript", json!(1)),
        ("bip32Derivations", json!([])),
        ("proprietaries", json!([])),
    ] {
        let mut invalid = super::canonical_test_pskt(base());
        invalid[0]["outputs"][0][field] = value;
        let json = serde_json::to_vec(&invalid).unwrap();
        let mut wire = b"PSKB".to_vec();
        wire.extend_from_slice(hex::encode(json).as_bytes());
        assert!(decode_root(&hex::encode(wire)).is_err());
    }

    let mut bad_script = base();
    bad_script[0]["outputs"][0]["scriptPublicKey"] = json!("zz");
    assert!(decode_root(&super::pskb_wire(bad_script)).is_err());
}

#[test]
fn strict_decode_covers_global_input_output_and_nested_schema_rejection_matrix() {
    let base = super::canonical_test_pskt(json!([{
        "global": {
            "fallbackLockTime": "0",
            "subnetworkId": "00".repeat(20),
            "gas": "0",
            "txPayload": ""
        },
        "inputs": [{
            "utxoEntry": {"amount": "2", "scriptPublicKey": "0000"},
            "previousOutpoint": {"transactionId": "00".repeat(32), "index": 0},
            "sequence": "0",
            "sigOpCount": 1,
            "minimumSignatures": 1,
            "sighashType": 1,
            "redeemScript": null,
            "partialSigs": {},
            "bip32Derivations": {},
            "proprietaries": {},
            "covenantExecution": null
        }],
        "outputs": [{
            "amount": "1",
            "scriptPublicKey": "0000",
            "redeemScript": null,
            "bip32Derivations": {},
            "proprietaries": {},
            "covenantBinding": null
        }]
    }]));
    assert!(
        decode_root(&raw_pskb(&base)).is_ok(),
        "canonical strict-decode fixture"
    );

    for key in ["global", "inputs", "outputs"] {
        assert_strict_rejects(base.clone(), |root| {
            root[0].as_object_mut().unwrap().remove(key);
        });
    }

    for (field, value) in [
        ("version", Value::Null),
        ("version", json!(1)),
        ("txVersion", json!(65_535)),
        ("inputCount", json!(2)),
        ("outputCount", json!(2)),
        ("fallbackLockTime", json!(true)),
        ("gas", json!([])),
        ("inputsModifiable", json!(1)),
        ("outputsModifiable", json!("false")),
        ("xpubs", json!([])),
        ("proprietaries", Value::Null),
        ("id", json!(1)),
        ("txPayload", json!("0")),
        ("subnetworkId", json!("00")),
        ("covenantBranch", json!("unsupported")),
    ] {
        assert_strict_rejects(base.clone(), |root| {
            root[0]["global"][field] = value;
        });
    }

    for (field, value) in [
        ("utxoEntry", json!([])),
        ("previousOutpoint", json!([])),
        ("sighashType", Value::Null),
        ("sighashType", json!(2)),
        ("sigOpCount", Value::Null),
        ("sigOpCount", json!(6)),
        ("minimumSignatures", Value::Null),
        ("minimumSignatures", json!(0)),
        ("minimumSignatures", json!(6)),
        ("redeemScript", json!(1)),
        ("partialSigs", Value::Null),
        ("partialSigs", json!([])),
        ("bip32Derivations", Value::Null),
        ("bip32Derivations", json!([])),
        ("proprietaries", Value::Null),
        ("proprietaries", json!([])),
        ("covenantExecution", json!(1)),
    ] {
        assert_strict_rejects(base.clone(), |root| {
            root[0]["inputs"][0][field] = value;
        });
    }

    for (field, value) in [
        ("amount", Value::Null),
        ("scriptPublicKey", Value::Null),
        ("scriptPublicKey", json!("0")),
        ("redeemScript", json!(1)),
        ("bip32Derivations", json!([])),
        ("proprietaries", json!([])),
        ("covenantBinding", json!(1)),
    ] {
        assert_strict_rejects(base.clone(), |root| {
            root[0]["outputs"][0][field] = value;
        });
    }

    for value in [
        json!({}),
        json!({"suppliedMask": 1}),
        json!({"suppliedMask": 1, "suppliedTrueMask": 2}),
        json!({"suppliedMask": 65_536, "suppliedTrueMask": 0}),
        json!({"suppliedMask": 1, "suppliedTrueMask": 0, "extra": 0}),
    ] {
        assert_strict_rejects(base.clone(), |root| {
            root[0]["inputs"][0]["covenantExecution"] = value;
        });
    }

    for value in [
        json!({}),
        json!({"authorizingInput": 0}),
        json!({"authorizingInput": 1, "covenantId": "00".repeat(32)}),
        json!({"authorizingInput": 0, "covenantId": "00"}),
        json!({"authorizingInput": 0, "covenantId": "00".repeat(32), "extra": 0}),
    ] {
        assert_strict_rejects(base.clone(), |root| {
            root[0]["outputs"][0]["covenantBinding"] = value;
        });
    }

    assert_strict_rejects(base.clone(), |root| {
        root[0]["inputs"][0]["utxoEntry"]["amount"] = Value::Null;
    });
    assert_strict_rejects(base.clone(), |root| {
        root[0]["inputs"][0]["utxoEntry"]["scriptPublicKey"] = json!("0");
    });
    assert_strict_rejects(base.clone(), |root| {
        root[0]["inputs"][0]["utxoEntry"]["covenantId"] = json!("00");
    });
    assert_strict_rejects(base.clone(), |root| {
        root[0]["inputs"][0]["previousOutpoint"]["transactionId"] = json!("00");
    });
    assert_strict_rejects(base, |root| {
        root[0]["inputs"][0]["previousOutpoint"]["index"] = json!(u64::from(u32::MAX) + 1);
    });
}
