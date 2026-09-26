// KasKold Companion Web — Watch-only companion wallet for KasKold
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! # KasKold Companion Web
//!
//! Watch-only companion wallet for KasKold, compiled to WebAssembly. Browser-facing
//! exports are grouped under `wasm_api`; domain modules retain address derivation,
//! transaction planning, networking, protocol encoding, privacy, and covenant logic.
//!
//! Signing never happens here. Companion builds unsigned KSPT or PSKB transactions, the
//! air-gapped KasKold signs them, and Companion broadcasts the signed result.

mod account;
mod contracts;
pub mod facade;
mod infrastructure;
mod multisig;
mod network;
mod privacy;
mod protocol;
pub mod serialization;
mod transaction_builder;
#[cfg(feature = "browser-api")]
mod wasm_api;

pub use account::bip32::WalletData;
pub use account::{balance::BalanceInfo, utxo::UtxoEntry};
pub use facade::WatchWallet;
#[cfg(feature = "browser-api")]
pub use wasm_api::*;
