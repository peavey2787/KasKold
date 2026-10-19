use serde_json::{json, Value};

use super::genesis::build_oracle_genesis_json;

fn request(hashfn: &str) -> String {
    json!({
        "genesis_price": "1000",
        "genesis_t": "2000",
        "image_id_hex": "11".repeat(32),
        "control_id_hex": "22".repeat(32),
        "set_root_hex": "33".repeat(32),
        "hashfn_hex": hashfn,
        "heartbeat_cov_id_hex": "44".repeat(32),
        "network": "testnet-12"
    })
    .to_string()
}

#[test]
fn oracle_genesis_builder_covers_valid_and_malformed_requests() {
    let document: Value =
        serde_json::from_str(&build_oracle_genesis_json(&request("01")).unwrap()).unwrap();
    assert_eq!(document["genesis_price"], 1000);
    assert_eq!(document["genesis_t"], 2000);
    assert!(document["address"]
        .as_str()
        .unwrap()
        .starts_with("kaspatest:"));

    assert_eq!(
        build_oracle_genesis_json("{}")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Bad oracle genesis request JSON: missing field `genesis_price` at line 1 column 2")
    );
    assert_eq!(
        build_oracle_genesis_json(&request(""))
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("hashfn must be 1 byte")
    );
    assert_eq!(
        build_oracle_genesis_json(&request("0102"))
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("hashfn must be 1 byte")
    );

    let mut bad: Value = serde_json::from_str(&request("01")).unwrap();
    bad["genesis_price"] = json!("not-a-number");
    assert_eq!(
        build_oracle_genesis_json(&bad.to_string())
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Invalid genesis_price: invalid digit found in string")
    );
    bad["genesis_price"] = json!("1");
    bad["image_id_hex"] = json!("00");
    assert_eq!(
        build_oracle_genesis_json(&bad.to_string())
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("image_id must be 32 bytes, got 1")
    );
}

#[test]
fn oracle_wasm_genesis_boundaries_are_directly_covered() {
    let genesis = crate::wasm_api::test_support::expect_js(
        super::covenant_oracle_mb(&request("01")),
        "oracle genesis boundary",
    );
    let genesis: Value = serde_json::from_str(&genesis).unwrap();
    assert!(genesis["address"]
        .as_str()
        .unwrap()
        .starts_with("kaspatest:"));

    let heartbeat = crate::wasm_api::test_support::expect_js(
        super::covenant_oracle_mb_heartbeat("testnet-12"),
        "heartbeat boundary",
    );
    let heartbeat: Value = serde_json::from_str(&heartbeat).unwrap();
    assert!(heartbeat["address"]
        .as_str()
        .unwrap()
        .starts_with("kaspatest:"));
}
