use super::*;

// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(any(test, feature = "verbose-boot"))]
/// Test: word input prefix matching.
pub fn test_word_input_matching() -> bool {
    let mut input = WordInput::new();
    input.push_char(b'a');
    input.push_char(b'b');
    // "ab" should match: abandon, ability, able, about, above, absent, absorb, abstract, absurd, abuse
    let has_matches = input.match_count > 5;

    input.push_char(b'o');
    input.push_char(b'u');
    input.push_char(b't');
    // "about" should match exactly one
    let exact = input.matched_index == Some(3); // "about" = index 3

    has_matches && exact
}

#[cfg(any(test, feature = "verbose-boot"))]
/// Test: Touch Seed transcript accepts movement only and cannot finalize early.
pub fn test_touch_entropy_collector() -> bool {
    let mut collector = TouchEntropyCollector::new();
    if collector.finish().is_some() || collector.count() != 0 {
        return false;
    }
    if !collector.record(1, 100, 100) {
        return false;
    }
    if collector.record(2, 100, 100) || collector.count() != 1 {
        return false;
    }
    let target = collector.target();
    for sample in 1..target {
        let x = 100u16.wrapping_add(u16::try_from(sample & 0x7f).unwrap());
        let y = 100u16.wrapping_add(u16::try_from((sample >> 7) & 0x7f).unwrap());
        if !collector.record((u32::try_from(sample).unwrap()).wrapping_mul(977), x, y) {
            return false;
        }
    }
    if collector.count() != target {
        return false;
    }
    let Some(digest) = collector.finish() else {
        return false;
    };
    digest.iter().any(|byte| *byte != 0) && collector.count() == 0
}

#[cfg(any(test, feature = "verbose-boot"))]
/// Run all mnemonic-domain tests.
pub fn run_mnemonic_tests() -> (u32, u32) {
    let mut passed = 0u32;
    let total = 2u32;

    if test_word_input_matching() {
        passed += 1;
    }
    if test_touch_entropy_collector() {
        passed += 1;
    }

    (passed, total)
}

#[test]
fn mnemonic_vectors_pass() {
    let (passed, total) = run_mnemonic_tests();
    assert_eq!(passed, total);
}
