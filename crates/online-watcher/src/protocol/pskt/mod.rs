// KasKold Companion Web — PSKT / PSKB protocol subsystem
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! PSKB encoding used by the Companion transaction builders. Review, relay,
//! merge, finalization, anti-klepto and sighash come from Kaspa Portal.

mod error;
pub(crate) mod exact_json;
mod model;
pub mod pskb;
pub(crate) mod wire;

pub use model::PsktFormat;

#[cfg(test)]
mod unit_tests;
