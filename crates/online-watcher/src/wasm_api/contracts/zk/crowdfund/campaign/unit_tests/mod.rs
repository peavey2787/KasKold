use super::*;

#[test]
fn setup_json_validation_is_host_testable_without_wasm_errors() {
    let generated =
        crate::wasm_api::test_support::expect_js(zk_crowdfund_setup(), "host setup wrapper");
    let generated_value: serde_json::Value = serde_json::from_str(&generated).unwrap();
    assert!(generated_value["vk_len"].as_u64().unwrap() > 0);
    let encoded = encode_setup_json(vec![1, 2, 3], vec![4, 5]).expect("setup json");
    let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(value["vk_len"], 2);
    assert_eq!(
        encode_setup_json(vec![1], Vec::new())
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding verifying key exceeds the supported size limit")
    );
    assert_eq!(
        encode_setup_json(vec![0; MAX_CROWDFUND_PK_BYTES + 1], vec![1])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding proving key exceeds the supported size limit")
    );
    assert_eq!(
        encode_setup_json(vec![1], vec![0; MAX_CROWDFUND_VK_BYTES + 1])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Crowdfunding verifying key exceeds the supported size limit")
    );
    assert!(encode_setup_json(vec![0; MAX_CROWDFUND_PK_BYTES], vec![1]).is_ok());
    assert!(encode_setup_json(vec![1], vec![0; MAX_CROWDFUND_VK_BYTES]).is_ok());
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

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn campaign_wasm_facade_paths_are_host_exercised_fail_closed() {
    assert_eq!(
        zk_crowdfund_prove("00", "00", "not-json")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Invalid crowdfunding amount JSON: expected ident at line 1 column 2"))"#
        )
    );
    assert_eq!(
        crowdfund_campaign_id("bad-address", 1, 1, "00")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Unknown address prefix"))"#)
    );
    assert_eq!(
        covenant_crowdfund("00", "bad-address", 1, 1, "00", "mainnet")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Pubkey must be 32 bytes"))"#)
    );
}
