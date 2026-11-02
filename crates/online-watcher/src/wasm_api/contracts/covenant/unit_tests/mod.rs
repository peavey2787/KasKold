#[test]
fn covenant_fee_exports_price_through_portal() {
    use super::{private_swap_max_fee_sompi, thread_maximum_withdraw, thread_withdraw_fee};

    assert_eq!(private_swap_max_fee_sompi(), 500_000_000);
    assert_eq!(
        thread_withdraw_fee(1_000_000_000, 100_000_000, 300_000, 100).ok(),
        Some(1_044_500)
    );
    let best = thread_maximum_withdraw(1_000_000_000, u64::MAX, 300_000, 1, 10_000_000);
    assert!(best > 0 && best < 1_000_000_000);
}

#[test]
fn covenant_spend_path_reads_the_selector_through_portal() {
    use super::covenant_spend_path;

    assert_eq!(covenant_spend_path("02aaaa510151", "51"), "owner");
    assert_eq!(covenant_spend_path("02aaaa000151", "51"), "heir");
    assert_eq!(covenant_spend_path("02aaaa520151", "51"), "unknown");
    assert_eq!(covenant_spend_path("zz", "51"), "unknown");
}

// Valid secp256k1 x-only keys for 1G and 2G.
const OWNER: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
const OTHER: &str = "c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";

fn key_bytes(key: &str) -> [u8; 32] {
    hex::decode(key).unwrap().try_into().unwrap()
}

fn built_dms(inactivity_daa: u64) -> serde_json::Value {
    let document = crate::contracts::covenant::construction::dms::build_json(
        OWNER,
        OTHER,
        inactivity_daa,
        "testnet-10",
    )
    .expect("switch builds");
    serde_json::from_str(&document).expect("switch JSON")
}

#[test]
fn covenant_script_terms_reads_the_script_and_refuses_what_it_does_not_match() {
    use super::covenant_script_terms;

    let invite = built_dms(900);
    let script = invite["redeem_script_hex"].as_str().unwrap();
    let address = invite["address"].as_str().unwrap();
    let terms = crate::wasm_api::test_support::expect_js(
        covenant_script_terms("dms", script, address, "testnet-10", ""),
        "genuine terms",
    );
    let terms: serde_json::Value = serde_json::from_str(&terms).unwrap();
    assert_eq!(terms["owner_pubkey_hex"], OWNER);
    assert_eq!(terms["heir_pubkey_hex"], OTHER);
    assert_eq!(terms["inactivity_daa"], "900");

    let forged = built_dms(901);
    let forged_address = forged["address"].as_str().unwrap();
    assert!(covenant_script_terms("dms", script, forged_address, "testnet-10", "").is_err());
    assert!(covenant_script_terms("no-such-family", script, address, "testnet-10", "").is_err());
}

#[test]
fn covenant_spend_document_encodes_a_named_path_and_refuses_a_spend_with_no_coins() {
    use super::covenant_spend_document;
    use crate::{
        account::address::encode_p2pk_address, contracts::covenant::script,
        wasm_api::test_support::covenant_address,
    };

    let redeem = script::build_dms_csv_script(&key_bytes(OWNER), &key_bytes(OTHER), 100);
    let request = |utxos: serde_json::Value| {
        serde_json::json!({
            "covenant_address": covenant_address(&redeem),
            "redeem_script_hex": hex::encode(&redeem),
            "path": "owner",
            "tx_version": 1,
            "lock_time": "0",
            "payload_hex": "",
            "utxos": utxos,
            "outputs": [{
                "address": encode_p2pk_address(&key_bytes(OTHER), "kaspa"),
                "amount": "29000000",
            }],
        })
        .to_string()
    };
    let funded =
        serde_json::json!([{ "tx_id": "63".repeat(32), "index": 0, "amount": "30000000" }]);
    let wire = crate::wasm_api::test_support::expect_js(
        covenant_spend_document(&request(funded)),
        "funded spend",
    );
    assert!(wire.starts_with(&hex::encode(b"PSKB")));

    assert_eq!(
        covenant_spend_document(&request(serde_json::json!([]))),
        Err(crate::wasm_api::JsValue::from_str(
            "No UTXOs at covenant address"
        ))
    );
}

#[test]
fn private_swap_refund_binding_builds_the_refund_for_its_own_script_only() {
    use super::create_private_swap_refund;
    use crate::{
        account::address::{address_to_script_pubkey, encode_p2pk_address},
        contracts::covenant::script,
        transaction_builder::covenant::private_swap,
        wasm_api::test_support::covenant_address,
    };

    let target = encode_p2pk_address(&key_bytes(OWNER), "kaspa");
    let redeem = script::build_private_swap_script(
        &key_bytes(OWNER),
        &key_bytes(OTHER),
        &address_to_script_pubkey(&target).unwrap(),
        500,
        &[4; 16],
    )
    .expect("swap script");
    let redeem_hex = hex::encode(&redeem);
    let address = covenant_address(&redeem);
    let coins = serde_json::json!([{ "tx_id": "63".repeat(32), "index": 0, "amount": "30000000" }])
        .to_string();

    let wire = crate::wasm_api::test_support::expect_js(
        create_private_swap_refund(&address, &target, &redeem_hex, &coins, 1_000_000),
        "refund",
    );
    assert_eq!(
        Ok(wire),
        private_swap::build_refund(&address, &target, &redeem_hex, &coins, 1_000_000)
    );
    assert!(create_private_swap_refund(&target, &target, &redeem_hex, &coins, 1_000_000).is_err());
}
