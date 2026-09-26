use crate::native_ffi::workflows::{hex_decode, parse_input, wallet_kind_label};
use crate::native_ffi::{
    kaskold_vault_accept_frame, kaskold_vault_approve, kaskold_vault_backup_seedqr,
    kaskold_vault_backup_words, kaskold_vault_backup_xprv, kaskold_vault_begin_scan,
    kaskold_vault_create, kaskold_vault_destroy, kaskold_vault_export_public_account,
    kaskold_vault_export_receive_key, kaskold_vault_last_bytes_copy, kaskold_vault_last_error_copy,
    kaskold_vault_last_text_copy, kaskold_vault_lock, kaskold_vault_new, kaskold_vault_reject,
    kaskold_vault_response_count, kaskold_vault_response_frame_copy, kaskold_vault_restore,
    kaskold_vault_review, kaskold_vault_seal, kaskold_vault_unlock_sealed,
    kaskold_vault_workflow_bytes, kaskold_vault_workflow_text, NativeVault,
};
use crate::WalletKind;
use zeroize::Zeroize;

const OK: i32 = 0;
const ERROR: i32 = -1;
const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn copy_result(
    handle: *mut NativeVault,
    copier: extern "C" fn(*mut NativeVault, *mut u8, usize) -> isize,
) -> Vec<u8> {
    let needed = copier(handle, core::ptr::null_mut(), 0);
    assert!(
        needed < 0,
        "non-empty FFI result must report required capacity"
    );
    let mut output = vec![0u8; (-needed) as usize];
    let written = copier(handle, output.as_mut_ptr(), output.len());
    assert_eq!(written, output.len() as isize);
    output
}

#[test]
fn native_ffi_round_trips_sealed_wallet_without_secret_accessor() {
    let handle = kaskold_vault_new();
    assert!(!handle.is_null());

    let phrase = MNEMONIC.as_bytes();
    assert_eq!(
        kaskold_vault_restore(handle, phrase.as_ptr(), phrase.len(), core::ptr::null(), 0),
        OK
    );
    assert_eq!(kaskold_vault_export_public_account(handle), OK);
    let public = String::from_utf8(copy_result(handle, kaskold_vault_last_text_copy)).unwrap();
    assert!(public.starts_with("kpub"));

    let key = [0x42u8; hot_wallet::PLATFORM_WRAPPING_KEY_LEN];
    assert_eq!(kaskold_vault_seal(handle, key.as_ptr(), key.len()), OK);
    let mut sealed = copy_result(handle, kaskold_vault_last_bytes_copy);
    assert!(sealed.starts_with(b"KVI1"));
    assert!(sealed.len() > hot_wallet::SEALED_WALLET_LEN);

    kaskold_vault_lock(handle);
    assert_eq!(
        kaskold_vault_unlock_sealed(
            handle,
            sealed.as_ptr(),
            sealed.len(),
            key.as_ptr(),
            key.len(),
        ),
        OK
    );
    let restored = String::from_utf8(copy_result(handle, kaskold_vault_last_text_copy)).unwrap();
    assert!(restored.contains("kpub"));

    sealed.zeroize();
    kaskold_vault_destroy(handle);
}

#[test]
fn native_ffi_covers_create_scan_review_approve_and_response_copy() {
    let handle = kaskold_vault_new();
    assert!(!handle.is_null());

    assert_eq!(kaskold_vault_create(handle, 12), OK);
    let created = String::from_utf8(copy_result(handle, kaskold_vault_last_text_copy)).unwrap();
    let created_json: serde_json::Value = serde_json::from_str(&created).expect("creation JSON");
    assert!(created_json["recoveryPhrase"].as_str().is_some());
    assert!(created_json.get("kpub").is_none());

    assert_eq!(kaskold_vault_export_public_account(handle), OK);
    let created_kpub =
        String::from_utf8(copy_result(handle, kaskold_vault_last_text_copy)).unwrap();
    assert!(created_kpub.starts_with("kpub"));

    let phrase = MNEMONIC.as_bytes();
    assert_eq!(
        kaskold_vault_restore(handle, phrase.as_ptr(), phrase.len(), core::ptr::null(), 0),
        OK
    );
    assert_eq!(kaskold_vault_begin_scan(handle), OK);

    let frames = super::owned_qr_frames();
    for frame in &frames[..frames.len() - 1] {
        assert_eq!(
            kaskold_vault_accept_frame(handle, frame.payload.as_ptr(), frame.payload.len()),
            1
        );
    }
    let last = frames.last().unwrap();
    assert_eq!(
        kaskold_vault_accept_frame(handle, last.payload.as_ptr(), last.payload.len()),
        2
    );

    assert_eq!(kaskold_vault_review(handle), OK);
    let review = String::from_utf8(copy_result(handle, kaskold_vault_last_text_copy)).unwrap();
    assert!(review.contains("\"state\":\"review\""));
    assert!(review.contains("\"fee\":\"1000\""));

    assert_eq!(kaskold_vault_response_count(handle), 0);
    assert_eq!(kaskold_vault_approve(handle, 0), OK);
    let response_count = kaskold_vault_response_count(handle);
    assert!(response_count > 0);

    let needed = kaskold_vault_response_frame_copy(handle, 0, core::ptr::null_mut(), 0);
    assert!(needed < 0);
    let mut frame = vec![0u8; (-needed) as usize];
    assert_eq!(
        kaskold_vault_response_frame_copy(handle, 0, frame.as_mut_ptr(), frame.len()),
        frame.len() as isize
    );
    assert!(!frame.is_empty());
    assert_eq!(
        kaskold_vault_response_frame_copy(handle, response_count, frame.as_mut_ptr(), frame.len(),),
        -1
    );

    kaskold_vault_reject(handle);
    assert_eq!(kaskold_vault_response_count(handle), 0);
    kaskold_vault_destroy(handle);
}

#[test]
fn native_ffi_covers_explicit_backup_exports_and_fail_closed_boundaries() {
    let handle = kaskold_vault_new();
    assert!(!handle.is_null());

    assert_eq!(kaskold_vault_backup_words(handle), ERROR);
    assert_eq!(kaskold_vault_backup_seedqr(handle, 0), ERROR);
    assert_eq!(kaskold_vault_backup_seedqr(handle, 1), ERROR);
    assert_eq!(kaskold_vault_backup_xprv(handle), ERROR);
    assert_eq!(kaskold_vault_export_receive_key(handle, 0), ERROR);

    let phrase = MNEMONIC.as_bytes();
    assert_eq!(
        kaskold_vault_restore(handle, phrase.as_ptr(), phrase.len(), core::ptr::null(), 0),
        OK
    );

    assert_eq!(kaskold_vault_backup_words(handle), OK);
    assert_eq!(copy_result(handle, kaskold_vault_last_text_copy), phrase);

    assert_eq!(kaskold_vault_backup_seedqr(handle, 0), OK);
    let seedqr = copy_result(handle, kaskold_vault_last_bytes_copy);
    assert_eq!(seedqr.len(), 48);
    assert!(seedqr.iter().all(u8::is_ascii_digit));

    assert_eq!(kaskold_vault_backup_seedqr(handle, 1), OK);
    assert_eq!(copy_result(handle, kaskold_vault_last_bytes_copy).len(), 16);

    assert_eq!(kaskold_vault_backup_xprv(handle), OK);
    let xprv = String::from_utf8(copy_result(handle, kaskold_vault_last_text_copy)).unwrap();
    assert!(xprv.starts_with("kprv"));

    assert_eq!(kaskold_vault_export_receive_key(handle, 0), OK);
    let key = String::from_utf8(copy_result(handle, kaskold_vault_last_text_copy)).unwrap();
    assert_eq!(key.len(), 64);
    assert!(key.bytes().all(|byte| byte.is_ascii_hexdigit()));

    assert_eq!(kaskold_vault_export_receive_key(handle, u32::MAX), ERROR);
    let error = String::from_utf8(copy_result(handle, kaskold_vault_last_error_copy)).unwrap();
    assert!(error.contains("65535"));

    kaskold_vault_destroy(handle);
}

#[test]
fn native_ffi_rejects_invalid_buffers_word_count_and_locked_operations() {
    assert_eq!(
        kaskold_vault_export_public_account(core::ptr::null_mut()),
        ERROR
    );
    assert_eq!(kaskold_vault_begin_scan(core::ptr::null_mut()), ERROR);
    assert_eq!(kaskold_vault_review(core::ptr::null_mut()), ERROR);
    assert_eq!(kaskold_vault_approve(core::ptr::null_mut(), 0), ERROR);

    let handle = kaskold_vault_new();
    assert_eq!(kaskold_vault_create(handle, 18), ERROR);
    assert!(!copy_result(handle, kaskold_vault_last_error_copy).is_empty());
    assert_eq!(
        kaskold_vault_restore(handle, core::ptr::null(), 1, core::ptr::null(), 0),
        ERROR
    );
    assert_eq!(
        kaskold_vault_seal(
            handle,
            core::ptr::null(),
            hot_wallet::PLATFORM_WRAPPING_KEY_LEN
        ),
        ERROR
    );

    let short_key = [0u8; hot_wallet::PLATFORM_WRAPPING_KEY_LEN - 1];
    assert_eq!(
        kaskold_vault_seal(handle, short_key.as_ptr(), short_key.len()),
        ERROR
    );
    assert_eq!(
        kaskold_vault_unlock_sealed(
            handle,
            core::ptr::null(),
            0,
            short_key.as_ptr(),
            short_key.len(),
        ),
        ERROR
    );

    assert_eq!(kaskold_vault_begin_scan(handle), ERROR);
    assert_eq!(kaskold_vault_review(handle), ERROR);
    assert_eq!(kaskold_vault_approve(handle, 0), ERROR);

    kaskold_vault_destroy(handle);
    kaskold_vault_destroy(core::ptr::null_mut());
}

fn run_text_workflow(handle: *mut NativeVault, operation: &str, input: &str) -> i32 {
    kaskold_vault_workflow_text(
        handle,
        operation.as_ptr(),
        operation.len(),
        input.as_ptr(),
        input.len(),
    )
}

fn run_byte_workflow(handle: *mut NativeVault, operation: &str, input: &str, data: &[u8]) -> i32 {
    kaskold_vault_workflow_bytes(
        handle,
        operation.as_ptr(),
        operation.len(),
        input.as_ptr(),
        input.len(),
        data.as_ptr(),
        data.len(),
    )
}

#[test]
fn native_workflow_parser_hex_and_wallet_labels_cover_all_branches() {
    assert!(parse_input("").unwrap().is_object());
    assert!(parse_input("{}").unwrap().is_object());
    assert!(parse_input("[]").is_err());
    assert!(parse_input("{").is_err());

    assert_eq!(wallet_kind_label(WalletKind::Mnemonic), "mnemonic");
    assert_eq!(wallet_kind_label(WalletKind::AccountXprv), "account-xprv");
    assert_eq!(
        wallet_kind_label(WalletKind::RawPrivateKey),
        "raw-private-key"
    );

    assert_eq!(hex_decode("00aF").unwrap(), vec![0x00, 0xaf]);
    assert!(hex_decode("0").is_err());
    assert!(hex_decode("zz").is_err());
}

#[test]
fn native_inventory_dispatch_rejects_non_inventory_operation_directly() {
    let mut vault = NativeVault::new();
    assert!(vault
        .dispatch_inventory_text("unsupported", &serde_json::json!({}))
        .is_err());
}

#[test]
fn native_workflow_dispatch_exercises_every_allowlisted_text_and_byte_route() {
    let handle = kaskold_vault_new();
    assert!(!handle.is_null());

    let phrase_json = serde_json::json!({"phrase": MNEMONIC, "passphrase": ""}).to_string();
    assert_eq!(run_text_workflow(handle, "add_restore", &phrase_json), OK);
    assert_eq!(run_text_workflow(handle, "wallets", "{}"), OK);
    assert_eq!(
        run_text_workflow(
            handle,
            "receive_address",
            r#"{"network":"mainnet","change":false,"index":0}"#
        ),
        OK
    );
    assert_eq!(
        run_text_workflow(handle, "set_wallet_name", r#"{"index":0,"name":"Primary"}"#),
        OK
    );
    assert_eq!(
        run_text_workflow(handle, "switch_wallet", r#"{"index":0}"#),
        OK
    );

    assert_eq!(run_text_workflow(handle, "multisig_kpub", "{}"), OK);
    let multisig_kpub_json: serde_json::Value =
        serde_json::from_slice(&copy_result(handle, kaskold_vault_last_text_copy))
            .expect("multisig kpub JSON");
    let own_kpub = multisig_kpub_json["kpub"]
        .as_str()
        .expect("kpub")
        .to_owned();
    assert_eq!(
        run_text_workflow(
            handle,
            "normalize_kpub",
            &serde_json::json!({"value": own_kpub}).to_string()
        ),
        OK
    );
    assert_eq!(
        run_text_workflow(handle, "validate_address", r#"{"value":"not-an-address"}"#),
        ERROR
    );

    assert_eq!(
        run_text_workflow(
            handle,
            "import_raw_key",
            &serde_json::json!({"privateKey": format!("{:064x}", 1u8)}).to_string()
        ),
        OK
    );
    assert_eq!(
        run_text_workflow(handle, "switch_wallet", r#"{"index":0}"#),
        OK
    );

    assert_eq!(
        run_text_workflow(handle, "bip85", r#"{"wordCount":12,"index":0}"#),
        OK
    );
    assert_eq!(
        run_text_workflow(handle, "sign_message", r#"{"message":"KasKold"}"#),
        OK
    );
    assert_eq!(
        run_text_workflow(handle, "commit_secret", r#"{"secret":"secret"}"#),
        OK
    );
    let committed: serde_json::Value =
        serde_json::from_slice(&copy_result(handle, kaskold_vault_last_text_copy))
            .expect("commit JSON");
    let payload = committed["payloadHex"].as_str().expect("payload hex");
    assert_eq!(
        run_text_workflow(
            handle,
            "decrypt_secret",
            &serde_json::json!({"payloadHex": payload}).to_string()
        ),
        OK
    );
    assert_eq!(
        run_text_workflow(
            handle,
            "set_signing_policy",
            r#"{"notBeforeUtc":"","weeklyWindows":""}"#
        ),
        OK
    );
    assert_eq!(run_text_workflow(handle, "clear_signing_policy", "{}"), OK);

    assert_eq!(run_text_workflow(handle, "add_create_12", "{}"), OK);
    assert_eq!(run_text_workflow(handle, "add_create_24", "{}"), OK);

    let static_descriptor = format!("multi(1,{},{})", "11".repeat(32), "22".repeat(32));
    let import_json = serde_json::json!({
        "descriptor": static_descriptor,
        "network": "mainnet",
        "chain": 0,
        "index": 0,
    })
    .to_string();
    assert_eq!(
        run_text_workflow(handle, "import_multisig", &import_json),
        OK
    );
    assert_eq!(
        run_text_workflow(
            handle,
            "create_multisig",
            r#"{"threshold":2,"cosigners":[],"network":"mainnet","chain":0,"index":0}"#
        ),
        ERROR
    );

    assert_eq!(
        run_text_workflow(handle, "import_xprv", r#"{"xprv":"invalid"}"#),
        ERROR
    );
    assert_eq!(
        run_text_workflow(handle, "delete_wallet", r#"{"index":999}"#),
        ERROR
    );
    assert_eq!(run_text_workflow(handle, "unsupported", "{}"), ERROR);
    assert_eq!(run_text_workflow(handle, "wallets", "[]"), ERROR);

    assert_eq!(
        run_byte_workflow(
            handle,
            "create_with_entropy",
            r#"{"wordCount":12,"dice":"7","passphrase":""}"#,
            b""
        ),
        ERROR
    );
    assert_eq!(
        run_byte_workflow(
            handle,
            "create_with_entropy",
            r#"{"wordCount":12,"dice":"123456","passphrase":""}"#,
            b"pointer transcript"
        ),
        OK
    );
    assert_eq!(
        run_byte_workflow(
            handle,
            "add_create_with_entropy",
            r#"{"wordCount":24,"dice":"654321","passphrase":""}"#,
            b"pointer transcript two"
        ),
        OK
    );

    assert_eq!(
        run_byte_workflow(handle, "transaction_file", "{}", b"bad"),
        ERROR
    );
    assert_eq!(
        run_byte_workflow(handle, "normalize_covenant_backup", "{}", b"bad"),
        ERROR
    );
    assert_eq!(
        run_byte_workflow(handle, "recover_material", "{}", b"bad"),
        ERROR
    );
    assert_eq!(
        run_byte_workflow(handle, "portable_backup", r#"{"password":"pw"}"#, b""),
        OK
    );
    let portable = copy_result(handle, kaskold_vault_last_bytes_copy);
    assert_eq!(
        run_byte_workflow(
            handle,
            "restore_portable",
            r#"{"password":"pw"}"#,
            &portable
        ),
        OK
    );
    assert_eq!(
        run_byte_workflow(handle, "portable_xprv_backup", r#"{"password":"pw"}"#, b""),
        OK
    );
    assert_eq!(
        run_byte_workflow(handle, "stego_backup", r#"{"password":"pw"}"#, b"bad-jpeg"),
        ERROR
    );
    assert_eq!(
        run_byte_workflow(handle, "restore_stego", r#"{"password":"pw"}"#, b"bad-jpeg"),
        ERROR
    );
    assert_eq!(run_byte_workflow(handle, "unsupported", "{}", b""), ERROR);

    kaskold_vault_destroy(handle);
}
