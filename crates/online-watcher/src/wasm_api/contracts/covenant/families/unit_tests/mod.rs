use serde_json::{json, Value};

use super::{
    dms::build_dms_json,
    escrow::{build_escrow_json, build_shipping_escrow_json, ship_escrow_terms_json},
    savings::build_timelocked_savings_json,
};

fn address(byte: u8, prefix: &str) -> String {
    crate::account::address::encode_p2pk_address(&[byte; 32], prefix)
}

fn key(byte: u8) -> String {
    let signing = k256::schnorr::SigningKey::from_bytes((&[byte; 32]).into())
        .expect("deterministic test signing key");
    hex::encode(signing.verifying_key().to_bytes())
}

fn parse(document: String) -> Value {
    serde_json::from_str(&document).expect("valid builder JSON")
}

fn expect_wasm_string(result: Result<String, crate::wasm_api::JsValue>, label: &str) -> String {
    crate::wasm_api::test_support::expect_js(result, label)
}

#[test]
fn dms_savings_and_timelocked_escrow_builders_validate_keys_and_addresses() {
    let dms = parse(build_dms_json(&key(4), &key(5), 100, "simnet").unwrap());
    assert_eq!(dms["inactivity_daa"], "100");
    assert!(dms["address"].as_str().unwrap().starts_with("kaspasim:"));
    assert_eq!(
        build_dms_json("00", &key(5), 1, "mainnet")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Pubkey must be 32 bytes")
    );

    let savings = parse(build_timelocked_savings_json(&key(6), &key(7), 200, "devnet").unwrap());
    assert_eq!(savings["locktime_daa"], "200");
    assert!(savings["address"]
        .as_str()
        .unwrap()
        .starts_with("kaspadev:"));
    assert_eq!(
        build_timelocked_savings_json(&key(6), "zz", 0, "mainnet")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Bad wallet2 pubkey hex: Invalid character 'z' at position 0")
    );
}

#[test]
fn escrow_builder_binds_deterministic_salt() {
    let escrow = parse(
        build_escrow_json(
            &key(10),
            &key(11),
            &key(12),
            &address(10, "kaspa"),
            &address(11, "kaspa"),
            "mainnet",
            [0x44; 8],
        )
        .unwrap(),
    );
    assert_eq!(escrow["salt"], "44".repeat(8));
    assert_eq!(
        build_escrow_json(
            &key(10),
            &key(11),
            &key(12),
            "bad",
            &address(11, "kaspa"),
            "mainnet",
            [0; 8],
        )
        .err()
        .map(|error| error.to_string())
        .as_deref(),
        Some("Unknown address prefix")
    );
}

fn shipping_request(product: &str, fee: &str) -> String {
    json!({
        "seller_pubkey_hex": key(20),
        "deliverer_pubkey_hex": key(21),
        "buyer_pubkey_hex": key(22),
        "arbiter_pubkey_hex": key(23),
        "product_sompi": product,
        "fee_sompi": fee,
        "cltv1_deadline": "100",
        "cltv2_deadline": "200",
        "network": "testnet-12"
    })
    .to_string()
}

#[test]
fn shipping_escrow_builder_covers_amount_partitioning_and_invalid_requests() {
    let document =
        parse(build_shipping_escrow_json(&shipping_request("101", "9"), [0x66; 8]).unwrap());
    assert_eq!(document["t1_sompi"], "50");
    assert_eq!(document["t2_sompi"], "51");
    assert_eq!(document["total_sompi"], "110");
    assert_eq!(document["rem_sompi"], "60");
    assert_eq!(document["salt"], "66".repeat(8));
    let terms = parse(
        ship_escrow_terms_json(
            document["redeem_script_hex"].as_str().unwrap(),
            "testnet-10",
        )
        .unwrap(),
    );
    assert_eq!(terms["total_sompi"], "110");
    assert_eq!(terms["rem_sompi"], "60");
    assert_eq!(terms["cltv2_deadline"], "200");
    assert!(terms["seller_addr"]
        .as_str()
        .unwrap()
        .starts_with("kaspatest:"));
    assert_eq!(
        ship_escrow_terms_json("51", "testnet-10")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("unrecognized ship-escrow redeem")
    );

    assert_eq!(build_shipping_escrow_json("{}", [0; 8]).err().map(|error| error.to_string()).as_deref(), Some("Bad shipping escrow request JSON: missing field `seller_pubkey_hex` at line 1 column 2"));
    assert_eq!(
        build_shipping_escrow_json(&shipping_request("bad", "1"), [0; 8])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Invalid product_sompi: invalid digit found in string")
    );
    assert_eq!(
        build_shipping_escrow_json(&shipping_request(&u64::MAX.to_string(), "1"), [0; 8],)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Shipping escrow total exceeds supported monetary range")
    );

    let key = [0x44; 32];
    assert_eq!(
        crate::contracts::shipping_escrow::script::build_ship_escrow_script(
            crate::contracts::shipping_escrow::script::ShippingEscrowScriptRequest {
                seller_pubkey: &key,
                deliverer_pubkey: &key,
                buyer_pubkey: &key,
                arbiter_pubkey: &key,
                product_sompi: u64::MAX,
                fee_sompi: 1,
                cltv1_deadline: 100,
                cltv2_deadline: 200,
                salt: &[0; 8],
            },
        )
        .err()
        .map(|error| error.to_string())
        .as_deref(),
        Some("Shipping escrow total exceeds supported monetary range")
    );

    let mut bad_key: Value = serde_json::from_str(&shipping_request("1", "1")).unwrap();
    bad_key["seller_pubkey_hex"] = json!("00");
    assert_eq!(
        build_shipping_escrow_json(&bad_key.to_string(), [0; 8])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Pubkey must be 32 bytes")
    );
}

#[test]
fn covenant_wasm_facades_use_valid_host_fixtures_without_crossing_error_abi() {
    use super::{
        covenant_additive_address, covenant_dms, covenant_escrow, covenant_global_allowance,
        covenant_global_spending_limit, covenant_payjoin, covenant_ship_escrow,
        covenant_timelocked_savings,
    };

    let owner = key(0x21);
    let beneficiary = key(0x22);
    let arbiter = key(0x23);
    let alice_address = address(0x31, "kaspa");
    let bob_address = address(0x32, "kaspa");

    assert!(expect_wasm_string(
        covenant_additive_address(&owner, 10_000_000, 900, "mainnet"),
        "additive covenant",
    )
    .contains("address"));
    assert!(expect_wasm_string(
        covenant_dms(&owner, &beneficiary, 144, "mainnet"),
        "DMS covenant",
    )
    .contains("redeem_script_hex"));
    assert!(expect_wasm_string(
        covenant_timelocked_savings(&owner, &beneficiary, 600, "mainnet"),
        "timelocked savings covenant",
    )
    .contains("locktime_daa"));
    assert!(expect_wasm_string(
        covenant_escrow(
            &owner,
            &beneficiary,
            &arbiter,
            &alice_address,
            &bob_address,
            "mainnet",
        ),
        "escrow covenant",
    )
    .contains("salt"));
    assert!(expect_wasm_string(
        covenant_payjoin(&owner, &beneficiary, 900, 2, 2, "mainnet"),
        "payjoin covenant",
    )
    .contains("redeem_script_hex"));
    assert!(expect_wasm_string(
        covenant_global_allowance(&owner, &beneficiary, 50_000_000, 10, 0, "mainnet"),
        "global allowance covenant",
    )
    .contains("global_allowance"));
    assert!(expect_wasm_string(
        covenant_global_spending_limit(&owner, 50_000_000, 10, "mainnet"),
        "global spending limit covenant",
    )
    .contains("redeem_script_hex"));
    assert!(expect_wasm_string(
        covenant_ship_escrow(&shipping_request("100000000", "1000000")),
        "shipping escrow covenant",
    )
    .contains("total_sompi"));
    assert!(expect_wasm_string(
        super::covenant_private_swap(
            &key(0x50),
            &key(0x51),
            &address(0x52, "kaspa"),
            50_000,
            &"53".repeat(16),
            "mainnet",
        ),
        "private swap covenant",
    )
    .contains("private-swap"));
    let request = expect_wasm_string(
        super::private_swap_key_request(),
        "private swap key request",
    );
    assert!(!request.is_empty());
}

mod feature_contracts;

mod coverage;
