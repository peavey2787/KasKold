// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Staged firmware boot orchestration.
//!
//! Shared boot policy lives in `shared/`; the qualified hardware path is
//! implemented by the M5Stack CoreS3 adapter.

#![cfg_attr(feature = "hardware-tests", allow(dead_code))]
#![cfg_attr(feature = "workflow-test-auto", allow(dead_code))]
pub(crate) mod shared;

#[cfg(feature = "m5stack")]
pub(crate) mod m5stack;

pub(crate) use shared::{application, security};
