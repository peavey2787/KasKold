use crate::security::boot_fuses::{
    boot_security_level, pop_it_preflight, AuthorityFuses, AuthorityProfile, BootSecurityLevel,
    LockFuses, PopItPreflightError, KEY_PURPOSE_SLOTS,
};

const LOCKED: LockFuses = LockFuses {
    secure_boot_en: 1,
    spi_boot_crypt_cnt: 0b001,
    dis_pad_jtag: 1,
    dis_usb_jtag: 1,
};

#[test]
fn boot_level_prefers_hardware_enforcement() {
    assert_eq!(
        boot_security_level(true, true),
        BootSecurityLevel::HardwareEnforced
    );
    assert_eq!(
        boot_security_level(true, false),
        BootSecurityLevel::HardwareEnforced
    );
    assert_eq!(
        boot_security_level(false, true),
        BootSecurityLevel::SoftwareVerified
    );
    assert_eq!(boot_security_level(false, false), BootSecurityLevel::None);
}

#[test]
fn flash_encryption_follows_crypt_count_parity() {
    let enabled: [bool; 8] = core::array::from_fn(|count| {
        LockFuses {
            spi_boot_crypt_cnt: count as u8,
            ..LOCKED
        }
        .flash_encryption_enabled()
    });
    assert_eq!(
        enabled,
        [false, true, true, false, true, false, false, true]
    );
}

#[test]
fn hardware_protection_needs_every_lock() {
    assert!(LOCKED.hardware_protection_active());
    assert!(LOCKED.secure_boot_enabled());
    let missing = [
        LockFuses {
            secure_boot_en: 0,
            ..LOCKED
        },
        LockFuses {
            spi_boot_crypt_cnt: 0b011,
            ..LOCKED
        },
        LockFuses {
            spi_boot_crypt_cnt: 0,
            ..LOCKED
        },
        LockFuses {
            dis_pad_jtag: 0,
            ..LOCKED
        },
        LockFuses {
            dis_usb_jtag: 0,
            ..LOCKED
        },
        LockFuses::default(),
    ];
    for fuses in missing {
        assert!(!fuses.hardware_protection_active(), "{fuses:?}");
    }
    assert!(!LockFuses {
        secure_boot_en: 0,
        ..LOCKED
    }
    .secure_boot_enabled());
}

/// Owner digest in slot 1 (purpose 0x0a), revoke 1 write-protected (WR_DIS bit
/// 6), slot 2 revoked; the vendor digest stays live in slot 0.
const DUAL: AuthorityFuses = AuthorityFuses {
    key_purposes: [0x09, 0x0a, 0, 0, 0, 0],
    key_revoke: [0, 0, 1],
    write_disable: 1 << 6,
};

/// Owner digest in slot 0 (purpose 0x09), revoke 0 write-protected (WR_DIS
/// bit 5), slots 1 and 2 revoked.
const OWNER_ONLY: AuthorityFuses = AuthorityFuses {
    key_purposes: [0, 0, 0x09, 0, 0, 0],
    key_revoke: [0, 1, 1],
    write_disable: 1 << 5,
};

fn enrolled(fuses: AuthorityFuses) -> [bool; 2] {
    [
        fuses.owner_authority_enrolled(AuthorityProfile::DualAuthority),
        fuses.owner_authority_enrolled(AuthorityProfile::OwnerOnly),
    ]
}

#[test]
fn each_profile_recognises_only_its_own_enrollment() {
    assert_eq!(enrolled(DUAL), [true, false]);
    assert_eq!(enrolled(OWNER_ONLY), [false, true]);
    assert_eq!(enrolled(AuthorityFuses::default()), [false, false]);
}

#[test]
fn owner_digest_may_sit_in_any_key_block() {
    for block in 0..KEY_PURPOSE_SLOTS {
        let mut dual = DUAL;
        dual.key_purposes = [0; KEY_PURPOSE_SLOTS];
        dual.key_purposes[block] = 0x0a;
        assert_eq!(
            enrolled(dual),
            [true, false],
            "dual digest in block {block}"
        );
        let mut owner = OWNER_ONLY;
        owner.key_purposes = [0; KEY_PURPOSE_SLOTS];
        owner.key_purposes[block] = 0x09;
        assert_eq!(
            enrolled(owner),
            [false, true],
            "owner-only digest in block {block}"
        );
    }
}

#[test]
fn dual_authority_fails_closed_on_any_single_fault() {
    let faults = [
        AuthorityFuses {
            key_purposes: [0x09, 0x0b, 0, 0, 0, 0],
            ..DUAL
        },
        AuthorityFuses {
            key_revoke: [0, 1, 1],
            ..DUAL
        },
        AuthorityFuses {
            key_revoke: [0, 0, 0],
            ..DUAL
        },
        AuthorityFuses {
            write_disable: 0,
            ..DUAL
        },
        AuthorityFuses {
            write_disable: 1 << 5 | 1 << 7,
            ..DUAL
        },
    ];
    for fuses in faults {
        assert!(
            !fuses.owner_authority_enrolled(AuthorityProfile::DualAuthority),
            "{fuses:?}"
        );
    }
    let extra_protection = AuthorityFuses {
        write_disable: u32::MAX,
        ..DUAL
    };
    assert!(extra_protection.owner_authority_enrolled(AuthorityProfile::DualAuthority));
}

#[test]
fn owner_only_fails_closed_on_any_single_fault() {
    let faults = [
        AuthorityFuses {
            key_purposes: [0, 0, 0x0a, 0, 0, 0],
            ..OWNER_ONLY
        },
        AuthorityFuses {
            key_revoke: [1, 1, 1],
            ..OWNER_ONLY
        },
        AuthorityFuses {
            key_revoke: [0, 0, 1],
            ..OWNER_ONLY
        },
        AuthorityFuses {
            key_revoke: [0, 1, 0],
            ..OWNER_ONLY
        },
        AuthorityFuses {
            write_disable: 0,
            ..OWNER_ONLY
        },
        AuthorityFuses {
            write_disable: 1 << 6 | 1 << 7,
            ..OWNER_ONLY
        },
    ];
    for fuses in faults {
        assert!(
            !fuses.owner_authority_enrolled(AuthorityProfile::OwnerOnly),
            "{fuses:?}"
        );
    }
}

#[test]
fn pop_it_preflight_checks_in_order() {
    assert_eq!(pop_it_preflight(false, 1, true), Ok(()));
    assert_eq!(
        pop_it_preflight(true, 0, false),
        Err(PopItPreflightError::AlreadyEnabled)
    );
    assert_eq!(
        pop_it_preflight(false, 0, false),
        Err(PopItPreflightError::SecurityVersionInvalid)
    );
    assert_eq!(
        pop_it_preflight(false, 7, false),
        Err(PopItPreflightError::BuildIdentityMissing)
    );
    assert_eq!(
        PopItPreflightError::AlreadyEnabled.message(),
        "Secure Boot v2 is already hardware enabled"
    );
    assert_eq!(
        PopItPreflightError::SecurityVersionInvalid.message(),
        "Firmware security version is invalid"
    );
    assert_eq!(
        PopItPreflightError::BuildIdentityMissing.message(),
        "Production build identity is missing"
    );
}
