// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! CoreS3 hardware façade.
//!
//! `shared/` owns ESP32-S3 and protocol primitives. `m5stack/` owns the
//! concrete CoreS3/CoreS3 Lite adapter selected by its declarative board
//! profile. The rest of the firmware imports stable `hw::*` names.

#![cfg_attr(feature = "hardware-tests", allow(dead_code))]
#![cfg_attr(feature = "workflow-test-auto", allow(dead_code))]
#[cfg(not(feature = "m5stack"))]
compile_error!("hardware firmware requires the m5stack CoreS3 platform feature");

pub(crate) mod shared;
mod m5stack;

pub(crate) use m5stack::{battery, camera, display, pmu, sdcard, sound, touch};
pub(crate) use m5stack::{entropy_imu as imu, rtc};
pub(crate) use m5stack::spi_bus::initialize as initialize_cores3_spi;

pub(crate) use shared::lockdown;
#[cfg(feature = "screenshot")]
pub(crate) use shared::screenshot;

pub(crate) const ACTIVE_BOARD_NAME: &str = m5stack::BOARD_NAME;
