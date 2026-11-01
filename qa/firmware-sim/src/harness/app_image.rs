//! The signed application image a retail chip boots, as the bootloader finds it in flash.
//! A chip without flash encryption attests these bytes, so they are built to the ESP format.
use esp_hal::board::flash;
use kaskold_hardware_core::update::attestation as layout;
use sha2::{Digest, Sha256};

const APP_FLASH_OFFSET: u32 = 0x0001_0000;
const SEGMENT_BYTES: usize = 1024;
const SEGMENT_LOAD_ADDRESS: u32 = 0x4200_0000;

#[derive(Clone, Copy)]
pub(crate) enum ImageFault {
    ChangedAfterHashing,
    NoBootSignature,
    SignatureOverOtherContent,
}

pub(super) fn install() {
    write_to_flash(&signed_image());
}

pub(super) fn install_with(fault: ImageFault) {
    let mut image = signed_image();
    let signature = image.len() - layout::SECURE_BOOT_SIGNATURE_PREFIX_SIZE;
    match fault {
        ImageFault::ChangedAfterHashing => image[layout::ESP_IMAGE_HEADER_SIZE] ^= 0x01,
        ImageFault::NoBootSignature => image[signature] = 0xFF,
        ImageFault::SignatureOverOtherContent => image[signature + 4] ^= 0x01,
    }
    write_to_flash(&image);
}

fn signed_image() -> Vec<u8> {
    let mut image = vec![0u8; layout::ESP_IMAGE_HEADER_SIZE];
    image[0] = layout::ESP_IMAGE_MAGIC;
    image[1] = 1;
    image[layout::ESP_IMAGE_HEADER_SIZE - 1] = 1;
    image.extend_from_slice(&SEGMENT_LOAD_ADDRESS.to_le_bytes());
    image.extend_from_slice(&(SEGMENT_BYTES as u32).to_le_bytes());
    image.extend((0..SEGMENT_BYTES).map(|index| index as u8));

    let hash_offset = layout::appended_hash_offset(image.len() as u32).expect("hash offset");
    image.resize(hash_offset as usize, 0);
    let appended_hash = Sha256::digest(&image);
    image.extend_from_slice(&appended_hash);

    let image_end = layout::signed_image_end(hash_offset).expect("image end");
    let signature_offset = layout::secure_boot_signature_offset(image_end).expect("signature");
    image.resize(signature_offset as usize, 0xFF);
    let digest = Sha256::digest(&image);
    image.extend_from_slice(&[
        layout::SECURE_BOOT_SIGNATURE_MAGIC,
        layout::SECURE_BOOT_SIGNATURE_VERSION,
        0,
        0,
    ]);
    image.extend_from_slice(&digest);
    image
}

fn write_to_flash(image: &[u8]) {
    assert_eq!(
        flash::write(APP_FLASH_OFFSET, image),
        0,
        "virtual flash write"
    );
}
