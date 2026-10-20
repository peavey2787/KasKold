//! User-facing boot trust classification and Pop It preflight.
//!
//! Secure Boot v2 eFuse programming remains bootloader-owned. The application
//! only records an integrity-checked one-shot UI request after its normal release verifier
//! has completed successfully, then restarts into the signed ESP-IDF
//! bootloader. The bootloader performs Espressif's final signature/eFuse
//! viability checks immediately before any irreversible write.

#[cfg(not(feature = "qemu"))]
use esp_hal::efuse::{DIS_PAD_JTAG, DIS_USB_JTAG, Efuse, SECURE_BOOT_EN, SPI_BOOT_CRYPT_CNT};
#[cfg(all(feature = "provisioning-ui", feature = "m5stack", not(feature = "qemu")))]
use esp_hal::efuse::{
    KEY_PURPOSE_0, KEY_PURPOSE_1, KEY_PURPOSE_2, KEY_PURPOSE_3, KEY_PURPOSE_4, KEY_PURPOSE_5,
    SECURE_BOOT_KEY_REVOKE0, SECURE_BOOT_KEY_REVOKE1, SECURE_BOOT_KEY_REVOKE2, WR_DIS,
};
use kaskold_hardware_core::security::boot_fuses;
pub use kaskold_hardware_core::security::boot_fuses::BootSecurityLevel;
#[cfg(feature = "secure-provisioning-core")]
pub use kaskold_hardware_core::security::boot_fuses::PopItPreflightError;
#[cfg(all(feature = "provisioning-ui", feature = "m5stack", not(feature = "production")))]
use core::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

#[cfg(all(feature = "provisioning-ui", feature = "m5stack", not(feature = "production")))]
static DEV_POP_IT_INDICATOR_DEMO: AtomicBool = AtomicBool::new(false);

pub fn level() -> BootSecurityLevel {
    // Normal UI is reached only after the profile-appropriate signature and
    // code hash have verified. Development uses a repository-public TEST key;
    // production uses the private release identity and may later add ROM eFuse enforcement.
    boot_fuses::boot_security_level(
        secure_boot_enabled(),
        crate::services::verify::software_verification_configured(),
    )
}

/// Trust icon state. Development may deliberately preview the teal hardware
/// badge without changing the truthful boot-security classification or touching
/// any eFuse. The preview is volatile and disappears on reset.
pub fn indicator_level() -> BootSecurityLevel {
    #[cfg(all(feature = "provisioning-ui", feature = "m5stack", not(feature = "production")))]
    if DEV_POP_IT_INDICATOR_DEMO.load(AtomicOrdering::Relaxed) {
        return BootSecurityLevel::HardwareEnforced;
    }
    level()
}

#[cfg(all(feature = "provisioning-ui", feature = "m5stack", not(feature = "production")))]
pub fn enable_dev_pop_it_indicator_demo() {
    DEV_POP_IT_INDICATOR_DEMO.store(true, AtomicOrdering::Relaxed);
}

#[cfg(all(feature = "provisioning-ui", feature = "m5stack", not(feature = "production")))]
pub fn dev_pop_it_indicator_demo_active() -> bool {
    DEV_POP_IT_INDICATOR_DEMO.load(AtomicOrdering::Relaxed)
}

#[cfg(all(
    feature = "provisioning-ui",
    not(all(feature = "m5stack", not(feature = "production")))
))]
pub const fn dev_pop_it_indicator_demo_active() -> bool { false }

#[cfg(feature = "secure-provisioning-core")]
pub fn pop_it_preflight() -> Result<(), PopItPreflightError> {
    boot_fuses::pop_it_preflight(
        secure_boot_enabled(),
        crate::services::verify::anti_rollback::APP_SECURITY_VERSION,
        crate::services::verify::attestation::build_commit().is_ok(),
    )
}

#[cfg(not(feature = "qemu"))]
fn lock_fuses() -> boot_fuses::LockFuses {
    boot_fuses::LockFuses {
        secure_boot_en: Efuse::read_field_le::<u8>(SECURE_BOOT_EN),
        spi_boot_crypt_cnt: Efuse::read_field_le::<u8>(SPI_BOOT_CRYPT_CNT),
        dis_pad_jtag: Efuse::read_field_le::<u8>(DIS_PAD_JTAG),
        dis_usb_jtag: Efuse::read_field_le::<u8>(DIS_USB_JTAG),
    }
}

#[cfg(not(feature = "qemu"))]
pub fn secure_boot_enabled() -> bool {
    lock_fuses().secure_boot_enabled()
}

#[cfg(feature = "qemu")]
pub const fn secure_boot_enabled() -> bool { false }

/// Whether the chip is hardware-locked. Production firmware stores wallets only
/// in this state, so a physically captured unlocked chip never holds a
/// readable wallet record.
#[cfg(not(feature = "qemu"))]
pub fn hardware_protection_active() -> bool {
    lock_fuses().hardware_protection_active()
}

#[cfg(feature = "qemu")]
pub const fn hardware_protection_active() -> bool { false }

#[cfg(all(feature = "provisioning-ui", feature = "m5stack", not(feature = "qemu")))]
const AUTHORITY_PROFILE: boot_fuses::AuthorityProfile = if cfg!(feature = "secure-owner-only") {
    boot_fuses::AuthorityProfile::OwnerOnly
} else {
    boot_fuses::AuthorityProfile::DualAuthority
};

/// Whether the device has the owner authority this provisioning profile requires.
#[cfg(all(feature = "provisioning-ui", feature = "m5stack", not(feature = "qemu")))]
pub fn owner_authority_enrolled() -> bool {
    let read = |field| Efuse::read_field_le::<u8>(field);
    boot_fuses::AuthorityFuses {
        key_purposes: [KEY_PURPOSE_0, KEY_PURPOSE_1, KEY_PURPOSE_2, KEY_PURPOSE_3, KEY_PURPOSE_4, KEY_PURPOSE_5]
            .map(read),
        key_revoke: [SECURE_BOOT_KEY_REVOKE0, SECURE_BOOT_KEY_REVOKE1, SECURE_BOOT_KEY_REVOKE2].map(read),
        write_disable: Efuse::read_field_le::<u32>(WR_DIS),
    }
    .owner_authority_enrolled(AUTHORITY_PROFILE)
}

#[cfg(all(feature = "provisioning-ui", not(all(feature = "m5stack", not(feature = "qemu")))))]
pub const fn owner_authority_enrolled() -> bool { false }
