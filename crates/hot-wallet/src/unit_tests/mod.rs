use super::{
    decode_recovery, encode_recovery, hex_string, HotWallet, HotWalletError, RecoveryMaterial,
    WalletKind, MAX_BIP39_PASSPHRASE_LEN, PLATFORM_WRAPPING_KEY_LEN, RECOVERY_RECORD_LEN,
    SEALED_WALLET_LEN,
};
use offline_signer::{
    derivation::bip32::{derive_account_key, derive_address_key, derive_change_key},
    transaction::{
        kspt::{parse_compact_kspt, serialize_compact_kspt_vec},
        model::Transaction,
    },
};

const MNEMONIC_12: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const MNEMONIC_24: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

fn restored_wallet() -> HotWallet {
    HotWallet::restore(MNEMONIC_12, "").expect("known mnemonic restores")
}

fn owned_compact_kspt(wallet: &HotWallet) -> Vec<u8> {
    let account = derive_account_key(&wallet.seed.bytes).expect("account derivation");
    let child = derive_address_key(&account, 0).expect("receive derivation");
    let xonly = child.public_key_x_only().expect("x-only public key");

    let mut transaction = Transaction::try_new().expect("transaction allocation");
    transaction.version = 1;
    transaction.network = offline_signer::address::KaspaNetwork::Mainnet;
    transaction.num_inputs = 1;
    transaction.num_outputs = 1;
    transaction.inputs[0].previous_outpoint.transaction_id = [0x11; 32];
    transaction.inputs[0].previous_outpoint.index = 7;
    transaction.inputs[0].sequence = u64::MAX;
    transaction.inputs[0].sig_op_count = 1;
    transaction.inputs[0].utxo_entry.amount = 100_000;
    transaction.inputs[0].has_derivation_hint = true;
    transaction.inputs[0].derivation_branch = 0;
    transaction.inputs[0].derivation_index = 0;
    {
        let script = &mut transaction.inputs[0].utxo_entry.script_public_key;
        script.script[0] = 0x20;
        script.script[1..33].copy_from_slice(&xonly);
        script.script[33] = 0xac;
        script.script_len = 34;
    }
    transaction.outputs[0].value = 99_000;
    {
        let script = &mut transaction.outputs[0].script_public_key;
        script.script[0] = 0x20;
        script.script[1..33].fill(0x44);
        script.script[33] = 0xac;
        script.script_len = 34;
    }
    serialize_compact_kspt_vec(&transaction).expect("compact KSPT fixture serializes")
}

fn owned_raw_compact_kspt(private_key: &[u8; 32]) -> Vec<u8> {
    let xonly = offline_signer::derivation::bip32::pubkey_from_raw_key(private_key)
        .expect("raw key public key");
    let mut transaction = Transaction::try_new().expect("transaction allocation");
    transaction.version = 1;
    transaction.network = offline_signer::address::KaspaNetwork::Mainnet;
    transaction.num_inputs = 1;
    transaction.num_outputs = 1;
    transaction.inputs[0].previous_outpoint.transaction_id = [0x19; 32];
    transaction.inputs[0].previous_outpoint.index = 1;
    transaction.inputs[0].sequence = u64::MAX;
    transaction.inputs[0].sig_op_count = 1;
    transaction.inputs[0].utxo_entry.amount = 25_000;
    {
        let script = &mut transaction.inputs[0].utxo_entry.script_public_key;
        script.script[0] = 0x20;
        script.script[1..33].copy_from_slice(&xonly);
        script.script[33] = 0xac;
        script.script_len = 34;
    }
    transaction.outputs[0].value = 24_000;
    {
        let script = &mut transaction.outputs[0].script_public_key;
        script.script[0] = 0x20;
        script.script[1..33].fill(0x55);
        script.script[33] = 0xac;
        script.script_len = 34;
    }
    serialize_compact_kspt_vec(&transaction).expect("raw-key compact KSPT serializes")
}

#[test]
fn restore_rejects_wrong_word_count() {
    let result = HotWallet::restore("abandon abandon abandon", "");
    assert!(matches!(
        result,
        Err(HotWalletError::InvalidMnemonicWordCount)
    ));
}

#[test]
fn known_valid_mnemonic_round_trips_to_public_account() {
    let wallet = restored_wallet();
    let kpub = wallet.export_kpub().unwrap();
    assert!(kpub.starts_with("kpub"));
}

#[test]
fn recovery_phrase_can_be_revealed_again_for_explicit_backup() {
    let wallet = restored_wallet();
    assert_eq!(
        wallet.backup_recovery_phrase().unwrap().as_str(),
        MNEMONIC_12
    );
}

#[test]
fn both_supported_mnemonic_lengths_restore() {
    let wallet = HotWallet::restore(MNEMONIC_24, "").expect("known 24-word mnemonic restores");
    assert!(wallet.export_kpub().unwrap().starts_with("kpub"));
}

#[test]
fn explicit_backup_exports_cover_words_indices_xprv_and_receive_key() {
    let wallet = restored_wallet();
    assert_eq!(
        wallet.backup_recovery_phrase().unwrap().as_str(),
        MNEMONIC_12
    );

    let mut indices = [u16::MAX; 24];
    assert_eq!(wallet.backup_mnemonic_indices(&mut indices).unwrap(), 12);
    assert!(indices[..11].iter().all(|index| *index == 0));
    assert_eq!(indices[11], 3);
    assert!(indices[12..].iter().all(|index| *index == 0));

    let xprv = wallet.backup_account_xprv().expect("account xprv backup");
    assert!(xprv.starts_with("kprv"));

    let receive_key = wallet
        .backup_receive_private_key_hex(0)
        .expect("receive private-key backup");
    assert_eq!(receive_key.len(), 64);
    assert!(receive_key.bytes().all(|byte| byte.is_ascii_hexdigit()));

    let account = derive_account_key(&wallet.seed.bytes).expect("account derivation");
    let child = derive_address_key(&account, 0).expect("receive derivation");
    let expected_receive_key = hex_string(child.private_key_bytes());
    assert_eq!(receive_key.as_str(), expected_receive_key.as_str());
}

#[test]
fn restore_covers_invalid_word_passphrase_and_both_supported_lengths() {
    let too_long = "x".repeat(MAX_BIP39_PASSPHRASE_LEN + 1);
    assert!(matches!(
        HotWallet::restore(MNEMONIC_12, &too_long),
        Err(HotWalletError::Bip39PassphraseTooLong)
    ));
    assert!(matches!(
        HotWallet::restore("notaword abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", ""),
        Err(HotWalletError::Bip39(_))
    ));
    assert!(HotWallet::restore(MNEMONIC_12, "passphrase").is_ok());
    assert!(HotWallet::restore(MNEMONIC_24, "passphrase").is_ok());
}

#[test]
fn recovery_record_decoder_covers_valid_empty_and_rejected_records() {
    let mut indices = [0u16; 24];
    indices[11] = 3;
    let recovery = RecoveryMaterial::from_parts(12, indices, "pw").expect("recovery material");
    let mut record = [0u8; RECOVERY_RECORD_LEN];
    encode_recovery(&recovery, &mut record);
    let decoded = decode_recovery(&record)
        .expect("valid recovery record")
        .expect("recovery material present");
    assert_eq!(decoded.word_count, 12);
    assert_eq!(decoded.passphrase_len, 2);

    assert!(decode_recovery(&record[..RECOVERY_RECORD_LEN - 1]).is_err());
    let empty = [0u8; RECOVERY_RECORD_LEN];
    assert!(decode_recovery(&empty).unwrap().is_none());

    let mut malformed_empty = empty;
    malformed_empty[2] = 1;
    assert!(decode_recovery(&malformed_empty).is_err());

    let mut invalid_word_count = record;
    invalid_word_count[0] = 13;
    assert!(decode_recovery(&invalid_word_count).is_err());

    let mut long_passphrase = record;
    long_passphrase[1] = (MAX_BIP39_PASSPHRASE_LEN + 1) as u8;
    assert!(decode_recovery(&long_passphrase).is_err());

    let mut invalid_index = record;
    invalid_index[2..4].copy_from_slice(&2048u16.to_le_bytes());
    assert!(decode_recovery(&invalid_index).is_err());

    let mut nonzero_trailing_index = record;
    let trailing_offset = 2 + 12 * 2;
    nonzero_trailing_index[trailing_offset..trailing_offset + 2]
        .copy_from_slice(&1u16.to_le_bytes());
    assert!(decode_recovery(&nonzero_trailing_index).is_err());

    let mut invalid_utf8 = record;
    invalid_utf8[1] = 1;
    invalid_utf8[50] = 0xff;
    invalid_utf8[51..].fill(0);
    assert!(decode_recovery(&invalid_utf8).is_err());

    let mut nonzero_passphrase_tail = record;
    nonzero_passphrase_tail[1] = 0;
    nonzero_passphrase_tail[50] = b'x';
    nonzero_passphrase_tail[51..].fill(0);
    assert!(decode_recovery(&nonzero_passphrase_tail).is_err());
}

#[test]
fn generated_wallets_cover_both_supported_word_counts() {
    let created_12 = HotWallet::create_12().expect("12-word wallet creation");
    assert_eq!(created_12.recovery_phrase.split_whitespace().count(), 12);
    assert!(created_12.wallet.export_kpub().unwrap().starts_with("kpub"));

    let created_24 = HotWallet::create_24().expect("24-word wallet creation");
    assert_eq!(created_24.recovery_phrase.split_whitespace().count(), 24);
    assert!(created_24.wallet.export_kpub().unwrap().starts_with("kpub"));
}

#[test]
fn review_and_sign_owned_compact_kspt() {
    let wallet = restored_wallet();
    let wire = owned_compact_kspt(&wallet);

    let review = wallet
        .review_compact_kspt(&wire)
        .expect("owned compact KSPT reviews");
    assert_eq!(review.input_count, 1);
    assert_eq!(review.output_count, 1);
    assert_eq!(review.input_total, 100_000);
    assert_eq!(review.output_total, 99_000);
    assert_eq!(review.fee, 1_000);

    let signed = wallet
        .sign_compact_kspt(&wire)
        .expect("owned compact KSPT signs");
    let mut parsed = Transaction::try_new().expect("transaction allocation");
    parse_compact_kspt(&signed, &mut parsed).expect("signed compact KSPT parses");
    assert_eq!(parsed.inputs[0].sig_count, 1);
    assert!(parsed.inputs[0].sigs[0].present);
}

#[test]
fn generic_vault_review_sign_and_serialize_reject_hidden_sighash_committed_semantics() {
    let wallet = restored_wallet();
    let base = owned_compact_kspt(&wallet);
    let reject = |mutate: fn(&mut Transaction)| {
        let mut tx = Transaction::try_new().expect("transaction allocation");
        parse_compact_kspt(&base, &mut tx).expect("fixture parses");
        mutate(&mut tx);
        let wire = serialize_compact_kspt_vec(&tx).expect("mutated transaction serializes");
        assert!(matches!(
            wallet.review_compact_kspt(&wire),
            Err(HotWalletError::InvalidToolInput)
        ));
        assert!(matches!(
            wallet.sign_compact_kspt(&wire),
            Err(HotWalletError::InvalidToolInput)
        ));
    };

    reject(|tx| tx.locktime = 1);
    reject(|tx| tx.gas = 1);
    reject(|tx| {
        tx.payload = vec![0x42];
    });
    reject(|tx| tx.inputs[0].sequence = 1);
    reject(|tx| tx.inputs[0].sig_op_count = 2);

    // Unsigned compact KSPT v1 carries SIGHASH_ALL implicitly, so an in-memory
    // model with a different declared sighash must be rejected by the canonical
    // serializer rather than silently normalized into a different wire meaning.
    let mut non_all = Transaction::try_new().expect("transaction allocation");
    parse_compact_kspt(&base, &mut non_all).expect("fixture parses");
    non_all.inputs[0].sighash_type = 0x02;
    assert!(matches!(
        serialize_compact_kspt_vec(&non_all),
        Err(offline_signer::transaction::kspt::PsktError::InvalidSigHashType)
    ));

    reject(|tx| {
        tx.outputs[0].has_covenant = true;
        tx.outputs[0].covenant_auth_input = 0;
        tx.outputs[0].covenant_id = [0x55; 32];
    });
    reject(|tx| tx.subnetwork_id[0] = 1);
}

#[test]
fn review_and_sign_reject_malformed_compact_kspt() {
    let wallet = restored_wallet();
    assert!(wallet.review_compact_kspt(b"KSPT").is_err());
    assert!(wallet.sign_compact_kspt(b"KSPT").is_err());
}

#[test]
fn platform_sealed_wallet_round_trips_without_exporting_seed() {
    let wallet = restored_wallet();
    let expected_kpub = wallet.export_kpub().unwrap();
    let key = [0x5au8; PLATFORM_WRAPPING_KEY_LEN];
    let sealed = wallet.seal_for_platform(&key).unwrap();
    assert_eq!(sealed.len(), SEALED_WALLET_LEN);
    let restored = HotWallet::restore_platform_sealed(&sealed, &key).unwrap();
    assert_eq!(restored.export_kpub().unwrap(), expected_kpub);
    assert_eq!(
        restored.backup_recovery_phrase().unwrap().as_str(),
        MNEMONIC_12
    );
}

#[test]
fn platform_sealed_wallet_rejects_tampering() {
    let wallet = restored_wallet();
    let key = [0x33u8; PLATFORM_WRAPPING_KEY_LEN];
    let mut sealed = wallet.seal_for_platform(&key).unwrap();
    sealed[20] ^= 0x80;
    assert!(matches!(
        HotWallet::restore_platform_sealed(&sealed, &key),
        Err(HotWalletError::SealedWalletAuthenticationFailed)
    ));
}

#[test]
fn receive_addresses_cover_network_chain_and_index() {
    let wallet = restored_wallet();
    let receive_zero = wallet
        .derive_address(offline_signer::address::KaspaNetwork::Mainnet, false, 0)
        .expect("mainnet receive address");
    let receive_one = wallet
        .derive_address(offline_signer::address::KaspaNetwork::Mainnet, false, 1)
        .expect("next receive address");
    let change_zero = wallet
        .derive_address(offline_signer::address::KaspaNetwork::Mainnet, true, 0)
        .expect("mainnet change address");
    let testnet_zero = wallet
        .derive_address(offline_signer::address::KaspaNetwork::Testnet, false, 0)
        .expect("testnet receive address");

    assert!(receive_zero.starts_with("kaspa:"));
    assert!(testnet_zero.starts_with("kaspatest:"));
    assert_ne!(receive_zero, receive_one);
    assert_ne!(receive_zero, change_zero);
}

#[test]
fn advanced_wallet_tools_cover_bip85_message_and_secret_round_trip() {
    let wallet = restored_wallet();
    let child_12 = wallet
        .derive_bip85_phrase(12, 0)
        .expect("BIP85 12-word child");
    let child_24 = wallet
        .derive_bip85_phrase(24, 1)
        .expect("BIP85 24-word child");
    assert_eq!(child_12.split_whitespace().count(), 12);
    assert_eq!(child_24.split_whitespace().count(), 24);

    let signed = wallet
        .sign_message(b"KasKold message")
        .expect("message signing");
    assert_ne!(signed.signature, [0u8; 64]);
    assert_ne!(signed.digest, [0u8; 32]);

    let committed = wallet
        .commit_secret(b"air gap secret")
        .expect("commit secret");
    assert_eq!(&committed.payload[..32], &committed.commitment);
    let revealed = wallet
        .decrypt_secret(&committed.payload)
        .expect("decrypt committed secret");
    assert_eq!(revealed.as_slice(), b"air gap secret");

    let mut tampered = committed.payload.to_vec();
    tampered[0] ^= 1;
    assert!(wallet.decrypt_secret(&tampered).is_err());
}

#[test]
fn seedqr_recovery_material_round_trips_standard_and_compact() {
    let wallet = restored_wallet();
    let expected_kpub = wallet.export_kpub().unwrap();
    let mut indices = [0u16; 24];
    let word_count = wallet.backup_mnemonic_indices(&mut indices).unwrap();

    let mut standard = [0u8; 96];
    let standard_len = shared_signer::seed_qr::encode_seedqr(&indices, word_count, &mut standard);
    let restored_standard = HotWallet::restore_recovery_material(&standard[..standard_len], "")
        .expect("standard SeedQR restore");
    assert_eq!(restored_standard.export_kpub().unwrap(), expected_kpub);

    let mut compact = [0u8; 32];
    let compact_len =
        shared_signer::seed_qr::encode_compact_seedqr(&indices, word_count, &mut compact);
    let restored_compact = HotWallet::restore_recovery_material(&compact[..compact_len], "")
        .expect("CompactSeedQR restore");
    assert_eq!(restored_compact.export_kpub().unwrap(), expected_kpub);
}

#[test]
fn portable_encrypted_backup_round_trips_and_rejects_wrong_password() {
    let wallet = restored_wallet();
    let expected_kpub = wallet.export_kpub().unwrap();
    let backup = wallet
        .portable_backup("correct horse battery staple")
        .expect("portable backup");
    let restored = HotWallet::restore_portable_backup(&backup, "correct horse battery staple")
        .expect("portable restore");
    assert_eq!(restored.export_kpub().unwrap(), expected_kpub);
    assert!(HotWallet::restore_portable_backup(&backup, "wrong password").is_err());
}

#[test]
fn multisig_create_and_import_share_canonical_descriptor_and_address() {
    let mut first = restored_wallet();
    let second = HotWallet::restore(MNEMONIC_24, "").expect("second wallet");
    let second_kpub = second.export_multisig_kpub().expect("multisig kpub");
    let created = first
        .create_multisig(
            2,
            &[second_kpub.as_str()],
            offline_signer::address::KaspaNetwork::Mainnet,
            0,
            0,
        )
        .expect("2-of-2 multisig");
    assert!(created.descriptor.starts_with("multi_hd45(2,"));
    assert!(created.address.starts_with("kaspa:"));
    assert_eq!(created.threshold, 2);
    assert_eq!(created.participants, 2);

    let imported = first
        .import_multisig_descriptor(
            &created.descriptor,
            offline_signer::address::KaspaNetwork::Mainnet,
            0,
            0,
        )
        .expect("multisig descriptor import");
    assert_eq!(imported.address, created.address);
}

#[test]
fn typed_wallet_sources_preserve_capabilities_and_khv3_identity() {
    let wrapping_key = [0x6bu8; PLATFORM_WRAPPING_KEY_LEN];
    let raw_text = format!("{:064x}", 1u8);
    let raw = HotWallet::import_raw_private_key_hex(&raw_text).expect("raw key import");
    assert_eq!(raw.wallet_kind(), WalletKind::RawPrivateKey);
    assert!(matches!(
        raw.export_kpub(),
        Err(HotWalletError::UnsupportedForWalletType)
    ));
    assert!(raw
        .derive_address(offline_signer::address::KaspaNetwork::Mainnet, false, 0)
        .is_ok());
    assert!(matches!(
        raw.derive_address(offline_signer::address::KaspaNetwork::Mainnet, true, 0),
        Err(HotWalletError::UnsupportedForWalletType)
    ));
    assert!(matches!(
        raw.derive_address(offline_signer::address::KaspaNetwork::Mainnet, false, 1),
        Err(HotWalletError::UnsupportedForWalletType)
    ));
    assert!(matches!(
        raw.backup_recovery_phrase(),
        Err(HotWalletError::RecoveryUnavailable)
    ));
    assert!(matches!(
        raw.backup_account_xprv(),
        Err(HotWalletError::UnsupportedForWalletType)
    ));
    assert!(matches!(
        raw.portable_backup("password"),
        Err(HotWalletError::UnsupportedForWalletType)
    ));
    assert_eq!(
        raw.backup_receive_private_key_hex(0).unwrap().as_str(),
        raw_text
    );

    let raw_sealed = raw.seal_for_platform(&wrapping_key).expect("raw KHV3 seal");
    let raw_restored =
        HotWallet::restore_platform_sealed(&raw_sealed, &wrapping_key).expect("raw KHV3 restore");
    assert_eq!(raw_restored.wallet_kind(), WalletKind::RawPrivateKey);
    assert_eq!(
        raw_restored
            .backup_receive_private_key_hex(0)
            .unwrap()
            .as_str(),
        raw_text
    );

    let mnemonic = restored_wallet();
    let xprv = mnemonic.backup_account_xprv().expect("account XPrv");
    let account = HotWallet::import_account_xprv(&xprv).expect("account XPrv import");
    assert_eq!(account.wallet_kind(), WalletKind::AccountXprv);
    assert!(account.export_kpub().unwrap().starts_with("kpub"));
    assert!(account
        .derive_address(offline_signer::address::KaspaNetwork::Mainnet, false, 0)
        .is_ok());
    assert!(matches!(
        account.backup_recovery_phrase(),
        Err(HotWalletError::RecoveryUnavailable)
    ));
    assert!(matches!(
        account.portable_backup("password"),
        Err(HotWalletError::UnsupportedForWalletType)
    ));

    let account_sealed = account
        .seal_for_platform(&wrapping_key)
        .expect("account KHV3 seal");
    let account_restored = HotWallet::restore_platform_sealed(&account_sealed, &wrapping_key)
        .expect("account KHV3 restore");
    assert_eq!(account_restored.wallet_kind(), WalletKind::AccountXprv);
    assert_eq!(
        account_restored.export_kpub().unwrap(),
        account.export_kpub().unwrap()
    );

    let xprv_backup = mnemonic
        .portable_xprv_backup("password")
        .expect("portable XPrv backup");
    let xprv_restored = HotWallet::restore_portable_backup(&xprv_backup, "password")
        .expect("portable XPrv restore");
    assert_eq!(xprv_restored.wallet_kind(), WalletKind::AccountXprv);
    assert_eq!(
        xprv_restored.export_kpub().unwrap(),
        mnemonic.export_kpub().unwrap()
    );
}

#[test]
fn imported_source_types_sign_only_with_their_real_key_material() {
    let mnemonic = restored_wallet();
    let account_xprv = mnemonic.backup_account_xprv().expect("account XPrv");
    let account = HotWallet::import_account_xprv(&account_xprv).expect("account XPrv import");
    let wire = owned_compact_kspt(&mnemonic);
    let signed = account
        .sign_transaction(&wire)
        .expect("account XPrv signs account input");
    let mut parsed = Transaction::try_new().expect("transaction allocation");
    parse_compact_kspt(&signed, &mut parsed).expect("signed account transaction parses");
    assert_eq!(parsed.inputs[0].sig_count, 1);

    let private_key = [1u8; 32];
    let raw_text = hex_string(&private_key);
    let raw = HotWallet::import_raw_private_key_hex(&raw_text).expect("raw key import");
    let raw_wire = owned_raw_compact_kspt(&private_key);
    let raw_signed = raw
        .sign_transaction(&raw_wire)
        .expect("raw key signs matching P2PK input");
    let mut parsed_raw = Transaction::try_new().expect("transaction allocation");
    parse_compact_kspt(&raw_signed, &mut parsed_raw).expect("signed raw transaction parses");
    assert_eq!(parsed_raw.inputs[0].sig_count, 1);

    assert!(raw.sign_transaction(&wire).is_err());
}
#[test]
fn covenant_custody_tools_cover_allocation_binding_and_anti_klepto_finalize() {
    let wallet = restored_wallet();
    let (key_id, pubkey) = wallet
        .covenant_allocate_key()
        .expect("covenant key allocation");
    assert_ne!(key_id, [0u8; 32]);
    assert_ne!(pubkey, [0u8; 32]);
    assert_eq!(wallet.covenant_public_key(&key_id).unwrap(), pubkey);

    let script_hash = [0x31u8; 32];
    let token = wallet
        .covenant_binding_token(&key_id, &script_hash)
        .expect("binding token");
    assert!(wallet
        .covenant_binding_matches(&key_id, &script_hash, &token)
        .expect("binding verification"));
    let mut wrong = token;
    wrong[0] ^= 1;
    assert!(!wallet
        .covenant_binding_matches(&key_id, &script_hash, &wrong)
        .expect("binding mismatch"));

    let commitment = [0x42u8; 32];
    let provisional = wallet
        .covenant_begin_signature(&key_id, &commitment)
        .expect("provisional covenant signature");
    assert_eq!(provisional.nonce_point[0], 0x02);
    let final_signature = wallet
        .covenant_finalize_signature(
            &key_id,
            &commitment,
            &provisional.signature,
            &provisional.nonce_point,
            &[0x53u8; shared_signer::covenant_sign::SESSION_ID_LEN],
            &[0x64u8; 32],
        )
        .expect("final covenant signature");
    assert_ne!(final_signature, provisional.signature);
}

#[test]
fn privacy_stego_and_static_multisig_paths_fail_closed_or_round_trip_public_data() {
    let mut wallet = restored_wallet();

    assert!(wallet
        .privacy_pairing_response(b"not-a-pairing-request")
        .is_err());
    assert!(wallet
        .stealth_scan_request(b"not-a-stealth-request")
        .is_err());
    assert!(wallet.stego_backup(b"not-a-jpeg", "pw").is_err());
    assert!(HotWallet::restore_stego_backup(b"not-a-jpeg", "pw").is_err());

    let static_a = "11".repeat(32);
    let static_b = "22".repeat(32);
    let descriptor = format!("multi(1,{static_a},{static_b})");
    let imported = wallet
        .import_multisig_descriptor(
            &descriptor,
            offline_signer::address::KaspaNetwork::Mainnet,
            0,
            0,
        )
        .expect("static multisig descriptor imports");
    assert_eq!(imported.threshold, 1);
    assert_eq!(imported.participants, 2);
    assert!(imported.address.starts_with("kaspa:"));

    let raw =
        HotWallet::import_raw_private_key_hex(&format!("{:064x}", 1u8)).expect("raw key import");
    assert!(matches!(
        raw.privacy_pairing_response(b"invalid"),
        Err(HotWalletError::UnsupportedForWalletType)
    ));
    assert!(matches!(
        raw.stealth_scan_request(b"invalid"),
        Err(HotWalletError::UnsupportedForWalletType)
    ));
}

#[test]
fn review_owned_receive_hint_covers_verified_output_derivation() {
    let wallet = restored_wallet();
    let mut transaction = Transaction::try_new().expect("transaction allocation");
    parse_compact_kspt(&owned_compact_kspt(&wallet), &mut transaction).expect("fixture parses");
    let account = derive_account_key(&wallet.seed.bytes).expect("account derivation");
    let child = derive_address_key(&account, 1).expect("receive derivation");
    let xonly = child.public_key_x_only().expect("x-only public key");
    transaction.outputs[0].has_derivation_hint = true;
    transaction.outputs[0].derivation_branch = 0;
    transaction.outputs[0].derivation_index = 1;
    transaction.outputs[0].script_public_key.script[0] = 0x20;
    transaction.outputs[0].script_public_key.script[1..33].copy_from_slice(&xonly);
    transaction.outputs[0].script_public_key.script[33] = 0xac;
    transaction.outputs[0].script_public_key.script_len = 34;
    let wire = serialize_compact_kspt_vec(&transaction).expect("hinted transaction serializes");

    let review = wallet
        .review_compact_kspt(&wire)
        .expect("hinted output reviews");
    assert_eq!(review.outputs[0].ownership, super::OutputOwnership::Receive);
    assert_eq!(review.own_receive_total, 99_000);
}

#[test]
fn review_raw_output_hints_cover_absent_valid_and_rejected_shapes() {
    let mut private_key = [0u8; 32];
    private_key[31] = 1;
    let wallet =
        HotWallet::import_raw_private_key_hex(&format!("{:064x}", 1u8)).expect("raw key imports");
    let base_wire = owned_raw_compact_kspt(&private_key);

    let review = wallet
        .review_compact_kspt(&base_wire)
        .expect("unhinted raw output reviews");
    assert_eq!(
        review.outputs[0].ownership,
        super::OutputOwnership::External
    );

    let xonly = offline_signer::derivation::bip32::pubkey_from_raw_key(&private_key)
        .expect("raw key public key");
    let parse_fixture = || {
        let mut transaction = Transaction::try_new().expect("transaction allocation");
        parse_compact_kspt(&base_wire, &mut transaction).expect("raw fixture parses");
        transaction
    };

    let mut valid = parse_fixture();
    valid.outputs[0].has_derivation_hint = true;
    valid.outputs[0].derivation_branch = 0;
    valid.outputs[0].derivation_index = 0;
    valid.outputs[0].script_public_key.script[0] = 0x20;
    valid.outputs[0].script_public_key.script[1..33].copy_from_slice(&xonly);
    valid.outputs[0].script_public_key.script[33] = 0xac;
    valid.outputs[0].script_public_key.script_len = 34;
    let review = wallet
        .review_compact_kspt(&serialize_compact_kspt_vec(&valid).expect("valid hint serializes"))
        .expect("valid raw hint reviews");
    assert_eq!(review.outputs[0].ownership, super::OutputOwnership::Receive);

    let mut wrong_branch = parse_fixture();
    wrong_branch.outputs[0].has_derivation_hint = true;
    wrong_branch.outputs[0].derivation_branch = 1;
    wrong_branch.outputs[0].derivation_index = 0;
    wrong_branch.outputs[0].script_public_key.script[0] = 0x20;
    wrong_branch.outputs[0].script_public_key.script[1..33].copy_from_slice(&xonly);
    wrong_branch.outputs[0].script_public_key.script[33] = 0xac;
    wrong_branch.outputs[0].script_public_key.script_len = 34;
    let review = wallet
        .review_compact_kspt(
            &serialize_compact_kspt_vec(&wrong_branch).expect("wrong branch serializes"),
        )
        .expect("wrong raw-key hint is treated as untrusted metadata");
    assert_eq!(
        review.outputs[0].ownership,
        super::OutputOwnership::External
    );

    let mut wrong_index = parse_fixture();
    wrong_index.outputs[0].has_derivation_hint = true;
    wrong_index.outputs[0].derivation_branch = 0;
    wrong_index.outputs[0].derivation_index = 1;
    wrong_index.outputs[0].script_public_key.script[0] = 0x20;
    wrong_index.outputs[0].script_public_key.script[1..33].copy_from_slice(&xonly);
    wrong_index.outputs[0].script_public_key.script[33] = 0xac;
    wrong_index.outputs[0].script_public_key.script_len = 34;
    let review = wallet
        .review_compact_kspt(
            &serialize_compact_kspt_vec(&wrong_index).expect("wrong index serializes"),
        )
        .expect("wrong raw-key index is treated as untrusted metadata");
    assert_eq!(
        review.outputs[0].ownership,
        super::OutputOwnership::External
    );

    let mut wrong_script = parse_fixture();
    wrong_script.outputs[0].has_derivation_hint = true;
    wrong_script.outputs[0].derivation_branch = 0;
    wrong_script.outputs[0].derivation_index = 0;
    wrong_script.outputs[0].script_public_key.script[0] = 0x20;
    wrong_script.outputs[0].script_public_key.script[1..33].fill(0x7a);
    wrong_script.outputs[0].script_public_key.script[33] = 0xac;
    wrong_script.outputs[0].script_public_key.script_len = 34;
    let review = wallet
        .review_compact_kspt(
            &serialize_compact_kspt_vec(&wrong_script).expect("wrong script serializes"),
        )
        .expect("wrong raw-key script is treated as untrusted metadata");
    assert_eq!(
        review.outputs[0].ownership,
        super::OutputOwnership::External
    );
}

#[test]
fn review_account_hint_fallback_scans_receive_change_and_missing_targets() {
    let wallet = restored_wallet();
    let base_wire = owned_compact_kspt(&wallet);
    let account = derive_account_key(&wallet.seed.bytes).expect("account derivation");

    let parse_fixture = || {
        let mut transaction = Transaction::try_new().expect("transaction allocation");
        parse_compact_kspt(&base_wire, &mut transaction).expect("wallet fixture parses");
        transaction
    };

    let receive = derive_address_key(&account, 1)
        .expect("receive derivation")
        .public_key_x_only()
        .expect("receive x-only key");
    let mut receive_fallback = parse_fixture();
    receive_fallback.outputs[0].has_derivation_hint = true;
    receive_fallback.outputs[0].derivation_branch = 0;
    receive_fallback.outputs[0].derivation_index = 7;
    receive_fallback.outputs[0].script_public_key.script[0] = 0x20;
    receive_fallback.outputs[0].script_public_key.script[1..33].copy_from_slice(&receive);
    receive_fallback.outputs[0].script_public_key.script[33] = 0xac;
    receive_fallback.outputs[0].script_public_key.script_len = 34;
    let review = wallet
        .review_compact_kspt(
            &serialize_compact_kspt_vec(&receive_fallback).expect("receive fallback serializes"),
        )
        .expect("receive fallback is found by scan");
    assert_eq!(review.outputs[0].ownership, super::OutputOwnership::Receive);

    let change = derive_change_key(&account, 2)
        .expect("change derivation")
        .public_key_x_only()
        .expect("change x-only key");
    let mut change_fallback = parse_fixture();
    change_fallback.outputs[0].has_derivation_hint = true;
    change_fallback.outputs[0].derivation_branch = 1;
    change_fallback.outputs[0].derivation_index = 7;
    change_fallback.outputs[0].script_public_key.script[0] = 0x20;
    change_fallback.outputs[0].script_public_key.script[1..33].copy_from_slice(&change);
    change_fallback.outputs[0].script_public_key.script[33] = 0xac;
    change_fallback.outputs[0].script_public_key.script_len = 34;
    let review = wallet
        .review_compact_kspt(
            &serialize_compact_kspt_vec(&change_fallback).expect("change fallback serializes"),
        )
        .expect("change fallback is found by scan");
    assert_eq!(review.outputs[0].ownership, super::OutputOwnership::Change);

    let mut missing = parse_fixture();
    missing.outputs[0].has_derivation_hint = true;
    missing.outputs[0].derivation_branch = 0;
    missing.outputs[0].derivation_index = 7;
    missing.outputs[0].script_public_key.script[0] = 0x20;
    missing.outputs[0].script_public_key.script[1..33].fill(0x99);
    missing.outputs[0].script_public_key.script[33] = 0xac;
    missing.outputs[0].script_public_key.script_len = 34;
    let review = wallet
        .review_compact_kspt(
            &serialize_compact_kspt_vec(&missing).expect("missing target serializes"),
        )
        .expect("non-owned hinted output remains external");
    assert_eq!(
        review.outputs[0].ownership,
        super::OutputOwnership::External
    );
}

fn swap_push_data(script: &mut Vec<u8>, data: &[u8]) {
    script.push(u8::try_from(data.len()).expect("private-swap fixture direct push"));
    script.extend_from_slice(data);
}

fn swap_push_int(script: &mut Vec<u8>, value: u64) {
    if value == 0 {
        script.push(0x00);
        return;
    }
    if value <= 16 {
        script.push(0x50 + value as u8);
        return;
    }
    let mut bytes = value.to_le_bytes().to_vec();
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    if bytes.last().is_some_and(|byte| byte & 0x80 != 0) {
        bytes.push(0);
    }
    swap_push_data(script, &bytes);
}

fn swap_redeem_script(claimer: [u8; 32], destination: &[u8]) -> Vec<u8> {
    const MAX_FEE: u64 = offline_signer::transaction::private_swap::PRIVATE_SWAP_MAX_FEE_SOMPI;
    let owner = [0x21u8; 32];
    let mut script = Vec::new();
    swap_push_data(&mut script, &[0x42; 16]);
    script.extend_from_slice(&[0x75, 0x63]); // OP_DROP OP_IF
    swap_push_data(&mut script, &claimer);
    script.extend_from_slice(&[0xad, 0xb3, 0x51, 0x9d, 0xb4, 0x51, 0x9d, 0x00, 0xc3]);
    swap_push_data(&mut script, destination);
    script.extend_from_slice(&[
        0x88, 0x00, 0xbe, 0x76, 0x00, 0xc2, 0xa2, 0x69, 0x00, 0xc2, 0x94,
    ]);
    swap_push_int(&mut script, MAX_FEE);
    script.extend_from_slice(&[0xa1, 0x69, 0x51, 0x67]);
    swap_push_data(&mut script, &owner);
    script.push(0xad);
    swap_push_int(&mut script, 50_000);
    script.extend_from_slice(&[0xb0, 0x51, 0x68]);
    script
}

fn private_swap_claim_wire(claimer: [u8; 32]) -> Vec<u8> {
    let mut transaction = Transaction::try_new().expect("private-swap transaction allocation");
    transaction.version = 0;
    transaction.network = offline_signer::address::KaspaNetwork::Mainnet;
    transaction.num_inputs = 1;
    transaction.num_outputs = 1;
    transaction.inputs[0].previous_outpoint.transaction_id = [0x11; 32];
    transaction.inputs[0].previous_outpoint.index = 3;
    transaction.inputs[0].sequence = u64::MAX;
    transaction.inputs[0].sig_op_count = 1;
    transaction.inputs[0].sighash_type = 0x01;
    transaction.inputs[0].utxo_entry.amount = 1_000_000_000;
    {
        let input = &mut transaction.inputs[0].utxo_entry.script_public_key;
        input.script[0] = 0xaa;
        input.script[1] = 0x20;
        input.script[2..34].fill(0x77);
        input.script[34] = 0x87;
        input.script_len = 35;
    }
    transaction.outputs[0].value = 999_999_000;
    let destination = {
        let output = &mut transaction.outputs[0].script_public_key;
        output.version = 0;
        output.script[0] = 0x20;
        output.script[1..33].fill(0x44);
        output.script[33] = 0xac;
        output.script_len = 34;
        let mut encoded = Vec::from(output.version.to_le_bytes());
        encoded.extend_from_slice(output.script_bytes());
        encoded
    };
    let redeem = swap_redeem_script(claimer, &destination);
    transaction
        .store_redeem(0, &redeem)
        .expect("private-swap redeem script");
    serialize_compact_kspt_vec(&transaction).expect("private-swap KSPT serializes")
}

fn encode_private_swap_request(
    request: &shared_signer::covenant_sign::private_swap::PrivateSwapRequest<'_>,
) -> Vec<u8> {
    use shared_signer::covenant_sign::private_swap as wire;
    let mut encoded = vec![0u8; wire::REQUEST_HEADER_LEN + request.payload.len()];
    let len = wire::encode_request(request, &mut encoded).expect("private-swap request encodes");
    encoded.truncate(len);
    encoded
}

struct SwapAwaitingReveal {
    session: super::PrivateSwapSession,
    key_info: shared_signer::covenant_sign::private_swap::PrivateSwapResponse,
    binding_token: [u8; 32],
    session_id: [u8; shared_signer::covenant_sign::private_swap::SESSION_ID_LEN],
    sighash: [u8; 32],
    host_secret: [u8; 32],
    claim_wire: Vec<u8>,
}

/// Drive a fresh session through key info, binding and pre-sign to the
/// point where it awaits the host's reveal.
fn private_swap_awaiting_reveal(wallet: &HotWallet) -> SwapAwaitingReveal {
    use shared_signer::covenant_sign::private_swap::{
        self as wire, PrivateSwapRequest, RequestKind, ResponseKind,
    };

    let mut session = super::PrivateSwapSession::new();
    let empty = PrivateSwapRequest {
        kind: RequestKind::KeyInfo,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: [0; 32],
        binding_token: [0; 32],
        adaptor_point: [0; 32],
        presignature: [0; 64],
        presignature_negated: false,
        payload: &[],
    };
    let key_info = match session
        .prepare_request(wallet, &encode_private_swap_request(&empty))
        .expect("key-info prepares")
    {
        super::PrivateSwapPrepared::Response(response) => {
            wire::parse_response(&response).expect("key-info response parses")
        }
        super::PrivateSwapPrepared::Review(_) => panic!("key-info must be immediate"),
    };
    assert_eq!(key_info.kind, ResponseKind::KeyInfo);

    let claim_wire = private_swap_claim_wire(key_info.claim_pubkey);
    let mut claim = Transaction::try_new().expect("claim allocation");
    parse_compact_kspt(&claim_wire, &mut claim).expect("claim parses");
    let redeem = claim.redeem_bytes(0).to_vec();

    let bind = PrivateSwapRequest {
        kind: RequestKind::Bind,
        key_id: key_info.key_id,
        adaptor_point: key_info.adaptor_point,
        payload: &redeem,
        ..empty
    };
    assert!(matches!(
        session
            .prepare_request(wallet, &encode_private_swap_request(&bind))
            .expect("binding prepares"),
        super::PrivateSwapPrepared::Review(_)
    ));
    let binding = wire::parse_response(&session.confirm(wallet).expect("binding confirms"))
        .expect("binding response parses");
    assert_eq!(binding.kind, ResponseKind::Binding);

    let host_secret = [0x64u8; 32];
    let host_commitment = shared_signer::anti_klepto::host_commitment(&host_secret);
    let session_id = wire::session_id(
        &host_commitment,
        &claim_wire,
        &key_info.key_id,
        &key_info.adaptor_point,
    );
    let presign = PrivateSwapRequest {
        kind: RequestKind::PreSign,
        session_id,
        host_commitment,
        key_id: key_info.key_id,
        binding_token: binding.binding_token,
        adaptor_point: key_info.adaptor_point,
        payload: &claim_wire,
        ..empty
    };
    assert!(matches!(
        session
            .prepare_request(wallet, &encode_private_swap_request(&presign))
            .expect("presign prepares"),
        super::PrivateSwapPrepared::Review(_)
    ));
    let nonce = wire::parse_response(&session.confirm(wallet).expect("presign confirms"))
        .expect("nonce response parses");
    assert_eq!(nonce.kind, ResponseKind::Nonce);
    assert!(session.awaiting_reveal());
    SwapAwaitingReveal {
        session,
        key_info,
        binding_token: binding.binding_token,
        session_id,
        sighash: nonce.commitment,
        host_secret,
        claim_wire,
    }
}

fn swap_reveal(
    reveal: &shared_signer::covenant_sign::private_swap::PrivateSwapReveal,
) -> [u8; shared_signer::covenant_sign::private_swap::REVEAL_LEN] {
    use shared_signer::covenant_sign::private_swap as wire;
    let mut out = [0u8; wire::REVEAL_LEN];
    wire::encode_reveal(reveal, &mut out).expect("reveal encodes");
    out
}

#[test]
fn private_swap_session_covers_key_bind_presign_reveal_and_completion() {
    use shared_signer::covenant_sign::private_swap::{
        self as wire, PrivateSwapRequest, PrivateSwapReveal, RequestKind, ResponseKind,
    };

    let wallet = restored_wallet();
    let mut swap = private_swap_awaiting_reveal(&wallet);
    let reveal = PrivateSwapReveal {
        session_id: swap.session_id,
        key_id: swap.key_info.key_id,
        sighash: swap.sighash,
        host_secret: swap.host_secret,
    };
    let presignature = wire::parse_response(
        &swap
            .session
            .finalize_reveal(&wallet, &swap_reveal(&reveal))
            .expect("reveal finalizes"),
    )
    .expect("presignature response parses");
    assert_eq!(presignature.kind, ResponseKind::PreSignature);
    assert!(!swap.session.awaiting_reveal());

    let complete = PrivateSwapRequest {
        kind: RequestKind::Complete,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: swap.key_info.key_id,
        binding_token: swap.binding_token,
        adaptor_point: swap.key_info.adaptor_point,
        presignature: presignature.signature,
        presignature_negated: presignature.negated,
        payload: &swap.claim_wire,
    };
    assert!(matches!(
        swap.session
            .prepare_request(&wallet, &encode_private_swap_request(&complete))
            .expect("completion prepares"),
        super::PrivateSwapPrepared::Review(_)
    ));
    let completed =
        wire::parse_response(&swap.session.confirm(&wallet).expect("completion confirms"))
            .expect("completed response parses");
    assert_eq!(completed.kind, ResponseKind::Completed);
    assert_ne!(completed.signature, [0; 64]);

    // A completion carrying another presignature is refused.
    let tampered = PrivateSwapRequest {
        presignature: [0x11; 64],
        ..complete
    };
    assert!(swap
        .session
        .prepare_request(&wallet, &encode_private_swap_request(&tampered))
        .is_err());
}

#[test]
fn private_swap_rejects_every_mismatched_reveal_field_and_out_of_order_steps() {
    use shared_signer::covenant_sign::private_swap::PrivateSwapReveal;

    let wallet = restored_wallet();
    let mut swap = private_swap_awaiting_reveal(&wallet);
    let good = PrivateSwapReveal {
        session_id: swap.session_id,
        key_id: swap.key_info.key_id,
        sighash: swap.sighash,
        host_secret: swap.host_secret,
    };
    for bad in [
        PrivateSwapReveal {
            session_id: [0x01; shared_signer::covenant_sign::private_swap::SESSION_ID_LEN],
            ..good
        },
        PrivateSwapReveal {
            key_id: [0x02; 32],
            ..good
        },
        PrivateSwapReveal {
            sighash: [0x03; 32],
            ..good
        },
        PrivateSwapReveal {
            host_secret: [0x04; 32],
            ..good
        },
    ] {
        assert!(matches!(
            swap.session.finalize_reveal(&wallet, &swap_reveal(&bad)),
            Err(HotWalletError::CryptoOperationFailed)
        ));
        assert!(swap.session.awaiting_reveal());
    }
    // Confirming again while awaiting the reveal is out of order.
    assert!(matches!(
        swap.session.confirm(&wallet),
        Err(HotWalletError::InvalidToolInput)
    ));
    // Raw-key wallets cannot run the protocol at all.
    let raw = HotWallet::import_raw_private_key_hex(&format!("{:064x}", 1u8)).unwrap();
    assert!(swap.session.confirm(&raw).is_err());
    assert!(swap
        .session
        .finalize_reveal(&raw, &swap_reveal(&good))
        .is_err());

    swap.session.reset_active();
    assert!(!swap.session.awaiting_reveal());
    assert!(matches!(
        swap.session.finalize_reveal(&wallet, &swap_reveal(&good)),
        Err(HotWalletError::CryptoOperationFailed)
    ));
}

#[test]
fn private_swap_binding_requires_the_allocated_key_and_adaptor_point() {
    use shared_signer::covenant_sign::private_swap::{
        self as wire, PrivateSwapRequest, RequestKind,
    };

    let wallet = restored_wallet();
    let swap = private_swap_awaiting_reveal(&wallet);
    let mut session = super::PrivateSwapSession::new();
    let key_request = PrivateSwapRequest {
        kind: RequestKind::KeyInfo,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: [0; 32],
        binding_token: [0; 32],
        adaptor_point: [0; 32],
        presignature: [0; 64],
        presignature_negated: false,
        payload: &[],
    };
    let super::PrivateSwapPrepared::Response(response) = session
        .prepare_request(&wallet, &encode_private_swap_request(&key_request))
        .unwrap()
    else {
        panic!("key-info must be immediate");
    };
    let key_info = wire::parse_response(&response).unwrap();
    let mut claim = Transaction::try_new().expect("claim allocation");
    parse_compact_kspt(&private_swap_claim_wire(key_info.claim_pubkey), &mut claim).unwrap();
    let redeem = claim.redeem_bytes(0).to_vec();
    fn bind<'a>(
        template: &PrivateSwapRequest<'a>,
        payload: &'a [u8],
        key_id: [u8; 32],
        adaptor_point: [u8; 32],
    ) -> PrivateSwapRequest<'a> {
        PrivateSwapRequest {
            kind: RequestKind::Bind,
            session_id: template.session_id,
            host_commitment: template.host_commitment,
            key_id,
            binding_token: template.binding_token,
            adaptor_point,
            presignature: template.presignature,
            presignature_negated: template.presignature_negated,
            payload,
        }
    }
    for wrong in [
        bind(
            &key_request,
            &redeem,
            swap.key_info.key_id,
            key_info.adaptor_point,
        ),
        bind(
            &key_request,
            &redeem,
            key_info.key_id,
            swap.key_info.adaptor_point,
        ),
    ] {
        assert!(matches!(
            session.prepare_request(&wallet, &encode_private_swap_request(&wrong)),
            Err(HotWalletError::InvalidToolInput)
        ));
    }
    assert!(session
        .prepare_request(
            &wallet,
            &encode_private_swap_request(&bind(
                &key_request,
                &redeem,
                key_info.key_id,
                key_info.adaptor_point
            ))
        )
        .is_ok());
}

#[test]
fn recovery_material_routes_words_and_rejects_malformed_inputs() {
    let expected_kpub = restored_wallet().export_kpub().unwrap();
    let from_words = HotWallet::restore_recovery_material(MNEMONIC_12.as_bytes(), "")
        .expect("UTF-8 recovery words restore");
    assert_eq!(from_words.export_kpub().unwrap(), expected_kpub);

    // Standard SeedQR digits whose word indices exceed the 2048-word list.
    assert!(matches!(
        HotWallet::restore_recovery_material(&[b'9'; 48], ""),
        Err(HotWalletError::InvalidToolInput)
    ));
    // Non-UTF-8 bytes of a length that is neither SeedQR form.
    assert!(matches!(
        HotWallet::restore_recovery_material(&[0xff; 5], ""),
        Err(HotWalletError::InvalidToolInput)
    ));
    // 48 bytes that are not all digits are treated as words and rejected.
    assert!(HotWallet::restore_recovery_material(&[b'a'; 48], "").is_err());
}

/// Run both anti-klepto rounds and check the signed transcript with the
/// host-side verifier a watcher uses.
fn anti_klepto_round_trip(wallet: &HotWallet, transaction: &[u8]) -> Result<(), HotWalletError> {
    use shared_signer::anti_klepto as wire;

    let host_secret = [0x3cu8; 32];
    let mut request = vec![0u8; 256 + transaction.len()];
    let length = wire::encode_request(&host_secret, transaction, &mut request).unwrap();
    request.truncate(length);
    let (mut session, commitment_wire) = wallet.prepare_anti_klepto(&request)?;
    let commitment = wire::parse_commitment(&commitment_wire).unwrap();
    offline_signer::transaction::kspt::validate_host_commitment_wire(transaction, &commitment)
        .expect("signer commitment is well formed");

    let mut reveal = vec![0u8; 128];
    let length = wire::encode_reveal(&commitment.session_id, &host_secret, &mut reveal).unwrap();
    reveal.truncate(length);
    let signed_wire = wallet.finalize_anti_klepto(&mut session, &reveal)?;
    let signed = wire::parse_signed(&signed_wire).unwrap();
    offline_signer::transaction::kspt::verify_host_transcript_wire(
        transaction,
        signed.transaction,
        &commitment,
        &signed,
        &host_secret,
    )
    .expect("finalized signatures commit to the host secret");
    Ok(())
}

#[test]
fn anti_klepto_signatures_verify_against_the_host_transcript_for_each_wallet_kind() {
    let wallet = restored_wallet();
    anti_klepto_round_trip(&wallet, &owned_compact_kspt(&wallet)).unwrap();

    let mut private_key = [0u8; 32];
    private_key[31] = 1;
    let raw = HotWallet::import_raw_private_key_hex(&format!("{:064x}", 1u8)).unwrap();
    anti_klepto_round_trip(&raw, &owned_raw_compact_kspt(&private_key)).unwrap();
}

#[test]
fn anti_klepto_reveal_must_match_session_and_commitment() {
    use shared_signer::anti_klepto as wire;

    let wallet = restored_wallet();
    let transaction = owned_compact_kspt(&wallet);
    let host_secret = [0x3cu8; 32];
    let mut request = vec![0u8; 256 + transaction.len()];
    let length = wire::encode_request(&host_secret, &transaction, &mut request).unwrap();
    request.truncate(length);
    let (mut session, commitment_wire) = wallet.prepare_anti_klepto(&request).unwrap();
    let session_id = wire::parse_commitment(&commitment_wire).unwrap().session_id;

    let reveal = |session_id: &[u8; wire::SESSION_ID_LEN], secret: &[u8; 32]| {
        let mut out = vec![0u8; 128];
        let length = wire::encode_reveal(session_id, secret, &mut out).unwrap();
        out.truncate(length);
        out
    };
    let mut other_session = session_id;
    other_session[0] ^= 1;
    for bad in [
        reveal(&other_session, &host_secret),
        reveal(&session_id, &[0x3d; 32]),
    ] {
        assert!(matches!(
            wallet.finalize_anti_klepto(&mut session, &bad),
            Err(HotWalletError::CryptoOperationFailed)
        ));
    }
    assert!(wallet
        .finalize_anti_klepto(&mut session, &reveal(&session_id, &host_secret))
        .is_ok());
}

fn owned_standard_pskb(wallet: &HotWallet) -> Vec<u8> {
    let account = derive_account_key(&wallet.seed.bytes).expect("account derivation");
    let xonly = derive_address_key(&account, 0)
        .expect("receive derivation")
        .public_key_x_only()
        .expect("x-only public key");
    let pskt = serde_json::json!([{
        "global": {
            "version": 0,
            "txVersion": 1,
            "fallbackLockTime": null,
            "inputsModifiable": false,
            "outputsModifiable": false,
            "inputCount": 1,
            "outputCount": 1,
            "xpubs": {},
            "id": null,
            "proprietaries": {},
            "subnetworkId": "00".repeat(20)
        },
        "inputs": [{
            "utxoEntry": {
                "amount": "100000",
                "scriptPublicKey": format!("000020{}ac", hex_string(&xonly)),
                "blockDaaScore": "0",
                "isCoinbase": false
            },
            "previousOutpoint": {"transactionId": "11".repeat(32), "index": 7},
            "sequence": u64::MAX.to_string(),
            "minTime": null,
            "partialSigs": {},
            "sighashType": 1,
            "redeemScript": null,
            "sigOpCount": 1,
            "bip32Derivations": {},
            "finalScriptSig": null,
            "proprietaries": {}
        }],
        "outputs": [{
            "amount": "99000",
            "scriptPublicKey": format!("000020{}ac", "44".repeat(32)),
            "redeemScript": null,
            "bip32Derivations": {},
            "proprietaries": {}
        }]
    }]);
    let mut wire = b"PSKB".to_vec();
    wire.extend_from_slice(hex_string(&serde_json::to_vec(&pskt).unwrap()).as_bytes());
    wire
}

#[test]
fn standard_pskb_signing_returns_a_verifiably_signed_pskb() {
    let wallet = restored_wallet();
    let signed = wallet
        .sign_transaction(&owned_standard_pskb(&wallet))
        .expect("standard PSKB signs");
    assert!(signed.starts_with(b"PSKB"));
    assert_eq!(
        kaskold_protocol::pskt_verified_signature_counts(
            &hex_string(&signed),
            kaskold_protocol::Network::Mainnet
        )
        .expect("signed PSKB verifies"),
        vec![1]
    );
}

#[test]
fn passphrases_are_bounded_on_every_restore_path() {
    let longest = "p".repeat(MAX_BIP39_PASSPHRASE_LEN);
    let too_long = "p".repeat(MAX_BIP39_PASSPHRASE_LEN + 1);
    assert!(HotWallet::restore(MNEMONIC_12, &longest).is_ok());
    assert!(matches!(
        HotWallet::restore(MNEMONIC_12, &too_long),
        Err(HotWalletError::Bip39PassphraseTooLong)
    ));

    // Standard SeedQR digits for the same twelve words.
    let seedqr = format!("{}0003", "0000".repeat(11));
    let restored = HotWallet::restore_recovery_material(seedqr.as_bytes(), &longest)
        .expect("SeedQR restores with the longest passphrase");
    assert_eq!(
        restored.export_kpub().unwrap(),
        HotWallet::restore(MNEMONIC_12, &longest)
            .unwrap()
            .export_kpub()
            .unwrap()
    );
    assert!(matches!(
        HotWallet::restore_recovery_material(seedqr.as_bytes(), &too_long),
        Err(HotWalletError::Bip39PassphraseTooLong)
    ));
}

#[test]
fn wallet_creation_validates_word_count_passphrase_and_user_entropy() {
    let longest = "p".repeat(MAX_BIP39_PASSPHRASE_LEN);
    assert!(HotWallet::create_with_additive_entropy(12, &[1, 6], b"touch", &longest).is_ok());
    assert!(matches!(
        HotWallet::create_with_additive_entropy(18, &[], &[], ""),
        Err(HotWalletError::InvalidMnemonicWordCount)
    ));
    assert!(matches!(
        HotWallet::create_with_additive_entropy(12, &[], &[], &format!("{longest}p")),
        Err(HotWalletError::Bip39PassphraseTooLong)
    ));
    assert!(matches!(
        HotWallet::create_with_additive_entropy(12, &[7], &[], ""),
        Err(HotWalletError::InvalidToolInput)
    ));
}

#[test]
fn raw_private_key_text_imports_the_exact_key_bytes() {
    // SHA-256 of 0x10 00..00 01 starts e9a5fbe5 (computed independently).
    let wallet =
        HotWallet::import_raw_private_key_hex(&format!("10{}01", "00".repeat(30))).unwrap();
    assert_eq!(wallet.fingerprint_hex().unwrap(), "e9a5fbe5");
    assert!(HotWallet::import_raw_private_key_hex(&format!("1g{}01", "00".repeat(30))).is_err());
    assert!(HotWallet::import_raw_private_key_hex(&format!("g0{}01", "00".repeat(30))).is_err());
    assert!(HotWallet::import_raw_private_key_hex("01").is_err());
}

#[test]
fn portable_backups_require_their_magic_and_digit_runs_route_by_length() {
    let backup = restored_wallet().portable_backup("password").unwrap();
    assert!(HotWallet::restore_portable_backup(&backup, "password").is_ok());
    let mut renamed = backup.clone();
    renamed[0] ^= 0x20;
    assert!(matches!(
        HotWallet::restore_portable_backup(&renamed, "password"),
        Err(HotWalletError::InvalidSealedWallet)
    ));

    // Thirty-two ASCII digits are CompactSeedQR entropy, not a SeedQR digit run.
    assert!(HotWallet::restore_recovery_material(&[b'0'; 32], "").is_ok());
    assert!(HotWallet::restore_recovery_material(&[b'0'; 16], "").is_ok());
}
