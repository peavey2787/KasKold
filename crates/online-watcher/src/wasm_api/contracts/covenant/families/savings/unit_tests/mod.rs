use super::SavingsClaimPlan;
use crate::transaction_builder::covenant::sweep::SweepSourceKind;

#[test]
fn savings_claim_plan_construction_and_specs_are_host_testable() {
    let plan = SavingsClaimPlan::new("51", 99).expect("savings plan");
    let automatic = plan.spec("source", "destination", 11, SweepSourceKind::Automatic);
    assert_eq!(automatic.fee, 11);
    assert_eq!(automatic.config.lock_time, 99);
    assert_eq!(automatic.config.branch, Some("savings"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn savings_automatic_claim_wasm_facade_rejects_bad_redeem() {
    let result = crate::wasm_api::test_support::ready(
        super::create_covenant_timelocked_savings_claim("bad", "bad", "zz", 1, 1, "ws://unused"),
    );
    assert_eq!(
        result.err().map(|error| format!("{error:?}")).as_deref(),
        Some(r#"JsValue(Some("Bad redeem hex: Invalid character 'z' at position 0"))"#)
    );
}
