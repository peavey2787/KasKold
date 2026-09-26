//! Stable C ABI exported by the native Vault runtime.

use super::{copy_out, with_handle_mut, with_input_bytes, with_input_text, NativeVault, ERROR};

#[no_mangle]
pub extern "C" fn kaskold_vault_new() -> *mut NativeVault {
    Box::into_raw(Box::new(NativeVault::new()))
}

#[no_mangle]
pub extern "C" fn kaskold_vault_destroy(handle: *mut NativeVault) {
    if handle.is_null() {
        return;
    }
    // SAFETY: the handle is an allocation returned by `kaskold_vault_new` and
    // native wrappers guarantee exactly one destroy call.
    unsafe { drop(Box::from_raw(handle)) };
}

#[no_mangle]
pub extern "C" fn kaskold_vault_create(handle: *mut NativeVault, words: u32) -> i32 {
    with_handle_mut(handle, |vault| vault.create(words)).unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_restore(
    handle: *mut NativeVault,
    phrase: *const u8,
    phrase_len: usize,
    passphrase: *const u8,
    passphrase_len: usize,
) -> i32 {
    with_handle_mut(handle, |vault| {
        with_input_text(phrase, phrase_len, |phrase| {
            with_input_text(passphrase, passphrase_len, |passphrase| {
                vault.restore(phrase, passphrase)
            })
            .unwrap_or_else(|| vault.fail("invalid passphrase UTF-8"))
        })
        .unwrap_or_else(|| vault.fail("invalid recovery phrase UTF-8"))
    })
    .unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_lock(handle: *mut NativeVault) {
    let _ = with_handle_mut(handle, |vault| {
        vault.clear_results();
        vault.runtime.lock_wallet();
    });
}

#[no_mangle]
pub extern "C" fn kaskold_vault_export_public_account(handle: *mut NativeVault) -> i32 {
    with_handle_mut(handle, NativeVault::export_public_account).unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_seal(
    handle: *mut NativeVault,
    key: *const u8,
    key_len: usize,
) -> i32 {
    with_handle_mut(handle, |vault| {
        with_input_bytes(key, key_len, |key| vault.seal(key))
            .unwrap_or_else(|| vault.fail("invalid platform key buffer"))
    })
    .unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_unlock_sealed(
    handle: *mut NativeVault,
    sealed: *const u8,
    sealed_len: usize,
    key: *const u8,
    key_len: usize,
) -> i32 {
    with_handle_mut(handle, |vault| {
        with_input_bytes(sealed, sealed_len, |sealed| {
            with_input_bytes(key, key_len, |key| vault.unlock_sealed(sealed, key))
                .unwrap_or_else(|| vault.fail("invalid platform key buffer"))
        })
        .unwrap_or_else(|| vault.fail("invalid sealed wallet buffer"))
    })
    .unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_backup_words(handle: *mut NativeVault) -> i32 {
    with_handle_mut(handle, NativeVault::backup_words).unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_backup_seedqr(handle: *mut NativeVault, compact: u8) -> i32 {
    with_handle_mut(handle, |vault| vault.backup_seedqr(compact != 0)).unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_backup_xprv(handle: *mut NativeVault) -> i32 {
    with_handle_mut(handle, NativeVault::backup_xprv).unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_export_receive_key(
    handle: *mut NativeVault,
    address_index: u32,
) -> i32 {
    with_handle_mut(handle, |vault| vault.export_receive_key(address_index)).unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_begin_scan(handle: *mut NativeVault) -> i32 {
    with_handle_mut(handle, NativeVault::begin_scan).unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_accept_frame(
    handle: *mut NativeVault,
    frame: *const u8,
    frame_len: usize,
) -> i32 {
    with_handle_mut(handle, |vault| {
        with_input_bytes(frame, frame_len, |frame| vault.accept_frame(frame))
            .unwrap_or_else(|| vault.fail("invalid QR frame buffer"))
    })
    .unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_review(handle: *mut NativeVault) -> i32 {
    with_handle_mut(handle, NativeVault::review).unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_approve(handle: *mut NativeVault, now_unix: u64) -> i32 {
    with_handle_mut(handle, |vault| vault.approve(now_unix)).unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_reject(handle: *mut NativeVault) {
    let _ = with_handle_mut(handle, |vault| {
        vault.clear_results();
        vault.runtime.reject();
    });
}

#[no_mangle]
pub extern "C" fn kaskold_vault_response_count(handle: *mut NativeVault) -> usize {
    with_handle_mut(handle, |vault| {
        vault.runtime.signed_response_frames().map_or(0, <[_]>::len)
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_response_frame_copy(
    handle: *mut NativeVault,
    index: usize,
    output: *mut u8,
    capacity: usize,
) -> isize {
    with_handle_mut(handle, |vault| {
        let Ok(frames) = vault.runtime.signed_response_frames() else {
            return -1;
        };
        frames
            .get(index)
            .map_or(-1, |frame| copy_out(&frame.payload, output, capacity))
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_last_text_copy(
    handle: *mut NativeVault,
    output: *mut u8,
    capacity: usize,
) -> isize {
    with_handle_mut(handle, |vault| copy_out(&vault.last_text, output, capacity)).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_last_bytes_copy(
    handle: *mut NativeVault,
    output: *mut u8,
    capacity: usize,
) -> isize {
    with_handle_mut(handle, |vault| {
        copy_out(&vault.last_bytes, output, capacity)
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_last_error_copy(
    handle: *mut NativeVault,
    output: *mut u8,
    capacity: usize,
) -> isize {
    with_handle_mut(handle, |vault| {
        copy_out(&vault.last_error, output, capacity)
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_workflow_text(
    handle: *mut NativeVault,
    operation: *const u8,
    operation_len: usize,
    input_json: *const u8,
    input_json_len: usize,
) -> i32 {
    with_handle_mut(handle, |vault| {
        with_input_text(operation, operation_len, |operation| {
            with_input_text(input_json, input_json_len, |input_json| {
                vault.workflow_text(operation, input_json)
            })
            .unwrap_or_else(|| vault.fail("invalid native workflow JSON UTF-8"))
        })
        .unwrap_or_else(|| vault.fail("invalid native workflow operation UTF-8"))
    })
    .unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_workflow_bytes(
    handle: *mut NativeVault,
    operation: *const u8,
    operation_len: usize,
    input_json: *const u8,
    input_json_len: usize,
    data: *const u8,
    data_len: usize,
) -> i32 {
    with_handle_mut(handle, |vault| {
        with_input_text(operation, operation_len, |operation| {
            with_input_text(input_json, input_json_len, |input_json| {
                with_input_bytes(data, data_len, |data| {
                    vault.workflow_bytes(operation, input_json, data)
                })
                .unwrap_or_else(|| vault.fail("invalid native workflow byte buffer"))
            })
            .unwrap_or_else(|| vault.fail("invalid native workflow JSON UTF-8"))
        })
        .unwrap_or_else(|| vault.fail("invalid native workflow operation UTF-8"))
    })
    .unwrap_or(ERROR)
}

#[no_mangle]
pub extern "C" fn kaskold_vault_workflow_text_with_bytes(
    handle: *mut NativeVault,
    operation: *const u8,
    operation_len: usize,
    input_json: *const u8,
    input_json_len: usize,
    data: *const u8,
    data_len: usize,
) -> i32 {
    with_handle_mut(handle, |vault| {
        with_input_text(operation, operation_len, |operation| {
            with_input_text(input_json, input_json_len, |input_json| {
                with_input_bytes(data, data_len, |data| {
                    vault.workflow_bytes(operation, input_json, data)
                })
                .unwrap_or_else(|| vault.fail("invalid native workflow byte buffer"))
            })
            .unwrap_or_else(|| vault.fail("invalid native workflow JSON UTF-8"))
        })
        .unwrap_or_else(|| vault.fail("invalid native workflow operation UTF-8"))
    })
    .unwrap_or(ERROR)
}
