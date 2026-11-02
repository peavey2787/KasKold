//! Runs the firmware's own connected workflow suite on the virtual board.
use std::time::{Duration, Instant};

use crate::harness::Device;

const PASS: &str = "KASKOLD_WORKFLOW_TESTS: PASS ALL";
const FAIL: &str = "KASKOLD_WORKFLOW_TESTS: FAIL";
const DEADLINE: Duration = Duration::from_secs(600);

#[test]
fn every_connected_workflow_passes() {
    esp_hal::board::sd::set_present(cfg!(feature = "workflow-hil-auto"));
    Device::boot_self_driving();
    let started = Instant::now();
    loop {
        let serial = esp_println::captured();
        if serial.contains(PASS) {
            return;
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
