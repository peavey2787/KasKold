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
    for unknown in ["", "main", "Testnet-10", "kaspa"] {
        assert!(
            network_prefix(unknown).is_err(),
            "{unknown:?} must not default to mainnet"
        );
    }

    assert_eq!(decode_pubkey32(&"11".repeat(32)).unwrap(), [0x11; 32]);
    assert!(decode_pubkey32("zz").is_err());
    assert!(decode_pubkey32("00").is_err());
    assert_eq!(
        decode_named_32(&"22".repeat(32), "field").unwrap(),
        [0x22; 32]
    );
    assert!(decode_named_32("00", "field").is_err());

    let request: Request = parse_request_string(r#"{"value":7}"#, "request").unwrap();
    assert_eq!(request, Request { value: 7 });
    assert!(parse_request_string::<Request>("{}", "request").is_err());
    assert_eq!(parse_u64_field_string("42", "value").unwrap(), 42);
    assert!(parse_u64_field_string("bad", "value").is_err());
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
    assert!(parse_wallet("not-json", "wallet").is_err());

    let request: Request = crate::wasm_api::test_support::expect_js(
        parse_request(r#"{"value":9}"#, "request"),
        "parse request",
    );
    assert_eq!(request.value, 9);
    assert!(parse_request::<Request>("{}", "request").is_err());

    assert_eq!(
        crate::wasm_api::test_support::expect_js(parse_u64_field("77", "value"), "parse u64"),
        77
    );
    assert!(parse_u64_field("bad", "value").is_err());
    assert_eq!(
        crate::wasm_api::test_support::expect_js(
            parse_utxo_indices("0, 2,,4"),
            "parse UTXO indices"
        ),
        vec![0, 2, 4]
    );
    assert!(parse_utxo_indices("0,bad").is_err());

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
    assert!(super::redeem_script_address("zz", "testnet-10").is_err());

    assert_eq!(network_address_prefix("mainnet"), "kaspa:");
    assert_eq!(network_address_prefix("testnet-12"), "kaspatest:");
    assert_eq!(network_address_prefix("bogus"), "");
    assert_eq!(address_network("kaspatest:qq", "testnet-12"), "testnet-12");
    assert_eq!(address_network("kaspatest:qq", "mainnet"), "testnet-10");
    assert_eq!(address_network("kaspasim:qq", "mainnet"), "simnet");
    assert_eq!(address_network("kaspadev:qq", "mainnet"), "devnet");
    assert_eq!(address_network("kaspa:qq", "testnet-10"), "mainnet");
    assert_eq!(address_network("", "testnet-10"), "mainnet");
}
