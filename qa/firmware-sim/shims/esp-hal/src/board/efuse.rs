/// A production-shaped part with the user HMAC key in slot 0.
const DEVICE_KEY: [u8; 32] = [0x5A; 32];
const KEY_PURPOSE_HMAC_UP: u8 = 8;

pub fn hmac_key(slot: u8, user_purpose: bool) -> Option<[u8; 32]> {
    (slot == 0 && user_purpose).then_some(DEVICE_KEY)
}

pub fn field(name: &str) -> [u8; 16] {
    let mut value = [0u8; 16];
    match name {
        "KEY_PURPOSE_0" => value[0] = KEY_PURPOSE_HMAC_UP,
        "OPTIONAL_UNIQUE_ID" => value = [0x11; 16],
        _ => {}
    }
    value
}

pub fn base_mac() -> [u8; 6] {
    [0x02, 0x00, 0x00, 0xC0, 0x1D, 0x00]
}
