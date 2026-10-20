//! Boot-trust decisions over raw ESP32-S3 eFuse values.
//!
//! The firmware only reads the fuses; what they mean is decided here, so every
//! combination is host-tested rather than trusted to a register read.

/// Key-purpose code for Secure Boot v2 digest slot `n` is `0x09 + n`.
const SECURE_BOOT_DIGEST0_PURPOSE: u8 = 0x09;
/// `WR_DIS` bit that write-protects `SECURE_BOOT_KEY_REVOKE{n}` is `5 + n`.
const KEY_REVOKE0_WR_DIS_BIT: u32 = 5;
pub const KEY_PURPOSE_SLOTS: usize = 6;
pub const SECURE_BOOT_DIGEST_SLOTS: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootSecurityLevel {
    None,
    SoftwareVerified,
    HardwareEnforced,
}

/// Hardware enforcement outranks the application's own release verification.
pub const fn boot_security_level(
    secure_boot_enabled: bool,
    software_verified: bool,
) -> BootSecurityLevel {
    if secure_boot_enabled {
        BootSecurityLevel::HardwareEnforced
    } else if software_verified {
        BootSecurityLevel::SoftwareVerified
    } else {
        BootSecurityLevel::None
    }
}

/// The fuses that decide whether a captured chip can be read or reflashed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LockFuses {
    pub secure_boot_en: u8,
    pub spi_boot_crypt_cnt: u8,
    pub dis_pad_jtag: u8,
    pub dis_usb_jtag: u8,
}

impl LockFuses {
    pub const fn secure_boot_enabled(self) -> bool {
        self.secure_boot_en != 0
    }

    /// Flash encryption is on while an odd number of `SPI_BOOT_CRYPT_CNT` bits are set.
    pub const fn flash_encryption_enabled(self) -> bool {
        self.spi_boot_crypt_cnt.count_ones() % 2 == 1
    }

    /// Secure Boot enforced, flash encrypted, and both JTAG paths disabled.
    pub const fn hardware_protection_active(self) -> bool {
        self.secure_boot_enabled()
            && self.flash_encryption_enabled()
            && self.dis_pad_jtag != 0
            && self.dis_usb_jtag != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityProfile {
    /// Vendor digest in slot 0, owner digest in slot 1, slot 2 closed.
    DualAuthority,
    /// Owner digest in slot 0 is the sole authority; slots 1 and 2 revoked.
    OwnerOnly,
}

impl AuthorityProfile {
    const fn owner_slot(self) -> usize {
        match self {
            Self::DualAuthority => 1,
            Self::OwnerOnly => 0,
        }
    }

    /// Every digest slot other than the owner's that must be revoked: slot 2
    /// always, and slot 1 too when the owner is the sole authority.
    const fn must_be_revoked(self, slot: usize) -> bool {
        match self {
            Self::DualAuthority => slot == 2,
            Self::OwnerOnly => slot != 0,
        }
    }
}

/// The fuses that record which Secure Boot digests are enrolled and revoked.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuthorityFuses {
    /// `KEY_PURPOSE_0..=5`. ESP-IDF may put a digest in any free key block.
    pub key_purposes: [u8; KEY_PURPOSE_SLOTS],
    /// `SECURE_BOOT_KEY_REVOKE0..=2`.
    pub key_revoke: [u8; SECURE_BOOT_DIGEST_SLOTS],
    /// `WR_DIS`.
    pub write_disable: u32,
}

impl AuthorityFuses {
    /// The owner digest is present and live, its revoke control is
    /// write-protected so it can never be revoked later, and every other slot
    /// the profile closes is revoked.
    pub fn owner_authority_enrolled(&self, profile: AuthorityProfile) -> bool {
        let owner = profile.owner_slot();
        let owner_purpose = SECURE_BOOT_DIGEST0_PURPOSE + owner as u8;
        let owner_revoke_locked =
            self.write_disable & (1 << (KEY_REVOKE0_WR_DIS_BIT + owner as u32)) != 0;
        self.key_purposes.contains(&owner_purpose)
            && self.key_revoke[owner] == 0
            && owner_revoke_locked
            && (0..SECURE_BOOT_DIGEST_SLOTS)
                .filter(|slot| profile.must_be_revoked(*slot))
                .all(|slot| self.key_revoke[slot] != 0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PopItPreflightError {
    AlreadyEnabled,
    SecurityVersionInvalid,
    BuildIdentityMissing,
}

impl PopItPreflightError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::AlreadyEnabled => "Secure Boot v2 is already hardware enabled",
            Self::SecurityVersionInvalid => "Firmware security version is invalid",
            Self::BuildIdentityMissing => "Production build identity is missing",
        }
    }
}

/// Conditions the application checks before it records a Pop It request; the
/// bootloader repeats its own checks before any irreversible eFuse write.
pub const fn pop_it_preflight(
    secure_boot_enabled: bool,
    security_version: u32,
    build_identity_present: bool,
) -> Result<(), PopItPreflightError> {
    if secure_boot_enabled {
        Err(PopItPreflightError::AlreadyEnabled)
    } else if security_version == 0 {
        Err(PopItPreflightError::SecurityVersionInvalid)
    } else if !build_identity_present {
        Err(PopItPreflightError::BuildIdentityMissing)
    } else {
        Ok(())
    }
}
