use crate::UtxoEntry;

fn utxo(byte: u8, amount: u64) -> UtxoEntry {
    UtxoEntry {
        tx_id: format!("{byte:02x}").repeat(32),
        index: u32::from(byte),
        amount,
        script_public_key: vec![0x20; 34],
        block_daa_score: 0,
        covenant_id: None,
    }
}

#[test]
fn covenant_dust_policy_maps_only_explicit_fold_types() {
    use super::{dust_policy_for, CovenantDustPolicy};

    for covenant_type in [
        "additive",
        "timelocked-savings",
        "dms",
        "global-spending-limit",
        "global-allowance",
    ] {
        assert_eq!(
            dust_policy_for(covenant_type),
            CovenantDustPolicy::FoldSubKip9Change,
            "{covenant_type} must fold sub-KIP-9 manual change"
        );
    }

    for covenant_type in ["", "allowance", "crowdfund", "private-swap", "unknown"] {
        assert_eq!(
            dust_policy_for(covenant_type),
            CovenantDustPolicy::Preserve,
            "{covenant_type} must preserve explicit change"
        );
    }
}

#[test]
fn allowance_remote_boundary_is_host_exercised_and_fails_closed() {
    use super::allowance::build_remote;
    use crate::{account::address, wasm_api::test_support::ready};

    let covenant = address::encode_p2pk_address(&[0x43; 32], "kaspa");
    let destination = address::encode_p2pk_address(&[0x44; 32], "kaspa");
    assert!(matches!(
        ready(build_remote(
            &covenant,
            &destination,
            "51",
            20_000_000,
            1_000_000,
            "wss://host-coverage.invalid",
        )),
        Err(error) if error.contains("browser WebSocket transport is unavailable on native hosts")
    ));
}

#[test]
fn allowance_remote_result_composes_fetch_success_and_failure() {
    use super::allowance::build_remote_result;
    use crate::account::address;

    let covenant = address::encode_p2pk_address(&[0x41; 32], "kaspa");
    let destination = address::encode_p2pk_address(&[0x42; 32], "kaspa");
    let values = vec![utxo(10, 50_000_000)];

    let withdrawal = build_remote_result(
        Ok(values),
        &covenant,
        &destination,
        "51",
        20_000_000,
        1_000_000,
    )
    .expect("fetched allowance UTXOs compose into a withdrawal");
    assert_eq!(withdrawal.total_balance, 50_000_000);
    assert_eq!(withdrawal.return_amount, 29_000_000);

    assert!(matches!(
        build_remote_result(
            Err("transport failed".to_string()),
            &covenant,
            &destination,
            "51",
            20_000_000,
            1_000_000,
        ),
        Err(error) if error == "transport failed"
    ));
}

#[test]
fn allowance_prepare_material_covers_validation_error_paths() {
    use super::allowance::prepare_material;
    use crate::account::address;

    let covenant = address::encode_p2pk_address(&[0x51; 32], "kaspa");
    let destination = address::encode_p2pk_address(&[0x52; 32], "kaspa");
    let funded = [utxo(12, 50_000_000)];

    assert!(matches!(
        prepare_material(&covenant, &destination, "zz", 1, 0, &funded),
        Err(error) if error.contains("Bad redeem hex")
    ));
    assert!(matches!(
        prepare_material(&covenant, &destination, "51", u64::MAX, 1, &[utxo(13, u64::MAX)]),
        Err(error) if error.contains("overflows u64")
    ));
    assert!(matches!(
        prepare_material(
            &covenant,
            &destination,
            "51",
            1,
            0,
            &[utxo(14, u64::MAX), utxo(15, 1)],
        ),
        Err(error) if error.contains("balance overflows u64")
    ));
    assert!(prepare_material("not-an-address", &destination, "51", 1, 0, &funded).is_err());
    assert!(prepare_material(&covenant, "not-an-address", "51", 1, 0, &funded).is_err());
}

#[test]
fn allowance_prepare_material_has_host_native_coverage() {
    use super::allowance::prepare_material;
    use crate::account::address;

    let covenant = address::encode_p2pk_address(&[0x31; 32], "kaspa");
    let destination = address::encode_p2pk_address(&[0x32; 32], "kaspa");
    let values = [utxo(9, 50_000_000)];

    assert!(prepare_material(
        &covenant,
        &destination,
        "51",
        20_000_000,
        1_000_000,
        &values,
    )
    .is_ok());

    assert!(prepare_material(&covenant, &destination, "51", 1, 1, &[]).is_err());
    assert!(matches!(
        prepare_material(
            &covenant,
            &destination,
            "51",
            49_500_000,
            1_000_000,
            &values,
        ),
        Err(error) if error.contains("total balance")
    ));
    assert!(matches!(
        prepare_material(
            &covenant,
            &destination,
            "51",
            49_000_001,
            0,
            &values,
        ),
        Err(error) if error.contains("too small")
    ));
}

#[test]
fn automatic_covenant_sweep_fails_closed_without_a_browser_transport() {
    use super::sweep::{build_automatic, CovenantSweepConfig, CovenantSweepSpec};
    use crate::wasm_api::test_support::ready;

    let covenant = crate::account::address::encode_p2pk_address(&[0x61; 32], "kaspa");
    let destination = crate::account::address::encode_p2pk_address(&[0x62; 32], "kaspa");
    let spec = CovenantSweepSpec {
        covenant_address: &covenant,
        destination_address: &destination,
        fee: 1_000_000,
        empty_error: "no covenant UTXOs",
        low_balance_error: "covenant balance too low",
        config: CovenantSweepConfig {
            redeem_script: &[0x51],
            input_sequence: 0,
            lock_time: 0,
            branch: None,
            minimum_signatures: None,
        },
    };
    let error =
        ready(build_automatic("ws://unused", spec)).expect_err("native host has no transport");
    assert!(error.contains("unavailable on native hosts"), "{error}");
}
