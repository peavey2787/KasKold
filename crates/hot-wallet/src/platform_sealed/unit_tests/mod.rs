use super::*;

const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn plaintext(source: u8) -> [u8; SEALED_WALLET_PLAINTEXT_LEN] {
    let mut plaintext = [0u8; SEALED_WALLET_PLAINTEXT_LEN];
    plaintext[0] = source;
    plaintext
}

fn recovery() -> RecoveryMaterial {
    RecoveryMaterial {
        word_count: 12,
        indices: [0; 24],
        passphrase: [0; MAX_BIP39_PASSPHRASE_LEN],
        passphrase_len: 0,
    }
}

#[test]
fn only_khv3_containers_of_the_exact_length_are_decrypted() {
    let key = [0x21u8; PLATFORM_WRAPPING_KEY_LEN];
    let mut sealed = HotWallet::restore(MNEMONIC, "")
        .unwrap()
        .seal_for_platform(&key)
        .unwrap();
    assert!(HotWallet::restore_platform_sealed(&sealed, &key).is_ok());
    sealed[3] = b'2';
    assert!(matches!(
        HotWallet::restore_platform_sealed(&sealed, &key),
        Err(HotWalletError::InvalidSealedWallet)
    ));
}

#[test]
fn mnemonic_secret_record_tail_must_be_zero() {
    let mut seed = plaintext(SOURCE_MNEMONIC);
    seed[1..65].fill(0x42);
    assert!(decode_v3_mnemonic(&seed, None).is_ok());
    seed[SECRET_RECORD_LEN] = 1;
    assert!(matches!(
        decode_v3_mnemonic(&seed, None),
        Err(HotWalletError::InvalidSealedWallet)
    ));
}

#[test]
fn raw_key_records_carry_no_recovery_and_a_zero_tail() {
    let mut raw = plaintext(SOURCE_RAW_PRIVATE_KEY);
    raw[32] = 1;
    assert!(decode_v3_raw_key(&raw, None).is_ok());
    assert!(matches!(
        decode_v3_raw_key(&raw, Some(recovery())),
        Err(HotWalletError::InvalidSealedWallet)
    ));
    raw[SECRET_RECORD_LEN] = 1;
    assert!(matches!(
        decode_v3_raw_key(&raw, None),
        Err(HotWalletError::InvalidSealedWallet)
    ));
}

#[test]
fn account_records_require_an_account_depth_key() {
    let wallet = HotWallet::restore(MNEMONIC, "").unwrap();
    let account =
        offline_signer::derivation::bip32::derive_account_key(wallet.seed_bytes().unwrap())
            .unwrap();
    let mut record = plaintext(SOURCE_ACCOUNT_XPRV);
    record[1..66].copy_from_slice(&account.to_raw());
    assert!(decode_v3_account(&record, None).is_ok());

    let mut shallow = account.to_raw();
    // `to_raw` is key || chain code || depth.
    shallow[64] = 2;
    record[1..66].copy_from_slice(&shallow);
    assert!(matches!(
        decode_v3_account(&record, None),
        Err(HotWalletError::InvalidSealedWallet)
    ));
}

#[test]
fn recovery_records_round_trip_indices_and_the_longest_passphrase() {
    let mut material = recovery();
    material.indices[0] = 0x0123;
    material.indices[11] = 0x07ff;
    material.passphrase.fill(b'p');
    material.passphrase_len = u8::try_from(MAX_BIP39_PASSPHRASE_LEN).unwrap();
    let mut record = [0u8; RECOVERY_RECORD_LEN];
    encode_recovery(&material, &mut record);
    let decoded = decode_recovery(&record).unwrap().expect("recovery present");
    assert_eq!(decoded.indices, material.indices);
    assert_eq!(decoded.passphrase_len, material.passphrase_len);
}
