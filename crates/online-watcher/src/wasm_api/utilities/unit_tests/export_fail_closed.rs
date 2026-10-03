//! Every exported WASM entry point fails closed on malformed input, natively.

use crate::wasm_api::test_support::ready;
use crate::wasm_api::*;

const BAD_KEY: &str = "zz";
const BAD_ADDRESS: &str = "not-an-address";
const NETWORK: &str = "kaspa";
const WS: &str = "ws://unused";

#[test]
fn covenant_address_exports_reject_malformed_keys() {
    assert!(covenant_additive_address(BAD_KEY, 1, 1, NETWORK).is_err());
    assert!(covenant_allowance(BAD_KEY, BAD_KEY, 1, 1, 1, NETWORK).is_err());
    assert!(covenant_dms(BAD_KEY, BAD_KEY, 1, NETWORK).is_err());
    assert!(covenant_escrow(BAD_KEY, BAD_KEY, BAD_KEY, BAD_ADDRESS, BAD_ADDRESS, NETWORK).is_err());
    assert!(covenant_ship_escrow("not-json").is_err());
    assert!(
        covenant_timelocked_escrow(BAD_KEY, BAD_KEY, BAD_ADDRESS, BAD_ADDRESS, 1, NETWORK).is_err()
    );
    assert!(covenant_oracle_v1(BAD_KEY, BAD_KEY, BAD_KEY, BAD_KEY, "release", 1, NETWORK).is_err());
    assert!(verify_oracle_v1_attestation(BAD_KEY, BAD_KEY, BAD_KEY).is_err());
    assert!(covenant_payjoin(BAD_KEY, BAD_KEY, 1, 1, 1, NETWORK).is_err());
    assert!(covenant_timelocked_savings(BAD_KEY, BAD_KEY, 1, NETWORK).is_err());
    assert!(covenant_oracle_mb("not-json").is_err());
    assert!(covenant_commit_reveal(BAD_KEY, BAD_KEY, 1, NETWORK).is_err());
    assert!(covenant_merkle_whitelist(BAD_KEY, BAD_KEY, 1, 1, NETWORK).is_err());
    assert!(covenant_crowdfund(BAD_KEY, BAD_ADDRESS, 1, 1, BAD_KEY, NETWORK).is_err());
}

#[test]
fn covenant_spend_exports_reject_malformed_requests_before_network_io() {
    assert!(ready(create_covenant_allowance_withdraw(
        BAD_ADDRESS,
        BAD_ADDRESS,
        BAD_KEY,
        1,
        1,
        WS
    ))
    .is_err());
    assert!(ready(create_covenant_beneficiary_spend(
        BAD_ADDRESS,
        BAD_ADDRESS,
        BAD_KEY,
        1,
        1,
        WS
    ))
    .is_err());
    assert!(create_covenant_beneficiary_spend_selected(
        BAD_ADDRESS,
        BAD_ADDRESS,
        BAD_KEY,
        1,
        "[]",
        1
    )
    .is_err());
    assert!(ready(create_covenant_oracle_v1_claim(
        BAD_ADDRESS,
        BAD_ADDRESS,
        BAD_KEY,
        BAD_KEY,
        BAD_KEY,
        BAD_KEY,
        1,
        WS
    ))
    .is_err());
    assert!(ready(create_covenant_payjoin_claim(
        BAD_ADDRESS,
        BAD_ADDRESS,
        BAD_KEY,
        BAD_ADDRESS,
        1,
        WS
    ))
    .is_err());
    assert!(ready(create_covenant_timelocked_savings_claim(
        BAD_ADDRESS,
        BAD_ADDRESS,
        BAD_KEY,
        1,
        1,
        WS
    ))
    .is_err());
    assert!(create_covenant_timelocked_savings_claim_selected(
        BAD_ADDRESS,
        BAD_ADDRESS,
        BAD_KEY,
        1,
        "[]",
        1
    )
    .is_err());
    assert!(ready(create_covenant_owner_spend(
        BAD_ADDRESS,
        BAD_ADDRESS,
        BAD_KEY,
        1,
        WS,
        "owner"
    ))
    .is_err());
    assert!(create_covenant_owner_spend_selected(
        BAD_ADDRESS,
        BAD_ADDRESS,
        BAD_KEY,
        "[]",
        1,
        "owner"
    )
    .is_err());
    assert!(ready(create_merkle_whitelist_spend(
        BAD_ADDRESS,
        BAD_ADDRESS,
        BAD_KEY,
        "[]",
        1,
        1,
        WS
    ))
    .is_err());
    assert!(ready(create_commit_reveal_spend("not-json")).is_err());
    assert!(ready(inspect_crowdfund_contributions("not-json", WS)).is_err());
    assert!(ready(create_crowdfund_sweep(
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
    .is_err());
    assert!(ready(create_covenant_pskb("not-json")).is_err());
    assert!(ready(create_covenant_pskb_with_payload("not-json")).is_err());
}

#[test]
fn utility_and_protocol_exports_reject_malformed_hex_and_documents() {
    assert!(build_covenant_payload(1, BAD_KEY).is_err());
    assert!(parse_covenant_payload(BAD_KEY).is_err());
    assert!(commit_hash(BAD_KEY).is_err());
    assert!(merkle_root_from_addresses("not-json").is_err());
    assert!(merkle_proof_for_address("not-json", BAD_ADDRESS).is_err());
    assert!(blake2b_hash(BAD_KEY).is_err());
    assert!(sha256_hash(BAD_KEY).is_err());
    assert!(encode_p2pk_address(BAD_KEY, None).is_err());
    assert!(parse_kpub("not-a-kpub").is_err());
    assert!(derive_covenant_payload_key("not-a-kpub").is_err());
    assert!(stealth_generate_payment(BAD_KEY, BAD_KEY, NETWORK).is_err());
    assert!(
        verify_covenant_anti_klepto(BAD_KEY, BAD_KEY, BAD_KEY, BAD_KEY, BAD_KEY, BAD_KEY).is_err()
    );
    assert!(pskt_summary(BAD_KEY, NETWORK).is_err());
    assert!(pskt_relay_to_kspt(BAD_KEY, NETWORK).is_err());
    assert!(pskt_merge_signed_kspt(BAD_KEY, BAD_KEY).is_err());
    assert!(ready(pskt_finalize_and_broadcast(BAD_KEY, WS)).is_err());
    assert!(generate_qr_frames(BAD_KEY).is_err());
    assert!(decode_qr_frame(BAD_KEY).is_err());
    assert!(ready(create_send_pskb_with_utxos(
        "not-json",
        BAD_ADDRESS,
        1,
        1,
        "[]"
    ))
    .is_err());
}

#[test]
fn deterministic_exports_succeed_on_valid_input() {
    let fee = estimate_covenant_fee(1, 40, 0, 0).expect("covenant fee estimate");
    assert!(fee.parse::<u64>().expect("decimal fee") > 0);
    let blake = blake2b_hash("00").expect("blake2b");
    assert_eq!(blake.len(), 64);
    let sha = sha256_hash("").expect("sha256");
    assert_eq!(
        sha,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert!(covenant_oracle_mb_heartbeat(NETWORK).is_ok());
    assert!(stealth_announcement_address(NETWORK).starts_with("kaspa:"));
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
    assert!(private_swap_insert_completed_signature(BAD_KEY, BAD_KEY, BAD_KEY).is_err());
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
