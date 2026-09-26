use serde_json::{json, Value};

use crate::pskt::test_support::{
    encode, finalize_json_unverified_for_test, sighash_all_for_pskt, Format,
};
use crate::{finalize_json, pskt_verified_signature_counts, Network};
use k256::schnorr::SigningKey;

#[test]
fn generic_finalizer_covers_p2pk_globals_defaults_and_explicit_covenants() {
    let mut document = unsigned_document(None, 0x11);
    document["global"] = json!({
        "version": 0,
        "txVersion": 1,
        "inputCount": 1,
        "outputCount": 1,
        "fallbackLockTime": "42",
        "subnetworkId": "ab".repeat(20),
        "gas": "7",
        "txPayload": "cafe"
    });
    document["inputs"][0]["sequence"] = json!("9");
    document["inputs"][0]["sigOpCount"] = json!(2);
    document["outputs"][0]["covenantBinding"] = json!({
        "authorizingInput": 0,
        "covenantId": "cd".repeat(32)
    });
    let document = sign_document(document, &[0x11]);
    let finalized: Value =
        serde_json::from_str(&finalize_json(&pskb(document)).expect("finalize P2PK"))
            .expect("final JSON");
    assert_eq!(finalized["version"], 1);
    assert_eq!(finalized["lockTime"], "42");
    assert_eq!(finalized["gas"], "7");
    assert_eq!(finalized["payload"], "cafe");
    assert_eq!(finalized["inputs"][0]["sequence"], "9");
    assert_eq!(finalized["outputs"][0]["covenant"]["authorizingInput"], 0);

    let mut defaults = unsigned_document(None, 0x11);
    defaults["global"] = json!({"version": 0, "txVersion": 0, "inputCount": 1, "outputCount": 1});
    defaults["inputs"][0]
        .as_object_mut()
        .unwrap()
        .remove("sequence");
    defaults["inputs"][0]
        .as_object_mut()
        .unwrap()
        .remove("sigOpCount");
    let defaults = sign_document(defaults, &[0x11]);
    let value: Value = serde_json::from_str(&finalize_json(&pskb(defaults)).expect("defaults"))
        .expect("default JSON");
    assert_eq!(value["lockTime"], "0");
    assert_eq!(value["gas"], "0");
    assert_eq!(value["inputs"][0]["sequence"], "0");
    assert_eq!(value["inputs"][0]["sigOpCount"], 1);
    assert!(value["outputs"][0]["covenant"].is_null());
}

#[test]
fn generic_finalizer_multisig_covers_reachable_push_encodings_with_five_key_cap() {
    for (keys, threshold, marker) in [
        (vec![0x11], 1, 36u16),
        (vec![0x11, 0x22, 0x33], 2, 0x4cu16),
        ((1u8..=5).collect::<Vec<_>>(), 2, 0x4cu16),
    ] {
        let redeem = multisig(&keys, threshold);
        let document = sign_document(
            unsigned_document(Some(&redeem), keys[0]),
            &keys[..usize::from(threshold)],
        );
        let finalized: Value =
            serde_json::from_str(&finalize_json(&pskb(document)).expect("multisig finalize"))
                .expect("multisig JSON");
        let script =
            hex::decode(finalized["inputs"][0]["signatureScript"].as_str().unwrap()).unwrap();
        if marker == 36 {
            assert!(script.ends_with(&redeem));
            assert_eq!(script[script.len() - redeem.len() - 1], redeem.len() as u8);
        } else {
            assert!(script.ends_with(&redeem));
            assert!(script[..script.len() - redeem.len()].contains(&(marker as u8)));
        }
    }

    let redeem = multisig(&[0x11, 0x22], 2);
    let insufficient = sign_document(unsigned_document(Some(&redeem), 0x11), &[0x11]);
    assert!(finalize_json(&pskb(insufficient))
        .unwrap_err()
        .message()
        .contains("PSKT is not cryptographically complete"));

    let mut extra_unknown = sign_document(unsigned_document(Some(&redeem), 0x11), &[0x11, 0x22]);
    extra_unknown["inputs"][0]["partialSigs"]
        .as_object_mut()
        .unwrap()
        .insert(
            format!("02{}", hex::encode(xonly(0x44))),
            json!({"schnorr": "77".repeat(64)}),
        );
    assert!(finalize_json(&pskb(extra_unknown)).is_err());
}

#[test]
fn verified_covenant_execution_is_preserved_across_pskt_and_kspt_materialization() {
    let owner = 0x31;
    let beneficiary = 0x32;
    let redeem = single_selector_covenant(owner, beneficiary);
    let mut document = unsigned_document(Some(&redeem), owner);
    document["inputs"][0]["covenantExecution"] = json!({
        "suppliedMask": "1",
        "suppliedTrueMask": "1"
    });
    let signed = sign_document(document, &[owner]);
    let wire = pskb(signed.clone());

    // Standard PSKT authorization returns a typed witness plan and serializes
    // only that plan. No proprietary branch hint participates.
    let finalized: Value =
        serde_json::from_str(&finalize_json(&wire).expect("verified covenant finalize"))
            .expect("final JSON");
    let pskt_script = hex::decode(
        finalized["inputs"][0]["signatureScript"]
            .as_str()
            .expect("signature script"),
    )
    .expect("signature script hex");
    assert_eq!(pskt_script[0], 65);
    assert_eq!(pskt_script[65], 1); // SIGHASH_ALL
    assert_eq!(pskt_script[66], 0x51); // exact proven outer IF selector
    assert!(pskt_script.ends_with(&redeem));

    // KSPT remains a signing transport and preserves covenantExecution for the
    // merge check, but raw covenant KSPT broadcast is deliberately disabled:
    // specialized proofs/preimages live in the original PSKT and cannot be
    // reconstructed safely from compact KSPT alone.
    let kspt = crate::encode_pskt(&wire, Network::Mainnet).expect("encode signed KSPT");
    let covenant_execution =
        crate::pskt::test_support::compact_covenant_execution_for_test(&kspt, 0)
            .expect("parse signed KSPT");
    assert_eq!(covenant_execution, Some((1, 1)));
    let error = crate::compat::verify_complete_kspt(&kspt)
        .expect_err("raw covenant KSPT broadcast must be disabled");
    assert!(error.contains("merge the signed KSPT into its original PSKT"));

    // Branch proof mutation leaves the transaction digest/signature unchanged,
    // so this specifically proves branch binding rather than signature failure.
    let mut wrong_branch = signed.clone();
    wrong_branch["inputs"][0]["covenantExecution"]["suppliedTrueMask"] = json!("0");
    let error = finalize_json(&pskb(wrong_branch)).expect_err("inactive signer branch must fail");
    assert!(
        error.message().contains("branch")
            || error.message().contains("complete")
            || error.message().contains("signature"),
        "unexpected branch-binding error: {}",
        error.message()
    );

    // Legacy/specialized routing is fail-closed at the verified consumer
    // boundary. It cannot become a second branch authority after signing.
    let mut specialized = signed.clone();
    specialized["inputs"][0]["proprietaries"] = json!({"escrowBranch": "buyer-release"});
    let error = finalize_json(&pskb(specialized)).expect_err("dual branch routing must fail");
    assert!(
        error.message().contains("typed verified witness plan")
            || error.message().contains("covenantExecution")
    );

    let mut legacy_global = signed;
    legacy_global["global"]["covenantBranch"] = json!("beneficiary");
    let error = finalize_json(&pskb(legacy_global)).expect_err("global branch hint must fail");
    assert!(error.message().contains("covenantBranch"));
}

#[test]
fn verified_generic_covenant_signature_count_covers_execution_binding() {
    let owner = 0x35;
    let beneficiary = 0x36;
    let redeem = single_selector_covenant(owner, beneficiary);
    let mut document = unsigned_document(Some(&redeem), owner);
    document["inputs"][0]["covenantExecution"] = json!({
        "suppliedMask": "1",
        "suppliedTrueMask": "1"
    });
    let signed = sign_document(document, &[owner]);
    let wire = pskb(signed.clone());
    assert_eq!(
        pskt_verified_signature_counts(&wire, Network::Mainnet)
            .expect("verified generic covenant count"),
        vec![1],
    );

    let mut missing = signed.clone();
    missing["inputs"][0]
        .as_object_mut()
        .expect("input object")
        .remove("covenantExecution");
    let error = pskt_verified_signature_counts(&pskb(missing), Network::Mainnet)
        .expect_err("generic covenant without execution must fail");
    assert!(error.message().contains("missing covenantExecution"));

    let mut incomplete = signed;
    incomplete["inputs"][0]["covenantExecution"] =
        json!({"suppliedMask": "1", "suppliedTrueMask": "2"});
    assert!(pskt_verified_signature_counts(&pskb(incomplete), Network::Mainnet).is_err());
}

#[test]
fn generic_finalizer_reports_input_global_and_output_shape_errors_without_panics() {
    let cases = malformed_cases();
    for (document, expected) in cases {
        let error = finalize_json_unverified_for_test(&pskb(document)).unwrap_err();
        assert!(
            error.contains(expected),
            "expected {expected:?}, got {error:?}"
        );
    }
}

fn malformed_cases() -> Vec<(Value, &'static str)> {
    let mut cases = Vec::new();
    cases.push((json!(null), "PSKT not object"));
    cases.push((json!({"inputs": [], "outputs": []}), "missing global"));
    cases.push((json!({"global": {}, "outputs": []}), "missing inputs"));
    cases.push((json!({"global": {}, "inputs": []}), "missing outputs"));

    for (mutation, expected) in [
        (0u8, "missing txVersion"),
        (1, "txVersion exceeds u16"),
        (2, "fallbackLockTime"),
        (3, "subnetworkId must be 20 bytes"),
        (4, "subnetworkId must be even-length lowercase hexadecimal"),
        (5, "subnetworkId must be a hex string"),
        (6, "gas must be a decimal string"),
        (7, "txPayload must be even-length lowercase hexadecimal"),
        (8, "txPayload must be a hex string"),
    ] {
        let mut doc = placeholder_signature_document(None, 0x11);
        match mutation {
            0 => {
                doc["global"].as_object_mut().unwrap().remove("txVersion");
            }
            1 => doc["global"]["txVersion"] = json!(65_536),
            2 => doc["global"]["fallbackLockTime"] = json!("01"),
            3 => doc["global"]["subnetworkId"] = json!("00"),
            4 => doc["global"]["subnetworkId"] = json!("zz"),
            5 => doc["global"]["subnetworkId"] = json!(true),
            6 => doc["global"]["gas"] = json!(true),
            7 => doc["global"]["txPayload"] = json!("z"),
            8 => doc["global"]["txPayload"] = json!(true),
            _ => unreachable!(),
        }
        cases.push((doc, expected));
    }

    for (mutation, expected) in [
        (0u8, "input[0]: not object"),
        (
            1,
            "generic KasKold SDK finalization does not own covenant execution policy",
        ),
        (2, "missing previousOutpoint"),
        (3, "missing transactionId"),
        (4, "transactionId must be even-length lowercase hexadecimal"),
        (5, "tx_id not 32 bytes"),
        (6, "missing index"),
        (7, "index exceeds u32"),
        (8, "sequence"),
        (9, "sigOpCount exceeds u8"),
        (10, "missing scriptPublicKey"),
        (11, "signed input has no partialSigs"),
        (12, "partial sig missing schnorr variant"),
        (13, "signature must be even-length lowercase hexadecimal"),
        (14, "Schnorr signature must be 64 bytes"),
    ] {
        let mut doc = placeholder_signature_document(None, 0x11);
        match mutation {
            0 => doc["inputs"][0] = Value::Null,
            1 => doc["inputs"][0]["proprietaries"] = json!({"persistentVault": true}),
            2 => {
                doc["inputs"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("previousOutpoint");
            }
            3 => {
                doc["inputs"][0]["previousOutpoint"]
                    .as_object_mut()
                    .unwrap()
                    .remove("transactionId");
            }
            4 => doc["inputs"][0]["previousOutpoint"]["transactionId"] = json!("zz"),
            5 => doc["inputs"][0]["previousOutpoint"]["transactionId"] = json!("00"),
            6 => {
                doc["inputs"][0]["previousOutpoint"]
                    .as_object_mut()
                    .unwrap()
                    .remove("index");
            }
            7 => doc["inputs"][0]["previousOutpoint"]["index"] = json!(u64::from(u32::MAX) + 1),
            8 => doc["inputs"][0]["sequence"] = json!("01"),
            9 => doc["inputs"][0]["sigOpCount"] = json!(256),
            10 => {
                doc["inputs"][0]["utxoEntry"]
                    .as_object_mut()
                    .unwrap()
                    .remove("scriptPublicKey");
            }
            11 => {
                doc["inputs"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("partialSigs");
            }
            12 => {
                doc["inputs"][0]["partialSigs"] = malformed_p2pk_partial(json!({"ecdsa": "00"}));
            }
            13 => {
                doc["inputs"][0]["partialSigs"] = malformed_p2pk_partial(json!({"schnorr": "zz"}));
            }
            14 => {
                doc["inputs"][0]["partialSigs"] = malformed_p2pk_partial(json!({"schnorr": "00"}));
            }
            _ => unreachable!(),
        }
        cases.push((doc, expected));
    }

    let redeem = multisig(&[0x11], 1);
    for (mutation, expected) in [
        (0u8, "P2SH input without redeem script"),
        (1, "redeemScript must be even-length lowercase hexadecimal"),
        (2, "host wallet must finalize specialized redeem scripts"),
        (3, "output[0]: not object"),
        (4, "missing amount"),
        (5, "amount must be a decimal string"),
        (6, "missing scriptPublicKey"),
        (7, "scriptPublicKey must be ASCII hex and at least 4 bytes"),
        (8, "covenantBinding not object"),
        (9, "missing authorizingInput"),
        (10, "authorizingInput exceeds u16"),
        (11, "missing covenantId"),
        (12, "covenantId must be even-length lowercase hexadecimal"),
        (13, "covenantId must be 32 bytes"),
    ] {
        let mut doc = placeholder_signature_document(Some(&redeem), 0x11);
        match mutation {
            0 => {
                doc["inputs"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("redeemScript");
            }
            1 => doc["inputs"][0]["redeemScript"] = json!("zz"),
            2 => doc["inputs"][0]["redeemScript"] = json!("51ae"),
            3 => doc["outputs"][0] = Value::Null,
            4 => {
                doc["outputs"][0].as_object_mut().unwrap().remove("amount");
            }
            5 => doc["outputs"][0]["amount"] = json!(true),
            6 => {
                doc["outputs"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("scriptPublicKey");
            }
            7 => doc["outputs"][0]["scriptPublicKey"] = json!("00"),
            8 => doc["outputs"][0]["covenantBinding"] = json!([]),
            9 => doc["outputs"][0]["covenantBinding"] = json!({}),
            10 => {
                doc["outputs"][0]["covenantBinding"] =
                    json!({"authorizingInput": 65_536, "covenantId": "00".repeat(32)})
            }
            11 => doc["outputs"][0]["covenantBinding"] = json!({"authorizingInput": 0}),
            12 => {
                doc["outputs"][0]["covenantBinding"] =
                    json!({"authorizingInput": 0, "covenantId": "zz"})
            }
            13 => {
                doc["outputs"][0]["covenantBinding"] =
                    json!({"authorizingInput": 0, "covenantId": "00"})
            }
            _ => unreachable!(),
        }
        cases.push((doc, expected));
    }
    cases
}

fn xonly(marker: u8) -> [u8; 32] {
    let signing = SigningKey::from_bytes(&[marker; 32]).expect("test signing key");
    signing.verifying_key().to_bytes().into()
}

fn unsigned_document(redeem: Option<&[u8]>, signer: u8) -> Value {
    let input_script = if let Some(redeem) = redeem {
        let hash = blake2b_simd::Params::new().hash_length(32).hash(redeem);
        format!("0000aa20{}87", hex::encode(hash.as_bytes()))
    } else {
        format!("000020{}ac", hex::encode(xonly(signer)))
    };
    json!({
        "global": {
            "version": 0,
            "txVersion": 0,
            "inputCount": 1,
            "outputCount": 1,
            "fallbackLockTime": "0",
            "subnetworkId": "00".repeat(20),
            "gas": "0",
            "txPayload": ""
        },
        "inputs": [{
            "previousOutpoint": {"transactionId": "22".repeat(32), "index": 7},
            "utxoEntry": {"amount": "100000", "scriptPublicKey": input_script},
            "sequence": "0",
            "sigOpCount": 1,
            "sighashType": 1,
            "redeemScript": redeem.map(hex::encode),
            "partialSigs": {},
            "proprietaries": {}
        }],
        "outputs": [{
            "amount": "90000",
            "scriptPublicKey": "000051",
            "covenantBinding": null
        }]
    })
}

fn sign_document(mut document: Value, signers: &[u8]) -> Value {
    let wire = pskb(document.clone());
    let digest = sighash_all_for_pskt(&wire, Network::Mainnet, 0).expect("test sighash");
    let partials = document["inputs"][0]["partialSigs"]
        .as_object_mut()
        .expect("partialSigs object");
    for marker in signers {
        let signing = SigningKey::from_bytes(&[*marker; 32]).expect("test signing key");
        let signature = signing
            .sign_raw(&digest, &[0u8; 32])
            .expect("test signature");
        partials.insert(
            format!("02{}", hex::encode(signing.verifying_key().to_bytes())),
            json!({"schnorr": hex::encode(signature.to_bytes())}),
        );
    }
    document
}

fn malformed_p2pk_partial(value: Value) -> Value {
    let mut partials = serde_json::Map::new();
    // Match placeholder_signature_document's real x-only P2PK script so the
    // malformed value reaches the signature-shape validation being tested.
    partials.insert(format!("02{}", hex::encode(xonly(0x11))), value);
    Value::Object(partials)
}

fn placeholder_signature_document(redeem: Option<&[u8]>, signer: u8) -> Value {
    let mut document = unsigned_document(redeem, signer);
    document["inputs"][0]["partialSigs"] = placeholder_signatures(&[signer]);
    document
}

fn placeholder_signatures(keys: &[u8]) -> Value {
    let mut map = serde_json::Map::new();
    for key in keys {
        map.insert(
            format!("02{}", hex::encode(xonly(*key))),
            json!({"schnorr": "77".repeat(64)}),
        );
    }
    Value::Object(map)
}

fn single_selector_covenant(owner: u8, beneficiary: u8) -> Vec<u8> {
    let mut script = vec![0x63, 0x20]; // OP_IF, PUSH32 owner
    script.extend_from_slice(&xonly(owner));
    script.extend_from_slice(&[0xac, 0x67, 0x20]); // CHECKSIG, ELSE, PUSH32 beneficiary
    script.extend_from_slice(&xonly(beneficiary));
    script.extend_from_slice(&[0xac, 0x68]); // CHECKSIG, ENDIF
    script
}

fn multisig(keys: &[u8], threshold: u8) -> Vec<u8> {
    let mut script = vec![0x50 + threshold];
    for key in keys {
        script.push(0x20);
        script.extend_from_slice(&xonly(*key));
    }
    script.push(0x50 + u8::try_from(keys.len()).unwrap());
    script.push(0xae);
    script
}

fn pskb(document: Value) -> String {
    encode(Format::Pskb, &json!([document])).expect("encode finalizer PSKB")
}

#[test]
fn malformed_finalizer_fixture_baselines_are_valid_before_single_field_mutations() {
    let p2pk = placeholder_signature_document(None, 0x11);
    finalize_json_unverified_for_test(&pskb(p2pk))
        .expect("P2PK malformed-case baseline must be structurally valid");

    let redeem = multisig(&[0x11], 1);
    let p2sh = placeholder_signature_document(Some(&redeem), 0x11);
    finalize_json_unverified_for_test(&pskb(p2sh))
        .expect("P2SH malformed-case baseline must be structurally valid");
}

#[test]
fn supported_specialized_covenants_are_template_bound_and_mutation_hardened_end_to_end() {
    // Private swap claim: branch selector, route metadata, signature, redeem
    // template and signed transaction fields are independently bound.
    let private = private_swap_claim_document();
    assert_specialized_success(&private, "private swap");
    assert_selector_mutation_fails(&private, "private swap");
    let mut private_route = private.clone();
    private_route["inputs"][0]["proprietaries"]["privateSwapClaim"] = json!(false);
    assert_specialized_failure(&private_route, "private swap route metadata mutation");
    let mut private_template = private.clone();
    mutate_redeem_byte(&mut private_template, 0);
    assert_specialized_failure(&private_template, "private swap redeem-template mutation");
    assert_signature_mutation_fails(&private, "private swap");
    assert_signed_transaction_mutation_fails(&private, "private swap");

    // Oracle-v1 claim: the external oracle attestation is cryptographically
    // checked against the key and commitment frozen into the recognized script.
    let oracle = oracle_v1_claim_document();
    assert_specialized_success(&oracle, "oracle-v1");
    assert_selector_mutation_fails(&oracle, "oracle-v1");
    let mut oracle_route = oracle.clone();
    oracle_route["inputs"][0]["proprietaries"]["oracleV1Claim"] = json!(false);
    assert_specialized_failure(&oracle_route, "oracle-v1 route metadata mutation");
    let mut oracle_proof = oracle.clone();
    mutate_hex_field(
        &mut oracle_proof["inputs"][0]["proprietaries"]["oracleV1Signature"],
        0,
    );
    assert_specialized_failure(&oracle_proof, "oracle-v1 attestation mutation");
    assert_redeem_template_mutation_fails(&oracle, "oracle-v1");
    assert_signature_mutation_fails(&oracle, "oracle-v1");
    assert_signed_transaction_mutation_fails(&oracle, "oracle-v1");

    // Commit/reveal claim: the supplied preimage is checked against the exact
    // commitment embedded in the recognized redeem script.
    let commit = commit_reveal_claim_document();
    assert_specialized_success(&commit, "commit/reveal");
    assert_selector_mutation_fails(&commit, "commit/reveal");
    let mut commit_route = commit.clone();
    commit_route["inputs"][0]["proprietaries"]["commitPartB"] = json!("ff");
    assert_specialized_failure(&commit_route, "commit/reveal preimage mutation");
    let mut commit_conflict = commit.clone();
    commit_conflict["inputs"][0]["proprietaries"]["privateSwapClaim"] = json!(true);
    assert_specialized_failure(&commit_conflict, "commit/reveal routing conflict");
    assert_redeem_template_mutation_fails(&commit, "commit/reveal");
    assert_signature_mutation_fails(&commit, "commit/reveal");
    assert_signed_transaction_mutation_fails(&commit, "commit/reveal");

    // Merkle claim: proof sibling, direction and covenantExecution selector bits
    // all describe one branch and must agree with the root frozen in the script.
    let merkle = merkle_claim_document();
    assert_specialized_success(&merkle, "merkle");
    let mut merkle_selector = merkle.clone();
    merkle_selector["inputs"][0]["covenantExecution"]["suppliedTrueMask"] = json!("2");
    assert_specialized_failure(&merkle_selector, "merkle selector mutation");
    let mut merkle_proof = merkle.clone();
    merkle_proof["inputs"][0]["proprietaries"]["merkleProof"][0]["direction"] = json!(1);
    assert_specialized_failure(&merkle_proof, "merkle direction mutation");
    let mut merkle_sibling = merkle.clone();
    mutate_hex_field(
        &mut merkle_sibling["inputs"][0]["proprietaries"]["merkleProof"][0]["sibling"],
        0,
    );
    assert_specialized_failure(&merkle_sibling, "merkle proof mutation");
    assert_redeem_template_mutation_fails(&merkle, "merkle");
    assert_signature_mutation_fails(&merkle, "merkle");
    assert_signed_transaction_mutation_fails(&merkle, "merkle");
}

#[test]
fn unsupported_and_zero_signature_covenant_routes_are_disabled_at_public_finalization() {
    let redeem = single_selector_covenant(0x31, 0x32);
    let mut zero = unsigned_document(Some(&redeem), 0x31);
    zero["inputs"][0]["minimumSignatures"] = json!(0);
    zero["inputs"][0]["covenantExecution"] = json!({"suppliedMask": "1", "suppliedTrueMask": "1"});
    let error = finalize_json(&pskb(zero)).expect_err("zero-signature route must be disabled");
    assert!(
        error.message().contains("minimumSignatures"),
        "unexpected error: {}",
        error.message()
    );

    let mut unsupported = unsigned_document(Some(&redeem), 0x31);
    unsupported["inputs"][0]["covenantExecution"] =
        json!({"suppliedMask": "1", "suppliedTrueMask": "1"});
    unsupported["inputs"][0]["proprietaries"] = json!({"escrowBranch": "buyer-release"});
    let unsupported = sign_document(unsupported, &[0x31]);
    let error =
        finalize_json(&pskb(unsupported)).expect_err("untyped specialized route must be disabled");
    assert!(
        error.message().contains("typed verified witness plan")
            || error.message().contains("no typed verified witness plan"),
        "unexpected error: {}",
        error.message()
    );
}

fn assert_specialized_success(document: &Value, label: &str) {
    let finalized = finalize_json(&pskb(document.clone()))
        .unwrap_or_else(|error| panic!("{label} public finalization failed: {}", error.message()));
    let value: Value = serde_json::from_str(&finalized).expect("specialized finalized JSON");
    let script = value["inputs"][0]["signatureScript"]
        .as_str()
        .unwrap_or_else(|| panic!("{label} missing signatureScript"));
    assert!(!script.is_empty(), "{label} produced empty signatureScript");
}

fn assert_specialized_failure(document: &Value, label: &str) {
    assert!(
        finalize_json(&pskb(document.clone())).is_err(),
        "{label} unexpectedly crossed the verified public boundary"
    );
}

fn assert_selector_mutation_fails(document: &Value, label: &str) {
    let mut changed = document.clone();
    let current = changed["inputs"][0]["covenantExecution"]["suppliedTrueMask"]
        .as_str()
        .expect("selector string")
        .parse::<u16>()
        .expect("selector integer");
    changed["inputs"][0]["covenantExecution"]["suppliedTrueMask"] =
        json!((current ^ 1).to_string());
    assert_specialized_failure(&changed, &format!("{label} selector mutation"));
}

fn assert_redeem_template_mutation_fails(document: &Value, label: &str) {
    let mut changed = document.clone();
    mutate_redeem_byte(&mut changed, 0);
    assert_specialized_failure(&changed, &format!("{label} redeem-template mutation"));
}

fn assert_signature_mutation_fails(document: &Value, label: &str) {
    let mut changed = document.clone();
    let partials = changed["inputs"][0]["partialSigs"]
        .as_object_mut()
        .expect("partialSigs object");
    let signature = partials
        .values_mut()
        .next()
        .and_then(|entry| entry.get_mut("schnorr"))
        .expect("real signature fixture");
    mutate_hex_field(signature, 0);
    assert_specialized_failure(&changed, &format!("{label} signature mutation"));
}

fn assert_signed_transaction_mutation_fails(document: &Value, label: &str) {
    let mut changed = document.clone();
    changed["outputs"][0]["amount"] = json!("89999");
    assert_specialized_failure(&changed, &format!("{label} signed transaction mutation"));
}

fn mutate_hex_field(value: &mut Value, byte_index: usize) {
    let text = value.as_str().expect("hex string");
    let mut bytes = hex::decode(text).expect("fixture hex");
    bytes[byte_index] ^= 1;
    *value = Value::String(hex::encode(bytes));
}

fn mutate_redeem_byte(document: &mut Value, byte_index: usize) {
    mutate_hex_field(&mut document["inputs"][0]["redeemScript"], byte_index);
}

fn private_swap_claim_document() -> Value {
    let claimer = 0x41;
    let owner = 0x42;
    let mut redeem = Vec::new();
    push_test_data(&mut redeem, &[0x01; 16]);
    redeem.extend_from_slice(&[0x75, 0x63]); // DROP IF
    push_test_data(&mut redeem, &xonly(claimer));
    redeem.extend_from_slice(&[0xad, 0xb3]); // CHECKSIGVERIFY TX_INPUT_COUNT
    push_test_int(&mut redeem, 1);
    redeem.extend_from_slice(&[0x9d, 0xb4]); // NUMEQUALVERIFY TX_OUTPUT_COUNT
    push_test_int(&mut redeem, 1);
    redeem.push(0x9d);
    push_test_int(&mut redeem, 0);
    redeem.push(0xc3); // TX_OUTPUT_SPK
    push_test_data(&mut redeem, &[0x00, 0x00, 0x51, 0x51, 0x51]);
    redeem.push(0x88); // EQUALVERIFY
    push_test_int(&mut redeem, 0);
    redeem.extend_from_slice(&[0xbe, 0x76]); // TX_INPUT_AMOUNT DUP
    push_test_int(&mut redeem, 0);
    redeem.extend_from_slice(&[0xc2, 0xa2, 0x69]); // TX_OUTPUT_AMOUNT >= VERIFY
    push_test_int(&mut redeem, 0);
    redeem.extend_from_slice(&[0xc2, 0x94]); // TX_OUTPUT_AMOUNT SUB
    push_test_int(&mut redeem, 500_000_000);
    redeem.extend_from_slice(&[0xa1, 0x69, 0x51, 0x67]); // <= VERIFY TRUE ELSE
    push_test_data(&mut redeem, &xonly(owner));
    redeem.push(0xad);
    push_test_int(&mut redeem, 1);
    redeem.extend_from_slice(&[0xb0, 0x51, 0x68]); // CLTV TRUE ENDIF

    let mut document = unsigned_document(Some(&redeem), claimer);
    document["inputs"][0]["covenantExecution"] =
        json!({"suppliedMask": "1", "suppliedTrueMask": "1"});
    document["inputs"][0]["proprietaries"] = json!({"privateSwapClaim": true});
    sign_document(document, &[claimer])
}

fn oracle_v1_claim_document() -> Value {
    let refund = 0x51;
    let claimer = 0x52;
    let oracle = SigningKey::from_bytes(&[0x53; 32]).expect("oracle signing key");
    let commitment = [0x5a; 32];
    let oracle_signature = oracle
        .sign_raw(&commitment, &[0u8; 32])
        .expect("oracle attestation signature");

    let mut redeem = Vec::new();
    push_test_data(&mut redeem, &[0x02; 16]);
    redeem.extend_from_slice(&[0x75, 0x63]);
    push_test_data(&mut redeem, &xonly(refund));
    redeem.push(0xad);
    push_test_int(&mut redeem, 1);
    redeem.extend_from_slice(&[0xb0, 0x51, 0x67]);
    push_test_data(&mut redeem, &xonly(claimer));
    redeem.push(0xad);
    push_test_data(&mut redeem, &commitment);
    let oracle_key: [u8; 32] = oracle.verifying_key().to_bytes().into();
    push_test_data(&mut redeem, &oracle_key);
    redeem.extend_from_slice(&[0xd7, 0x69, 0x51, 0x68]);

    let mut document = unsigned_document(Some(&redeem), claimer);
    document["inputs"][0]["covenantExecution"] =
        json!({"suppliedMask": "1", "suppliedTrueMask": "0"});
    document["inputs"][0]["proprietaries"] = json!({
        "oracleV1Claim": true,
        "oracleV1Signature": hex::encode(oracle_signature.to_bytes())
    });
    sign_document(document, &[claimer])
}

fn commit_reveal_claim_document() -> Value {
    let owner = 0x61;
    let part_a = b"commit-".to_vec();
    let part_b = b"reveal".to_vec();
    let mut preimage = part_a.clone();
    preimage.extend_from_slice(&part_b);
    let commitment = test_blake2b32(&preimage);

    let mut redeem = vec![0x63]; // IF
    push_test_data(&mut redeem, &xonly(owner));
    redeem.push(0xad);
    push_test_int(&mut redeem, 1);
    redeem.extend_from_slice(&[0xb0, 0x51, 0x67]);
    push_test_data(&mut redeem, &xonly(owner));
    redeem.extend_from_slice(&[0xad, 0x7e, 0xaa]); // CHECKSIGVERIFY CAT BLAKE2B
    push_test_data(&mut redeem, &commitment);
    redeem.extend_from_slice(&[0x88, 0x51, 0x68]);

    let mut document = unsigned_document(Some(&redeem), owner);
    document["inputs"][0]["covenantExecution"] =
        json!({"suppliedMask": "1", "suppliedTrueMask": "0"});
    document["inputs"][0]["proprietaries"] = json!({
        "commitPartA": hex::encode(part_a),
        "commitPartB": hex::encode(part_b)
    });
    sign_document(document, &[owner])
}

fn merkle_claim_document() -> Value {
    let owner = 0x71;
    let destination = vec![0x00, 0x00, 0x51];
    let sibling = [0x72; 32];
    let leaf = test_blake2b32(&destination);
    let mut pair = [0u8; 64];
    pair[..32].copy_from_slice(&sibling);
    pair[32..].copy_from_slice(&leaf);
    let root = test_blake2b32(&pair);

    let mut redeem = vec![0x63];
    push_test_data(&mut redeem, &xonly(owner));
    redeem.push(0xad);
    push_test_int(&mut redeem, 1);
    redeem.extend_from_slice(&[0xb0, 0x51, 0x67]);
    push_test_data(&mut redeem, &xonly(owner));
    redeem.extend_from_slice(&[0xad, 0xaa]); // CHECKSIGVERIFY BLAKE2B
    redeem.extend_from_slice(&[0x7c, 0x63, 0x7c, 0x68, 0x7e, 0xaa]);
    push_test_data(&mut redeem, &root);
    redeem.extend_from_slice(&[0x88, 0x00, 0xc3, 0x88, 0x51, 0x68]);

    let mut document = unsigned_document(Some(&redeem), owner);
    document["inputs"][0]["covenantExecution"] =
        json!({"suppliedMask": "3", "suppliedTrueMask": "0"});
    document["inputs"][0]["proprietaries"] = json!({
        "merkleDestSpk": hex::encode(destination),
        "merkleProof": [{"sibling": hex::encode(sibling), "direction": 0}]
    });
    sign_document(document, &[owner])
}

fn push_test_data(script: &mut Vec<u8>, data: &[u8]) {
    assert!(data.len() <= 75, "test helper only emits direct pushes");
    script.push(u8::try_from(data.len()).expect("test push length"));
    script.extend_from_slice(data);
}

fn push_test_int(script: &mut Vec<u8>, value: u64) {
    match value {
        0 => script.push(0x00),
        1..=16 => script.push(0x50 + u8::try_from(value).expect("small script integer")),
        _ => {
            let mut bytes = value.to_le_bytes().to_vec();
            while bytes.last() == Some(&0) {
                bytes.pop();
            }
            if bytes.last().is_some_and(|byte| byte & 0x80 != 0) {
                bytes.push(0);
            }
            push_test_data(script, &bytes);
        }
    }
}

fn test_blake2b32(data: &[u8]) -> [u8; 32] {
    let hash = blake2b_simd::Params::new().hash_length(32).hash(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}
