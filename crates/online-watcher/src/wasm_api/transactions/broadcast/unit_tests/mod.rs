use super::*;
use crate::wasm_api::test_support::ready;

#[test]
fn broadcast_boundary_rejects_malformed_signed_transactions_before_network_io() {
    assert_eq!(
        ready(broadcast_signed("00", "ws://unused"))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("KSPT is truncated"))"#)
    );
}
