use serde_json::json;

use super::explicit_utxos::parse_explicit_utxos_string;

fn utxo(script: serde_json::Value) -> serde_json::Value {
    json!({
        "tx_id": "11".repeat(32),
        "index": 2,
        "amount": 3,
        "script_public_key": script,
        "block_daa_score": 4
    })
}

#[test]
fn explicit_utxo_parser_accepts_hex_and_bytes_and_rejects_invalid_shapes() {
    let script_hex = format!("20{}ac", "22".repeat(32));
    let parsed = parse_explicit_utxos_string(
        &json!([utxo(json!(script_hex)), utxo(json!([0, 1, 255]))]).to_string(),
    )
    .unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].index, 2);
    assert_eq!(parsed[1].script_public_key, vec![0, 1, 255]);

    assert_eq!(
        parse_explicit_utxos_string("not-json")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Bad UTXOs JSON: expected ident at line 1 column 2")
    );
    assert_eq!(
        parse_explicit_utxos_string(&json!([utxo(json!([]))]).to_string())
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("UTXO script_public_key must not be empty")
    );
    assert_eq!(
        parse_explicit_utxos_string(&json!([utxo(json!([256]))]).to_string())
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("UTXO script_public_key byte exceeds 255")
    );
    assert_eq!(
        parse_explicit_utxos_string(&json!([utxo(json!("zz"))]).to_string())
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Invalid UTXO script hex: Invalid character 'z' at position 0")
    );

    let mut short_id = utxo(json!([1]));
    short_id["tx_id"] = json!("00");
    assert_eq!(
        parse_explicit_utxos_string(&json!([short_id]).to_string())
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("UTXO transaction ID must be 32 bytes")
    );

    let mut bad_id = utxo(json!([1]));
    bad_id["tx_id"] = json!("zz");
    assert_eq!(
        parse_explicit_utxos_string(&json!([bad_id]).to_string())
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Invalid UTXO transaction ID: Invalid character 'z' at position 0")
    );

    let mut large_index = utxo(json!([1]));
    large_index["index"] = json!(u64::from(u32::MAX) + 1);
    assert_eq!(
        parse_explicit_utxos_string(&json!([large_index]).to_string())
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("UTXO index exceeds u32")
    );
}
