use serde_json::json;

use crate::transaction_builder::pskb::{prepare_selected_sweep, prepare_sweep_from_utxos};

fn address(byte: u8) -> String {
    crate::account::address::encode_p2pk_address(&[byte; 32], "kaspa")
}

#[test]
fn selected_sweep_preparation_covers_valid_rows_and_all_validation_stages() {
    let source = address(1);
    let destination = address(2);
    let valid = json!([
        {"tx_id": "11".repeat(32), "index": 1, "amount": 40},
        {"tx_id": "22".repeat(32), "index": 2, "amount": 60}
    ])
    .to_string();
    let prepared =
        prepare_selected_sweep(&valid, &source, &destination, 10, "missing", "low").unwrap();
    assert_eq!(prepared.total, 100);
    assert_eq!(prepared.send_amount, 90);
    assert_eq!(prepared.utxos.len(), 2);

    for bad in ["not-json", "[]"] {
        assert!(prepare_selected_sweep(bad, &source, &destination, 0, "missing", "low",).is_err());
    }

    let missing = json!([{"index": 0, "amount": 1}]).to_string();
    assert!(prepare_selected_sweep(&missing, &source, &destination, 0, "missing", "low",).is_err());

    let bad_txid = json!([{"tx_id": "zz", "index": 0, "amount": 1}]).to_string();
    assert!(
        prepare_selected_sweep(&bad_txid, &source, &destination, 0, "missing", "low",).is_err()
    );

    let large_index = json!([{
        "tx_id": "11".repeat(32),
        "index": u64::from(u32::MAX) + 1,
        "amount": 1
    }])
    .to_string();
    assert!(
        prepare_selected_sweep(&large_index, &source, &destination, 0, "missing", "low",).is_err()
    );

    assert!(prepare_selected_sweep(&valid, &source, &destination, 100, "missing", "low",).is_err());

    let overflow = json!([
        {"tx_id": "11".repeat(32), "index": 0, "amount": u64::MAX},
        {"tx_id": "22".repeat(32), "index": 1, "amount": 1}
    ])
    .to_string();
    assert!(
        prepare_selected_sweep(&overflow, &source, &destination, 0, "missing", "low",).is_err()
    );
}

fn utxo(byte: u8, amount: u64) -> crate::account::utxo::UtxoEntry {
    crate::account::utxo::UtxoEntry {
        tx_id: format!("{byte:02x}").repeat(32),
        index: u32::from(byte),
        amount,
        script_public_key: Vec::new(),
        block_daa_score: 0,
        covenant_id: None,
    }
}

#[test]
fn fetched_sweep_preparation_is_native_testable_without_jsvalue_errors() {
    let source = address(8);
    let destination = address(9);
    let prepared = prepare_sweep_from_utxos(
        vec![utxo(1, 40), utxo(2, 60)],
        &source,
        &destination,
        10,
        "missing",
        "low",
    )
    .unwrap();
    assert_eq!(prepared.total, 100);
    assert_eq!(prepared.send_amount, 90);
    assert_eq!(prepared.utxos.len(), 2);

    assert!(
        prepare_sweep_from_utxos(Vec::new(), &source, &destination, 0, "missing", "low",)
            .err()
            .unwrap()
            .contains("missing")
    );
    assert!(prepare_sweep_from_utxos(
        vec![utxo(1, 10)],
        &source,
        &destination,
        10,
        "missing",
        "low",
    )
    .err()
    .unwrap()
    .contains("low"));
    assert!(prepare_sweep_from_utxos(
        vec![utxo(1, u64::MAX), utxo(2, 1)],
        &source,
        &destination,
        0,
        "missing",
        "low",
    )
    .is_err());
    assert!(prepare_sweep_from_utxos(
        vec![utxo(1, 100)],
        "bad-address",
        &destination,
        1,
        "missing",
        "low",
    )
    .is_err());
    assert!(prepare_sweep_from_utxos(
        vec![utxo(1, 100)],
        &source,
        "bad-address",
        1,
        "missing",
        "low",
    )
    .is_err());
}
