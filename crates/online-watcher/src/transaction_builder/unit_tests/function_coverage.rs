use crate::account::utxo::UtxoEntry;

fn utxo(byte: u8, index: u32, amount: u64) -> UtxoEntry {
    UtxoEntry {
        tx_id: format!("{byte:02x}").repeat(32),
        index,
        amount,
        script_public_key: vec![0x20; 34],
        block_daa_score: 0,
        covenant_id: None,
    }
}

#[test]
fn thread_policy_helpers_have_direct_function_coverage() {
    use crate::transaction_builder::pskb::{
        topup_policy_for, withdrawal_policy_for, GlobalThreadFamily, GlobalThreadPolicy,
    };
    let allowance = GlobalThreadPolicy::allowance(123);
    let spending = GlobalThreadPolicy::spending_limit();
    let topup = GlobalThreadPolicy::spending_limit_topup(9);
    assert_ne!(format!("{allowance:?}"), format!("{spending:?}"));
    assert!(format!("{topup:?}").contains("GlobalThreadPolicy"));

    let (locktime, _) = withdrawal_policy_for(GlobalThreadFamily::SpendingLimit, &[])
        .expect("spending-limit withdrawal policy");
    assert_eq!(locktime, 0);
    let (sequence, _) =
        topup_policy_for(GlobalThreadFamily::Allowance, &[]).expect("allowance top-up policy");
    assert_eq!(sequence, 0);
}

fn selected_utxos_json(entries: &[(u8, u32, u64)]) -> String {
    serde_json::to_string(
        &entries
            .iter()
            .map(|(byte, index, amount)| {
                serde_json::json!({
                    "tx_id": format!("{byte:02x}").repeat(32),
                    "index": index,
                    "amount": amount.to_string(),
                })
            })
            .collect::<Vec<_>>(),
    )
    .expect("selected UTXO JSON")
}

#[test]
fn selected_covenant_sweep_wrappers_have_direct_function_entry_coverage() {
    use crate::transaction_builder::covenant::{
        sweep::SweepSourceKind,
        sweeps::{owner, savings, timelocked},
    };

    let covenant_address = crate::account::address::encode_p2pk_address(&[0x61; 32], "kaspa");
    let destination_address = crate::account::address::encode_p2pk_address(&[0x62; 32], "kaspa");
    let selected = selected_utxos_json(&[(0x63, 0, 30_000_000)]);

    assert_eq!(
        SweepSourceKind::Automatic.choose("automatic", "selected"),
        "automatic"
    );
    assert_eq!(
        SweepSourceKind::Selected.choose("automatic", "selected"),
        "selected"
    );

    let (owner_prepared, owner_wire, owner_locktime) = owner::build_selected(
        &covenant_address,
        &destination_address,
        "51",
        &selected,
        1_000_000,
        "owner",
    )
    .expect("selected owner sweep");
    assert_eq!(owner_prepared.total, 30_000_000);
    assert_eq!(owner_prepared.send_amount, 29_000_000);
    assert_eq!(owner_locktime, 0);
    assert!(!owner_wire.is_empty());

    let (savings_prepared, savings_wire) = savings::build_selected(
        &covenant_address,
        &destination_address,
        "51",
        123,
        &selected,
        1_000_000,
    )
    .expect("selected savings sweep");
    assert_eq!(savings_prepared.send_amount, 29_000_000);
    assert!(!savings_wire.is_empty());

    let (beneficiary_prepared, beneficiary_wire, displayed_locktime) =
        timelocked::build_beneficiary_selected(
            &covenant_address,
            &destination_address,
            "51",
            456,
            &selected,
            1_000_000,
        )
        .expect("selected beneficiary sweep");
    assert_eq!(beneficiary_prepared.send_amount, 29_000_000);
    assert_eq!(displayed_locktime, 456);
    assert!(!beneficiary_wire.is_empty());

    let timeout_spec = timelocked::timeout_refund_spec(
        &covenant_address,
        &destination_address,
        1_000_000,
        &[0x51],
        789,
    );
    assert_eq!(timeout_spec.config.lock_time, 789);
    assert_eq!(timeout_spec.config.minimum_signatures, Some(0));
}

#[test]
fn global_thread_request_material_and_wire_wrappers_have_direct_coverage() {
    use crate::transaction_builder::pskb::{
        build_global_thread_topup, build_global_thread_withdrawal,
        prepare_global_thread_topup_material, GlobalThreadFamily, WithdrawalBuildRequest,
    };

    let covenant_address = crate::account::address::encode_p2pk_address(&[0x71; 32], "kaspa");
    let destination_address = crate::account::address::encode_p2pk_address(&[0x72; 32], "kaspa");
    let selected = selected_utxos_json(&[(0x73, 2, 25_000_000)]);
    let covenant_id_hex = "74".repeat(32);

    let withdrawal = build_global_thread_withdrawal(WithdrawalBuildRequest {
        family: GlobalThreadFamily::SpendingLimit,
        covenant_address: &covenant_address,
        destination_address: &destination_address,
        redeem_script_hex: "51",
        covenant_id_hex: &covenant_id_hex,
        withdrawal: 25_000_000,
        fee: 1_000_000,
        selected_utxos_json: &selected,
    })
    .expect("typed global-thread withdrawal");
    assert_eq!(withdrawal.input_count, 1);
    assert_eq!(withdrawal.total, 25_000_000);
    assert!(withdrawal.is_close);
    assert!(!withdrawal.wire.is_empty());

    let thread_json = serde_json::json!({
        "tx_id": "75".repeat(32),
        "index": 3,
        "amount": "50_000_000".replace('_', ""),
        "block_daa_score": "123",
    })
    .to_string();
    let topup_material = prepare_global_thread_topup_material(
        GlobalThreadFamily::SpendingLimit,
        &covenant_address,
        "51",
        &covenant_id_hex,
        &thread_json,
    )
    .expect("typed global-thread top-up material");
    let topup =
        build_global_thread_topup(topup_material, vec![utxo(0x76, 4, 20_000_000)], 1_000_000)
            .expect("typed global-thread top-up");
    assert_eq!(topup.selected_count, 1);
    assert_eq!(topup.thread_amount, 50_000_000);
    assert!(topup.continuation > topup.thread_amount);
    assert!(!topup.wire.is_empty());
}

#[test]
fn measured_domain_uncovered_entries_have_direct_native_coverage() {
    use crate::contracts::{
        oracle::script::{build_oracle_mb_genesis_redeem, build_oracle_mb_heartbeat_script},
        zk::crowdfund,
    };

    let heartbeat = build_oracle_mb_heartbeat_script();
    assert!(!heartbeat.is_empty());
    let genesis = build_oracle_mb_genesis_redeem(
        123,
        456,
        &[0x11; 32],
        &[0x22; 32],
        &[0x33; 32],
        1,
        &[0x44; 32],
    );
    assert!(!genesis.is_empty());

    assert_eq!(
        crowdfund::decode_hex("00ff", "fixture"),
        Ok(vec![0x00, 0xff])
    );
    assert!(crowdfund::decode_hex("0z", "fixture").is_err());
}
