mod spend_paths;
mod typed_spend_paths;
mod wire_vectors;

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
fn automatic_covenant_sweep_fails_closed_without_a_browser_transport() {
    use super::sweep::{build_automatic, CovenantSweepConfig, CovenantSweepSpec};
    use crate::wasm_api::test_support::ready;

    let covenant = crate::wasm_api::test_support::covenant_address(&[0x51]);
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
            path: super::execution::SpendPath::Owner,
        },
    };
    let error =
        ready(build_automatic("ws://unused", spec)).expect_err("native host has no transport");
    assert!(error.contains("unavailable on native hosts"), "{error}");
}
