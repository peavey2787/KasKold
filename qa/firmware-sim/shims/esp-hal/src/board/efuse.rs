use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, PoisonError};

/// A production-shaped part: the read-protected user HMAC key the wallet store seals with.
const DEVICE_KEY_SLOT: u8 = 3;
const DEVICE_KEY_BYTE: u8 = 0x5A;
const ANOTHER_DEVICE_KEY_BYTE: u8 = 0xA5;
const KEY_PURPOSE_HMAC_UP: u8 = 8;

static KEY_BYTE: AtomicU8 = AtomicU8::new(DEVICE_KEY_BYTE);
static BURNED: Mutex<BTreeMap<&str, u8>> = Mutex::new(BTreeMap::new());

pub const RETAIL_FUSES: [&str; 6] = [
    "SECURE_BOOT_EN",
    "SPI_BOOT_CRYPT_CNT",
    "DIS_PAD_JTAG",
    "DIS_USB_JTAG",
    "DIS_DOWNLOAD_MANUAL_ENCRYPT",
    "SECURE_VERSION",
];

/// What the factory burns into a retail unit, leaving out the fuses `skip` names.
pub fn burn_retail_fuses(skip: &[&str]) {
    let mut burned = BURNED.lock().unwrap_or_else(PoisonError::into_inner);
    let readable_key = burned.remove("RD_DIS");
    burned.clear();
    burned.extend(readable_key.map(|value| ("RD_DIS", value)));
    for name in RETAIL_FUSES {
        if !skip.contains(&name) {
            burned.insert(name, 1);
        }
    }
}

/// Leaves the wallet-sealing key unprotected, as a badly provisioned part would.
pub fn leave_device_key_readable() {
    BURNED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert("RD_DIS", 0);
}

/// Storage moved to a different chip: the same flash and card, another burned key.
pub fn become_another_device() {
    KEY_BYTE.store(ANOTHER_DEVICE_KEY_BYTE, Ordering::SeqCst);
}

pub fn hmac_key(slot: u8, user_purpose: bool) -> Option<[u8; 32]> {
    (slot == DEVICE_KEY_SLOT && user_purpose).then(|| [KEY_BYTE.load(Ordering::SeqCst); 32])
}

pub fn field(name: &str) -> [u8; 16] {
    let mut value = [0u8; 16];
    if let Some(burned) = BURNED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(name)
    {
        value[0] = *burned;
        return value;
    }
    match name {
        "KEY_PURPOSE_3" => value[0] = KEY_PURPOSE_HMAC_UP,
        "RD_DIS" => value[0] = 1 << DEVICE_KEY_SLOT,
        "OPTIONAL_UNIQUE_ID" => value = [0x11; 16],
        _ => {}
    }
    value
}

pub fn base_mac() -> [u8; 6] {
    [0x02, 0x00, 0x00, 0xC0, 0x1D, 0x00]
}
