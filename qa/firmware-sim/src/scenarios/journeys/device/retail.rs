//! The retail build refuses every chip and image the factory did not finish properly.
//! A finished chip walks every other journey, because this build runs them all.
use crate::harness::{Device, ImageFault};
use crate::scenarios::journeys::known::{self, RECEIVE_0};
use crate::scenarios::journeys::walk;
use esp_hal::board::efuse;

const OPEN_DEBUG_PORT: [&str; 1] = ["DIS_USB_JTAG"];
const UNLOCKED_CHIP_REASON: &str = "Pop It required to save";
const MISSING_KEY_REASON: &str = "Device key not provisioned";

journey!(an_image_with_the_public_development_signature_never_boots: development_signature);
journey!(a_chip_without_a_rollback_floor_never_boots: no_rollback_floor);
journey!(a_chip_without_flash_encryption_never_boots: no_flash_encryption);
journey!(a_chip_with_development_flash_encryption_never_boots: development_flash_encryption);
journey!(a_changed_image_on_an_unlocked_chip_never_boots: image_changed_after_hashing);
journey!(an_unsigned_image_on_an_unlocked_chip_never_boots: image_without_boot_signature);
journey!(a_missigned_image_on_an_unlocked_chip_never_boots: image_signature_over_other_content);
journey!(an_open_debug_port_keeps_the_wallet_off_the_chip: open_debug_port, still_new_with_an_open_debug_port);
journey!(a_readable_device_key_keeps_the_wallet_off_the_chip: readable_device_key);
journey!(an_unlocked_chip_attests_its_image_and_only_holds_a_session_wallet: unlocked_chip);
journey!(a_finished_chip_keeps_the_wallet_across_a_reboot: save_on_a_finished_chip, reopen_on_a_finished_chip);

fn development_signature() {
    Device::boot_with_development_signature().expect_refusal("SIGNATURE INVALID");
}

fn no_rollback_floor() {
    Device::boot_refused_without(&["SECURE_VERSION"])
        .expect_refusal("ANTI-ROLLBACK NOT PROVISIONED");
}

fn no_flash_encryption() {
    Device::boot_refused_without(&["SPI_BOOT_CRYPT_CNT"])
        .expect_refusal("ATTESTATION: FLASH ENCRYPTION OFF");
}

fn development_flash_encryption() {
    Device::boot_refused_without(&["DIS_DOWNLOAD_MANUAL_ENCRYPT"])
        .expect_refusal("ATTESTATION: DEV ENCRYPTION");
}

fn image_changed_after_hashing() {
    Device::boot_unlocked_with_damaged_image(ImageFault::ChangedAfterHashing)
        .expect_refusal("ATTESTATION: IMAGE HASH INVALID");
}

fn image_without_boot_signature() {
    Device::boot_unlocked_with_damaged_image(ImageFault::NoBootSignature)
        .expect_refusal("ATTESTATION: NO BOOT SIGNATURE");
}

fn image_signature_over_other_content() {
    Device::boot_unlocked_with_damaged_image(ImageFault::SignatureOverOtherContent)
        .expect_refusal("ATTESTATION: BOOT DIGEST INVALID");
}

fn open_debug_port() {
    let device = Device::boot_without(&OPEN_DEBUG_PORT);
    expect_saving_refused(&device, UNLOCKED_CHIP_REASON);
}

fn still_new_with_an_open_debug_port() {
    Device::boot_without(&OPEN_DEBUG_PORT).expect_text("Restore Wallet");
}

fn readable_device_key() {
    efuse::leave_device_key_readable();
    let device = Device::boot();
    expect_saving_refused(&device, MISSING_KEY_REASON);
}

fn unlocked_chip() {
    let device = Device::boot_unlocked();
    device.expect_was_shown("SOFTWARE");
    device.expect_was_shown("Secure Boot v2: READY");
    expect_saving_refused(&device, UNLOCKED_CHIP_REASON);
    device.tap("Use for This Session Only");
    walk::open_receive(&device);
    walk::expect_address(&device, RECEIVE_0);
}

fn expect_saving_refused(device: &Device, reason: &str) {
    walk::restore_words(device, known::ABOUT_WORDS, "No Passphrase");
    walk::name_wallet(device);
    device.tap("Save Securely on Device");
    device.expect_was_shown(reason);
    device.expect_text("Save Securely on Device");
}

fn save_on_a_finished_chip() {
    let device = Device::boot();
    device.expect_was_shown("HARDWARE");
    device.expect_was_shown("Secure Boot v2: ON");
    walk::restore_words(&device, known::ABOUT_WORDS, "No Passphrase");
    walk::save_with_pin(&device, known::PIN);
}

fn reopen_on_a_finished_chip() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, known::PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, RECEIVE_0);
}
