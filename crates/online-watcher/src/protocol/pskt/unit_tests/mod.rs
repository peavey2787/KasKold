// KasKold Companion Web — organized PSKT subsystem
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use serde_json::Value;

use super::wire::decode_root;
use super::*;

mod consensus_finalizer;
mod exact_json;
mod kspt_bridge;
mod kspt_compact;
mod review;
mod review_boundaries;

#[test]
fn detects_supported_wire_magics() {
    assert_eq!(detect_format_hex("50534b42"), PsktFormat::Pskb);
    assert_eq!(detect_format_hex("50534b54"), PsktFormat::PsktSingle);
    assert_eq!(detect_format_hex("4b535054"), PsktFormat::Unknown);
}

#[test]
fn transaction_lane_mutation_sets_all_lane_fields() {
    let wire = pskb_wire(serde_json::json!([{
        "global": {},
        "inputs": [],
        "outputs": []
    }]));
    let subnetwork_id = "11".repeat(20);

    let result = set_tx_lane(&wire, &subnetwork_id, 42, 1, &[0xde, 0xad]).unwrap();
    let (_, root) = decode_root(&result).unwrap();
    let global = &root[0]["global"];

    assert_eq!(global["subnetworkId"], Value::String(subnetwork_id));
    assert_eq!(global["gas"], Value::from(42));
    assert_eq!(global["txVersion"], Value::from(1));
    assert_eq!(global["txPayload"], Value::String("dead".into()));
}

#[test]
fn transaction_lane_and_payload_mutation_reject_invalid_envelopes_and_subnetworks() {
    assert!(set_tx_lane("4b535054", &"11".repeat(20), 0, 0, &[])
        .unwrap_err()
        .contains("not a PSKB"));

    let wire = pskb_wire(serde_json::json!([{
        "global": {},
        "inputs": [],
        "outputs": []
    }]));
    assert!(set_tx_lane(&wire, "zz", 0, 0, &[])
        .unwrap_err()
        .contains("lowercase hexadecimal"));
    assert!(set_tx_lane(&wire, &"11".repeat(19), 0, 0, &[])
        .unwrap_err()
        .contains("exactly 20 bytes"));

    let missing_global = pskb_wire(serde_json::json!([{
        "inputs": [],
        "outputs": []
    }]));
    assert!(set_tx_lane(&missing_global, &"11".repeat(20), 0, 0, &[])
        .unwrap_err()
        .contains("missing PSKT.global"));
}

fn pskb_wire(document: Value) -> String {
    let document = canonical_test_pskt(document);
    let json = serde_json::to_vec(&document).unwrap();
    let mut wire = b"PSKB".to_vec();
    wire.extend_from_slice(hex::encode(json).as_bytes());
    hex::encode(wire)
}

pub(super) fn canonical_test_pskt(mut value: Value) -> Value {
    fn normalize_one(pskt: &mut Value) {
        let input_count = pskt
            .get("inputs")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let output_count = pskt
            .get("outputs")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        if let Some(global) = pskt.get_mut("global").and_then(Value::as_object_mut) {
            global.entry("version").or_insert(Value::from(0u8));
            global.entry("txVersion").or_insert(Value::from(0u8));
            global
                .entry("inputCount")
                .or_insert(Value::from(input_count));
            global
                .entry("outputCount")
                .or_insert(Value::from(output_count));
        }
        if let Some(inputs) = pskt.get_mut("inputs").and_then(Value::as_array_mut) {
            for input in inputs {
                if let Some(obj) = input.as_object_mut() {
                    obj.entry("sighashType").or_insert(Value::from(1u8));
                    if !obj.get("proprietaries").is_some_and(Value::is_object) {
                        obj.insert(
                            "proprietaries".into(),
                            Value::Object(serde_json::Map::new()),
                        );
                    }
                }
            }
        }
        if let Some(outputs) = pskt.get_mut("outputs").and_then(Value::as_array_mut) {
            for output in outputs {
                if let Some(obj) = output.as_object_mut() {
                    if !obj.get("proprietaries").is_some_and(Value::is_object) {
                        obj.insert(
                            "proprietaries".into(),
                            Value::Object(serde_json::Map::new()),
                        );
                    }
                }
            }
        }
    }
    match &mut value {
        Value::Array(items) => items.iter_mut().for_each(normalize_one),
        Value::Object(_) => normalize_one(&mut value),
        _ => {}
    }
    value
}

mod wire;

#[test]
fn transaction_lane_rejects_uppercase_subnetworks_and_oversized_payloads() {
    let wire = pskb_wire(serde_json::json!([{"global": {}, "inputs": [], "outputs": []}]));
    assert!(set_tx_lane(&wire, &"AA".repeat(20), 0, 0, &[])
        .unwrap_err()
        .contains("lowercase hexadecimal"));
    let oversized =
        vec![0u8; usize::from(kaskold_protocol::SIGNER_CAPABILITIES.max_payload_bytes) + 1];
    assert!(set_tx_lane(&wire, &"00".repeat(20), 0, 0, &oversized)
        .unwrap_err()
        .contains("exceeds signer capability"));
}
