//! Inventory naming/active-slot bookkeeping and the privacy tools return the
//! active wallet's real responses.

use super::{MNEMONIC, MNEMONIC_24};
use crate::{VaultRuntime, VaultRuntimeError};

fn names(runtime: &VaultRuntime) -> Vec<String> {
    runtime
        .wallet_summaries()
        .unwrap()
        .into_iter()
        .map(|summary| summary.name)
        .collect()
}

fn active(runtime: &VaultRuntime) -> usize {
    runtime
        .wallet_summaries()
        .unwrap()
        .into_iter()
        .position(|summary| summary.active)
        .expect("an active wallet")
}

#[test]
fn added_wallets_get_sequential_default_names_and_become_active() {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    runtime.add_restored_wallet(MNEMONIC_24, "").unwrap();
    runtime.add_wallet_12().unwrap();
    runtime
        .add_raw_private_key(&format!("{:064x}", 1u8))
        .unwrap();
    assert_eq!(
        names(&runtime),
        ["Wallet 1", "Wallet 2", "Wallet 3", "Wallet 4"]
    );
    assert_eq!(active(&runtime), 3);

    runtime.add_wallet_24().unwrap();
    assert_eq!(
        names(&runtime).len(),
        crate::wallet_tools::MAX_SOFTWARE_WALLETS
    );
    assert!(matches!(
        runtime.add_wallet_12(),
        Err(VaultRuntimeError::WalletCapacity)
    ));
}

#[test]
fn deleting_the_active_last_wallet_activates_the_new_last_wallet() {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    runtime.add_restored_wallet(MNEMONIC_24, "").unwrap();
    runtime.add_wallet_12().unwrap();
    assert_eq!(active(&runtime), 2);
    runtime.delete_wallet(2).unwrap();
    assert_eq!(active(&runtime), 1);
    runtime.delete_wallet(0).unwrap();
    assert_eq!(active(&runtime), 0);
    runtime.delete_wallet(0).unwrap();
    assert!(matches!(
        runtime.wallet_summaries(),
        Err(VaultRuntimeError::Locked)
    ));
}

#[test]
fn watch_kpub_normalization_returns_the_canonical_account() {
    let mut runtime = VaultRuntime::new();
    let kpub = runtime.restore_wallet(MNEMONIC, "").unwrap();
    assert_eq!(
        VaultRuntime::normalize_watch_kpub(kpub.as_bytes()).unwrap(),
        kpub
    );
}

#[test]
fn privacy_tools_answer_with_the_active_wallets_keys() {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    let wallet = hot_wallet::HotWallet::restore(MNEMONIC, "").unwrap();

    let request = kaskold_protocol::create_privacy_pairing_request([0x5a; 16], 0, 2, 0, 1).unwrap();
    let response = runtime.privacy_pairing_response(&request.payload).unwrap();
    let batch = kaskold_protocol::accept_privacy_pairing_response(
        &request,
        &response,
        kaskold_protocol::Network::Mainnet,
        None,
    )
    .unwrap();
    assert_eq!(batch.receive_addresses.len(), 2);
    assert_eq!(batch.change_addresses.len(), 1);

    let mut scan = [0u8; 37];
    scan[..4].copy_from_slice(b"STLH");
    scan[4] = 1;
    scan[5..].copy_from_slice(&[0x11; 32]);
    let answer = runtime.stealth_scan_request(&scan).unwrap();
    assert_eq!(answer, wallet.stealth_scan_request(&scan).unwrap());
    assert_eq!(answer.len(), 5 + 64);
    assert_eq!(answer[4], 1);
}

#[test]
fn portable_xprv_backup_restores_the_same_account() {
    let mut runtime = VaultRuntime::new();
    let kpub = runtime.restore_wallet(MNEMONIC, "").unwrap();
    let backup = runtime.portable_xprv_backup("pw").unwrap();
    assert!(backup.len() > 1);
    let mut restored = VaultRuntime::new();
    restored.restore_wallet(MNEMONIC_24, "").unwrap();
    let summary = restored.add_portable_backup(&backup, "pw").unwrap();
    assert_eq!(summary.kpub.as_deref(), Some(kpub.as_str()));
}
