use super::*;

#[test]
fn private_storage_mass_helpers_cover_zero_relaxed_arithmetic_and_overflow_paths() {
    assert_eq!(plurality_total(&[], "Input"), Ok(0));
    assert_eq!(plurality_total(&[(1, 1), (2, 2)], "Input"), Ok(3));
    assert!(plurality_total(&[(1, u64::MAX), (2, 1)], "Input").is_err());

    assert_eq!(storage_mass_term(0, 1, "Input"), Ok(STORAGE_MASS_C));
    assert_eq!(storage_mass_term(10, 2, "Input"), Ok(400_000_000_000));
    assert!(storage_mass_term(1, u64::MAX, "Input").is_err());

    assert_eq!(harmonic_mass(&[], "Output"), Ok(0));
    assert_eq!(harmonic_mass(&[(10, 1)], "Output"), Ok(100_000_000_000));
    assert!(harmonic_mass(&[(1, u64::MAX)], "Output").is_err());

    assert!(uses_relaxed_storage_mass(1, 9));
    assert!(uses_relaxed_storage_mass(9, 1));
    assert!(uses_relaxed_storage_mass(2, 2));
    assert!(!uses_relaxed_storage_mass(2, 3));

    assert_eq!(
        arithmetic_input_mass(&[(100, 1), (100, 1)], 2),
        Ok(20_000_000_000)
    );
    assert!(arithmetic_input_mass(&[(u64::MAX, 1), (1, 1)], 2).is_err());
}

#[test]
fn dust_and_plurality_cover_exact_thresholds() {
    assert!(is_dust(0));
    assert!(is_dust(1));
    assert!(!is_dust(DUST_THRESHOLD));
    assert!(!is_dust(DUST_THRESHOLD.saturating_add(1)));
    assert_eq!(utxo_plurality(0, false), 1);
    assert_eq!(utxo_plurality(37, false), 1);
    assert_eq!(utxo_plurality(38, false), 2);
    assert_eq!(utxo_plurality(6, true), 2);
}
