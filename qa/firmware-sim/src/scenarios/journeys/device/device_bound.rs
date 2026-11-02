//! A thief with the storage but not the chip it was written on gets nothing, even with the PIN.
use crate::harness::Device;
use crate::scenarios::journeys::known::{self, PIN};
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use esp_hal::board::efuse;

const BACKOFF_OVER: std::time::Duration = std::time::Duration::from_secs(65);

journey!(
    flash_moved_to_another_chip_stays_locked_with_the_right_pin:
    restore_the_known_wallet,
    open_on_another_chip,
    still_opens_on_its_own_chip,
);

fn open_on_another_chip() {
    efuse::become_another_device();
    let device = Device::boot();
    device.expect_text("UNLOCK PIN");
    device.enter_pin(PIN);
    device.settle();
    device.expect_absent("Scan QR");
    device.let_time_pass(BACKOFF_OVER);
    device.settle();
    device.expect_absent("Scan QR");
}

/// The attempt elsewhere counted as a failure, and the right chip still opens afterwards.
fn still_opens_on_its_own_chip() {
    let device = Device::boot();
    device.expect_text("Invalid PIN");
    device.let_time_pass(BACKOFF_OVER);
    device.settle();
    device.enter_pin(PIN);
    device.wait_for_text("Scan QR");
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}
