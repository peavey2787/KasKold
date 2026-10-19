use super::*;
use crate::wasm_api::test_support::ready;

fn contribution(address: String, redeem: &str) -> ContributionRef {
    ContributionRef {
        address,
        contributor_pubkey_hex: "11".repeat(32),
        redeem_script_hex: redeem.to_string(),
        crowdfund_salt_hex: "22".repeat(8),
    }
}

fn contribution_json(count: usize) -> String {
    serde_json::to_string(
        &(0..count)
            .map(|index| {
                serde_json::json!({
                    "address": format!("kaspa:item{index}"),
                    "contributor_pubkey_hex": "11".repeat(32),
                    "redeem_script_hex": "51",
                    "crowdfund_salt_hex": "22".repeat(8),
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

fn invalid_request<'a>(json: &'a str) -> CrowdfundSweepRequest<'a> {
    CrowdfundSweepRequest {
        contributions_json: json,
        organizer_address: "bad",
        goal_sompi: 1,
        locktime_daa: 1,
        verifying_key_hex: "00",
        proof_hex: "00",
        public_input_hex: "00",
        requested_fee: 1,
        fetched: Vec::new(),
    }
}

#[test]
fn crowdfund_async_entrypoints_preserve_validation_errors() {
    assert_eq!(
        ready(inspect_crowdfund_contributions_string(
            "not-json",
            "ws://unused"
        ))
        .err()
        .map(|error| error.to_string())
        .as_deref(),
        Some("Invalid crowdfunding contribution JSON: expected ident at line 1 column 2")
    );
    assert_eq!(
        ready(create_crowdfund_sweep_string(
            "not-json",
            "bad",
            1,
            1,
            "00",
            "00",
            "00",
            1,
            "ws://unused"
        ))
        .err()
        .map(|error| error.to_string())
        .as_deref(),
        Some("Invalid crowdfunding contribution JSON: expected ident at line 1 column 2")
    );
    assert_eq!(
        ready(submit_crowdfund_sweep(
            invalid_request("not-json"),
            "ws://unused"
        ))
        .err()
        .map(|error| error.to_string())
        .as_deref(),
        Some("Invalid crowdfunding contribution JSON: expected ident at line 1 column 2")
    );
    assert_eq!(
        ready(fetch_contributions_json("not-json", "ws://unused"))
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Invalid crowdfunding contribution JSON: expected ident at line 1 column 2")
    );
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(
        ready(fetch_contributions(
            &[contribution("kaspa:invalid".into(), "51")],
            "ws://unused"
        ))
        .err()
        .map(|error| error.to_string())
        .as_deref(),
        Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)")
    );
}

#[test]
fn canonical_contribution_checks_address_and_redeem_independently() {
    let canonical = [0x51];
    let address = crate::protocol::script::p2sh::script_to_address(&canonical, "kaspa").unwrap();
    assert_eq!(
        validate_canonical_contribution(&contribution(address, "52"), &canonical)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding contribution does not match the canonical campaign covenant")
    );
    let other = crate::protocol::script::p2sh::script_to_address(&[0x52], "kaspa").unwrap();
    assert_eq!(
        validate_canonical_contribution(&contribution(other, "51"), &canonical)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding contribution does not match the canonical campaign covenant")
    );
}

#[test]
fn contribution_count_sweep_totals_fee_and_hex_boundaries_are_exact() {
    assert_eq!(
        parse_contributions("[]")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding requires 1..=8 contribution addresses")
    );
    assert_eq!(
        parse_contributions(&contribution_json(CROWDFUND_MAX_CONTRIBUTORS))
            .unwrap()
            .len(),
        CROWDFUND_MAX_CONTRIBUTORS
    );
    assert_eq!(
        parse_contributions(&contribution_json(CROWDFUND_MAX_CONTRIBUTORS + 1))
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding requires 1..=8 contribution addresses")
    );
    let duplicate = serde_json::json!([
        {"address":"kaspa:x","contributor_pubkey_hex":"11","redeem_script_hex":"51","crowdfund_salt_hex":"22"},
        {"address":"kaspa:x","contributor_pubkey_hex":"11","redeem_script_hex":"51","crowdfund_salt_hex":"22"}
    ]).to_string();
    assert_eq!(
        parse_contributions(&duplicate)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Duplicate crowdfunding contribution address")
    );

    let total = 100u64;
    let public = proof::serialize_total(total).unwrap();
    assert_eq!(validate_sweep_totals(total, total, 1, &public), Ok(()));
    let below = proof::serialize_total(total - 1).unwrap();
    assert_eq!(
        validate_sweep_totals(total - 1, total, 1, &below)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding goal has not been reached")
    );
    assert_eq!(
        validate_sweep_totals(total, total, 0, &public)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding sweep supports at most 16 transaction inputs")
    );
    assert_eq!(
        validate_sweep_totals(total, total, CROWDFUND_MAX_TX_INPUTS as usize, &public),
        Ok(())
    );
    assert_eq!(
        validate_sweep_totals(total, total, CROWDFUND_MAX_TX_INPUTS as usize + 1, &public)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding sweep supports at most 16 transaction inputs")
    );

    let expected_floor = cost::groth16_min_fee_sompi(1).checked_mul(12).unwrap() / 10;
    assert_eq!(calculate_fee(0, 1), Ok(expected_floor));
    assert_eq!(
        calculate_fee(CROWDFUND_MAX_SWEEP_FEE_SOMPI, 1),
        Ok(CROWDFUND_MAX_SWEEP_FEE_SOMPI)
    );
    assert_eq!(
        calculate_fee(CROWDFUND_MAX_SWEEP_FEE_SOMPI + 1, 1)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding fee exceeds the on-chain 500000000 sompi safety ceiling")
    );

    assert_eq!(
        decode_hex_bounded("aabb", "field", 2).unwrap(),
        vec![0xaa, 0xbb]
    );
    assert_eq!(
        decode_hex_bounded("aabbcc", "field", 2)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("field exceeds the supported size limit")
    );
}

#[test]
fn contribution_fetch_results_require_utxos_at_every_address() {
    let reference = contribution("kaspa:campaign".into(), "51");
    let Err(error) = require_nonempty_contribution(&reference, Vec::new()) else {
        panic!("an unfunded contribution address must be rejected");
    };
    assert_eq!(error, "No UTXOs at crowdfunding address kaspa:campaign");
    let utxo = UtxoEntry {
        tx_id: "33".repeat(32),
        index: 0,
        amount: 10,
        script_public_key: vec![0x51],
        block_daa_score: 0,
        covenant_id: None,
    };
    let Ok((kept, utxos)) = require_nonempty_contribution(&reference, vec![utxo.clone()]) else {
        panic!("a funded contribution address must be kept");
    };
    assert_eq!(kept.address, "kaspa:campaign");
    assert_eq!(utxos, vec![utxo]);
}

#[test]
fn sweep_totals_must_match_the_proven_public_total() {
    let proven = proof::serialize_total(99).unwrap();
    assert_eq!(
        validate_sweep_totals(100, 100, 1, &proven),
        Err("Crowdfunding proof public total does not match the actual UTXO total".to_string())
    );
}

#[test]
fn sweep_material_rejects_undecodable_proof_and_public_input() {
    let mut request = invalid_request("[]");
    request.proof_hex = "zz";
    let Err(error) = verified_sweep_material(&request) else {
        panic!("an undecodable proof must be rejected");
    };
    assert!(error.contains("crowdfunding proof"), "{error}");
    request.proof_hex = "00";
    request.public_input_hex = "zz";
    let Err(error) = verified_sweep_material(&request) else {
        panic!("an undecodable public input must be rejected");
    };
    assert!(error.contains("crowdfunding public input"), "{error}");
}

#[test]
fn contribution_totals_reject_u64_overflow() {
    let utxo = |amount| UtxoEntry {
        tx_id: "33".repeat(32),
        index: 0,
        amount,
        script_public_key: vec![0, 0, 0x51],
        block_daa_score: 0,
        covenant_id: None,
    };
    assert_eq!(checked_total(&[utxo(4), utxo(5)]), Ok(9));
    assert_eq!(
        checked_total(&[utxo(u64::MAX), utxo(1)]),
        Err("Crowdfunding balance overflow".to_string())
    );
}
