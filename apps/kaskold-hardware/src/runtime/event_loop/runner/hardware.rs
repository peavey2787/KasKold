// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Hardware and input ownership the entry point hands to the event loop.

#[cfg(not(feature = "workflow-test-auto"))]
use crate::{hw::touch::TouchTracker, runtime::touch_dispatch::NavigationZones};

use crate::{
    hw::display::BootDisplay,
    services::{camera_device::CameraPort, persistent_wallet::PersistentWallet},
};
use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};

/// Hardware singletons the entry point hands to the application event loop.
pub(crate) struct RuntimeHardware<'d> {
    pub(crate) persistent_wallet: PersistentWallet<'d>,
    pub(crate) boot_display: BootDisplay<'d>,
    pub(crate) delay: Delay,
    pub(crate) i2c: I2c<'d, Blocking>,
    pub(crate) camera: CameraPort<'d>,
}

/// Touch, audio, and zone state that only the interactive loop polls.
#[cfg(not(feature = "workflow-test-auto"))]
pub(crate) struct InteractiveInput {
    pub(crate) tracker: TouchTracker,
    #[cfg(feature = "m5stack")]
    pub(crate) runtime_audio: Option<crate::hw::sound::RuntimeAudio>,
    pub(crate) zones: NavigationZones,
}
