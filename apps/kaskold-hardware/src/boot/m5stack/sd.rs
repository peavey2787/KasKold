//! CoreS3 SD-card probe on the board-owned shared SPI2 bus.

use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};

/// Probe the slot once at power-on for the boot log. Card operations probe it
/// again, so a card inserted later is still found.
pub(crate) fn initialize(i2c: &mut I2c<'_, Blocking>, delay: &mut Delay) {
    crate::log!("   BOOT PHASE sd BEGIN");
    match crate::hw::sdcard::detect_card(i2c, delay) {
        Some(card_type) => crate::log!("   SD card shared-SPI init OK: {:?}", card_type),
        None => crate::log!("   SD card absent or unusable (continuing without SD)"),
    }
    crate::log!("   BOOT PHASE sd DONE");
}
