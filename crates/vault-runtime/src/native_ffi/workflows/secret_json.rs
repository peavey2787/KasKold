//! Workflow input JSON that wipes every string it holds when dropped.
//!
//! Inputs carry passwords, passphrases and recovery phrases. Parsing copies
//! them into `serde_json` strings, so the parsed tree is wiped like any other
//! secret buffer once the workflow returns.

use core::ops::Deref;

use zeroize::Zeroize;

pub(crate) struct SecretJson(pub(crate) serde_json::Value);

impl Deref for SecretJson {
    type Target = serde_json::Value;

    fn deref(&self) -> &serde_json::Value {
        &self.0
    }
}

impl Zeroize for SecretJson {
    fn zeroize(&mut self) {
        wipe_strings(&mut self.0);
    }
}

fn wipe_strings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => text.zeroize(),
        serde_json::Value::Array(items) => items.iter_mut().for_each(wipe_strings),
        serde_json::Value::Object(fields) => fields.values_mut().for_each(wipe_strings),
        _ => {}
    }
}
