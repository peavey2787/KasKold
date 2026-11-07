//! Test-driver boundary injected after authoritative production startup.

use crate::{
    hw::display::BootDisplay,
    runtime::data::AppData,
    services::{camera_device::CameraPort, persistent_wallet::PersistentWallet},
};
use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};

pub(super) fn run(
    ad: &mut AppData,
    mut display: BootDisplay<'_>,
    mut i2c: I2c<'_, Blocking>,
    mut delay: Delay,
    mut camera: CameraPort<'_>,
    mut persistent_wallet: PersistentWallet<'_>,
    mut watchdog_feed: impl FnMut(),
) -> ! {
    crate::runtime::workflow_tests::run_connected_gate(
        ad,
        &mut display,
        &mut i2c,
        &mut delay,
        &mut camera,
        &mut persistent_wallet,
        &mut watchdog_feed,
    );
    crate::runtime::workflow_tests::park_after_gate(&mut delay, &mut watchdog_feed);
}
