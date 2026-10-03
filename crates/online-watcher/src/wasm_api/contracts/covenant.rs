// KasKold Companion Web — KIP-10 covenant WASM exports.
// Split out of lib.rs; behaviour unchanged. License: GPL-3.0.

//! wasm-bindgen exports for the KIP-10 covenant suite: address builders and
//! PSKB spend constructors for every covenant type.

use crate::wasm_api::JsValue;

mod global_thread;
mod logging;

mod families;

pub use families::*;

#[cfg(test)]
mod unit_tests;
