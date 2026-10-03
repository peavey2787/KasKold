#[test]
fn merkle_and_script_construction_helpers_have_direct_function_coverage() {
    let first = crate::account::address::encode_p2pk_address(&[0x11; 32], "kaspa");
    let second = crate::account::address::encode_p2pk_address(&[0x22; 32], "kaspa");
    let addresses = serde_json::to_string(&vec![first.clone(), second]).expect("addresses JSON");

    let root_json = crate::contracts::merkle::application::root_from_addresses(&addresses)
        .expect("merkle root");
    let root: serde_json::Value = serde_json::from_str(&root_json).expect("root JSON");
    let root_hex = root["root"].as_str().expect("root hex");
    assert_eq!(root_hex.len(), 64);

    let proof_json = crate::contracts::merkle::application::proof_for_address(&addresses, &first)
        .expect("merkle proof");
    let proof: serde_json::Value = serde_json::from_str(&proof_json).expect("proof JSON");
    assert_eq!(proof["leaf_index"].as_u64(), Some(0));

    let whitelist = crate::contracts::merkle::application::build_whitelist_json(
        &"33".repeat(32),
        root_hex,
        1,
        100,
        "kaspa",
    )
    .expect("whitelist covenant");
    assert!(whitelist.contains("redeem_script_hex"));

    let commit = crate::contracts::commit_reveal::script::build_commit_reveal_script(
        &[0x44; 32],
        &[0x55; 32],
        77,
    );
    assert!(!commit.is_empty());

    let escrow = crate::contracts::covenant::script::build_escrow_script(
        &[0x61; 32],
        &[0x62; 32],
        &[0x63; 32],
        &[0x20; 34],
        &[0x21; 34],
        &[0x64; 8],
    );
    assert!(!escrow.is_empty());

    let timed = crate::contracts::covenant::script::build_timelocked_escrow_script(
        &[0x71; 32],
        &[0x72; 32],
        &[0x20; 34],
        &[0x21; 34],
        1234,
    );
    assert!(!timed.is_empty());
}

#[test]
fn oracle_and_zk_helpers_have_direct_function_coverage() {
    let heartbeat = crate::contracts::oracle::genesis::build_heartbeat_json("kaspa")
        .expect("heartbeat covenant");
    assert!(heartbeat.contains("redeem_script_hex"));

    let total = crate::contracts::zk::proof::serialize_total(123).expect("field serialization");
    assert!(!total.is_empty());

    assert!(crate::contracts::zk::proof::verify_proof(&[], &[], &[]).is_err());

    let versioned = crate::contracts::zk::crowdfund::versioned_spk(&[0x51, 0xac]);
    assert_eq!(versioned, vec![0, 0, 0x51, 0xac]);

    assert!(crate::contracts::covenant::oracle_v1::verify_attestation("00", "00", "00").is_err());
}

#[test]
fn merkle_application_rejects_malformed_whitelists_proofs_and_roots() {
    use crate::contracts::merkle::application::{
        build_whitelist_json, proof_for_address, root_from_addresses,
    };

    let member = crate::account::address::encode_p2pk_address(&[0x11; 32], "kaspa");
    let outsider = crate::account::address::encode_p2pk_address(&[0x99; 32], "kaspa");
    let addresses = serde_json::to_string(&vec![member.clone()]).expect("addresses JSON");

    assert!(root_from_addresses("not-json")
        .unwrap_err()
        .starts_with("Bad JSON"));
    assert!(root_from_addresses(r#"["not-an-address"]"#).is_err());
    assert!(proof_for_address("not-json", &member)
        .unwrap_err()
        .starts_with("Bad JSON"));
    assert!(proof_for_address(r#"["not-an-address"]"#, &member).is_err());
    assert!(proof_for_address(&addresses, "not-an-address").is_err());
    assert_eq!(
        proof_for_address(&addresses, &outsider),
        Err("Address not found in whitelist".to_string())
    );

    let owner = "33".repeat(32);
    assert!(build_whitelist_json("zz", &"44".repeat(32), 1, 0, "kaspa").is_err());
    assert!(build_whitelist_json(&owner, "zz", 1, 0, "kaspa")
        .unwrap_err()
        .starts_with("Bad root hex"));
    assert_eq!(
        build_whitelist_json(&owner, &"44".repeat(31), 1, 0, "kaspa"),
        Err("Merkle root must be 32 bytes, got 31".to_string())
    );
}

#[test]
fn oracle_v1_attestation_and_keys_reject_malformed_hex_and_points() {
    use crate::contracts::covenant::oracle_v1::{
        build_json_with_salt, checked_redeem_and_attestation, decode_attestation,
    };

    let key = |seed: u8| crate::wasm_api::test_support::xonly_key(seed);
    let (owner, beneficiary, oracle) = (key(1), key(2), key(3));
    let not_on_curve = "ff".repeat(32);

    assert_eq!(
        decode_attestation(&not_on_curve, &"11".repeat(64), &"22".repeat(32)).unwrap_err(),
        "Oracle oracle key is not a valid secp256k1 x-only public key"
    );
    assert!(decode_attestation(&oracle, "zz", &"22".repeat(32))
        .unwrap_err()
        .starts_with("Bad oracle signature hex"));
    assert!(decode_attestation(&oracle, &"11".repeat(64), "zz")
        .unwrap_err()
        .starts_with("Bad message commitment hex"));
    assert!(
        checked_redeem_and_attestation("zz", &oracle, &"11".repeat(64), &"22".repeat(32))
            .unwrap_err()
            .starts_with("Bad redeem hex")
    );
    assert_eq!(
        build_json_with_salt(
            &owner,
            &beneficiary,
            &oracle,
            "zz",
            "release",
            1,
            "kaspa",
            [7; 16]
        )
        .unwrap_err(),
        "Oracle covenant key ID must be 32-byte hex"
    );
}
