use crate::wasm_api::test_support::ready;

fn address(byte: u8, prefix: &str) -> String {
    crate::account::address::encode_p2pk_address(&[byte; 32], prefix)
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn allowance_automatic_wasm_facade_reaches_native_transport() {
    let owner = super::key(0x11);
    let beneficiary = super::key(0x22);
    let covenant_json = crate::contracts::covenant::construction::allowance::build_local_json(
        &owner,
        &beneficiary,
        20_000_000,
        1,
        0,
        "mainnet",
    )
    .expect("allowance covenant");
    let covenant: serde_json::Value = serde_json::from_str(&covenant_json).unwrap();
    let covenant_address = covenant["address"].as_str().unwrap();
    let redeem = covenant["redeem_script_hex"].as_str().unwrap();
    let destination = address(0x73, "kaspa");
    assert!(
        ready(super::super::allowance::create_covenant_allowance_withdraw(
            covenant_address,
            &destination,
            redeem,
            10_000_000,
            1,
            "ws://unused",
        ))
        .is_err()
    );
}
