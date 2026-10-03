#[cfg(not(target_arch = "wasm32"))]
#[test]
fn oracle_publish_export_fails_closed_until_its_witness_plan_is_typed() {
    use crate::wasm_api::test_support::ready;

    let error = ready(super::create_oracle_mb_publish("{}")).expect_err("publish stays disabled");
    assert!(format!("{error:?}").contains("disabled"));
}
