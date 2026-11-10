use super::CANDIDATES;
use crate::crypto::password_kdf::PasswordKdfParams;

/// The device compares its Argon2 output with these values, so each one must be
/// what the shared KDF actually derives. The reference libargon2 implementation
/// gives the same outputs for these inputs. Kaspa Portal exposes the benchmark
/// derivation only to `verbose-boot` builds.
#[cfg(feature = "verbose-boot")]
#[test]
fn every_calibration_vector_matches_the_shared_kdf() {
    use super::{PASSWORD, PURPOSE, SALT};
    use crate::crypto::password_kdf::derive_benchmark_key_32;

    for candidate in &CANDIDATES {
        let derived = derive_benchmark_key_32(PURPOSE, PASSWORD, &SALT, candidate.parameters());
        assert_eq!(
            derived,
            Ok(candidate.expected),
            "m={} KiB",
            candidate.memory_kib
        );
    }
}

/// The hardware test must exercise the profile that ships. If Kaspa Portal
/// changes it, the device has to be recalibrated at the new size first.
#[test]
fn the_shipping_profile_is_a_calibrated_size() {
    let current = PasswordKdfParams::current();
    assert!(
        CANDIDATES
            .iter()
            .any(|candidate| candidate.parameters() == current),
        "{current:?} is not among the device calibration sizes"
    );
}
