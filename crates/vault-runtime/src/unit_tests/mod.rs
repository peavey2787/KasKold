use super::{ScanResult, VaultRuntime, VaultRuntimeError, WalletKind};
use hot_wallet::PLATFORM_WRAPPING_KEY_LEN;
use kaskold_protocol::{encode_qr_frames, QrFrame};
use offline_signer::{
    derivation::{
        bip32::{derive_account_key, derive_address_key},
        bip39::{self, Mnemonic12},
    },
    transaction::{kspt::serialize_compact_kspt_vec, model::Transaction},
    OfflineSigner,
};

const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const MNEMONIC_24: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

fn mnemonic_seed() -> [u8; 64] {
    let mut mnemonic = Mnemonic12 { indices: [0; 12] };
    for (slot, word) in mnemonic.indices.iter_mut().zip(MNEMONIC.split_whitespace()) {
        *slot = bip39::word_to_index(word).expect("known BIP39 word");
    }
    OfflineSigner::new()
        .restore_wallet_12(&mnemonic, "")
        .expect("known mnemonic restores")
        .bytes
}

fn owned_compact_kspt(payload_len: usize) -> Vec<u8> {
    let seed = mnemonic_seed();
    let account = derive_account_key(&seed).expect("account derivation");
    let child = derive_address_key(&account, 0).expect("receive derivation");
    let xonly = child.public_key_x_only().expect("x-only public key");

    let mut transaction = Transaction::try_new().expect("transaction allocation");
    transaction.version = 1;
    transaction.network = offline_signer::address::KaspaNetwork::Mainnet;
    transaction.num_inputs = 1;
    transaction.num_outputs = 1;
    transaction.inputs[0].previous_outpoint.transaction_id = [0x21; 32];
    transaction.inputs[0].previous_outpoint.index = 3;
    transaction.inputs[0].sequence = u64::MAX;
    transaction.inputs[0].sig_op_count = 1;
    transaction.inputs[0].utxo_entry.amount = 80_000;
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
    transaction.outputs[0].value = 79_000;
    {
        let script = &mut transaction.outputs[0].script_public_key;
        script.script[0] = 0x20;
        script.script[1..33].fill(0x45);
        script.script[33] = 0xac;
        script.script_len = 34;
    }
    transaction.payload = vec![0x5a; payload_len];
    serialize_compact_kspt_vec(&transaction).expect("compact KSPT fixture serializes")
}

pub(super) fn owned_qr_frames() -> Vec<QrFrame> {
    // The 1-in/1-out compact KSPT already exceeds single-frame QR capacity.
    // Keep the transaction payload empty so generic Vault payment review remains valid.
    let wire = owned_compact_kspt(0);
    let frames = encode_qr_frames(&wire).expect("QR frames encode");
    assert!(
        frames.len() > 1,
        "fixture must exercise multi-frame progress"
    );
    frames
}

#[test]
fn fresh_runtime_is_locked() {
    let runtime = VaultRuntime::new();
    assert!(!runtime.is_unlocked());
    assert!(matches!(
        runtime.export_public_account(),
        Err(VaultRuntimeError::Locked)
    ));
}

#[test]
fn lock_clears_unlocked_state() {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    assert!(runtime.is_unlocked());
    runtime.lock_wallet();
    assert!(!runtime.is_unlocked());
}

#[test]
fn sealed_persistence_round_trip_preserves_only_public_identity() {
    let key = [0xabu8; PLATFORM_WRAPPING_KEY_LEN];
    let mut runtime = VaultRuntime::new();
    let expected = runtime.restore_wallet(MNEMONIC, "").unwrap();
    let sealed = runtime.seal_wallet(&key).unwrap();
    runtime.lock_wallet();
    let restored = runtime.unlock_sealed_wallet(&sealed, &key).unwrap();
    assert_eq!(restored, expected);
}

#[test]
fn native_inventory_round_trip_preserves_multiple_wallets_names_and_active_slot() {
    let key = [0x39u8; PLATFORM_WRAPPING_KEY_LEN];
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    runtime.set_wallet_name(0, "Primary").unwrap();
    runtime.add_restored_wallet(MNEMONIC_24, "").unwrap();
    runtime.set_wallet_name(1, "Savings").unwrap();
    runtime.switch_wallet(0).unwrap();

    let sealed = runtime.seal_native_inventory(&key).unwrap();
    assert!(sealed.starts_with(b"KVI1"));

    let mut restored = VaultRuntime::new();
    let kpub = restored.unlock_native_inventory(&sealed, &key).unwrap();
    assert!(kpub
        .as_deref()
        .is_some_and(|value| value.starts_with("kpub")));
    let wallets = restored.wallet_summaries().unwrap();
    assert_eq!(wallets.len(), 2);
    assert_eq!(wallets[0].name, "Primary");
    assert_eq!(wallets[1].name, "Savings");
    assert!(wallets[0].active);
    assert!(!wallets[1].active);
}

#[test]
fn native_inventory_authentication_rejects_tampering_and_restores_raw_key_slot() {
    let key = [0x52u8; PLATFORM_WRAPPING_KEY_LEN];
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    runtime
        .add_raw_private_key(&format!("{:064x}", 1u8))
        .unwrap();
    let sealed = runtime.seal_native_inventory(&key).unwrap();

    let mut tampered = sealed.clone();
    tampered[8] ^= 0x01;
    assert!(matches!(
        VaultRuntime::new().unlock_native_inventory(&tampered, &key),
        Err(VaultRuntimeError::InvalidSealedInventory)
    ));

    let mut restored = VaultRuntime::new();
    assert_eq!(
        restored.unlock_native_inventory(&sealed, &key).unwrap(),
        None
    );
    let wallets = restored.wallet_summaries().unwrap();
    assert_eq!(wallets.len(), 2);
    assert_eq!(wallets[1].kind, WalletKind::RawPrivateKey);
    assert!(wallets[1].active);
}

#[test]
fn native_inventory_accepts_legacy_single_wallet_ciphertext_as_migration_input() {
    let key = [0x71u8; PLATFORM_WRAPPING_KEY_LEN];
    let mut runtime = VaultRuntime::new();
    let expected = runtime.restore_wallet(MNEMONIC, "").unwrap();
    let legacy_single = runtime.seal_wallet(&key).unwrap();

    let mut restored = VaultRuntime::new();
    assert_eq!(
        restored
            .unlock_native_inventory(&legacy_single, &key)
            .unwrap(),
        Some(expected)
    );
    assert_eq!(restored.wallet_summaries().unwrap().len(), 1);
}

#[test]
fn backup_workflows_are_locked_by_default_and_cover_supported_exports() {
    let mut runtime = VaultRuntime::new();
    assert!(matches!(
        runtime.backup_recovery_phrase(),
        Err(VaultRuntimeError::Locked)
    ));
    assert!(matches!(
        runtime.backup_seedqr(),
        Err(VaultRuntimeError::Locked)
    ));
    assert!(matches!(
        runtime.backup_compact_seedqr(),
        Err(VaultRuntimeError::Locked)
    ));
    assert!(matches!(
        runtime.backup_account_xprv(),
        Err(VaultRuntimeError::Locked)
    ));
    assert!(matches!(
        runtime.backup_receive_private_key_hex(0),
        Err(VaultRuntimeError::Locked)
    ));

    runtime.restore_wallet(MNEMONIC, "").unwrap();
    assert_eq!(runtime.backup_recovery_phrase().unwrap().as_str(), MNEMONIC);
    let seedqr = runtime.backup_seedqr().expect("standard SeedQR backup");
    assert_eq!(seedqr.len(), 48);
    assert!(seedqr.iter().all(u8::is_ascii_digit));
    assert_eq!(&seedqr[44..], b"0003");
    assert_eq!(
        runtime
            .backup_compact_seedqr()
            .expect("compact SeedQR backup")
            .len(),
        16
    );
    assert!(runtime.backup_account_xprv().unwrap().starts_with("kprv"));
    let receive_key = runtime.backup_receive_private_key_hex(0).unwrap();
    assert_eq!(receive_key.len(), 64);
    assert!(receive_key.bytes().all(|byte| byte.is_ascii_hexdigit()));

    runtime.restore_wallet(MNEMONIC_24, "").unwrap();
    assert_eq!(runtime.backup_seedqr().unwrap().len(), 96);
    assert_eq!(runtime.backup_compact_seedqr().unwrap().len(), 32);
}

#[test]
fn signing_session_covers_progress_review_approval_and_reset() {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    runtime.begin_scan().expect("unlocked scan begins");

    let frames = owned_qr_frames();
    for frame in &frames[..frames.len() - 1] {
        match runtime
            .accept_qr_frame(&frame.payload)
            .expect("intermediate QR frame accepted")
        {
            ScanResult::Progress(progress) => {
                assert!(progress.received > 0);
                assert_eq!(progress.total as usize, frames.len());
            }
            ScanResult::Ready(_) => panic!("request completed before final frame"),
        }
    }

    let ready = runtime
        .accept_qr_frame(&frames.last().unwrap().payload)
        .expect("final QR frame completes request");
    let ScanResult::Ready(review) = ready else {
        panic!("final frame must produce review");
    };
    assert_eq!(review.input_count, 1);
    assert_eq!(review.output_count, 1);
    assert_eq!(review.input_total, 80_000);
    assert_eq!(review.output_total, 79_000);
    assert_eq!(review.fee, 1_000);
    assert_eq!(runtime.review_transaction().unwrap(), review);
    assert_eq!(runtime.scan_progress().received, 0);

    assert!(matches!(
        runtime.accept_qr_frame(&frames[0].payload),
        Err(VaultRuntimeError::SigningSessionAlreadyComplete)
    ));

    let response_count = runtime.approve(0).expect("reviewed request signs").len();
    assert!(response_count > 0);
    assert_eq!(
        runtime.signed_response_frames().unwrap().len(),
        response_count
    );

    runtime.reject();
    assert!(matches!(
        runtime.signed_response_frames(),
        Err(VaultRuntimeError::NoSignedResponse)
    ));
    assert!(matches!(
        runtime.review_transaction(),
        Err(VaultRuntimeError::NoPendingReview)
    ));
}

#[test]
fn signing_session_rejects_locked_and_missing_review_states() {
    let mut runtime = VaultRuntime::new();
    assert!(matches!(
        runtime.begin_scan(),
        Err(VaultRuntimeError::Locked)
    ));
    assert!(matches!(
        runtime.accept_qr_frame(b"KSPT"),
        Err(VaultRuntimeError::Locked)
    ));
    assert!(matches!(
        runtime.review_transaction(),
        Err(VaultRuntimeError::Locked)
    ));
    assert!(matches!(runtime.approve(0), Err(VaultRuntimeError::Locked)));

    runtime.restore_wallet(MNEMONIC, "").unwrap();
    runtime.begin_scan().unwrap();
    assert!(matches!(
        runtime.review_transaction(),
        Err(VaultRuntimeError::NoPendingReview)
    ));
    assert!(matches!(
        runtime.approve(0),
        Err(VaultRuntimeError::NoPendingReview)
    ));
}

mod covenant_guards;
mod native_ffi;
mod native_ffi_outputs;
mod session_guards;
mod wallet_inventory;

#[test]
fn wallet_inventory_receive_and_switching_follow_active_wallet() {
    let mut runtime = VaultRuntime::new();
    let first = runtime.restore_wallet(MNEMONIC, "").expect("first wallet");
    let first_address = runtime
        .derive_receive_address("mainnet", false, 0)
        .expect("first receive address");

    let second = runtime
        .add_restored_wallet(MNEMONIC_24, "")
        .expect("second wallet");
    assert_ne!(first, second);
    let summaries = runtime.wallet_summaries().expect("wallet inventory");
    assert_eq!(summaries.len(), 2);
    assert!(!summaries[0].active);
    assert!(summaries[1].active);

    let second_address = runtime
        .derive_receive_address("mainnet", false, 0)
        .expect("second receive address");
    assert_ne!(first_address, second_address);
    assert_eq!(runtime.switch_wallet(0).unwrap(), Some(first));
    assert_eq!(
        runtime.derive_receive_address("mainnet", false, 0).unwrap(),
        first_address
    );
    assert!(matches!(
        runtime.switch_wallet(99),
        Err(VaultRuntimeError::InvalidWalletIndex)
    ));
}

#[test]
fn advanced_runtime_tools_and_portable_recovery_are_shared_and_locked() {
    let mut runtime = VaultRuntime::new();
    assert!(matches!(
        runtime.derive_receive_address("mainnet", false, 0),
        Err(VaultRuntimeError::Locked)
    ));
    runtime.restore_wallet(MNEMONIC, "").unwrap();

    assert!(runtime
        .derive_receive_address("mainnet", false, 0)
        .unwrap()
        .starts_with("kaspa:"));
    assert!(matches!(
        runtime.derive_receive_address("invalid", false, 0),
        Err(VaultRuntimeError::InvalidNetwork)
    ));
    assert_eq!(
        runtime
            .derive_bip85_phrase(12, 0)
            .unwrap()
            .split_whitespace()
            .count(),
        12
    );

    let signed = runtime.sign_message(b"runtime message").unwrap();
    assert_ne!(signed.signature, [0u8; 64]);
    let committed = runtime.commit_secret(b"runtime secret").unwrap();
    assert_eq!(
        runtime
            .decrypt_secret(&committed.payload)
            .unwrap()
            .as_slice(),
        b"runtime secret"
    );

    let backup = runtime.portable_backup("backup password").unwrap();
    let expected = runtime.export_public_account().unwrap();
    runtime.lock_wallet();
    assert!(matches!(
        runtime.portable_backup("backup password"),
        Err(VaultRuntimeError::Locked)
    ));
    let restored = runtime
        .add_portable_backup(&backup, "backup password")
        .expect("portable wallet restored");
    assert_eq!(restored.kind, WalletKind::Mnemonic);
    assert_eq!(restored.kpub.as_deref(), Some(expected.as_str()));
}

#[test]
fn wallet_names_use_software_limit_and_stay_aligned_with_inventory() {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    runtime.set_active_wallet_name("Primary").unwrap();
    runtime.add_restored_wallet(MNEMONIC_24, "").unwrap();
    runtime.set_active_wallet_name("Savings").unwrap();

    let summaries = runtime.wallet_summaries().unwrap();
    assert_eq!(summaries[0].name, "Primary");
    assert_eq!(summaries[1].name, "Savings");
    assert!(summaries[1].active);

    let max_name = "x".repeat(64);
    assert!(runtime.set_active_wallet_name(&max_name).is_ok());
    let summaries = runtime.wallet_summaries().unwrap();
    assert_eq!(summaries[1].name, max_name);
    assert!(summaries[1].active);

    assert!(matches!(
        runtime.set_active_wallet_name(""),
        Err(VaultRuntimeError::InvalidWalletName)
    ));
    assert!(matches!(
        runtime.set_active_wallet_name(&"x".repeat(65)),
        Err(VaultRuntimeError::InvalidWalletName)
    ));
    assert!(matches!(
        runtime.set_active_wallet_name("bad\nname"),
        Err(VaultRuntimeError::InvalidWalletName)
    ));

    runtime.delete_wallet(0).unwrap();
    let summaries = runtime.wallet_summaries().unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].name, max_name);
    assert!(summaries[0].active);
}

#[test]
fn session_signing_policy_maps_every_decision_to_the_expected_runtime_error() {
    use shared_signer::advanced_policy::{SigningPolicy, SigningWindow, UtcDateTime};

    let base = UtcDateTime::new(2000, 1, 3, 0, 0, 0)
        .to_unix_seconds()
        .expect("known supported UTC date");
    let mut runtime = VaultRuntime::new();

    runtime.signing_policy = SigningPolicy::disabled();
    assert!(runtime.check_session_signing_policy(base).is_ok());

    runtime.signing_policy = SigningPolicy {
        not_before_unix: base + 60,
        weekly_enabled: false,
        weekly_count: 0,
        windows: [SigningWindow::EMPTY; 4],
        rtc_floor_unix: 0,
    };
    assert!(matches!(
        runtime.check_session_signing_policy(base),
        Err(VaultRuntimeError::SigningBlockedNotBefore)
    ));

    runtime.signing_policy = SigningPolicy {
        not_before_unix: 0,
        weekly_enabled: false,
        weekly_count: 0,
        windows: [SigningWindow::EMPTY; 4],
        rtc_floor_unix: base + 60,
    };
    assert!(matches!(
        runtime.check_session_signing_policy(base),
        Err(VaultRuntimeError::SigningBlockedClockRollback)
    ));

    let mut windows = [SigningWindow::EMPTY; 4];
    windows[0] = SigningWindow {
        weekday: 0,
        start_minute: 60,
        end_minute: 120,
    };
    runtime.signing_policy = SigningPolicy {
        not_before_unix: 0,
        weekly_enabled: true,
        weekly_count: 1,
        windows,
        rtc_floor_unix: 0,
    };
    assert!(matches!(
        runtime.check_session_signing_policy(base),
        Err(VaultRuntimeError::SigningBlockedWeeklyWindow)
    ));
    assert!(matches!(
        runtime.check_session_signing_policy(u64::MAX),
        Err(VaultRuntimeError::SigningBlockedClockInvalid)
    ));

    runtime.signing_policy = SigningPolicy {
        not_before_unix: 0,
        weekly_enabled: true,
        weekly_count: 0,
        windows: [SigningWindow::EMPTY; 4],
        rtc_floor_unix: 0,
    };
    assert!(matches!(
        runtime.check_session_signing_policy(base),
        Err(VaultRuntimeError::InvalidSigningPolicy)
    ));
}

#[test]
fn approve_enforces_session_policy_atomically_without_consuming_blocked_request() {
    use shared_signer::advanced_policy::{SigningPolicy, SigningWindow, UtcDateTime};

    let base = UtcDateTime::new(2000, 1, 3, 0, 0, 0)
        .to_unix_seconds()
        .expect("known supported UTC date");
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    runtime.begin_scan().unwrap();
    for frame in owned_qr_frames() {
        let _ = runtime
            .accept_qr_frame(&frame.payload)
            .expect("QR frame accepted");
    }
    runtime.signing_policy = SigningPolicy {
        not_before_unix: base + 60,
        weekly_enabled: false,
        weekly_count: 0,
        windows: [SigningWindow::EMPTY; 4],
        rtc_floor_unix: 0,
    };

    assert!(matches!(
        runtime.approve(base),
        Err(VaultRuntimeError::SigningBlockedNotBefore)
    ));
    assert!(
        runtime.review_transaction().is_ok(),
        "blocked approval must retain the reviewed request"
    );
    assert!(!runtime
        .approve(base + 60)
        .expect("allowed approval signs")
        .is_empty());
}

fn encode_covenant_request(
    request: &shared_signer::covenant_sign::CovenantSignRequest<'_>,
) -> Vec<u8> {
    use shared_signer::covenant_sign as wire;
    let mut out =
        vec![0u8; wire::REQUEST_HEADER_LEN + request.script.len() + request.context.len()];
    let len = wire::encode_request(request, &mut out).expect("covenant request encodes");
    out.truncate(len);
    out
}

#[test]
fn covenant_runtime_covers_key_binding_opaque_nonce_and_reveal() {
    use shared_signer::covenant_sign::{
        self as wire, BindingHint, CovenantSignRequest, CovenantSignReveal, KnownScheme,
        RequestKind, ResponseKind,
    };

    let mut runtime = VaultRuntime::new();
    runtime
        .restore_wallet(MNEMONIC, "")
        .expect("wallet restores");

    let key_info_request = CovenantSignRequest {
        kind: RequestKind::KeyInfo,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: [0; 32],
        binding_token: [0; 32],
        commitment: [0; 32],
        script: &[],
        context: &[],
    };
    let review = runtime
        .covenant_prepare(&encode_covenant_request(&key_info_request))
        .expect("key-info prepares");
    assert_eq!(review.mode, super::CovenantMode::KeyInfo);
    let key_info = wire::parse_response(&runtime.covenant_confirm().expect("key-info confirms"))
        .expect("key-info response parses");
    assert_eq!(key_info.kind, ResponseKind::KeyInfo);

    let script = b"third-party covenant script";
    let bind_request = CovenantSignRequest {
        kind: RequestKind::Bind,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: key_info.key_id,
        binding_token: [0; 32],
        commitment: [0; 32],
        script,
        context: &[],
    };
    let review = runtime
        .covenant_prepare(&encode_covenant_request(&bind_request))
        .expect("binding prepares");
    assert_eq!(review.mode, super::CovenantMode::BindOpaque);
    let binding = wire::parse_response(&runtime.covenant_confirm().expect("binding confirms"))
        .expect("binding response parses");
    assert_eq!(binding.kind, ResponseKind::Binding);

    let host_secret = [0x51u8; 32];
    let host_commitment = shared_signer::anti_klepto::host_commitment(&host_secret);
    let commitment = [0x62u8; 32];
    let session_id = [0x73u8; wire::SESSION_ID_LEN];
    let opaque_request = CovenantSignRequest {
        kind: RequestKind::Opaque,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id,
        host_commitment,
        key_id: key_info.key_id,
        binding_token: binding.binding_token,
        commitment,
        script,
        context: &[],
    };
    let review = runtime
        .covenant_prepare(&encode_covenant_request(&opaque_request))
        .expect("opaque signing prepares");
    assert_eq!(review.mode, super::CovenantMode::Opaque);
    let nonce = wire::parse_response(&runtime.covenant_confirm().expect("signing confirms"))
        .expect("nonce response parses");
    assert_eq!(nonce.kind, ResponseKind::NonceCommitment);
    assert_ne!(nonce.nonce_point, [0; 33]);

    let reveal = CovenantSignReveal {
        session_id,
        key_id: key_info.key_id,
        commitment,
        host_secret,
    };
    let mut reveal_wire = [0u8; wire::REVEAL_LEN];
    wire::encode_reveal(&reveal, &mut reveal_wire).expect("reveal encodes");
    let signature = wire::parse_response(
        &runtime
            .covenant_finalize_reveal(&reveal_wire)
            .expect("reveal finalizes"),
    )
    .expect("signature response parses");
    assert_eq!(signature.kind, ResponseKind::Signature);
    assert_ne!(signature.signature, [0; 64]);
}

#[test]
fn covenant_runtime_known_shape_covers_invalid_context_commitment_script_and_success() {
    use shared_signer::covenant_sign::{
        self as wire, BindingHint, CovenantSignRequest, KnownScheme, RequestKind, ResponseKind,
    };

    let mut runtime = VaultRuntime::new();
    runtime
        .restore_wallet(MNEMONIC, "")
        .expect("wallet restores");

    let key_info = CovenantSignRequest {
        kind: RequestKind::KeyInfo,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: [0; 32],
        binding_token: [0; 32],
        commitment: [0; 32],
        script: &[],
        context: &[],
    };
    runtime
        .covenant_prepare(&encode_covenant_request(&key_info))
        .expect("key-info prepares");
    let key_info = wire::parse_response(&runtime.covenant_confirm().expect("key-info confirms"))
        .expect("key-info response parses");
    assert_eq!(key_info.kind, ResponseKind::KeyInfo);

    let context = b"runtime known covenant";
    let commitment = wire::recompute_known_commitment(KnownScheme::Sha256Preimage, context)
        .expect("known commitment recomputes");
    let mut script = [0u8; 67];
    script[0] = 0x20;
    script[1..33].copy_from_slice(&commitment);
    script[33] = 0x20;
    script[34..66].copy_from_slice(&key_info.pubkey_x);
    script[66] = 0xd7;

    let mut invalid_script = script;
    invalid_script[66] = 0xac;
    let bad_script_bind = CovenantSignRequest {
        kind: RequestKind::Bind,
        scheme: KnownScheme::Sha256Preimage,
        binding: BindingHint::FixedCheckSigFromStack,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: key_info.key_id,
        binding_token: [0; 32],
        commitment,
        script: &invalid_script,
        context,
    };
    assert!(matches!(
        runtime.covenant_prepare(&encode_covenant_request(&bad_script_bind)),
        Err(VaultRuntimeError::InvalidCovenantContext)
    ));

    let invalid_context = [0xffu8];
    let bad_context_bind = CovenantSignRequest {
        kind: RequestKind::Bind,
        scheme: KnownScheme::Sha256Preimage,
        binding: BindingHint::FixedCheckSigFromStack,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: key_info.key_id,
        binding_token: [0; 32],
        commitment,
        script: &script,
        context: &invalid_context,
    };
    assert!(matches!(
        runtime.covenant_prepare(&encode_covenant_request(&bad_context_bind)),
        Err(VaultRuntimeError::InvalidCovenantContext)
    ));

    let bad_commitment_bind = CovenantSignRequest {
        kind: RequestKind::Bind,
        scheme: KnownScheme::Sha256Preimage,
        binding: BindingHint::FixedCheckSigFromStack,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: key_info.key_id,
        binding_token: [0; 32],
        commitment: [0x44; 32],
        script: &script,
        context,
    };
    assert!(matches!(
        runtime.covenant_prepare(&encode_covenant_request(&bad_commitment_bind)),
        Err(VaultRuntimeError::InvalidCovenantContext)
    ));

    let bind = CovenantSignRequest {
        kind: RequestKind::Bind,
        scheme: KnownScheme::Sha256Preimage,
        binding: BindingHint::FixedCheckSigFromStack,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: key_info.key_id,
        binding_token: [0; 32],
        commitment,
        script: &script,
        context,
    };
    let review = runtime
        .covenant_prepare(&encode_covenant_request(&bind))
        .expect("known binding prepares");
    assert_eq!(review.mode, super::CovenantMode::BindKnown);
    let binding = wire::parse_response(&runtime.covenant_confirm().expect("binding confirms"))
        .expect("binding response parses");
    assert_eq!(binding.kind, ResponseKind::Binding);

    let host_secret = [0x61u8; 32];
    let known = CovenantSignRequest {
        kind: RequestKind::Known,
        scheme: KnownScheme::Sha256Preimage,
        binding: BindingHint::FixedCheckSigFromStack,
        session_id: [0x62; wire::SESSION_ID_LEN],
        host_commitment: shared_signer::anti_klepto::host_commitment(&host_secret),
        key_id: key_info.key_id,
        binding_token: binding.binding_token,
        commitment,
        script: &script,
        context,
    };
    let review = runtime
        .covenant_prepare(&encode_covenant_request(&known))
        .expect("known request prepares");
    assert_eq!(review.mode, super::CovenantMode::Known);
    assert_eq!(review.commitment, commitment);
}

#[test]
fn runtime_anti_klepto_round_trip_covers_reveal_transition_and_finalization() {
    use shared_signer::anti_klepto;

    let mut runtime = VaultRuntime::new();
    runtime
        .restore_wallet(MNEMONIC, "")
        .expect("wallet restores");
    runtime.begin_scan().expect("scan begins");

    let transaction = owned_compact_kspt(0);
    let host_secret = [0x35u8; anti_klepto::HASH_LEN];
    let mut request = vec![0u8; transaction.len() + 256];
    let request_len = anti_klepto::encode_request(&host_secret, &transaction, &mut request)
        .expect("anti-klepto request encodes");
    request.truncate(request_len);
    let parsed_request = anti_klepto::parse_request(&request).expect("request parses");
    let frames = encode_qr_frames(&request).expect("request QR frames encode");
    for frame in &frames {
        let _ = runtime
            .accept_qr_frame(&frame.payload)
            .expect("anti-klepto frame accepted");
    }

    let commitment_frames = runtime.approve(0).expect("commitment response produced");
    assert!(!commitment_frames.is_empty());
    assert!(runtime.anti_klepto_awaiting_reveal());
    runtime
        .begin_anti_klepto_reveal()
        .expect("reveal phase begins");
    assert!(matches!(
        runtime.signed_response_wire(),
        Err(VaultRuntimeError::NoSignedResponse)
    ));

    let mut reveal = [0u8; 96];
    let reveal_len =
        anti_klepto::encode_reveal(&parsed_request.session_id, &host_secret, &mut reveal)
            .expect("anti-klepto reveal encodes");
    let signed_frames = runtime
        .finalize_anti_klepto_reveal(&reveal[..reveal_len], 0)
        .expect("anti-klepto reveal finalizes");
    assert!(!signed_frames.is_empty());
    let signed = anti_klepto::parse_signed(runtime.signed_response_wire().unwrap())
        .expect("signed anti-klepto response parses");
    assert_eq!(signed.session_id, parsed_request.session_id);
    assert!(!signed.transaction.is_empty());
}

fn runtime_swap_push_data(script: &mut Vec<u8>, data: &[u8]) {
    script.push(u8::try_from(data.len()).expect("private-swap fixture direct push"));
    script.extend_from_slice(data);
}

fn runtime_swap_push_int(script: &mut Vec<u8>, value: u64) {
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
    runtime_swap_push_data(script, &bytes);
}

fn runtime_swap_script(claimer: [u8; 32], destination: &[u8]) -> Vec<u8> {
    let mut script = Vec::new();
    runtime_swap_push_data(&mut script, &[0x42; 16]);
    script.extend_from_slice(&[0x75, 0x63]);
    runtime_swap_push_data(&mut script, &claimer);
    script.extend_from_slice(&[0xad, 0xb3, 0x51, 0x9d, 0xb4, 0x51, 0x9d, 0x00, 0xc3]);
    runtime_swap_push_data(&mut script, destination);
    script.extend_from_slice(&[
        0x88, 0x00, 0xbe, 0x76, 0x00, 0xc2, 0xa2, 0x69, 0x00, 0xc2, 0x94,
    ]);
    runtime_swap_push_int(
        &mut script,
        offline_signer::transaction::private_swap::PRIVATE_SWAP_MAX_FEE_SOMPI,
    );
    script.extend_from_slice(&[0xa1, 0x69, 0x51, 0x67]);
    runtime_swap_push_data(&mut script, &[0x21; 32]);
    script.push(0xad);
    runtime_swap_push_int(&mut script, 50_000);
    script.extend_from_slice(&[0xb0, 0x51, 0x68]);
    script
}

fn runtime_private_swap_claim(claimer: [u8; 32]) -> Vec<u8> {
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
    let redeem = runtime_swap_script(claimer, &destination);
    transaction
        .store_redeem(0, &redeem)
        .expect("redeem script stores");
    serialize_compact_kspt_vec(&transaction).expect("private-swap KSPT serializes")
}

fn runtime_encode_private_swap_request(
    request: &shared_signer::covenant_sign::private_swap::PrivateSwapRequest<'_>,
) -> Vec<u8> {
    use shared_signer::covenant_sign::private_swap as wire;
    let mut out = vec![0u8; wire::REQUEST_HEADER_LEN + request.payload.len()];
    let len = wire::encode_request(request, &mut out).expect("private-swap request encodes");
    out.truncate(len);
    out
}

#[test]
fn runtime_private_swap_adapter_covers_review_response_confirm_and_reveal() {
    use shared_signer::covenant_sign::private_swap::{
        self as wire, PrivateSwapRequest, PrivateSwapReveal, RequestKind, ResponseKind,
    };

    let mut runtime = VaultRuntime::new();
    runtime
        .restore_wallet(MNEMONIC, "")
        .expect("wallet restores");

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
    let key_response = match runtime
        .private_swap_prepare(&runtime_encode_private_swap_request(&key_request))
        .expect("key-info prepares")
    {
        super::PrivateSwapPrepareResult::Response(response) => {
            wire::parse_response(&response).expect("key-info response parses")
        }
        super::PrivateSwapPrepareResult::Review(_) => panic!("key-info must be immediate"),
    };
    assert_eq!(key_response.kind, ResponseKind::KeyInfo);

    let claim_wire = runtime_private_swap_claim(key_response.claim_pubkey);
    let mut claim = Transaction::try_new().expect("claim allocation");
    offline_signer::transaction::kspt::parse_compact_kspt(&claim_wire, &mut claim)
        .expect("claim parses");
    let redeem = claim.redeem_bytes(0).to_vec();
    let bind_request = PrivateSwapRequest {
        kind: RequestKind::Bind,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: key_response.key_id,
        binding_token: [0; 32],
        adaptor_point: key_response.adaptor_point,
        presignature: [0; 64],
        presignature_negated: false,
        payload: &redeem,
    };
    assert!(matches!(
        runtime
            .private_swap_prepare(&runtime_encode_private_swap_request(&bind_request))
            .expect("binding prepares"),
        super::PrivateSwapPrepareResult::Review(_)
    ));
    let binding = wire::parse_response(&runtime.private_swap_confirm().expect("binding confirms"))
        .expect("binding response parses");

    let host_secret = [0x64u8; 32];
    let host_commitment = shared_signer::anti_klepto::host_commitment(&host_secret);
    let session_id = wire::session_id(
        &host_commitment,
        &claim_wire,
        &key_response.key_id,
        &key_response.adaptor_point,
    );
    let presign_request = PrivateSwapRequest {
        kind: RequestKind::PreSign,
        session_id,
        host_commitment,
        key_id: key_response.key_id,
        binding_token: binding.binding_token,
        adaptor_point: key_response.adaptor_point,
        presignature: [0; 64],
        presignature_negated: false,
        payload: &claim_wire,
    };
    assert!(matches!(
        runtime
            .private_swap_prepare(&runtime_encode_private_swap_request(&presign_request))
            .expect("presign prepares"),
        super::PrivateSwapPrepareResult::Review(_)
    ));
    let nonce = wire::parse_response(&runtime.private_swap_confirm().expect("presign confirms"))
        .expect("nonce response parses");
    assert!(runtime.private_swap_awaiting_reveal());

    let reveal = PrivateSwapReveal {
        session_id,
        key_id: key_response.key_id,
        sighash: nonce.commitment,
        host_secret,
    };
    let mut reveal_wire = [0u8; wire::REVEAL_LEN];
    wire::encode_reveal(&reveal, &mut reveal_wire).expect("reveal encodes");
    let presignature = wire::parse_response(
        &runtime
            .private_swap_reveal(&reveal_wire)
            .expect("reveal finalizes"),
    )
    .expect("presignature response parses");
    assert!(!runtime.private_swap_awaiting_reveal());
    assert_eq!(presignature.kind, ResponseKind::PreSignature);

    let complete_request = PrivateSwapRequest {
        kind: RequestKind::Complete,
        session_id: [0; wire::SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: key_response.key_id,
        binding_token: binding.binding_token,
        adaptor_point: key_response.adaptor_point,
        presignature: presignature.signature,
        presignature_negated: presignature.negated,
        payload: &claim_wire,
    };
    assert!(matches!(
        runtime
            .private_swap_prepare(&runtime_encode_private_swap_request(&complete_request))
            .expect("completion prepares"),
        super::PrivateSwapPrepareResult::Review(_)
    ));
    let completed =
        wire::parse_response(&runtime.private_swap_confirm().expect("completion confirms"))
            .expect("completed response parses");
    assert_eq!(completed.kind, ResponseKind::Completed);
}
