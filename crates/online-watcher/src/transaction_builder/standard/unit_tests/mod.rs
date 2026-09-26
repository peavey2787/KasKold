use super::*;

fn utxo(byte: u8, index: u32, amount: u64) -> UtxoEntry {
    UtxoEntry {
        tx_id: format!("{byte:02x}").repeat(32),
        index,
        amount,
        script_public_key: vec![0x51],
        block_daa_score: 0,
        covenant_id: None,
    }
}

#[test]
fn selected_send_preparation_covers_input_amount_fee_and_storage_boundaries() {
    let destination = crate::account::address::encode_p2pk_address(&[0x66; 32], "kaspa");
    assert!(prepare_selected_send(
        "not-an-address",
        20_000_000,
        300_000,
        &[utxo(1, 0, 30_000_000)]
    )
    .is_err());
    assert!(prepare_selected_send(&destination, 0, 300_000, &[utxo(1, 0, 30_000_000)]).is_err());
    assert!(prepare_selected_send(&destination, 1, 300_000, &[utxo(1, 0, 30_000_000)]).is_err());
    assert!(prepare_selected_send(&destination, 20_000_000, 300_000, &[]).is_err());

    let too_many = (0..=SIGNER_MAX_INPUTS)
        .map(|index| utxo((index as u8).wrapping_add(1), index as u32, 1_000_000))
        .collect::<Vec<_>>();
    assert!(prepare_selected_send(&destination, 20_000_000, 300_000, &too_many).is_err());

    let overflow = [utxo(0x71, 0, u64::MAX), utxo(0x72, 1, 1)];
    assert!(prepare_selected_send(&destination, 20_000_000, 300_000, &overflow).is_err());

    let selected = [utxo(0x73, 0, 80_000_000)];
    let (prepared, fee) = prepare_selected_send(&destination, 20_000_000, 300_000, &selected)
        .expect("valid selected send");
    assert_eq!(prepared.output.amount, 20_000_000);
    assert!(fee >= 300_000);
}

#[test]
fn storage_mass_fee_covers_requested_fee_dust_and_arithmetic_error_paths() {
    let one = [utxo(0x74, 0, 50_000_000)];
    let computed = storage_mass_fee(&one, 50_000_000, 20_000_000, 0).expect("computed fee");
    assert!(computed >= 300_000);
    assert_eq!(
        storage_mass_fee(&one, 50_000_000, 20_000_000, computed + 1).expect("requested fee"),
        computed + 1
    );
    assert!(storage_mass_fee(&one, 0, u64::MAX, 1).is_err());

    let dust_change = [utxo(0x75, 0, 15_000_000)];
    let non_dust_change = [utxo(0x76, 0, 40_000_000)];
    assert!(storage_mass_fee(&dust_change, 15_000_000, 10_000_000, 0).is_ok());
    assert!(storage_mass_fee(&non_dust_change, 40_000_000, 10_000_000, 0).is_ok());
}
