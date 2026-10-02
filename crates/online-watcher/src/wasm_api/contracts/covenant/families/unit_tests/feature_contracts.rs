use super::*;

#[test]
fn timeout_refund_spec_is_directly_covered() {
    let redeem = [0x51];
    let spec = super::super::escrow::timeout_refund_spec("source", "destination", 7, &redeem, 99);
    assert_eq!(spec.fee, 7);
    assert_eq!(spec.config.lock_time, 99);
    assert_eq!(spec.config.minimum_signatures, Some(0));
}
#[test]
fn oracle_v1_statement_is_unique_embedded_and_rejects_role_aliasing() {
    let owner = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    let beneficiary = "c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";
    let oracle = "f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9";
    let key_id = "1111111111111111111111111111111111111111111111111111111111111111";
    let build = |salt| {
        crate::contracts::covenant::oracle_v1::build_json_with_salt(
            owner,
            beneficiary,
            oracle,
            key_id,
            "Release if shipment arrived",
            100,
            "kaspa",
            salt,
        )
    };
    let first = parse(build([0x11; 16]).unwrap());
    let second = parse(build([0x22; 16]).unwrap());
    assert_ne!(
        first["attestation_statement"],
        second["attestation_statement"]
    );
    assert_ne!(
        first["message_commitment_hex"],
        second["message_commitment_hex"]
    );
    assert_ne!(first["address"], second["address"]);
    assert_eq!(first["salt"].as_str().unwrap().len(), 32);
    assert!(first["attestation_statement"]
        .as_str()
        .unwrap()
        .starts_with("KaspaPortal Oracle v1 "));
    assert!(crate::contracts::covenant::oracle_v1::build_json_with_salt(
        owner, owner, oracle, key_id, "Release", 100, "kaspa", [0x33; 16],
    )
    .is_err());
    assert!(crate::contracts::covenant::oracle_v1::build_json_with_salt(
        owner,
        beneficiary,
        oracle,
        key_id,
        "Release",
        0,
        "kaspa",
        [0x44; 16],
    )
    .is_err());
    assert!(crate::contracts::covenant::oracle_v1::build_json_with_salt(
        "00",
        beneficiary,
        oracle,
        key_id,
        "Release",
        100,
        "kaspa",
        [0x55; 16],
    )
    .is_err());
}

#[test]
fn private_swap_builder_and_device_key_request_are_live_current_features() {
    let destination = address(0x52, "kaspa");
    let document = parse(
        crate::contracts::covenant::construction::private_swap::build_json(
            &key(0x50),
            &key(0x51),
            &destination,
            50_000,
            &"53".repeat(16),
            "mainnet",
        )
        .unwrap(),
    );
    assert_eq!(document["type"], "private-swap");
    assert_eq!(document["locktime_daa"], "50000");
    assert_eq!(document["destination"], destination);
    assert!(!document["redeem_script_hex"].as_str().unwrap().is_empty());

    let request = super::super::private_swap::private_swap_key_request_string().unwrap();
    let bytes = hex::decode(request).unwrap();
    let parsed = shared_signer::covenant_sign::private_swap::parse_request(&bytes).unwrap();
    assert_eq!(
        parsed.kind,
        shared_signer::covenant_sign::private_swap::RequestKind::KeyInfo
    );
}
