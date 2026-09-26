use crate::transaction_builder::planning::amounts::storage_mass_estimate;
use crate::transaction_builder::{model::PlannedOutput, planning::amounts::utxo_plurality};

#[test]
fn consolidation_rejects_empty_and_singleton_sets_before_sorting() {
    assert_eq!(
        super::super::selection::select_for_consolidation(Vec::new(), 5).unwrap_err(),
        "No UTXOs to consolidate"
    );
    assert_eq!(
        super::super::selection::select_for_consolidation(vec![super::utxo(0x11, 0, 10)], 5)
            .unwrap_err(),
        "Only 1 UTXO — nothing to consolidate"
    );
}

#[test]
fn consolidation_keeps_largest_utxos_when_limited() {
    let selected = super::super::selection::select_for_consolidation(
        vec![
            super::utxo(0x11, 0, 10),
            super::utxo(0x22, 1, 40),
            super::utxo(0x33, 2, 20),
            super::utxo(0x44, 3, 30),
        ],
        3,
    )
    .expect("consolidation selection");
    assert_eq!(
        selected
            .iter()
            .map(|entry| entry.amount)
            .collect::<Vec<_>>(),
        vec![40, 30, 20]
    );
}

#[test]
fn smallest_first_sort_is_not_a_noop() {
    let mut utxos = vec![
        super::utxo(0x11, 0, 30),
        super::utxo(0x22, 1, 10),
        super::utxo(0x33, 2, 20),
    ];
    super::super::selection::sort_smallest_first(&mut utxos);
    assert_eq!(
        utxos.iter().map(|entry| entry.amount).collect::<Vec<_>>(),
        vec![10, 20, 30]
    );
}

#[test]
fn multisig_accepts_exactly_three_inputs_and_rejects_four() {
    let destination = PlannedOutput::new(25, vec![0x51]);
    let three = vec![
        super::utxo(0x11, 0, 10),
        super::utxo(0x22, 1, 10),
        super::utxo(0x33, 2, 10),
    ];
    let (plan, change) = super::super::planning::plan_multisig(
        three.clone(),
        destination.clone(),
        5,
        vec![0x52],
        &[0x51],
        1,
    )
    .expect("three inputs are the protocol maximum");
    assert_eq!(change, 0);
    assert_eq!(plan.inputs.len(), 3);
    assert_eq!(plan.outputs.len(), 1);

    let mut four = three;
    four.push(super::utxo(0x44, 3, 10));
    let error = super::super::planning::plan_multisig(four, destination, 5, vec![0x52], &[0x51], 1)
        .unwrap_err();
    assert!(error.contains("limited to 3 inputs"));
}

#[test]
fn multisig_adds_only_positive_non_dust_change() {
    let destination = PlannedOutput::new(20_000_000, vec![0x51]);
    let (plan, change) = super::super::planning::plan_multisig(
        vec![super::utxo(0x11, 0, 40_000_001)],
        destination,
        1,
        vec![0x52],
        &[0x51],
        1,
    )
    .expect("multisig plan");
    assert_eq!(change, 20_000_000);
    assert_eq!(plan.outputs.len(), 2);
    assert_eq!(plan.outputs[1].amount, 20_000_000);
}

#[test]
fn storage_mass_uses_relaxed_harmonic_rule_for_two_by_two_plurality() {
    let mass = storage_mass_estimate(
        &[(90_000_000, 1), (10_000_000, 1)],
        &[(10_000_000, 1), (10_000_000, 1)],
    )
    .expect("storage mass");
    assert_eq!(mass, 88_889);
}

#[test]
fn utxo_plurality_counts_covenant_bytes_across_the_storage_unit_boundary() {
    // 63 fixed bytes + 6 script bytes fit in one 100-byte storage unit.
    assert_eq!(utxo_plurality(6, false), 1);
    // A 32-byte covenant id pushes the same UTXO to 101 bytes => two units.
    assert_eq!(utxo_plurality(6, true), 2);
}

#[test]
fn storage_mass_fee_distinguishes_dust_and_non_dust_change() {
    let dust_selected = vec![super::utxo(0x51, 0, 15_000_000)];
    assert_eq!(
        super::super::standard::storage_mass_fee(&dust_selected, 15_000_000, 10_000_000, 0,)
            .expect("dust-change fee"),
        3_666_740
    );

    let non_dust_selected = vec![super::utxo(0x52, 0, 30_000_000)];
    assert_eq!(
        super::super::standard::storage_mass_fee(&non_dust_selected, 30_000_000, 10_000_000, 0,)
            .expect("non-dust-change fee"),
        16_017_540
    );
}

#[test]
fn standard_input_count_follows_public_signer_capability() {
    let limit = usize::from(kaskold_protocol::SIGNER_CAPABILITIES.max_inputs);
    assert_eq!(limit, 32);
    assert_eq!(
        super::super::standard::validate_signer_input_count(0).unwrap_err(),
        "No UTXOs provided"
    );
    assert!(super::super::standard::validate_signer_input_count(1).is_ok());
    assert!(super::super::standard::validate_signer_input_count(limit).is_ok());
    assert!(super::super::standard::validate_signer_input_count(limit + 1).is_err());
}

#[test]
fn multisig_standard_fee_shape_covers_push_prefix_and_requested_fee_boundaries() {
    let fee_75 = super::super::multisig::multisig_standard_fee_for_shape(1, 75, 1, 1, 34, 34, 0)
        .expect("75-byte redeem fee");
    let fee_76 = super::super::multisig::multisig_standard_fee_for_shape(1, 76, 1, 1, 34, 34, 0)
        .expect("76-byte redeem fee");
    let fee_255 = super::super::multisig::multisig_standard_fee_for_shape(2, 255, 2, 2, 34, 34, 0)
        .expect("255-byte redeem fee");
    let fee_256 = super::super::multisig::multisig_standard_fee_for_shape(2, 256, 2, 2, 34, 34, 0)
        .expect("256-byte redeem fee");
    assert!(fee_76 >= fee_75);
    assert!(fee_256 >= fee_255);

    let requested = fee_256.saturating_add(1_000_000);
    assert_eq!(
        super::super::multisig::multisig_standard_fee_for_shape(2, 256, 2, 2, 34, 34, requested,)
            .expect("requested fee dominates"),
        requested
    );

    assert!(super::super::multisig::multisig_standard_fee_for_shape(
        u8::MAX,
        usize::MAX,
        u8::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        0,
    )
    .is_err());
}

#[test]
fn automatic_selection_and_amount_helpers_cover_error_and_tie_boundaries() {
    use crate::transaction_builder::planning::amounts::{checked_required, checked_sum, is_dust};

    assert!(is_dust(0));
    assert!(is_dust(1));
    assert!(!is_dust(20_000_000));
    assert_eq!(checked_required(10, 5), Ok(15));
    assert!(checked_required(u64::MAX, 1).is_err());
    assert_eq!(checked_sum([1, 2, 3]), Ok(6));
    assert!(checked_sum([u64::MAX, 1]).is_err());

    assert!(super::super::selection::select_automatic_with_limit(
        vec![super::utxo(0x10, 0, 1)],
        1,
        0,
    )
    .unwrap_err()
    .contains("at least 1"));

    let exact = super::super::selection::select_automatic_with_limit(
        vec![super::utxo(0x10, 0, 5), super::utxo(0x11, 1, 10)],
        10,
        1,
    )
    .expect("largest input exactly satisfies target");
    assert_eq!(exact.len(), 1);
    assert_eq!(exact[0].amount, 10);

    assert!(super::super::selection::select_automatic_with_limit(
        vec![super::utxo(0x10, 0, u64::MAX), super::utxo(0x11, 1, 1)],
        u64::MAX,
        2,
    )
    .is_ok());
}

#[test]
fn selection_display_and_checked_total_cover_ordering_and_overflow() {
    let mut values = vec![
        super::utxo(0x22, 2, 10),
        super::utxo(0x11, 1, 10),
        super::utxo(0x33, 0, 20),
    ];
    super::super::selection::sort_for_display(&mut values);
    assert_eq!(values[0].amount, 20);
    assert_eq!(values[1].tx_id, "11".repeat(32));
    assert_eq!(values[2].tx_id, "22".repeat(32));

    assert_eq!(
        super::super::selection::checked_total(&values)
            .expect("checked total")
            .0,
        40
    );
    assert!(super::super::selection::checked_total(&[
        super::utxo(0x44, 0, u64::MAX),
        super::utxo(0x55, 1, 1),
    ])
    .is_err());

    let selected = super::super::selection::select_explicit(values.clone(), &[])
        .expect("empty explicit selection is representable");
    assert!(selected.is_empty());
    assert!(super::super::selection::select_explicit(values, &[99]).is_err());
}

#[test]
fn storage_mass_boundaries_cover_relaxed_arithmetic_zero_and_overflow_paths() {
    assert_eq!(storage_mass_estimate(&[], &[]).expect("empty mass"), 0);
    assert_eq!(
        storage_mass_estimate(&[(0, 1)], &[(0, 1)]).expect("zero amounts"),
        0
    );

    let arithmetic = storage_mass_estimate(
        &[(100_000_000, 2), (50_000_000, 2)],
        &[(25_000_000, 2), (25_000_000, 2)],
    )
    .expect("arithmetic branch");
    assert_eq!(arithmetic, 213_336);

    assert!(storage_mass_estimate(&[(1, u64::MAX), (1, 1)], &[]).is_err());
    assert!(storage_mass_estimate(&[], &[(1, u64::MAX), (1, 1)]).is_err());
    assert!(storage_mass_estimate(&[(1, u64::MAX)], &[(1, 1)]).is_err());
}
