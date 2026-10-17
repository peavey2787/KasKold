#[test]
fn covenant_fee_exports_price_through_portal() {
    use super::{private_swap_max_fee_sompi, thread_maximum_withdraw, thread_withdraw_fee};

    assert_eq!(private_swap_max_fee_sompi(), 500_000_000);
    assert_eq!(
        thread_withdraw_fee(1_000_000_000, 100_000_000, 300_000, 100).ok(),
        Some(1_044_500)
    );
    let best = thread_maximum_withdraw(1_000_000_000, u64::MAX, 300_000, 1, 10_000_000);
    assert!(best > 0 && best < 1_000_000_000);
}

#[test]
fn covenant_spend_path_reads_the_selector_through_portal() {
    use super::covenant_spend_path;

    assert_eq!(covenant_spend_path("02aaaa510151", "51"), "owner");
    assert_eq!(covenant_spend_path("02aaaa000151", "51"), "heir");
    assert_eq!(covenant_spend_path("02aaaa520151", "51"), "unknown");
    assert_eq!(covenant_spend_path("zz", "51"), "unknown");
}
