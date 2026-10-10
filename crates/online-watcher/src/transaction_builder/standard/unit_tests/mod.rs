use super::*;
use crate::wasm_api::test_support::ready;

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

fn wallet() -> WalletData {
    WalletData {
        kpub: "coverage".to_string(),
        receive_addresses: vec![],
        change_addresses: vec![crate::account::address::encode_p2pk_address(
            &[0x55; 32],
            "kaspa",
        )],
        next_receive_index: 0,
        next_change_index: 0,
    }
}

#[test]
fn signer_input_count_follows_the_public_signer_capability() {
    assert_eq!(
        validate_signer_input_count(0),
        Err("No UTXOs provided".to_string())
    );
    assert_eq!(validate_signer_input_count(1), Ok(()));
    assert_eq!(validate_signer_input_count(SIGNER_MAX_INPUTS), Ok(()));
    assert!(validate_signer_input_count(SIGNER_MAX_INPUTS + 1)
        .unwrap_err()
        .contains("KasKold supports at most"));
}

#[test]
fn selected_utxo_sends_are_bounded_by_the_signer_before_building() {
    let destination = crate::account::address::encode_p2pk_address(&[0x66; 32], "kaspa");
    let too_many = (0..=SIGNER_MAX_INPUTS)
        .map(|index| utxo((index as u8).wrapping_add(1), index as u32, 1_000_000))
        .collect::<Vec<_>>();
    assert!(
        create_pskb_with_utxos(&wallet(), &destination, 20_000_000, 300_000, too_many)
            .unwrap_err()
            .contains("KasKold supports at most")
    );
    assert_eq!(
        create_pskb_with_utxos(&wallet(), &destination, 20_000_000, 300_000, Vec::new()),
        Err("No UTXOs provided".to_string())
    );
    let wire = create_pskb_with_utxos(
        &wallet(),
        &destination,
        20_000_000,
        300_000,
        vec![utxo(0x73, 0, 80_000_000)],
    )
    .expect("selected send");
    assert!(hex::decode(wire).expect("wire hex").starts_with(b"PSKB"));
}

#[test]
fn networked_sends_check_signer_capacity_before_reaching_the_node() {
    let wallet = wallet();
    let too_many = vec![0usize; SIGNER_MAX_INPUTS + 1];
    for result in [
        ready(create_send_limited(
            &wallet,
            "kaspa:any",
            1,
            1,
            0,
            "ws://unused",
        )),
        ready(create_send_limited(
            &wallet,
            "kaspa:any",
            1,
            1,
            SIGNER_MAX_INPUTS + 1,
            "ws://unused",
        )),
        ready(create_send_selected(
            &wallet,
            "kaspa:any",
            1,
            1,
            &[],
            "ws://unused",
        )),
        ready(create_send_selected(
            &wallet,
            "kaspa:any",
            1,
            1,
            &too_many,
            "ws://unused",
        )),
    ] {
        let error = result.unwrap_err();
        assert!(
            error == "No UTXOs provided" || error.contains("KasKold supports at most"),
            "{error}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    for result in [
        ready(create_send(&wallet, "kaspa:any", 1, 1, "ws://unused")),
        ready(create_send_limited(
            &wallet,
            "kaspa:any",
            1,
            1,
            2,
            "ws://unused",
        )),
        ready(create_send_selected(
            &wallet,
            "kaspa:any",
            1,
            1,
            &[0],
            "ws://unused",
        )),
        ready(create_consolidation(&wallet, 1, "ws://unused")),
    ] {
        assert!(result.unwrap_err().contains("unavailable on native hosts"));
    }
}
