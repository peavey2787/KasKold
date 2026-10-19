mod export_fail_closed;

use serde::Deserialize;

use crate::account::address::network_prefix;

use super::common::{
    decode_named_32, decode_pubkey32, parse_request_string, parse_u64_field_string,
    serialize_json_string,
};

#[derive(Deserialize, serde::Serialize, PartialEq, Debug)]
struct Request {
    value: u64,
}

#[test]
fn pure_wasm_boundary_utilities_cover_networks_parsing_and_hex_validation() {
    assert_eq!(network_prefix("testnet-10"), Ok("kaspatest"));
    assert_eq!(network_prefix("testnet-11"), Ok("kaspatest"));
    assert_eq!(network_prefix("testnet-12"), Ok("kaspatest"));
    assert_eq!(network_prefix("simnet"), Ok("kaspasim"));
    assert_eq!(network_prefix("devnet"), Ok("kaspadev"));
    assert_eq!(network_prefix("mainnet"), Ok("kaspa"));
    for (unknown, expected) in [
        ("", r#"unsupported Kaspa network: """#),
        ("main", r#"unsupported Kaspa network: "main""#),
        ("Testnet-10", r#"unsupported Kaspa network: "Testnet-10""#),
        ("kaspa", r#"unsupported Kaspa network: "kaspa""#),
    ] {
        assert_eq!(
            network_prefix(unknown)
                .err()
                .map(|error| error.to_string())
                .as_deref(),
            Some(expected),
            "{unknown:?} must not default to mainnet"
        );
    }

    assert_eq!(decode_pubkey32(&"11".repeat(32)).unwrap(), [0x11; 32]);
    assert_eq!(
        decode_pubkey32("zz")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Bad hex: Invalid character 'z' at position 0")
    );
    assert_eq!(
        decode_pubkey32("00")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Pubkey must be 32 bytes")
    );
    assert_eq!(
        decode_named_32(&"22".repeat(32), "field").unwrap(),
        [0x22; 32]
    );
    assert_eq!(
        decode_named_32("00", "field")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("field must be 32 bytes, got 1")
    );

    let request: Request = parse_request_string(r#"{"value":7}"#, "request").unwrap();
    assert_eq!(request, Request { value: 7 });
    assert_eq!(
        parse_request_string::<Request>("{}", "request")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Bad request JSON: missing field `value` at line 1 column 2")
    );
    assert_eq!(parse_u64_field_string("42", "value").unwrap(), 42);
    assert_eq!(
        parse_u64_field_string("bad", "value")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Invalid value: invalid digit found in string")
    );
    assert_eq!(serialize_json_string(&request).unwrap(), r#"{"value":7}"#);
}

#[test]
fn wasm_utility_boundaries_are_host_testable() {
    use super::common::{
        hex_to_pubkey32, parse_request, parse_u64_field, parse_utxo_indices, parse_wallet,
        serialize_json,
    };

    let wallet_json = serde_json::json!({
        "kpub": "test",
        "receive_addresses": [crate::account::address::encode_p2pk_address(&[0x31; 32], "kaspa")],
        "change_addresses": [crate::account::address::encode_p2pk_address(&[0x32; 32], "kaspa")],
        "next_receive_index": 0,
        "next_change_index": 0
    })
    .to_string();
    assert_eq!(
        crate::wasm_api::test_support::expect_js(
            parse_wallet(&wallet_json, "wallet"),
            "parse wallet"
        )
        .kpub,
        "test"
    );
    assert_eq!(
        parse_wallet("not-json", "wallet")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("wallet: expected ident at line 1 column 2"))"#)
    );

    let request: Request = crate::wasm_api::test_support::expect_js(
        parse_request(r#"{"value":9}"#, "request"),
        "parse request",
    );
    assert_eq!(request.value, 9);
    assert_eq!(
        parse_request::<Request>("{}", "request")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad request JSON: missing field `value` at line 1 column 2"))"#)
    );

    assert_eq!(
        crate::wasm_api::test_support::expect_js(parse_u64_field("77", "value"), "parse u64"),
        77
    );
    assert_eq!(
        parse_u64_field("bad", "value")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Invalid value: invalid digit found in string"))"#)
    );
    assert_eq!(
        crate::wasm_api::test_support::expect_js(
            parse_utxo_indices("0, 2,,4"),
            "parse UTXO indices"
        ),
        vec![0, 2, 4]
    );
    assert_eq!(
        parse_utxo_indices("0,bad")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Invalid UTXO index: invalid digit found in string"))"#)
    );

    assert_eq!(
        crate::wasm_api::test_support::expect_js(serialize_json(&request), "serialize JSON"),
        r#"{"value":9}"#
    );
    assert_eq!(
        crate::wasm_api::test_support::expect_js(
            hex_to_pubkey32(&"44".repeat(32)),
            "decode public key"
        ),
        [0x44; 32]
    );
}

#[test]
fn hash_boundaries_cover_sha256() {
    assert_eq!(
        crate::wasm_api::test_support::expect_js(super::crypto::sha256_hash(""), "SHA-256 hash"),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    );
}

#[test]
fn address_exports_validate_and_map_networks_through_portal() {
    use super::{address_network, is_kaspa_address, network_address_prefix};

    let address = "kaspatest:qzkpehfq6c0gua6lwanp8my4hm0saars3px2kw0gpvtwvky5zk0nu7lm5nrty";
    assert!(is_kaspa_address(address));
    assert!(!is_kaspa_address("kaspatest:qzkpehfq"));
    assert!(!is_kaspa_address("kpub1:abc"));
    assert_eq!(
        super::address_script_public_key(address)
            .ok()
            .map(|spk| spk.len()),
        Some(68)
    );

    let spk = super::address_script_public_key(address).unwrap();
    assert_eq!(
        super::script_public_key_address(&spk, "testnet-10"),
        address
    );
    assert_eq!(super::script_public_key_address("51", "testnet-10"), "");
    let p2sh = super::redeem_script_address("51", "testnet-10").unwrap();
    let p2sh_spk = super::address_script_public_key(&p2sh).unwrap();
    assert_eq!(
        super::script_public_key_address(&p2sh_spk, "testnet-10"),
        p2sh
    );
    assert_eq!(
        super::redeem_script_address("zz", "testnet-10")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Invalid character 'z' at position 0"))"#)
    );

    assert_eq!(network_address_prefix("mainnet"), "kaspa:");
    assert_eq!(network_address_prefix("testnet-12"), "kaspatest:");
    assert_eq!(network_address_prefix("bogus"), "");
    let on =
        |prefix: &str| kaspa_portal::primitives::address::encode_p2pk_address(&[0x42; 32], prefix);
    let network = |address: &str, selected: &str| address_network(address, selected).ok();
    let testnet = on("kaspatest");
    assert_eq!(
        network(&testnet, "testnet-12").as_deref(),
        Some("testnet-12")
    );
    assert_eq!(network(&testnet, "mainnet").as_deref(), Some("testnet-10"));
    assert_eq!(network(&testnet, "testnet").as_deref(), Some("testnet-10"));
    assert_eq!(
        network(&testnet, "testnet-custom").as_deref(),
        Some("testnet-10")
    );
    assert_eq!(
        network(&on("kaspasim"), "mainnet").as_deref(),
        Some("simnet")
    );
    assert_eq!(
        network(&on("kaspadev"), "mainnet").as_deref(),
        Some("devnet")
    );
    assert_eq!(
        network(&on("kaspa"), "testnet-10").as_deref(),
        Some("mainnet")
    );
    for invalid in [
        "",
        "kaspatest:qq",
        "kaspa:qq",
        "bitcoin:qq",
        &testnet[..testnet.len() - 1],
    ] {
        assert_eq!(network(invalid, "mainnet"), None, "{invalid}");
    }
}
