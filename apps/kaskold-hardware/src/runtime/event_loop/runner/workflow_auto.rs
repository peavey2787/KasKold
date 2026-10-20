//! Test-driver boundary injected after authoritative production startup.

use crate::{
    hw::{display::BootDisplay, sdcard::SdCardType},
    runtime::{data::AppData, event_loop::camera::CameraHardware},
    services::persistent_wallet::PersistentWallet,
};
use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};

pub(super) fn run(
    ad: &mut AppData,
    mut display: BootDisplay<'_>,
    mut i2c: I2c<'_, Blocking>,
    sd_card_type: Option<SdCardType>,
    mut delay: Delay,
    camera: CameraHardware<'_>,
    mut persistent_wallet: PersistentWallet<'_>,
    mut watchdog_feed: impl FnMut(),
) -> ! {
    let (mut camera, mut camera_dma, mut camera_status) = camera;
    crate::runtime::workflow_tests::run_connected_gate(
        ad,
        &mut display,
        &mut i2c,
        &sd_card_type,
        &mut delay,
        &mut camera,
        &mut camera_dma,
        &mut camera_status,
        &mut persistent_wallet,
        &mut watchdog_feed,
    );
    crate::runtime::workflow_tests::park_after_gate(&mut delay, &mut watchdog_feed);
}
