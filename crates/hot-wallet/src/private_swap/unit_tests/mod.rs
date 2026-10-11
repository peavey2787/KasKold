//! Private-swap sessions wipe pending keys and refuse a corrupted nonce.

use core::ptr::addr_of;

use super::PrivateSwapSession;
use crate::unit_tests::zeroization::dropped;
use crate::unit_tests::{private_swap_awaiting_reveal, restored_wallet, swap_reveal};
use crate::HotWalletError;

fn pending_session() -> PrivateSwapSession {
    let mut session = PrivateSwapSession::new();
    session.pending_key_id = [1; 32];
    session.pending_pubkey = [2; 32];
    session.pending_adaptor_point = [3; 32];
    session.key_id = [4; 32];
    session
}

#[test]
fn clear_all_wipes_pending_and_active_keys() {
    let mut session = pending_session();
    session.clear_all();
    assert_eq!(session.pending_key_id, [0; 32]);
    assert_eq!(session.pending_pubkey, [0; 32]);
    assert_eq!(session.pending_adaptor_point, [0; 32]);
    assert_eq!(session.key_id, [0; 32]);
}

#[test]
fn dropping_a_session_wipes_its_pending_keys() {
    let slot = dropped(pending_session());
    let session = slot.as_ptr();
    // SAFETY: the fields are plain byte arrays left by `drop`.
    let keys = unsafe {
        [
            addr_of!((*session).pending_key_id).read(),
            addr_of!((*session).pending_pubkey).read(),
            addr_of!((*session).pending_adaptor_point).read(),
            addr_of!((*session).key_id).read(),
        ]
    };
    assert_eq!(keys, [[0; 32]; 4]);
}

#[test]
fn a_corrupted_nonce_commitment_never_yields_a_presignature() {
    use shared_signer::covenant_sign::private_swap::PrivateSwapReveal;

    let wallet = restored_wallet();
    let mut swap = private_swap_awaiting_reveal(&wallet);
    swap.session.nonce_point[1] ^= 1;
    let reveal = PrivateSwapReveal {
        session_id: swap.session_id,
        key_id: swap.key_info.key_id,
        sighash: swap.sighash,
        host_secret: swap.host_secret,
    };
    assert!(matches!(
        swap.session.finalize_reveal(&wallet, &swap_reveal(&reveal)),
        Err(HotWalletError::CryptoOperationFailed)
    ));
}

#[test]
fn only_mnemonic_wallets_may_start_a_private_swap() {
    use shared_signer::covenant_sign::private_swap::{
        self as wire, PrivateSwapRequest, RequestKind,
    };

    let raw = crate::HotWallet::import_raw_private_key_hex(&format!("{:064x}", 1u8)).unwrap();
    let request = PrivateSwapRequest {
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
    let mut session = PrivateSwapSession::new();
    assert!(matches!(
        session.prepare_request(
            &raw,
            &crate::unit_tests::encode_private_swap_request(&request)
        ),
        Err(HotWalletError::UnsupportedForWalletType)
    ));
}
