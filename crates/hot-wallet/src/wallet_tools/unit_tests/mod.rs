use super::*;

const LEGAL_12: &str =
    "legal winner thank year wave sausage worth useful legal winner thank yellow";
const LEGAL_24: &str = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";
const LETTER_24: &str = "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic bless";

#[test]
fn mnemonic_fingerprint_hashes_reconstructed_entropy_and_passphrase() {
    // Expected values are SHA-256(entropy || passphrase)[..4] computed
    // independently from the BIP39 test-vector entropy.
    let fingerprint = |phrase: &str, passphrase: &str| {
        HotWallet::restore(phrase, passphrase)
            .unwrap()
            .fingerprint_hex()
            .unwrap()
    };
    assert_eq!(fingerprint(LEGAL_12, "TREZOR"), "715fb54a"); // 7f * 16
    assert_eq!(fingerprint(LEGAL_24, ""), "17a7384b"); // 7f * 32
    assert_eq!(fingerprint(LETTER_24, "pw"), "79577c3b"); // 80 * 32
}

#[test]
fn raw_key_fingerprint_hashes_the_key() {
    let wallet = HotWallet::import_raw_private_key_hex(&format!("{:064x}", 1u8)).unwrap();
    assert_eq!(wallet.fingerprint_hex().unwrap(), "ec4916dd");
}

#[test]
fn public_account_normalization_returns_the_canonical_text() {
    let wallet = HotWallet::restore(LEGAL_12, "").unwrap();
    let kpub = wallet.export_kpub().unwrap();
    assert_eq!(normalize_public_account(kpub.as_bytes()).unwrap(), kpub);
    assert!(normalize_public_account(b"kpub-not").is_err());
}

#[test]
fn address_requests_need_a_known_network_and_a_non_hardened_index() {
    let wallet = HotWallet::restore(LEGAL_12, "").unwrap();
    assert!(wallet
        .derive_address(KaspaNetwork::Mainnet, false, 0x7fff_ffff)
        .is_ok());
    assert!(matches!(
        wallet.derive_address(KaspaNetwork::Mainnet, false, 0x8000_0000),
        Err(HotWalletError::InvalidToolInput)
    ));
    assert!(matches!(
        wallet.derive_address(KaspaNetwork::Unknown, false, 0),
        Err(HotWalletError::InvalidToolInput)
    ));
}

#[test]
fn commit_secrets_must_be_non_empty_and_bounded() {
    assert!(matches!(
        validate_commit_secret(b""),
        Err(HotWalletError::InvalidToolInput)
    ));
    assert!(validate_commit_secret(&[0x41; 1]).is_ok());
    assert!(validate_commit_secret(&[0x41; MAX_COMMIT_SECRET_LEN]).is_ok());
    assert!(matches!(
        validate_commit_secret(&[0x41; MAX_COMMIT_SECRET_LEN + 1]),
        Err(HotWalletError::InvalidToolInput)
    ));
    let wallet = HotWallet::restore(LEGAL_12, "").unwrap();
    assert!(wallet.commit_secret(b"").is_err());
}
