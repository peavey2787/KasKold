//! Narrow C ABI used by the native Vault shells.
//! It exposes opaque runtime state, public/review data, authenticated ciphertext,
//! QR frame bytes, and deliberately named user-initiated backup exports. Secret
//! backup results live only in the wipe-on-reuse result buffers.

use core::{ptr, slice, str};

use zeroize::Zeroize;

use crate::{ScanResult, VaultRuntime};

const OK: i32 = 0;
const ERROR: i32 = -1;

pub(crate) struct NativeVault {
    runtime: VaultRuntime,
    last_text: Vec<u8>,
    last_bytes: Vec<u8>,
    last_error: Vec<u8>,
}

impl NativeVault {
    pub(crate) fn new() -> Self {
        Self {
            runtime: VaultRuntime::new(),
            last_text: Vec::new(),
            last_bytes: Vec::new(),
            last_error: Vec::new(),
        }
    }

    fn clear_results(&mut self) {
        self.last_text.zeroize();
        self.last_text.clear();
        self.last_bytes.zeroize();
        self.last_bytes.clear();
        self.last_error.zeroize();
        self.last_error.clear();
    }

    fn set_text(&mut self, value: String) {
        self.last_text.zeroize();
        self.last_text = value.into_bytes();
    }

    fn set_bytes(&mut self, mut value: Vec<u8>) {
        self.last_bytes.zeroize();
        self.last_bytes = core::mem::take(&mut value);
        value.zeroize();
    }

    fn fail(&mut self, error: impl core::fmt::Debug) -> i32 {
        self.last_error.zeroize();
        self.last_error = format!("{error:?}").into_bytes();
        ERROR
    }

    fn create(&mut self, words: u32) -> i32 {
        self.clear_results();
        let result = match words {
            12 => self.runtime.create_wallet_12(),
            24 => self.runtime.create_wallet_24(),
            _ => {
                self.last_error = b"wallet word count must be 12 or 24".to_vec();
                return ERROR;
            }
        };
        match result {
            Ok(created) => {
                let json = serde_json::json!({
                    "recoveryPhrase": created.recovery_phrase.as_str(),
                });
                self.set_text(json.to_string());
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn restore(&mut self, phrase: &str, passphrase: &str) -> i32 {
        self.clear_results();
        match self.runtime.restore_wallet(phrase, passphrase) {
            Ok(kpub) => {
                self.set_text(serde_json::json!({"kpub": kpub}).to_string());
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn export_public_account(&mut self) -> i32 {
        self.clear_results();
        match self.runtime.export_public_account() {
            Ok(kpub) => {
                self.set_text(kpub);
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn seal(&mut self, key: &[u8]) -> i32 {
        self.clear_results();
        let Ok(key): Result<&[u8; hot_wallet::PLATFORM_WRAPPING_KEY_LEN], _> = key.try_into()
        else {
            self.last_error = b"platform wrapping key must be 32 bytes".to_vec();
            return ERROR;
        };
        match self.runtime.seal_native_inventory(key) {
            Ok(sealed) => {
                self.set_bytes(sealed);
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn unlock_sealed(&mut self, sealed: &[u8], key: &[u8]) -> i32 {
        self.clear_results();
        let Ok(key): Result<&[u8; hot_wallet::PLATFORM_WRAPPING_KEY_LEN], _> = key.try_into()
        else {
            self.last_error = b"platform wrapping key must be 32 bytes".to_vec();
            return ERROR;
        };
        match self.runtime.unlock_native_inventory(sealed, key) {
            Ok(kpub) => {
                self.set_text(serde_json::json!({"kpub": kpub}).to_string());
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn backup_words(&mut self) -> i32 {
        self.clear_results();
        match self.runtime.backup_recovery_phrase() {
            Ok(value) => {
                self.set_text(value.to_string());
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn backup_seedqr(&mut self, compact: bool) -> i32 {
        self.clear_results();
        let result = if compact {
            self.runtime.backup_compact_seedqr()
        } else {
            self.runtime.backup_seedqr()
        };
        match result {
            Ok(value) => {
                self.set_bytes(value.to_vec());
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn backup_xprv(&mut self) -> i32 {
        self.clear_results();
        match self.runtime.backup_account_xprv() {
            Ok(value) => {
                self.set_text(value.to_string());
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn export_receive_key(&mut self, address_index: u32) -> i32 {
        self.clear_results();
        let Ok(address_index) = u16::try_from(address_index) else {
            self.last_error = b"address index must be between 0 and 65535".to_vec();
            return ERROR;
        };
        match self.runtime.backup_receive_private_key_hex(address_index) {
            Ok(value) => {
                self.set_text(value.to_string());
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn begin_scan(&mut self) -> i32 {
        self.clear_results();
        match self.runtime.begin_scan() {
            Ok(()) => OK,
            Err(error) => self.fail(error),
        }
    }

    fn accept_frame(&mut self, frame: &[u8]) -> i32 {
        self.clear_results();
        match self.runtime.accept_qr_frame(frame) {
            Ok(ScanResult::Progress(progress)) => {
                self.set_text(
                    serde_json::json!({
                        "state": "progress",
                        "received": progress.received,
                        "total": progress.total,
                    })
                    .to_string(),
                );
                1
            }
            Ok(ScanResult::Ready(review)) => {
                self.set_text(
                    serde_json::json!({
                        "state": "review",
                        "network": review.network,
                        "inputCount": review.input_count,
                        "outputCount": review.output_count,
                        "inputTotal": review.input_total.to_string(),
                        "outputTotal": review.output_total.to_string(),
                        "fee": review.fee.to_string(),
                        "inputs": review.inputs.iter().map(|input| serde_json::json!({
                            "index": input.index,
                            "outpoint": input.outpoint,
                            "amount": input.amount.to_string(),
                            "scriptType": input.script_type,
                            "address": input.address,
                        })).collect::<Vec<_>>(),
                        "outputs": review.outputs.iter().map(|output| serde_json::json!({
                            "index": output.index,
                            "amount": output.amount.to_string(),
                            "ownership": output.ownership,
                            "address": output.address,
                        })).collect::<Vec<_>>(),
                    })
                    .to_string(),
                );
                2
            }
            Err(error) => self.fail(error),
        }
    }

    fn review(&mut self) -> i32 {
        self.clear_results();
        match self.runtime.review_transaction() {
            Ok(review) => {
                self.set_text(
                    serde_json::json!({
                        "state": "review",
                        "network": review.network,
                        "inputCount": review.input_count,
                        "outputCount": review.output_count,
                        "inputTotal": review.input_total.to_string(),
                        "outputTotal": review.output_total.to_string(),
                        "fee": review.fee.to_string(),
                        "inputs": review.inputs.iter().map(|input| serde_json::json!({
                            "index": input.index,
                            "outpoint": input.outpoint,
                            "amount": input.amount.to_string(),
                            "scriptType": input.script_type,
                            "address": input.address,
                        })).collect::<Vec<_>>(),
                        "outputs": review.outputs.iter().map(|output| serde_json::json!({
                            "index": output.index,
                            "amount": output.amount.to_string(),
                            "ownership": output.ownership,
                            "address": output.address,
                        })).collect::<Vec<_>>(),
                    })
                    .to_string(),
                );
                OK
            }
            Err(error) => self.fail(error),
        }
    }

    fn approve(&mut self, now_unix: u64) -> i32 {
        self.clear_results();
        match self.runtime.approve(now_unix) {
            Ok(frames) => {
                let frame_count = frames.len();
                self.set_text(
                    serde_json::json!({
                        "frameCount": frame_count,
                    })
                    .to_string(),
                );
                OK
            }
            Err(error) => self.fail(error),
        }
    }
}

impl Drop for NativeVault {
    fn drop(&mut self) {
        self.clear_results();
    }
}

fn with_handle_mut<R>(
    handle: *mut NativeVault,
    operation: impl for<'a> FnOnce(&'a mut NativeVault) -> R,
) -> Option<R> {
    if handle.is_null() {
        return None;
    }
    // SAFETY: handles originate exclusively from `kaskold_vault_new`; native
    // wrappers own one handle and destroy it once. The higher-ranked closure
    // prevents the temporary mutable reference from escaping this helper.
    Some(operation(unsafe { &mut *handle }))
}

fn with_input_bytes<R>(
    data: *const u8,
    len: usize,
    operation: impl for<'a> FnOnce(&'a [u8]) -> R,
) -> Option<R> {
    let bytes = if data.is_null() {
        if len != 0 {
            return None;
        }
        &[]
    } else {
        // SAFETY: the FFI caller supplies a readable buffer of `len` bytes for
        // the duration of this call. The higher-ranked closure prevents the
        // borrowed slice from escaping this helper.
        unsafe { slice::from_raw_parts(data, len) }
    };
    Some(operation(bytes))
}

fn with_input_text<R>(
    data: *const u8,
    len: usize,
    operation: impl for<'a> FnOnce(&'a str) -> R,
) -> Option<R> {
    with_input_bytes(data, len, |bytes| str::from_utf8(bytes).ok().map(operation)).flatten()
}

fn copy_out(data: &[u8], output: *mut u8, capacity: usize) -> isize {
    if output.is_null() || capacity < data.len() {
        return -(isize::try_from(data.len()).unwrap_or(isize::MAX));
    }
    if !data.is_empty() {
        // SAFETY: capacity was checked above and the destination is caller-owned
        // writable memory. Source/destination do not overlap across the FFI ABI.
        unsafe { ptr::copy_nonoverlapping(data.as_ptr(), output, data.len()) };
    }
    isize::try_from(data.len()).unwrap_or(isize::MAX)
}

mod exports;
pub(crate) mod workflows;

pub use exports::*;
