use super::*;
use serde_json::{json, Map, Value};

#[test]
fn outer_hex_format_and_magic_cover_each_boundary() {
    assert!(validate_outer_hex("").is_ok());
    assert!(validate_outer_hex("0").is_err());
    assert!(validate_outer_hex("AA").is_err());
    assert!(validate_outer_hex("zz").is_err());

    assert!(matches!(decoded_format("50534b42"), Ok(PsktFormat::Pskb)));
    assert!(matches!(
        decoded_format("50534b54"),
        Ok(PsktFormat::PsktSingle)
    ));
    assert!(matches!(
        decoded_format("00000000"),
        Err(PsktWireError::UnknownFormat)
    ));

    assert!(matches!(
        validate_wire_magic(&[], PsktFormat::Pskb),
        Err(PsktWireError::TooShort)
    ));
    assert!(validate_wire_magic(PSKB_MAGIC, PsktFormat::Pskb).is_ok());
    assert!(validate_wire_magic(PSKT_MAGIC, PsktFormat::PsktSingle).is_ok());
    assert!(matches!(
        validate_wire_magic(PSKT_MAGIC, PsktFormat::Pskb),
        Err(PsktWireError::MagicMismatch)
    ));
    assert!(matches!(
        validate_wire_magic(PSKB_MAGIC, PsktFormat::Unknown),
        Err(PsktWireError::UnknownFormat)
    ));
}

#[test]
fn consumer_document_rejects_each_wrong_shape() {
    let object = json!({"global": {}, "inputs": [], "outputs": []});
    assert!(consumer_document(PsktFormat::PsktSingle, &object).is_ok());
    assert!(consumer_document(PsktFormat::PsktSingle, &json!([])).is_err());
    assert!(consumer_document(PsktFormat::Pskb, &json!({})).is_err());
    assert!(consumer_document(PsktFormat::Pskb, &json!([])).is_err());
    assert!(consumer_document(PsktFormat::Pskb, &json!([{}, {}])).is_err());
    assert!(consumer_document(PsktFormat::Pskb, &json!([1])).is_err());
    assert!(consumer_document(PsktFormat::Pskb, &json!([object])).is_ok());
    assert!(consumer_document(PsktFormat::Unknown, &json!({})).is_err());
}

#[test]
fn exact_scalar_helpers_cover_absent_null_type_and_value_paths() {
    let mut object = Map::new();
    assert_eq!(optional_exact_u64(&object, "n", "n").unwrap(), None);
    object.insert("n".into(), Value::Null);
    assert_eq!(optional_exact_u64(&object, "n", "n").unwrap(), None);
    object.insert("n".into(), json!(7));
    assert_eq!(optional_exact_u64(&object, "n", "n").unwrap(), Some(7));
    object.insert("n".into(), json!("7"));
    assert_eq!(optional_exact_u64(&object, "n", "n").unwrap(), Some(7));
    object.insert("n".into(), json!("07"));
    assert!(optional_exact_u64(&object, "n", "n").is_err());
    object.insert("n".into(), json!(true));
    assert!(optional_exact_u64(&object, "n", "n").is_err());

    object.clear();
    assert_eq!(optional_bool(&object, "b", "b").unwrap(), None);
    object.insert("b".into(), Value::Null);
    assert!(optional_bool(&object, "b", "b").is_err());
    object.insert("b".into(), json!(true));
    assert_eq!(optional_bool(&object, "b", "b").unwrap(), Some(true));
    object.insert("b".into(), json!(false));
    assert_eq!(optional_bool(&object, "b", "b").unwrap(), Some(false));
    object.insert("b".into(), json!(0));
    assert!(optional_bool(&object, "b", "b").is_err());

    object.clear();
    assert!(optional_null_or_string(&object, "s", "s").is_ok());
    object.insert("s".into(), Value::Null);
    assert!(optional_null_or_string(&object, "s", "s").is_ok());
    object.insert("s".into(), json!("x"));
    assert!(optional_null_or_string(&object, "s", "s").is_ok());
    object.insert("s".into(), json!(1));
    assert!(optional_null_or_string(&object, "s", "s").is_err());
}

#[test]
fn object_and_hex_helpers_cover_canonical_boundaries() {
    let mut object = Map::new();
    assert!(optional_object_strict(&object, "o", "o").unwrap().is_none());
    object.insert("o".into(), Value::Null);
    assert!(optional_object_strict(&object, "o", "o").is_err());
    object.insert("o".into(), json!({"x": 1}));
    assert!(optional_object_strict(&object, "o", "o").unwrap().is_some());
    object.insert("o".into(), json!([]));
    assert!(optional_object_strict(&object, "o", "o").is_err());

    object.clear();
    assert!(optional_object_no_null(&object, "o", "o")
        .unwrap()
        .is_none());
    object.insert("o".into(), Value::Null);
    assert!(optional_object_no_null(&object, "o", "o").is_err());
    object.insert("o".into(), json!({}));
    assert!(optional_object_no_null(&object, "o", "o")
        .unwrap()
        .is_some());

    assert!(validate_exact_hex("00ff", "h", 2).is_ok());
    assert!(validate_exact_hex("00", "h", 2).is_err());
    assert!(validate_exact_hex("00FF", "h", 2).is_err());
    assert!(validate_canonical_hex("", "h", 2).is_ok());
    assert!(validate_canonical_hex("00ff", "h", 2).is_ok());
    assert!(validate_canonical_hex("000000", "h", 2).is_err());
    assert!(validate_canonical_hex("0", "h", 2).is_err());
    assert!(validate_canonical_hex("GG", "h", 2).is_err());
}

#[test]
fn pskt_shape_and_error_styles_cover_standard_review_and_unknown_paths() {
    let object = json!({});
    assert!(
        validate_single_pskt(&object, PsktFormat::PsktSingle, PskbShapeStyle::Standard).is_ok()
    );
    assert!(validate_single_pskt(&object, PsktFormat::Unknown, PskbShapeStyle::Standard).is_err());
    assert!(validate_single_pskt(&object, PsktFormat::Pskb, PskbShapeStyle::Standard).is_err());
    assert!(validate_single_pskt(&object, PsktFormat::Pskb, PskbShapeStyle::Review).is_err());
    assert!(validate_single_pskt(&json!([]), PsktFormat::Pskb, PskbShapeStyle::Standard).is_err());
    assert!(
        validate_single_pskt(&json!([{}, {}]), PsktFormat::Pskb, PskbShapeStyle::Review).is_err()
    );
    assert!(validate_single_pskt(&json!([{}]), PsktFormat::Pskb, PskbShapeStyle::Standard).is_ok());

    assert_eq!(
        format_wire_error(PsktWireError::UnknownFormat, ErrorStyle::Standard),
        "Not a PSKT/PSKB payload"
    );
    assert!(
        format_wire_error(PsktWireError::OuterHex("bad".into()), ErrorStyle::Standard)
            .starts_with("outer hex:")
    );
    assert!(
        format_wire_error(PsktWireError::OuterHex("bad".into()), ErrorStyle::Review)
            .starts_with("Bad outer hex:")
    );
    assert_eq!(
        format_wire_error(PsktWireError::TooShort, ErrorStyle::Standard),
        "payload too short"
    );
    assert_eq!(
        format_wire_error(PsktWireError::TooShort, ErrorStyle::Review),
        "Payload too short"
    );
    assert_eq!(
        format_wire_error(PsktWireError::MagicMismatch, ErrorStyle::Review),
        "wire magic does not match detected format"
    );
    assert!(
        format_wire_error(PsktWireError::InnerHex("bad".into()), ErrorStyle::Standard)
            .starts_with("inner hex:")
    );
    assert!(
        format_wire_error(PsktWireError::InnerHex("bad".into()), ErrorStyle::Review)
            .starts_with("Bad inner hex:")
    );
    assert!(
        format_wire_error(PsktWireError::Json("bad".into()), ErrorStyle::Standard)
            .starts_with("JSON parse:")
    );
}

#[test]
fn signer_count_script_and_covenant_binding_helpers_cover_fail_closed_boundaries() {
    let limits = kaskold_protocol::SIGNER_CAPABILITIES;
    assert!(validate_sig_op_count(&Value::Null, 0, limits).is_err());
    assert!(validate_sig_op_count(&json!(0), 0, limits).is_ok());
    assert!(validate_sig_op_count(&json!(limits.max_signatures_per_input), 0, limits).is_ok());
    assert!(validate_sig_op_count(
        &json!(u64::from(limits.max_signatures_per_input) + 1),
        0,
        limits
    )
    .is_err());

    assert!(validate_minimum_signatures(&Value::Null, 0, limits).is_err());
    assert!(validate_minimum_signatures(&json!(0), 0, limits).is_err());
    assert!(validate_minimum_signatures(&json!(1), 0, limits).is_ok());
    assert!(
        validate_minimum_signatures(&json!(limits.max_signatures_per_input), 0, limits).is_ok()
    );
    assert!(validate_minimum_signatures(
        &json!(u64::from(limits.max_signatures_per_input) + 1),
        0,
        limits
    )
    .is_err());

    assert!(validate_script_public_key("0000", "spk", 0).is_ok());
    assert!(validate_script_public_key("00", "spk", 32).is_err());
    assert!(validate_script_public_key("000G", "spk", 32).is_err());
    assert!(validate_script_public_key(&format!("0000{}", "11".repeat(33)), "spk", 32).is_err());

    assert!(validate_output_covenant_binding(None, 0, 1).is_ok());
    assert!(validate_output_covenant_binding(Some(&Value::Null), 0, 1).is_ok());
    assert!(validate_output_covenant_binding(Some(&json!(1)), 0, 1).is_err());
    assert!(validate_output_covenant_binding(Some(&json!({})), 0, 1).is_err());
    assert!(validate_output_covenant_binding(
        Some(&json!({
            "authorizingInput": 1,
            "covenantId": "11".repeat(32)
        })),
        0,
        1
    )
    .is_err());
    assert!(validate_output_covenant_binding(
        Some(&json!({
            "authorizingInput": 0,
            "covenantId": "11"
        })),
        0,
        1
    )
    .is_err());
    assert!(validate_output_covenant_binding(
        Some(&json!({
            "authorizingInput": 0,
            "covenantId": "11".repeat(32)
        })),
        0,
        1
    )
    .is_ok());
}

fn canonical_input() -> Value {
    json!({
        "utxoEntry": {"amount": 2, "scriptPublicKey": "0000"},
        "previousOutpoint": {"transactionId": "00".repeat(32), "index": 0},
        "sighashType": 1,
        "proprietaries": {}
    })
}

#[test]
fn input_and_output_validation_cover_nested_optional_and_range_boundaries() {
    let limits = kaskold_protocol::SIGNER_CAPABILITIES;
    assert!(validate_input(&json!(1), 0, limits).is_err());
    let valid = canonical_input();
    assert!(validate_input(&valid, 0, limits).is_ok());

    for mutate in [
        ("utxoEntry", json!([])),
        ("previousOutpoint", json!([])),
        ("proprietaries", json!([])),
    ] {
        let mut input = canonical_input();
        input[mutate.0] = mutate.1;
        assert!(validate_input(&input, 0, limits).is_err());
    }

    let mut input = canonical_input();
    input["utxoEntry"]["isCoinbase"] = Value::Null;
    assert!(validate_input(&input, 0, limits).is_err());
    let mut input = canonical_input();
    input["utxoEntry"]["covenantId"] = json!("11");
    assert!(validate_input(&input, 0, limits).is_err());
    let mut input = canonical_input();
    input["previousOutpoint"]["transactionId"] = json!("11");
    assert!(validate_input(&input, 0, limits).is_err());
    let mut input = canonical_input();
    input["previousOutpoint"]["index"] = json!(u64::from(u32::MAX) + 1);
    assert!(validate_input(&input, 0, limits).is_err());
    let mut input = canonical_input();
    input["sighashType"] = json!(2);
    assert!(validate_input(&input, 0, limits).is_err());

    let valid_output = json!({"amount": 1, "scriptPublicKey": "0000", "proprietaries": {}});
    assert!(validate_output(&valid_output, 0, 1, limits).is_ok());
    assert!(validate_output(&json!(1), 0, 1, limits).is_err());
    let mut output = valid_output.clone();
    output["amount"] = Value::Null;
    assert!(validate_output(&output, 0, 1, limits).is_err());
    let mut output = valid_output.clone();
    output["scriptPublicKey"] = json!("00");
    assert!(validate_output(&output, 0, 1, limits).is_err());
    let mut output = valid_output.clone();
    output["redeemScript"] = json!(1);
    assert!(validate_output(&output, 0, 1, limits).is_err());
    let mut output = valid_output.clone();
    output["proprietaries"] = json!([]);
    assert!(validate_output(&output, 0, 1, limits).is_err());
}

#[test]
fn global_validation_covers_counts_payload_routes_and_every_supported_covenant_branch() {
    let limits = kaskold_protocol::SIGNER_CAPABILITIES;
    let base = || {
        let mut global = Map::new();
        global.insert(
            "version".into(),
            json!(kaskold_protocol::wire::pskt_schema::PSKT_VERSION),
        );
        global.insert("txVersion".into(), json!(0));
        global.insert("inputCount".into(), json!(1));
        global.insert("outputCount".into(), json!(1));
        global
    };

    assert!(validate_collection_limits(&[Value::Null], &[Value::Null], limits).is_ok());
    assert!(validate_collection_limits(
        &vec![Value::Null; usize::from(limits.max_inputs) + 1],
        &[],
        limits,
    )
    .is_err());
    assert!(validate_collection_limits(
        &[],
        &vec![Value::Null; usize::from(limits.max_outputs) + 1],
        limits,
    )
    .is_err());

    let global = base();
    assert!(validate_global_fields(&global, 1, 1, limits).is_ok());

    let mut wrong = base();
    wrong.insert(
        "version".into(),
        json!(kaskold_protocol::wire::pskt_schema::PSKT_VERSION + 1),
    );
    assert!(validate_global_fields(&wrong, 1, 1, limits).is_err());

    let mut wrong = base();
    wrong.insert("txVersion".into(), json!(u64::MAX));
    assert!(validate_global_fields(&wrong, 1, 1, limits).is_err());

    let mut wrong = base();
    wrong.insert("inputCount".into(), json!(2));
    assert!(validate_global_fields(&wrong, 1, 1, limits).is_err());
    let mut wrong = base();
    wrong.insert("outputCount".into(), json!(2));
    assert!(validate_global_fields(&wrong, 1, 1, limits).is_err());

    for key in ["fallbackLockTime", "gas"] {
        let mut global = base();
        global.insert(key.into(), json!(7));
        assert!(validate_global_fields(&global, 1, 1, limits).is_ok());
        global.insert(key.into(), json!(true));
        assert!(validate_global_fields(&global, 1, 1, limits).is_err());
    }
    for key in ["inputsModifiable", "outputsModifiable"] {
        let mut global = base();
        global.insert(key.into(), json!(true));
        assert!(validate_global_fields(&global, 1, 1, limits).is_ok());
        global.insert(key.into(), Value::Null);
        assert!(validate_global_fields(&global, 1, 1, limits).is_err());
    }
    for key in ["xpubs", "proprietaries"] {
        let mut global = base();
        global.insert(key.into(), json!({}));
        assert!(validate_global_fields(&global, 1, 1, limits).is_ok());
        global.insert(key.into(), json!([]));
        assert!(validate_global_fields(&global, 1, 1, limits).is_err());
    }

    let mut global = base();
    global.insert("id".into(), Value::Null);
    assert!(validate_global_fields(&global, 1, 1, limits).is_ok());
    global.insert("id".into(), json!(7));
    assert!(validate_global_fields(&global, 1, 1, limits).is_err());

    let mut global = base();
    global.insert("txPayload".into(), json!("00ff"));
    assert!(validate_global_fields(&global, 1, 1, limits).is_ok());
    global.insert("txPayload".into(), json!("Ff"));
    assert!(validate_global_fields(&global, 1, 1, limits).is_err());

    let mut global = base();
    global.insert("subnetworkId".into(), json!("11".repeat(20)));
    assert!(validate_global_fields(&global, 1, 1, limits).is_ok());
    global.insert("subnetworkId".into(), json!("11"));
    assert!(validate_global_fields(&global, 1, 1, limits).is_err());

    for branch in [
        "owner",
        "owner-time",
        "beneficiary",
        "savings",
        "oracle-v1-claim",
    ] {
        let mut global = base();
        global.insert("covenantBranch".into(), json!(branch));
        assert!(
            validate_global_fields(&global, 1, 1, limits).is_ok(),
            "{branch}"
        );
    }
    let mut global = base();
    global.insert("covenantBranch".into(), json!("unknown-branch"));
    assert!(validate_global_fields(&global, 1, 1, limits).is_err());
}

#[test]
fn input_signing_fields_cover_capability_shape_and_selector_boundaries() {
    let limits = kaskold_protocol::SIGNER_CAPABILITIES;
    let key = |byte: u8| format!("02{}", format!("{byte:02x}").repeat(32));
    let schnorr = || json!({"schnorr": "11".repeat(64)});

    let mut input = canonical_input();
    input["finalScriptSig"] = json!("51");
    assert!(validate_input(&input, 0, limits).is_ok());
    input["finalScriptSig"] = json!("5");
    assert!(validate_input(&input, 0, limits).is_err());

    let too_many = usize::from(limits.max_signatures_per_input) + 1;
    let mut input = canonical_input();
    input["partialSigs"] = Value::Object(
        (0..too_many)
            .map(|i| (key(i as u8 + 1), schnorr()))
            .collect(),
    );
    assert!(validate_input(&input, 0, limits)
        .unwrap_err()
        .contains("exceeds signer capability"));
    let mut input = canonical_input();
    input["bip32Derivations"] = Value::Object(
        (0..too_many)
            .map(|i| (key(i as u8 + 1), Value::Null))
            .collect(),
    );
    assert!(validate_input(&input, 0, limits)
        .unwrap_err()
        .contains("exceeds signer capability"));

    assert!(validate_partial_signature(&key(1), &schnorr(), 0).is_ok());
    assert!(validate_partial_signature(&key(1), &json!({}), 0).is_err());
    assert!(validate_partial_signature(
        &key(1),
        &json!({"schnorr": "11".repeat(64), "extra": 1}),
        0
    )
    .is_err());
    assert!(validate_derivation_entry(&key(1), &Value::Null, 0).is_ok());
    assert!(validate_derivation_entry(&key(1), &json!({}), 0).is_ok());
    assert!(validate_derivation_entry(&key(1), &json!(1), 0)
        .unwrap_err()
        .contains("object or null"));

    let execution = |value: Value| value.as_object().cloned().expect("object");
    assert!(validate_covenant_execution_shape(
        &execution(json!({"suppliedMask": 1, "suppliedTrueMask": 1})),
        0
    )
    .is_ok());
    for bad_shape in [
        json!({"suppliedMask": 1}),
        json!({"suppliedMask": 1, "other": 1}),
        json!({"suppliedMask": 1, "suppliedTrueMask": 1, "x": 1}),
    ] {
        assert!(validate_covenant_execution_shape(&execution(bad_shape), 0).is_err());
    }
    assert!(validate_covenant_execution_values(
        &execution(json!({"suppliedMask": 3, "suppliedTrueMask": 1})),
        0
    )
    .is_ok());
    for bad_values in [
        json!({"suppliedMask": 65_536, "suppliedTrueMask": 0}),
        json!({"suppliedMask": 1, "suppliedTrueMask": 65_536}),
        json!({"suppliedMask": 1, "suppliedTrueMask": 2}),
    ] {
        assert!(validate_covenant_execution_values(&execution(bad_values), 0).is_err());
    }
}

#[test]
fn output_binding_and_field_helpers_cover_remaining_rejections() {
    let limits = kaskold_protocol::SIGNER_CAPABILITIES;
    let output = |extra: Value| {
        let mut value = json!({"amount": 1, "scriptPublicKey": "0000", "proprietaries": {}});
        for (key, field) in extra.as_object().unwrap() {
            value[key] = field.clone();
        }
        value
    };
    assert!(validate_output(&output(json!({"redeemScript": "51"})), 0, 1, limits).is_ok());
    assert!(validate_output(&output(json!({"redeemScript": "5"})), 0, 1, limits).is_err());
    let binding = |value: Value| output(json!({"covenantBinding": value}));
    assert!(validate_output(&binding(json!({"authorizingInput": 0})), 0, 1, limits).is_err());
    assert!(validate_output(
        &binding(json!({"covenantId": "11".repeat(32)})),
        0,
        1,
        limits
    )
    .is_err());
    assert!(validate_output(
        &binding(json!({"authorizingInput": 1, "covenantId": "11".repeat(32)})),
        0,
        1,
        limits
    )
    .is_err());
    assert!(validate_output(
        &binding(json!({"authorizingInput": 0, "covenantId": "11".repeat(32)})),
        0,
        1,
        limits
    )
    .is_ok());

    assert!(validate_compressed_pubkey(&format!("04{}", "11".repeat(32)), "key").is_err());
    let oversized = "00".repeat(usize::from(limits.max_script_bytes) + 3);
    assert!(validate_output(&output(json!({"scriptPublicKey": oversized})), 0, 1, limits).is_err());
}
