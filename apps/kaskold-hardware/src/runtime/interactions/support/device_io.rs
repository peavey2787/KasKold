// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Peripheral handles a long-running interaction borrows from the event loop.

use crate::hw::display::BootDisplay;
use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};

/// Display, timing, bus, and watchdog handles lent to one interaction.
pub(crate) struct DeviceIo<'io, 'display, 'hal> {
    pub(crate) boot_display: &'io mut BootDisplay<'display>,
    pub(crate) delay: &'io mut Delay,
    pub(crate) i2c: &'io mut I2c<'hal, Blocking>,
    pub(crate) liveness: &'io mut dyn FnMut(),
}
