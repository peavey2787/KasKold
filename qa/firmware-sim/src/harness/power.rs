//! Gives every journey a whole device of its own.
//!
//! The firmware owns process-wide statics as it owns the chip, so each power-on is
//! one process. What survives losing power is carried to the next power-on in a file.
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use esp_hal::board::nonvolatile;

const PHASE: &str = "KASKOLD_SIM_PHASE";
const BOARD_FILE: &str = "KASKOLD_SIM_BOARD_FILE";

/// Runs one journey: each phase is one power-on of the same device, in order.
pub(crate) fn across_power_cycles(test_path: &str, phases: &[fn()]) {
    match env::var(PHASE) {
        Ok(phase) => power_on_once(phases[phase.parse::<usize>().expect("phase index")]),
        Err(_) => run_every_power_on(test_path, phases.len()),
    }
}

fn power_on_once(phase: fn()) {
    let board = PathBuf::from(env::var(BOARD_FILE).expect("board file"));
    if board.exists() {
        nonvolatile::load(&board).expect("load board");
    }
    phase();
    nonvolatile::save(&board).expect("save board");
}

fn run_every_power_on(test_path: &str, phases: usize) {
    let test = test_path.split_once("::").map_or(test_path, |(_, path)| path);
    let board = env::temp_dir().join(format!("kaskold-sim-{}-{}.board", std::process::id(), test.replace("::", "-")));
    for phase in 0..phases {
        run_power_on(test, phase, &board);
    }
    let _ = std::fs::remove_file(&board);
}

fn run_power_on(test: &str, phase: usize, board: &Path) {
    let output = Command::new(env::current_exe().expect("test binary"))
        .args(["--exact", test, "--nocapture", "--include-ignored"])
        .env(PHASE, phase.to_string())
        .env(BOARD_FILE, board)
        .output()
        .expect("power on");
    assert!(
        output.status.success(),
        "power-on {phase} of {test} failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
