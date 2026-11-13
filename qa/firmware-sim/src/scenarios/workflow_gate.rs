//! Runs the firmware's own connected workflow suite on the virtual board.
use std::time::{Duration, Instant};

use crate::harness::Device;

const PASS: &str = "KASKOLD_WORKFLOW_TESTS: PASS ALL";
const FAIL: &str = "KASKOLD_WORKFLOW_TESTS: FAIL";
const DEADLINE: Duration = Duration::from_secs(600);
const PHYSICAL_TRANCHE: &str = "KASKOLD_WORKFLOW_HIL: PHYSICAL SD TRANCHE ENABLED";

#[test]
fn every_connected_workflow_passes() {
    let hil = cfg!(feature = "workflow-hil-auto");
    let serial = run_suite(hil);
    assert_eq!(serial.contains(PHYSICAL_TRANCHE), hil, "{}", tail(&serial));
}

/// The HIL image booted without a card skips only its physical SD tranche and passes the rest.
#[cfg(feature = "workflow-hil-auto")]
#[test]
fn hil_without_a_card_skips_only_the_physical_sd_tranche() {
    let serial = run_suite(false);
    assert!(
        serial.contains("KASKOLD_WORKFLOW_HIL: SD MEDIA ABSENT - PHYSICAL SD TRANCHE SKIPPED"),
        "{}",
        tail(&serial)
    );
    assert!(!serial.contains(PHYSICAL_TRANCHE), "{}", tail(&serial));
}

/// Boots the self-driving suite with or without a card and returns its passing serial log.
fn run_suite(card_present: bool) -> String {
    esp_hal::board::sd::set_present(card_present);
    Device::boot_self_driving();
    let started = Instant::now();
    loop {
        let serial = esp_println::captured();
        if serial.contains(PASS) {
            return serial;
        }
        assert!(
            !serial.contains(FAIL) && started.elapsed() < DEADLINE,
            "{}",
            tail(&serial)
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn tail(serial: &str) -> String {
    let lines: Vec<&str> = serial.lines().collect();
    let failed = lines.iter().copied().filter(|line| line.contains("FAIL"));
    let last = lines[lines.len().saturating_sub(40)..].iter().copied();
    failed.chain(last).collect::<Vec<_>>().join("\n")
}
