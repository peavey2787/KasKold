use super::*;

#[test]
fn exact_balance_does_not_append_zero_change_output() {
    let change = crate::account::address::encode_p2pk_address(&[0x44; 32], "kaspa");
    let selected = vec![UtxoEntry {
        tx_id: "11".repeat(32),
        index: 0,
        amount: 10_000,
        script_public_key: vec![0x51],
        block_daa_score: 0,
        covenant_id: None,
    }];
    let recipients = vec![PlannedOutput::new(9_000, vec![0x52])];
    let plan =
        plan_payment_with_change(selected, recipients, 1_000, &change, 7).expect("exact balance");
    assert_eq!(plan.outputs.len(), 1);
    assert_eq!(plan.outputs[0].amount, 9_000);
}

#[test]
fn payment_and_consolidation_fail_closed_boundaries_are_directly_exercised() {
    let valid_change = crate::account::address::encode_p2pk_address(&[0x45; 32], "kaspa");
    let selected = vec![UtxoEntry {
        tx_id: "22".repeat(32),
        index: 0,
        amount: 30_000_000,
        script_public_key: vec![0x51],
        block_daa_score: 0,
        covenant_id: None,
    }];

    assert!(payment_change(&selected, &[PlannedOutput::new(u64::MAX, vec![0x51])], 1,).is_err());
    assert!(append_change(&mut Vec::new(), 1, "not-an-address", 0).is_err());

    let wallet_without_change = WalletData {
        kpub: "coverage".to_string(),
        receive_addresses: vec![valid_change.clone()],
        change_addresses: vec![],
        next_receive_index: 0,
        next_change_index: 0,
    };
    assert!(plan_payment(
        &wallet_without_change,
        selected.clone(),
        vec![PlannedOutput::new(1_000_000, vec![0x51])],
        1_000_000,
    )
    .unwrap_err()
    .contains("No more change addresses"));

    let empty_wallet = WalletData {
        kpub: "coverage".to_string(),
        receive_addresses: vec![],
        change_addresses: vec![],
        next_receive_index: 0,
        next_change_index: 0,
    };
    assert!(plan_consolidation(&empty_wallet, selected.clone(), 30_000_001).is_err());
    assert!(plan_consolidation(&empty_wallet, selected.clone(), 30_000_000).is_err());
    assert!(
        plan_consolidation(&empty_wallet, selected.clone(), 1_000_000)
            .unwrap_err()
            .contains("Wallet has no receive address")
    );

    let wallet = WalletData {
        kpub: "coverage".to_string(),
        receive_addresses: vec![valid_change],
        change_addresses: vec![],
        next_receive_index: 0,
        next_change_index: 0,
    };
    let plan = plan_consolidation(&wallet, selected, 1_000_000).expect("consolidation plan");
    assert_eq!(plan.outputs.len(), 1);
    assert_eq!(plan.outputs[0].amount, 29_000_000);
}
