//! Native FFI workflows return the exact JSON each operation produced, and a
//! failing operation reports its own error rather than "unsupported".

use crate::native_ffi::{
    kaskold_vault_create, kaskold_vault_destroy, kaskold_vault_export_public_account,
    kaskold_vault_last_error_copy, kaskold_vault_last_text_copy, kaskold_vault_lock,
    kaskold_vault_new, kaskold_vault_restore, kaskold_vault_workflow_bytes,
    kaskold_vault_workflow_text, kaskold_vault_workflow_text_with_bytes, NativeVault,
};

const OK: i32 = 0;
const ERROR: i32 = -1;
const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn copy(
    handle: *mut NativeVault,
    copier: extern "C" fn(*mut NativeVault, *mut u8, usize) -> isize,
) -> String {
    let needed = copier(handle, core::ptr::null_mut(), 0);
    let mut output = vec![0u8; usize::try_from(-needed).expect("required capacity")];
    // One byte short of the required capacity still reports the full size.
    if !output.is_empty() {
        assert_eq!(
            copier(handle, output.as_mut_ptr(), output.len() - 1),
            needed
        );
    }
    assert_eq!(
        copier(handle, output.as_mut_ptr(), output.len()),
        isize::try_from(output.len()).expect("length")
    );
    String::from_utf8(output).expect("UTF-8 result")
}

fn text(handle: *mut NativeVault, operation: &str, input: &str) -> serde_json::Value {
    let status = kaskold_vault_workflow_text(
        handle,
        operation.as_ptr(),
        operation.len(),
        input.as_ptr(),
        input.len(),
    );
    assert_eq!(
        status,
        OK,
        "{operation}: {}",
        copy(handle, kaskold_vault_last_error_copy)
    );
    serde_json::from_str(&copy(handle, kaskold_vault_last_text_copy)).expect("result JSON")
}

fn text_error(handle: *mut NativeVault, operation: &str, input: &str) -> String {
    let status = kaskold_vault_workflow_text(
        handle,
        operation.as_ptr(),
        operation.len(),
        input.as_ptr(),
        input.len(),
    );
    assert_eq!(status, ERROR, "{operation} must fail");
    copy(handle, kaskold_vault_last_error_copy)
}

fn bytes_error(handle: *mut NativeVault, operation: &str, input: &str, data: &[u8]) -> String {
    let status = kaskold_vault_workflow_bytes(
        handle,
        operation.as_ptr(),
        operation.len(),
        input.as_ptr(),
        input.len(),
        data.as_ptr(),
        data.len(),
    );
    assert_eq!(status, ERROR, "{operation} must fail");
    copy(handle, kaskold_vault_last_error_copy)
}

fn assert_specific(error: &str) {
    assert!(!error.is_empty());
    assert!(!error.contains("unsupported"), "{error}");
}

fn restored() -> *mut NativeVault {
    let handle = kaskold_vault_new();
    let phrase = serde_json::json!({"phrase": MNEMONIC, "passphrase": ""}).to_string();
    let restored = text(handle, "add_restore", &phrase);
    assert!(restored["kpub"].as_str().expect("kpub").starts_with("kpub"));
    handle
}

#[test]
fn inventory_workflows_return_the_wallet_state_they_changed() {
    let handle = restored();

    let flow = text(handle, "creation_flow", "{}");
    assert!(flow.is_object() && !flow.as_object().expect("object").is_empty());

    let address = |change: bool, index: u32| {
        let input = serde_json::json!({"network": "mainnet", "change": change, "index": index});
        text(handle, "receive_address", &input.to_string())["address"]
            .as_str()
            .expect("address")
            .to_owned()
    };
    let receive0 = address(false, 0);
    assert!(receive0.starts_with("kaspa:"));
    assert_eq!(address(false, 0), receive0);
    assert_ne!(address(true, 0), receive0);
    assert_ne!(address(false, 2), receive0);
    assert_ne!(address(false, 2), address(false, 1));

    assert_eq!(
        text(handle, "set_wallet_name", r#"{"index":0,"name":"Primary"}"#),
        serde_json::json!({"index": 0})
    );
    let wallets = text(handle, "wallets", "{}");
    let first = &wallets["wallets"][0];
    assert_eq!(first["index"], 0);
    assert_eq!(first["name"], "Primary");
    assert_eq!(first["active"], true);
    assert_eq!(first["kind"], "mnemonic");
    assert!(first["kpub"].as_str().expect("kpub").starts_with("kpub"));
    assert!(!first["fingerprint"]
        .as_str()
        .expect("fingerprint")
        .is_empty());

    let created12 = text(handle, "add_create_12", "{}");
    let created24 = text(handle, "add_create_24", "{}");
    let words = |value: &serde_json::Value| {
        value["recoveryPhrase"]
            .as_str()
            .expect("phrase")
            .split_whitespace()
            .count()
    };
    assert_eq!(words(&created12), 12);
    assert_eq!(words(&created24), 24);

    let count = text(handle, "wallets", "{}")["wallets"]
        .as_array()
        .expect("wallets")
        .len();
    assert_eq!(count, 3);
    assert_eq!(
        text(handle, "delete_wallet", r#"{"index":2}"#),
        serde_json::json!({"deleted": 2})
    );
    assert_specific(&text_error(handle, "delete_wallet", r#"{"index":99}"#));
    kaskold_vault_destroy(handle);
}

#[test]
fn key_workflows_return_normalized_keys_and_multisig_material() {
    let handle = restored();
    let own = text(handle, "multisig_kpub", "{}")["kpub"]
        .as_str()
        .expect("kpub")
        .to_owned();
    assert_eq!(
        text(
            handle,
            "normalize_kpub",
            &serde_json::json!({"value": own}).to_string()
        ),
        serde_json::json!({"value": own})
    );

    let raw = text(
        handle,
        "import_raw_key",
        &serde_json::json!({"privateKey": format!("{:064x}", 1u8)}).to_string(),
    );
    assert_eq!(raw["kind"], "raw-private-key");
    assert_eq!(raw["index"], 1);

    let descriptor = format!("multi(1,{},{})", "11".repeat(32), "22".repeat(32));
    let imported = text(
        handle,
        "import_multisig",
        &serde_json::json!({"descriptor": descriptor, "network": "mainnet", "chain": 0, "index": 0})
            .to_string(),
    );
    assert_eq!(imported["threshold"], 1);
    assert_eq!(imported["participants"], 2);
    assert_eq!(imported["chain"], 0);
    assert_eq!(imported["index"], 0);
    assert!(imported["address"]
        .as_str()
        .expect("address")
        .starts_with("kaspa:"));
    assert!(imported["descriptor"]
        .as_str()
        .expect("descriptor")
        .starts_with("multi("));

    assert_specific(&text_error(handle, "import_xprv", r#"{"xprv":"invalid"}"#));
    assert_specific(&text_error(
        handle,
        "validate_address",
        r#"{"value":"bad"}"#,
    ));
    assert_specific(&text_error(
        handle,
        "create_multisig",
        r#"{"threshold":2,"cosigners":[],"network":"mainnet","chain":0,"index":0}"#,
    ));
    kaskold_vault_destroy(handle);
}

#[test]
fn tool_workflows_return_derived_signed_and_decrypted_values() {
    let handle = restored();
    let phrase = text(handle, "bip85", r#"{"wordCount":12,"index":0}"#);
    assert_eq!(
        phrase["phrase"]
            .as_str()
            .expect("phrase")
            .split_whitespace()
            .count(),
        12
    );

    let signed = text(handle, "sign_message", r#"{"message":"KasKold"}"#);
    assert_eq!(signed["digestHex"].as_str().expect("digest").len(), 64);
    assert_eq!(
        signed["signatureHex"].as_str().expect("signature").len(),
        128
    );

    let committed = text(handle, "commit_secret", r#"{"secret":"secret"}"#);
    let payload = committed["payloadHex"]
        .as_str()
        .expect("payload")
        .to_owned();
    assert_eq!(
        text(
            handle,
            "decrypt_secret",
            &serde_json::json!({"payloadHex": payload}).to_string()
        ),
        serde_json::json!({"secret": "secret"})
    );

    assert_eq!(
        text(
            handle,
            "set_signing_policy",
            r#"{"notBeforeUtc":"","weeklyWindows":""}"#
        ),
        serde_json::json!({})
    );
    assert_eq!(
        text(handle, "clear_signing_policy", "{}"),
        serde_json::json!({})
    );
    kaskold_vault_destroy(handle);
}

#[test]
fn byte_workflows_report_their_own_failures() {
    let handle = restored();
    for operation in [
        "recover_material",
        "transaction_file",
        "normalize_covenant_backup",
    ] {
        assert_specific(&bytes_error(handle, operation, "{}", b"bad"));
    }
    for operation in ["stego_backup", "restore_stego"] {
        assert_specific(&bytes_error(
            handle,
            operation,
            r#"{"password":"pw"}"#,
            b"bad-jpeg",
        ));
    }
    kaskold_vault_destroy(handle);
}

#[test]
fn text_with_bytes_entry_point_returns_the_byte_workflow_status() {
    let handle = restored();
    let call = |operation: &str, input: &str, data: &[u8]| {
        kaskold_vault_workflow_text_with_bytes(
            handle,
            operation.as_ptr(),
            operation.len(),
            input.as_ptr(),
            input.len(),
            data.as_ptr(),
            data.len(),
        )
    };
    assert_eq!(call("transaction_file", "{}", b"bad"), ERROR);
    assert_eq!(call("portable_backup", r#"{"password":"pw"}"#, b""), OK);
    kaskold_vault_destroy(handle);
}

#[test]
fn create_accepts_both_word_counts_and_lock_forgets_the_wallet() {
    let handle = kaskold_vault_new();
    for words in [12u32, 24] {
        assert_eq!(kaskold_vault_create(handle, words), OK);
        let created: serde_json::Value =
            serde_json::from_str(&copy(handle, kaskold_vault_last_text_copy)).expect("JSON");
        assert_eq!(
            created["recoveryPhrase"]
                .as_str()
                .expect("phrase")
                .split_whitespace()
                .count(),
            words as usize
        );
    }
    assert_eq!(kaskold_vault_create(handle, 18), ERROR);
    assert_eq!(
        copy(handle, kaskold_vault_last_error_copy),
        "wallet word count must be 12 or 24"
    );

    let phrase = MNEMONIC.as_bytes();
    assert_eq!(
        kaskold_vault_restore(handle, phrase.as_ptr(), phrase.len(), core::ptr::null(), 0),
        OK
    );
    assert_eq!(kaskold_vault_export_public_account(handle), OK);
    kaskold_vault_lock(handle);
    // Locking clears the last result and the unlocked wallet.
    assert_eq!(
        kaskold_vault_last_text_copy(handle, core::ptr::null_mut(), 0),
        0
    );
    assert_eq!(kaskold_vault_export_public_account(handle), ERROR);
    kaskold_vault_destroy(handle);
}

#[test]
fn copy_entry_points_fail_closed_without_a_handle_or_a_signing_session() {
    use crate::native_ffi::{kaskold_vault_last_bytes_copy, kaskold_vault_response_frame_copy};

    let null = core::ptr::null_mut();
    assert_eq!(kaskold_vault_last_text_copy(null, null.cast(), 0), -1);
    assert_eq!(kaskold_vault_last_bytes_copy(null, null.cast(), 0), -1);
    assert_eq!(kaskold_vault_last_error_copy(null, null.cast(), 0), -1);
    assert_eq!(
        kaskold_vault_response_frame_copy(null, 0, null.cast(), 0),
        -1
    );

    let handle = restored();
    assert_eq!(
        kaskold_vault_response_frame_copy(handle, 0, core::ptr::null_mut(), 0),
        -1
    );
    kaskold_vault_destroy(handle);
}
