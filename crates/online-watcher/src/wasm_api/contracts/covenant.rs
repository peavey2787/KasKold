// KasKold Companion Web — KIP-10 covenant WASM exports.
// Split out of lib.rs; behaviour unchanged. License: GPL-3.0.

//! wasm-bindgen exports for the KIP-10 covenant suite: address builders and
//! PSKB spend constructors for every covenant type.

use crate::wasm_api::JsValue;

mod fees;
mod global_thread;
mod logging;
mod spend_path;

mod families;

pub use families::*;
pub use fees::*;
pub use spend_path::*;

#[cfg(test)]
mod unit_tests;
