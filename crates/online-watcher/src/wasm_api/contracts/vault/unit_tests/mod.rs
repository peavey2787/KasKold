use super::{
    genesis::{
        finalize_vault_genesis, prepare_vault_genesis, prepare_vault_genesis_request,
        VaultGenesisKind,
    },
    spend::{
        encode_vault_spend_pskb, encode_vault_spend_response, finalize_vault_spend,
        prepare_from_utxos, prepare_vault_spend_material, split_vault_amounts,
        validate_covenant_address, VaultSpendKind,
    },
};

fn address(byte: u8) -> String {
    crate::account::address::encode_p2pk_address(&[byte; 32], "kaspa")
}

fn utxo(byte: u8, amount: u64) -> crate::account::utxo::UtxoEntry {
    crate::account::utxo::UtxoEntry {
        tx_id: format!("{byte:02x}").repeat(32),
        index: u32::from(byte),
        amount,
        script_public_key: vec![0x20]
            .into_iter()
            .chain([byte; 32])
            .chain([0xac])
            .collect(),
        block_daa_score: 0,
        covenant_id: None,
    }
}

#[test]
fn vault_watch_only_request_and_response_boundaries_are_exact() {
    let change = address(5);
    let wallet_json = serde_json::json!({
        "kpub": "",
        "receive_addresses": [],
        "change_addresses": [change],
        "next_receive_index": 0,
        "next_change_index": 0,
    })
    .to_string();
    let owner_hex = "31".repeat(32);

    let tagged = prepare_vault_genesis_request(
        VaultGenesisKind::Tagged,
        &wallet_json,
        &owner_hex,
        "mainnet",
    )
    .expect("watch-only tagged genesis request");
    let split =
        prepare_vault_genesis_request(VaultGenesisKind::Split, &wallet_json, &owner_hex, "mainnet")
            .expect("watch-only split genesis request");
    assert_ne!(tagged.material.redeem_script, split.material.redeem_script);
    assert_eq!(tagged.change_address, address(5));
    assert_eq!(
        prepare_vault_genesis_request(VaultGenesisKind::Tagged, &wallet_json, "31", "mainnet",)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Pubkey must be 32 bytes")
    );
    let no_change = serde_json::json!({
        "kpub": "",
        "receive_addresses": [],
        "change_addresses": [],
        "next_receive_index": 0,
        "next_change_index": 0,
    })
    .to_string();
    assert_eq!(
        prepare_vault_genesis_request(VaultGenesisKind::Tagged, &no_change, &owner_hex, "mainnet",)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Watch-only wallet has no change address")
    );

    let response = finalize_vault_genesis(
        &tagged.material,
        12_345_678,
        ("50534b42".to_string(), Some([0x55; 32])),
    )
    .expect("bound genesis response");
    let value: serde_json::Value = serde_json::from_str(&response).expect("genesis response JSON");
    assert_eq!(value["pskb_hex"], "50534b42");
    assert_eq!(value["covenant_id_hex"], "55".repeat(32));
    assert_eq!(value["send_amount"], "12345678");
    assert_eq!(value["covenant_address"], tagged.material.covenant_address);
    assert_eq!(
        finalize_vault_genesis(&tagged.material, 1, ("50534b42".to_string(), None),)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Tagged vault genesis did not produce a covenant ID")
    );

    let tagged_spend = prepare_vault_spend_material(VaultSpendKind::Tagged, &owner_hex)
        .expect("tagged public spend material");
    let split_spend = prepare_vault_spend_material(VaultSpendKind::Split, &owner_hex)
        .expect("split public spend material");
    assert!(!tagged_spend.split);
    assert!(split_spend.split);
    assert_ne!(tagged_spend.redeem_script, split_spend.redeem_script);
    assert_eq!(
        prepare_vault_spend_material(VaultSpendKind::Tagged, "31")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Pubkey must be 32 bytes")
    );

    let continuation =
        encode_vault_spend_response(&"66".repeat(32), ("50534b42".to_string(), 90, None))
            .expect("single continuation response");
    let continuation_json: serde_json::Value =
        serde_json::from_str(&continuation).expect("continuation JSON");
    assert_eq!(continuation_json["new_amount"], "90");
    assert!(continuation_json.get("amount_a").is_none());

    let split_response = encode_vault_spend_response(
        &"77".repeat(32),
        ("50534b42".to_string(), 91, Some((45, 46))),
    )
    .expect("split continuation response");
    let split_json: serde_json::Value =
        serde_json::from_str(&split_response).expect("split response JSON");
    assert_eq!(split_json["amount_a"], "45");
    assert_eq!(split_json["amount_b"], "46");
    assert!(split_json.get("new_amount").is_none());
}

#[test]
fn vault_genesis_material_and_identifiers_are_public_network_bound_data() {
    let owner = [0x31u8; 32];
    let tagged_mainnet = prepare_vault_genesis(VaultGenesisKind::Tagged, &owner, "mainnet")
        .expect("tagged public genesis material");
    let tagged_testnet = prepare_vault_genesis(VaultGenesisKind::Tagged, &owner, "testnet-12")
        .expect("tagged testnet public genesis material");
    let split_mainnet = prepare_vault_genesis(VaultGenesisKind::Split, &owner, "mainnet")
        .expect("split public genesis material");

    assert!(!tagged_mainnet.redeem_script.is_empty());
    assert_ne!(tagged_mainnet.redeem_script, split_mainnet.redeem_script);
    assert_ne!(
        tagged_mainnet.covenant_address,
        tagged_testnet.covenant_address
    );
    assert_eq!(split_vault_amounts(9), Ok((4, 5)));
    assert_eq!(
        split_vault_amounts(u64::MAX),
        Ok((u64::MAX / 2, u64::MAX - u64::MAX / 2))
    );
}

#[test]
fn vault_spend_preparation_covers_success_and_balance_validation() {
    let owner = [0x41; 32];
    let redeem = crate::contracts::vault::script::build_tagged_vault_script(&owner);
    let prepared =
        prepare_from_utxos(&"11".repeat(32), &redeem, 10, "mainnet", vec![utxo(1, 100)]).unwrap();
    assert_eq!(prepared.spendable, 90);
    assert_eq!(prepared.utxos.len(), 1);
    assert_eq!(prepared.covenant_id, [0x11; 32]);
    assert!(!prepared.covenant_script_pubkey.is_empty());
    let derived_address = crate::protocol::script::p2sh::script_to_address(
        &redeem,
        crate::account::address::network_prefix("mainnet").unwrap(),
    )
    .expect("derived covenant address");
    assert!(validate_covenant_address(&derived_address, &redeem, "mainnet").is_ok());
    assert_eq!(
        validate_covenant_address(&address(9), &redeem, "mainnet")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Covenant address does not match the supplied redeem script")
    );

    assert_eq!(
        prepare_from_utxos(&"11".repeat(32), &redeem, 0, "mainnet", vec![])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("No UTXOs at covenant address")
    );
    assert_eq!(
        prepare_from_utxos("00", &redeem, 0, "mainnet", vec![utxo(1, 1)])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("covenant_id must be 32 bytes, got 1")
    );
    assert_eq!(
        prepare_from_utxos(&"11".repeat(32), &redeem, 1, "mainnet", vec![utxo(1, 1)])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Balance 1 <= fee 1")
    );
    assert_eq!(
        prepare_from_utxos(
            &"11".repeat(32),
            &redeem,
            0,
            "mainnet",
            vec![utxo(1, u64::MAX), utxo(2, 1)],
        )
        .err()
        .map(|error| error.to_string())
        .as_deref(),
        Some("Vault balance overflows u64")
    );
}

#[test]
fn vault_pskb_plans_require_hardware_signing_and_preserve_covenant_binding() {
    let owner = [0x42; 32];
    let redeem = crate::contracts::vault::script::build_tagged_vault_script(&owner);
    let prepared = prepare_from_utxos(&"22".repeat(32), &redeem, 10, "mainnet", vec![utxo(1, 100)])
        .expect("prepared tagged vault spend");
    let (wire, spendable, split) =
        encode_vault_spend_pskb(prepared, &redeem, false).expect("watch-only continuation PSKB");
    assert!(
        wire.starts_with("50534b42"),
        "vault continuation must be an unsigned PSKB"
    );
    assert_eq!(spendable, 90);
    assert_eq!(split, None);
    let outer = hex::decode(&wire).expect("PSKB outer hex");
    assert_eq!(&outer[..4], b"PSKB");
    let body_hex = core::str::from_utf8(&outer[4..]).expect("PSKB body hex");
    let body = hex::decode(body_hex).expect("PSKB JSON hex");
    let document: serde_json::Value = serde_json::from_slice(&body).expect("PSKB JSON");
    assert_eq!(
        document[0]["outputs"][0]["covenantBinding"]["covenantId"],
        serde_json::Value::String("22".repeat(32)),
    );
    assert_eq!(
        document[0]["inputs"][0]["partialSigs"],
        serde_json::json!({}),
        "watcher must not synthesize a wallet signature",
    );

    let prepared_split =
        prepare_from_utxos(&"33".repeat(32), &redeem, 10, "mainnet", vec![utxo(2, 101)])
            .expect("prepared split");
    let (split_wire, _, amounts) =
        encode_vault_spend_pskb(prepared_split, &redeem, true).expect("watch-only split PSKB");
    assert!(split_wire.starts_with("50534b42"));
    assert_eq!(amounts, Some((45, 46)));

    let finalized_tagged =
        prepare_from_utxos(&"44".repeat(32), &redeem, 10, "mainnet", vec![utxo(5, 101)])
            .expect("tagged finalize prepare");
    let tagged_json = finalize_vault_spend(finalized_tagged, &redeem, false, &"44".repeat(32))
        .expect("tagged finalize");
    assert!(tagged_json.contains("new_amount"));

    let finalized_split =
        prepare_from_utxos(&"44".repeat(32), &redeem, 10, "mainnet", vec![utxo(6, 101)])
            .expect("split finalize prepare");
    let split_json = finalize_vault_spend(finalized_split, &redeem, true, &"44".repeat(32))
        .expect("split finalize");
    assert!(split_json.contains("amount_a"));
    assert!(split_json.contains("amount_b"));

    let multiple = prepare_from_utxos(
        &"44".repeat(32),
        &redeem,
        10,
        "mainnet",
        vec![utxo(3, 60), utxo(4, 60)],
    )
    .expect("multiple covenant UTXOs prepare");
    assert_eq!(
        encode_vault_spend_pskb(multiple, &redeem, false)
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("KIP-20 vault continuation requires exactly one covenant UTXO, got 2"),
        "ambiguous multi-UTXO KIP-20 continuation must fail closed"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn vault_async_builders_and_public_wrappers_reach_native_transport_fail_closed() {
    use super::{genesis, spend, split, tagged};
    use crate::wasm_api::test_support::ready;

    let owner_hex = "31".repeat(32);
    let change = address(5);
    let wallet_json = serde_json::json!({
        "kpub": "",
        "receive_addresses": [address(6)],
        "change_addresses": [change],
        "next_receive_index": 0,
        "next_change_index": 0,
    })
    .to_string();

    assert_eq!(
        ready(genesis::build_vault_genesis_pskb(
            VaultGenesisKind::Tagged,
            &wallet_json,
            &owner_hex,
            1_000_000,
            1,
            "mainnet",
            "ws://unused",
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(
            r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
        )
    );
    assert_eq!(
        ready(tagged::tagged_vault_genesis_pskb(
            &wallet_json,
            &owner_hex,
            1_000_000,
            1,
            "mainnet",
            "ws://unused",
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(
            r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
        )
    );
    assert_eq!(
        ready(split::split_vault_genesis_pskb(
            &wallet_json,
            &owner_hex,
            1_000_000,
            1,
            "mainnet",
            "ws://unused",
        ))
        .err()
        .map(|error| format!("{error:?}"))
        .as_deref(),
        Some(
            r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
        )
    );

    for (kind, split_mode) in [
        (VaultSpendKind::Tagged, false),
        (VaultSpendKind::Split, true),
    ] {
        let material =
            spend::prepare_vault_spend_material(kind, &owner_hex).expect("spend material");
        assert_eq!(material.split, split_mode);
        let covenant_address =
            crate::protocol::script::p2sh::script_to_address(&material.redeem_script, "kaspa")
                .expect("vault address");
        assert_eq!(
            ready(spend::build_vault_spend_pskb(
                kind,
                &covenant_address,
                &owner_hex,
                &"44".repeat(32),
                1,
                "mainnet",
                "ws://unused",
            ))
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
            Some(
                r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
            )
        );
        if split_mode {
            assert_eq!(
                ready(split::split_vault_spend_pskb(
                    &covenant_address,
                    &owner_hex,
                    &"44".repeat(32),
                    1,
                    "mainnet",
                    "ws://unused",
                ))
                .err()
                .map(|error| format!("{error:?}"))
                .as_deref(),
                Some(
                    r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
                )
            );
        } else {
            assert_eq!(
                ready(tagged::tagged_vault_spend_pskb(
                    &covenant_address,
                    &owner_hex,
                    &"44".repeat(32),
                    1,
                    "mainnet",
                    "ws://unused",
                ))
                .err()
                .map(|error| format!("{error:?}"))
                .as_deref(),
                Some(
                    r#"JsValue(Some("browser WebSocket transport is unavailable on native hosts (endpoint=ws://unused)"))"#
                )
            );
        }
    }
}
