use super::*;
use crate::wasm_api::test_support::ready;

const KPUB_A: &str = "kpub1:038f332e03405ab68380000000f0453f0894cc8c84ebf6e6208e0c7916e9ddbd14919f9bbb92b0690b4e353392020327c7136972883eab5a7722ec3d4302f888804ecce61658ae962a2c56bb7571";
const KPUB_B: &str = "kpub1:038f332e03a7457270800000002908be01d75735944f29befbdbcd173ab00df2d44c6d5ab51a839413fda90cbf035b986b584de244f5d6a1939192f676a9f2992a63b0f43cdc452dcb40d9dd7081";

fn hd45_descriptor() -> String {
    format!("multi_hd45(1,{KPUB_A},{KPUB_B})")
}

fn hd45_source(descriptor_text: &str) -> String {
    let descriptor = kaspa_portal::wallet::multisig::MultisigDescriptor::parse(descriptor_text)
        .expect("45' descriptor");
    let keys = descriptor
        .public_keys_at(0, 0, 0)
        .expect("45' receive keys");
    let redeem = kaspa_portal::wallet::multisig::build_redeem_script(descriptor.threshold(), &keys)
        .expect("45' redeem script");
    crate::protocol::script::p2sh::script_to_address(&redeem, "kaspa").expect("45' source address")
}

fn request() -> MultisigApiRequest {
    MultisigApiRequest {
        descriptor: format!("multi(1,{})", "11".repeat(32)),
        source_address: kaskold_protocol::encode_p2sh_address(&[0x21; 32], "kaspa"),
        dest_address: crate::account::address::encode_p2pk_address(&[0x22; 32], "kaspa"),
        amount_sompi: "0".to_string(),
        fee_sompi: "300000".to_string(),
        change_address: kaskold_protocol::encode_p2sh_address(&[0x21; 32], "kaspa"),
        ws_url: "ws://unused".to_string(),
        addr_index: 0,
        change_index_hint: 0,
        utxo_csv: "0".to_string(),
    }
}

#[test]
fn multisig_transaction_boundaries_are_host_testable() {
    let request = request();
    let automatic = request.automatic().expect("automatic request conversion");
    assert_eq!(
        ready(create_multisig(automatic))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Need at least M and 2 cosigners"))"#)
    );

    assert_eq!(
        ready(create_multisig_pskb("not-json"))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Bad multisig request JSON: expected ident at line 1 column 2"))"#)
    );
    assert_eq!(
        ready(create_multisig_pskb_selected("not-json"))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Bad selected multisig request JSON: expected ident at line 1 column 2"))"#
        )
    );

    let valid = serde_json::json!({
        "descriptor": request.descriptor,
        "source_address": request.source_address,
        "dest_address": request.dest_address,
        "amount_sompi": "10000000",
        "fee_sompi": "300000",
        "change_address": request.change_address,
        "ws_url": "ws://unused",
        "addr_index": 0,
        "utxo_csv": "0"
    })
    .to_string();
    assert_eq!(
        ready(create_multisig_pskb_selected(&valid))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Need at least M and 2 cosigners"))"#)
    );
}

#[test]
fn restored_hd45_wasm_exports_are_host_testable() {
    assert_eq!(
        ready(scan_multisig_branch_js("not-json"))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Bad multisig branch scan JSON: expected ident at line 1 column 2"))"#
        )
    );
    assert_eq!(
        ready(create_multisig_pskb_multi_js("not-json"))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Bad multi-address multisig request JSON: expected ident at line 1 column 2"))"#
        )
    );

    let static_scan = serde_json::json!({
        "descriptor": format!("multi(1,{},{})", "11".repeat(32), "22".repeat(32)),
        "cosigner_index": 0,
        "depth": 1,
        "ws_url": "ws://unused",
        "address_prefix": "kaspa"
    })
    .to_string();
    assert_eq!(
        ready(scan_multisig_branch_js(&static_scan))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
        )
    );

    let descriptor = hd45_descriptor();
    let source = hd45_source(&descriptor);
    let scan = serde_json::json!({
        "descriptor": descriptor,
        "cosigner_index": 0,
        "depth": 2,
        "ws_url": "ws://unused",
        "address_prefix": "kaspa"
    })
    .to_string();
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(
        ready(scan_multisig_branch_js(&scan))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
        )
    );

    let sources_json = serde_json::json!([{
        "address": source.clone(),
        "tx_id": "11".repeat(32),
        "index": 0
    }])
    .to_string();
    let multi = serde_json::json!({
        "descriptor": hd45_descriptor(),
        "sources_json": sources_json,
        "dest_address": source,
        "amount_sompi": "1000000",
        "fee_sompi": "1000",
        "cosigner_index": 0,
        "change_index_hint": 0,
        "ws_url": "ws://unused"
    })
    .to_string();
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(
        ready(create_multisig_pskb_multi_js(&multi))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
        )
    );

    let bad_amount = multi.replace("\"1000000\"", "\"not-a-number\"");
    assert_eq!(
        ready(create_multisig_pskb_multi_js(&bad_amount))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("Invalid amount_sompi: invalid digit found in string"))"#)
    );
}

#[test]
fn descriptor_summary_reports_kind_threshold_and_participants() {
    let summary: serde_json::Value =
        serde_json::from_str(&multisig_descriptor_summary_string(&hd45_descriptor()).unwrap())
            .unwrap();
    assert_eq!(summary["kind"], "hd45");
    assert_eq!(summary["threshold"], 1);
    assert_eq!(summary["participants"], 2);
    assert_eq!(
        multisig_descriptor_summary_string("not a descriptor")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Descriptor must be multi(M,...), multi_hd(M,...), or multi_hd45(M,...)")
    );
}

#[test]
fn descriptor_summary_export_names_every_supported_kind() {
    let kind = |text: &str| {
        let json = crate::wasm_api::test_support::expect_js(
            multisig_descriptor_summary(text),
            "descriptor summary",
        );
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["kind"].clone()
    };
    let static_keys = format!("{},{}", "11".repeat(32), "22".repeat(32));
    assert_eq!(kind(&format!("multi(1,{static_keys})")), "static");
    let hd44_keys = format!("02{},03{}", "31".repeat(64), "52".repeat(64));
    assert_eq!(kind(&format!("multi_hd(1,{hd44_keys})")), "hd44");
    assert_eq!(kind(&hd45_descriptor()), "hd45");
    assert_eq!(
        multisig_descriptor_summary("not a descriptor")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"JsValue(Some("Descriptor must be multi(M,...), multi_hd(M,...), or multi_hd45(M,...)"))"#
        )
    );
}
