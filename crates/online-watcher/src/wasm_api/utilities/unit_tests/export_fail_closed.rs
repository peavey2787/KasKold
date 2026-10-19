//! Every exported WASM entry point fails closed on malformed input, natively.

use crate::wasm_api::test_support::ready;
use crate::wasm_api::*;

const BAD_KEY: &str = "zz";
const BAD_ADDRESS: &str = "not-an-address";
const NETWORK: &str = "mainnet";
const WS: &str = "ws://unused";

#[test]
fn covenant_address_exports_reject_malformed_keys() {
    assert_eq!(
        covenant_additive_address(BAD_KEY, 1, 1, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        covenant_allowance(BAD_KEY, BAD_KEY, 1, 1, 1, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        covenant_dms(BAD_KEY, BAD_KEY, 1, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        covenant_escrow(BAD_KEY, BAD_KEY, BAD_KEY, BAD_ADDRESS, BAD_ADDRESS, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        covenant_ship_escrow("not-json")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Bad shipping escrow request JSON: expected ident at line 1 column 2"))"#
        )
    );
    assert_eq!(
        covenant_timelocked_escrow(BAD_KEY, BAD_KEY, BAD_ADDRESS, BAD_ADDRESS, 1, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad alice pubkey hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        covenant_oracle_v1(BAD_KEY, BAD_KEY, BAD_KEY, BAD_KEY, "release", 1, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Oracle owner key must be 32-byte hex"))"#)
    );
    assert_eq!(
        verify_oracle_v1_attestation(BAD_KEY, BAD_KEY, BAD_KEY)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Oracle oracle key must be 32-byte hex"))"#)
    );
    assert_eq!(
        covenant_payjoin(BAD_KEY, BAD_KEY, 1, 1, 1, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        covenant_timelocked_savings(BAD_KEY, BAD_KEY, 1, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad wallet1 pubkey hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        covenant_oracle_mb("not-json")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Bad oracle genesis request JSON: expected ident at line 1 column 2"))"#
        )
    );
    assert_eq!(
        covenant_commit_reveal(BAD_KEY, BAD_KEY, 1, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        covenant_merkle_whitelist(BAD_KEY, BAD_KEY, 1, 1, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        covenant_crowdfund(BAD_KEY, BAD_ADDRESS, 1, 1, BAD_KEY, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad hex: Invalid character 'z' at position 0"))"#)
    );
}

#[test]
fn covenant_spend_exports_reject_malformed_requests_before_network_io() {
    assert_eq!(
        ready(create_covenant_allowance_withdraw(
            BAD_ADDRESS,
            BAD_ADDRESS,
            BAD_KEY,
            1,
            1,
            WS
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(
            r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
        )
    );
    assert_eq!(
        ready(create_covenant_beneficiary_spend(
            BAD_ADDRESS,
            BAD_ADDRESS,
            BAD_KEY,
            1,
            1,
            WS
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(r#"JsValue(Some("Bad redeem hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        create_covenant_beneficiary_spend_selected(BAD_ADDRESS, BAD_ADDRESS, BAD_KEY, 1, "[]", 1)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad redeem hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        ready(create_covenant_oracle_v1_claim(
            BAD_ADDRESS,
            BAD_ADDRESS,
            BAD_KEY,
            BAD_KEY,
            BAD_KEY,
            BAD_KEY,
            1,
            WS
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(r#"JsValue(Some("Bad redeem hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        ready(create_covenant_payjoin_claim(
            BAD_ADDRESS,
            BAD_ADDRESS,
            BAD_KEY,
            BAD_ADDRESS,
            1,
            WS
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(
            r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
        )
    );
    assert_eq!(
        ready(create_covenant_timelocked_savings_claim(
            BAD_ADDRESS,
            BAD_ADDRESS,
            BAD_KEY,
            1,
            1,
            WS
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(r#"JsValue(Some("Bad redeem hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        create_covenant_timelocked_savings_claim_selected(
            BAD_ADDRESS,
            BAD_ADDRESS,
            BAD_KEY,
            1,
            "[]",
            1
        )
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(r#"JsValue(Some("Bad redeem hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        ready(create_covenant_owner_spend(
            BAD_ADDRESS,
            BAD_ADDRESS,
            BAD_KEY,
            1,
            WS,
            "owner"
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(r#"JsValue(Some("Bad redeem hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        create_covenant_owner_spend_selected(BAD_ADDRESS, BAD_ADDRESS, BAD_KEY, "[]", 1, "owner")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad redeem hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        ready(create_merkle_whitelist_spend(
            BAD_ADDRESS,
            BAD_ADDRESS,
            BAD_KEY,
            "[]",
            1,
            1,
            WS
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(
            r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
        )
    );
    assert_eq!(
        ready(create_commit_reveal_spend("not-json"))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Bad commit-reveal spend request JSON: expected ident at line 1 column 2"))"#
        )
    );
    assert_eq!(
        ready(inspect_crowdfund_contributions("not-json", WS))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Invalid crowdfunding contribution JSON: expected ident at line 1 column 2"))"#
        )
    );
    assert_eq!(
        ready(create_crowdfund_sweep(
            "not-json",
            BAD_ADDRESS,
            1,
            1,
            BAD_KEY,
            BAD_KEY,
            BAD_KEY,
            1,
            WS
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(
            r#"JsValue(Some("Invalid crowdfunding contribution JSON: expected ident at line 1 column 2"))"#
        )
    );
    assert_eq!(
        ready(create_covenant_pskb("not-json"))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad covenant request JSON: expected ident at line 1 column 2"))"#)
    );
    assert_eq!(
        ready(create_covenant_pskb_with_payload("not-json"))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Bad covenant payload request JSON: expected ident at line 1 column 2"))"#
        )
    );
}

#[test]
fn utility_and_protocol_exports_reject_malformed_hex_and_documents() {
    assert_eq!(
        build_covenant_payload(1, BAD_KEY)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad params hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        parse_covenant_payload(BAD_KEY)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad plaintext hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        commit_hash(BAD_KEY)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad preimage hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        merkle_root_from_addresses("not-json")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad JSON: expected ident at line 1 column 2"))"#)
    );
    assert_eq!(
        merkle_proof_for_address("not-json", BAD_ADDRESS)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad JSON: expected ident at line 1 column 2"))"#)
    );
    assert_eq!(
        sha256_hash(BAD_KEY)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("bad hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        encode_p2pk_address(BAD_KEY, None)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Invalid hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        parse_kpub("not-a-kpub")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Invalid kpub"))"#)
    );
    assert_eq!(
        derive_covenant_payload_key("not-a-kpub")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Invalid kpub"))"#)
    );
    assert_eq!(
        stealth_generate_payment(BAD_KEY, BAD_KEY, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Stealth meta-address must be 128 hex chars, got 2"))"#)
    );
    assert_eq!(
        verify_covenant_anti_klepto(BAD_KEY, BAD_KEY, BAD_KEY, BAD_KEY, BAD_KEY, BAD_KEY)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("invalid covenant public key hex"))"#)
    );
    assert_eq!(
        pskt_summary(BAD_KEY, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Bad outer hex: outer PSKT/PSKB must be even-length lowercase hexadecimal"))"#
        )
    );
    assert_eq!(
        pskt_relay_to_kspt(BAD_KEY, NETWORK)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Encoding: outer PSKT/PSKB hex must be even-length lowercase hexadecimal"))"#
        )
    );
    assert_eq!(
        pskt_merge_signed_kspt(BAD_KEY, BAD_KEY)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("KSPT hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        ready(pskt_finalize_and_broadcast(BAD_KEY, WS))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("PSKT cryptographic verification failed: outer PSKT/PSKB hex must be even-length lowercase hexadecimal"))"#
        )
    );
    assert_eq!(
        generate_qr_frames(BAD_KEY)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Invalid hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        decode_qr_frame(BAD_KEY)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Invalid hex: Invalid character 'z' at position 0"))"#)
    );
    assert_eq!(
        ready(create_send_pskb_with_utxos(
            "not-json",
            BAD_ADDRESS,
            1,
            1,
            "[]"
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(r#"JsValue(Some("Bad wallet: expected ident at line 1 column 2"))"#)
    );
}

#[test]
fn deterministic_exports_succeed_on_valid_input() {
    let fee = estimate_covenant_fee(1, 40, 0, 0).expect("covenant fee estimate");
    assert!(fee.parse::<u64>().expect("decimal fee") > 0);
    let sha = sha256_hash("").expect("sha256");
    assert_eq!(
        sha,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert!(covenant_oracle_mb_heartbeat(NETWORK).is_ok());
    assert!(stealth_announcement_address(NETWORK)
        .unwrap()
        .starts_with("kaspa:"));
}

#[test]
fn private_swap_exports_reject_malformed_wires_and_fields() {
    const GENERATOR: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    let key_id = "11".repeat(32);

    let redeem_error = expect_err_text(
        private_swap_bind_request(&key_id, GENERATOR, BAD_KEY),
        "bind request",
    );
    assert!(redeem_error.contains("Bad redeem script"), "{redeem_error}");
    let session_error = expect_err_text(
        private_swap_reveal_request(BAD_KEY, &key_id, &"22".repeat(32), &"33".repeat(32)),
        "reveal request",
    );
    assert!(session_error.contains("Bad session"), "{session_error}");
    let response_error = expect_err_text(private_swap_parse_response("00"), "response parse");
    assert!(
        response_error.contains("Invalid Private Swap response"),
        "{response_error}"
    );
    assert_eq!(
        private_swap_insert_completed_signature(BAD_KEY, BAD_KEY, BAD_KEY)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Bad Private Swap completed signature: Invalid character 'z' at position 0"))"#
        )
    );
}

fn expect_err_text(result: Result<String, JsValue>, label: &str) -> String {
    match result {
        Ok(value) => panic!("{label} unexpectedly succeeded: {value}"),
        Err(error) => format!("{error:?}"),
    }
}

#[test]
fn selected_covenant_sweep_exports_build_and_log_valid_pskbs() {
    use crate::wasm_api::test_support::expect_js;

    let covenant = crate::account::address::encode_p2pk_address(&[0x61; 32], "kaspa");
    let destination = crate::account::address::encode_p2pk_address(&[0x62; 32], "kaspa");
    let utxos = serde_json::json!([
        {"tx_id": "63".repeat(32), "index": 0, "amount": 30_000_000u64}
    ])
    .to_string();
    for wire in [
        expect_js(
            create_covenant_timelocked_savings_claim_selected(
                &covenant,
                &destination,
                "51",
                123,
                &utxos,
                1_000_000,
            ),
            "savings claim",
        ),
        expect_js(
            create_covenant_beneficiary_spend_selected(
                &covenant,
                &destination,
                "51",
                123,
                &utxos,
                1_000_000,
            ),
            "beneficiary spend",
        ),
        expect_js(
            create_covenant_owner_spend_selected(
                &covenant,
                &destination,
                "51",
                &utxos,
                1_000_000,
                "owner",
            ),
            "owner spend",
        ),
    ] {
        assert!(wire.starts_with("50534b42"), "{wire}");
    }
}
