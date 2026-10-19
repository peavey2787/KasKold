use super::*;
use crate::wasm_api::test_support::ready;

#[test]
fn commit_reveal_spend_boundary_rejects_bad_json_before_network_io() {
    assert_eq!(
        ready(create_commit_reveal_spend("not-json"))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Bad commit-reveal spend request JSON: expected ident at line 1 column 2"))"#
        )
    );
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn valid_commit_reveal_spend_reaches_native_transport_fail_closed() {
    use crate::wasm_api::test_support::ready;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let address = |byte: u8| crate::account::address::encode_p2pk_address(&[byte; 32], "kaspa");
    let request = serde_json::json!({
        "covenant_address": address(0x71),
        "dest_address": address(0x72),
        "redeem_script_hex": "51",
        "part_a_hex": "aa",
        "part_b_hex": "bb",
        "payload_hex": "cc",
        "fee": "300000",
        "ws_url": "ws://unused"
    })
    .to_string();

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        ready(create_commit_reveal_spend(&request))
    }));
    assert!(
        matches!(outcome, Ok(Err(_)) | Err(_)),
        "native commit-reveal transport boundary unexpectedly succeeded"
    );
}

#[test]
fn reveal_reader_reports_the_preimage_and_both_hashes() {
    use kaspa_portal::contract::{commit_reveal::build_commit_reveal_script, script::push_data};
    use kaspa_portal::primitives::hash::blake2b_256;

    let preimage = b"saltsaltsecret";
    let redeem = build_commit_reveal_script(&[3; 32], &blake2b_256(preimage), 900);
    let mut script = Vec::new();
    for push in [&preimage[..10], &preimage[10..], &[9; 65][..]] {
        push_data(&mut script, push);
    }
    script.push(0);
    push_data(&mut script, &redeem);
    let read: serde_json::Value =
        serde_json::from_str(&read_reveal_json(&hex::encode(&script)).unwrap()).unwrap();
    assert_eq!(read["preimage_hex"], hex::encode(preimage));
    assert_eq!(read["matches"], true);
    assert_eq!(read["committed_hash"], read["computed_hash"]);
    assert_eq!(
        read_reveal_json("51")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("not a commit-reveal reveal signature script")
    );
}
