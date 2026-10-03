#[cfg(not(target_arch = "wasm32"))]
#[test]
fn shipping_borrower_exports_fail_closed_until_their_witness_plans_are_typed() {
    use crate::wasm_api::test_support::ready;

    let spend = ready(super::deposit::create_covenant_borrower_spend())
        .expect_err("borrower spend stays disabled");
    assert!(format!("{spend:?}").contains("disabled"));
    let withdraw = ready(super::withdraw::create_covenant_borrower_withdraw())
        .expect_err("borrower withdraw stays disabled");
    assert!(format!("{withdraw:?}").contains("disabled"));
}
