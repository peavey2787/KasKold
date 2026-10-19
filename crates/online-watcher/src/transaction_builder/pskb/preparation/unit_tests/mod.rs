use super::*;

fn selected(tx_id: &str) -> SelectedUtxo {
    SelectedUtxo {
        tx_id: tx_id.to_string(),
        index: 0,
        amount: 1,
    }
}

#[test]
fn selected_utxo_txid_checks_length_and_hex_independently() {
    assert!(selected_utxo_entry(selected(&"11".repeat(32))).is_ok());
    assert_eq!(
        selected_utxo_entry(selected(&"11".repeat(31)))
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("UTXO tx_id must be 32-byte hex")
    );
    let mut invalid_hex = "11".repeat(32);
    invalid_hex.replace_range(62..64, "zz");
    assert_eq!(
        selected_utxo_entry(selected(&invalid_hex))
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("UTXO tx_id must be 32-byte hex")
    );
}
