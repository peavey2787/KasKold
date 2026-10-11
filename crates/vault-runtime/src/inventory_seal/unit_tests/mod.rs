use super::*;

const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const KEY: [u8; PLATFORM_WRAPPING_KEY_LEN] = [0x5a; PLATFORM_WRAPPING_KEY_LEN];

fn two_wallet_inventory() -> Vec<u8> {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    runtime
        .add_raw_private_key(&format!("{:064x}", 1u8))
        .unwrap();
    runtime.switch_wallet(1).unwrap();
    runtime.seal_native_inventory(&KEY).unwrap()
}

fn rejected(sealed: &[u8]) -> bool {
    matches!(
        VaultRuntime::new().unlock_native_inventory(sealed, &KEY),
        Err(VaultRuntimeError::InvalidSealedInventory)
    )
}

/// Re-authenticate an edited body so only the structural check under test
/// can reject it.
fn resealed(mut body: Vec<u8>) -> Vec<u8> {
    let tag = inventory_mac(&KEY, &body).unwrap().finalize().into_bytes();
    body.extend_from_slice(&tag);
    body
}

#[test]
fn inventory_tag_is_hmac_sha256_over_the_domain_and_body() {
    // Recorded from the pre-`hmac`-crate implementation: inventories sealed
    // by earlier builds must keep authenticating.
    let tag = inventory_mac(&[7; 32], b"KasKold inventory KAT")
        .unwrap()
        .finalize()
        .into_bytes();
    assert_eq!(
        tag.as_slice(),
        [
            0x5d, 0xc7, 0x25, 0x27, 0xb1, 0xdb, 0xc2, 0xfd, 0xae, 0xdd, 0x08, 0xf4, 0x2b, 0x2c,
            0x1b, 0x3e, 0xdf, 0x26, 0xea, 0x01, 0x66, 0x51, 0x6c, 0xff, 0x08, 0xc0, 0x16, 0x83,
            0x83, 0x50, 0xce, 0x04,
        ]
    );
}

#[test]
fn structurally_valid_tampering_is_rejected_by_the_tag_alone() {
    let sealed = two_wallet_inventory();
    assert_eq!(sealed[4], 1, "fixture's active slot is the second wallet");

    // Selecting the other (valid) slot changes no structure.
    let mut other_active = sealed.clone();
    other_active[4] = 0;
    assert!(rejected(&other_active));
    // So does a bit flip in the tag itself.
    let mut bad_tag = sealed.clone();
    *bad_tag.last_mut().unwrap() ^= 0x01;
    assert!(rejected(&bad_tag));
    // A different wrapping key never authenticates.
    assert!(matches!(
        VaultRuntime::new().unlock_native_inventory(&sealed, &[0x5b; PLATFORM_WRAPPING_KEY_LEN]),
        Err(VaultRuntimeError::InvalidSealedInventory)
    ));
    assert!(VaultRuntime::new()
        .unlock_native_inventory(&sealed, &KEY)
        .is_ok());
}

#[test]
fn inventories_shorter_than_header_and_tag_are_rejected() {
    let sealed = two_wallet_inventory();
    let minimum = HEADER_LEN + TAG_LEN;
    assert!(matches!(
        authenticated_inventory_body(&sealed[..minimum - 1], &KEY),
        Err(VaultRuntimeError::InvalidSealedInventory)
    ));
    // Exactly header + tag is long enough to be authenticated (and then
    // rejected for its empty slot set).
    let empty = resealed(sealed[..HEADER_LEN].to_vec());
    assert_eq!(empty.len(), minimum);
    assert!(authenticated_inventory_body(&empty, &KEY).is_ok());
}

#[test]
fn header_requires_zero_padding_a_bounded_count_and_an_existing_active_slot() {
    let max = crate::wallet_tools::MAX_SOFTWARE_WALLETS;
    let header = |active: u8, count: u8, pad: [u8; 2]| {
        let mut body = b"KVI1".to_vec();
        body.extend_from_slice(&[active, count, pad[0], pad[1]]);
        body
    };
    assert_eq!(
        parse_inventory_header(&header(1, 2, [0, 0])).unwrap(),
        (1, 2)
    );
    assert!(parse_inventory_header(&header(0, 2, [1, 0])).is_err());
    assert!(parse_inventory_header(&header(0, 2, [0, 1])).is_err());
    assert!(parse_inventory_header(&header(0, 0, [0, 0])).is_err());
    let max = u8::try_from(max).unwrap();
    assert_eq!(
        parse_inventory_header(&header(0, max, [0, 0])).unwrap(),
        (0, usize::from(max))
    );
    assert!(parse_inventory_header(&header(0, max + 1, [0, 0])).is_err());
    assert!(parse_inventory_header(&header(2, 2, [0, 0])).is_err());
}

#[test]
fn sealed_slots_must_be_non_empty_and_bounded() {
    let slot = |len: u16| {
        let mut body = len.to_le_bytes().to_vec();
        body.resize(2 + usize::from(len), 0);
        body
    };
    for len in [0, u16::try_from(MAX_SEALED_SLOT_LEN + 1).unwrap()] {
        let body = slot(len);
        let mut cursor = 0;
        assert!(matches!(
            read_inventory_wallet(&body, &mut cursor, &KEY),
            Err(VaultRuntimeError::InvalidSealedInventory)
        ));
    }
    // A maximal slot passes the length gate and reaches wallet decryption.
    let body = slot(u16::try_from(MAX_SEALED_SLOT_LEN).unwrap());
    let mut cursor = 0;
    assert!(matches!(
        read_inventory_wallet(&body, &mut cursor, &KEY),
        Err(VaultRuntimeError::Custody(_))
    ));
}

#[test]
fn sealing_requires_an_unlocked_bounded_inventory() {
    assert!(matches!(
        validate_inventory_for_seal(&VaultRuntime::new()),
        Err(VaultRuntimeError::Locked)
    ));
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    while runtime.wallets.len() < crate::wallet_tools::MAX_SOFTWARE_WALLETS {
        runtime
            .add_raw_private_key(&format!("{:064x}", runtime.wallets.len() + 1))
            .unwrap();
    }
    assert!(validate_inventory_for_seal(&runtime).is_ok());
    runtime
        .wallets
        .push(hot_wallet::HotWallet::restore(MNEMONIC, "").unwrap());
    runtime.wallet_names.push("Extra".to_owned());
    assert!(matches!(
        validate_inventory_for_seal(&runtime),
        Err(VaultRuntimeError::InvalidWalletIndex)
    ));
}
