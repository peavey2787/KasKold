use super::*;

#[test]
fn private_swap_claim_builder_returns_real_pskb_wire() {
    let destination = crate::account::address::encode_p2pk_address(&[0x32; 32], "kaspa");
    let redeem = crate::contracts::covenant::script::build_private_swap_script(
        &[0x31; 32],
        &[0x33; 32],
        &crate::account::address::address_to_script_pubkey(&destination).expect("destination"),
        100,
        &[0x34; 16],
    )
    .expect("swap script");
    let source = crate::wasm_api::test_support::covenant_address(&redeem);
    let selected = serde_json::json!([{
        "tx_id": "11".repeat(32), "index": 0, "amount": "20000000"
    }])
    .to_string();
    let wire = build_claim(
        &source,
        &destination,
        &hex::encode(&redeem),
        &selected,
        1_000,
    )
    .expect("claim");
    let decoded = hex::decode(wire).expect("wire hex");
    assert_eq!(&decoded[..4], b"PSKB");
}
