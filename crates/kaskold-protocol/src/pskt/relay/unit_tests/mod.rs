use super::*;
use serde_json::json;

#[test]
fn stealth_tweak_discovery_covers_absent_invalid_duplicate_and_conflict_paths() {
    assert_eq!(find_stealth(&[]).unwrap(), None);
    assert_eq!(find_stealth(&[json!({})]).unwrap(), None);
    assert!(find_stealth(&[json!({"proprietaries": []})]).is_err());
    assert!(find_stealth(&[json!({"proprietaries": {"stealthTweak": 1}})]).is_err());
    assert!(find_stealth(&[json!({"proprietaries": {"stealthTweak": "00"}})]).is_err());

    let one = "11".repeat(32);
    let two = "22".repeat(32);
    assert_eq!(
        find_stealth(&[json!({"proprietaries": {"stealthTweak": one.clone()}})]).unwrap(),
        Some([0x11; 32])
    );
    assert_eq!(
        find_stealth(&[
            json!({"proprietaries": {"stealthTweak": one.clone()}}),
            json!({"proprietaries": {"stealthTweak": one}}),
        ])
        .unwrap(),
        Some([0x11; 32])
    );
    assert!(find_stealth(&[
        json!({"proprietaries": {"stealthTweak": "11".repeat(32)}}),
        json!({"proprietaries": {"stealthTweak": two}}),
    ])
    .is_err());
}

#[test]
fn first_outpoint_covers_every_required_shape_and_numeric_boundary() {
    assert_eq!(first_outpoint(&[]).unwrap(), None);
    assert!(first_outpoint(&[json!(1)]).is_err());
    assert!(first_outpoint(&[json!({})]).is_err());
    assert!(first_outpoint(&[json!({"previousOutpoint": 1})]).is_err());
    assert!(first_outpoint(&[json!({"previousOutpoint": {"index": 0}})]).is_err());
    assert!(
        first_outpoint(&[json!({"previousOutpoint": {"transactionId": 1, "index": 0}})]).is_err()
    );
    assert!(
        first_outpoint(&[json!({"previousOutpoint": {"transactionId": "00", "index": 0}})])
            .is_err()
    );
    assert!(
        first_outpoint(&[json!({"previousOutpoint": {"transactionId": "11".repeat(32)}})]).is_err()
    );
    assert!(first_outpoint(&[json!({"previousOutpoint": {"transactionId": "11".repeat(32), "index": u64::from(u32::MAX) + 1}})]).is_err());

    let parsed = first_outpoint(&[json!({
        "previousOutpoint": {
            "transactionId": "11".repeat(32),
            "index": "7"
        }
    })])
    .expect("valid outpoint")
    .expect("present outpoint");
    assert_eq!(parsed, ([0x11; 32], 7));
}

#[test]
fn transaction_payload_decoding_covers_default_hex_type_and_capability_limits() {
    let global = |payload: Option<serde_json::Value>| {
        let mut map = serde_json::Map::new();
        if let Some(payload) = payload {
            map.insert("txPayload".to_string(), payload);
        }
        map
    };
    assert_eq!(decode_payload(&global(None)), Ok(Vec::new()));
    assert_eq!(decode_payload(&global(Some(json!(null)))), Ok(Vec::new()));
    assert_eq!(
        decode_payload(&global(Some(json!("0aff")))),
        Ok(vec![0x0a, 0xff])
    );
    assert!(decode_payload(&global(Some(json!("0AFF")))).is_err());
    assert_eq!(
        decode_payload(&global(Some(json!(7)))),
        Err("txPayload must be a hex string or null".to_string())
    );
    let limit = usize::from(crate::SIGNER_CAPABILITIES.max_payload_bytes);
    assert_eq!(
        decode_payload(&global(Some(json!("00".repeat(limit))))).map(|payload| payload.len()),
        Ok(limit)
    );
    let error = decode_payload(&global(Some(json!("00".repeat(limit + 1))))).unwrap_err();
    assert!(error.contains("exceeds signer capabilities"), "{error}");
}
