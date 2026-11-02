//! Gives every journey a whole device of its own.
//!
//! The firmware owns process-wide statics as it owns the chip, so each power-on is
//! one process. What survives losing power is carried to the next power-on in a file.
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use esp_hal::board::{mmio, nonvolatile};

const PHASE: &str = "KASKOLD_SIM_PHASE";
const BOARD_FILE: &str = "KASKOLD_SIM_BOARD_FILE";

static BOARD_DISCARDED: AtomicBool = AtomicBool::new(false);

/// Runs one journey: each phase is one power-on of the same device, in order.
pub(crate) fn across_power_cycles(test_path: &str, phases: &[fn()]) {
    match env::var(PHASE) {
        Ok(phase) => {
            let phase = phase.parse::<usize>().expect("phase index");
            mmio::start_noise_for_power_on(phase as u32);
            power_on_once(phases[phase]);
        }
        Err(_) => run_every_power_on(test_path, phases.len()),
    }
}

/// Writes something down between power-ons, the way a person keeps a paper note.
pub(crate) fn note(name: &str, value: &str) {
    std::fs::write(note_path(name), value).expect("note");
}

pub(crate) fn recall(name: &str) -> String {
    std::fs::read_to_string(note_path(name)).expect("recall")
}

fn note_path(name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}.{name}",
        env::var(BOARD_FILE).expect("board file")
    ))
}

/// Ends this power-on by throwing the device away: the next one is factory-new.
pub(crate) fn discard_board() {
    BOARD_DISCARDED.store(true, Ordering::SeqCst);
}

fn power_on_once(phase: fn()) {
    let board = PathBuf::from(env::var(BOARD_FILE).expect("board file"));
    if board.exists() {
        nonvolatile::load(&board).expect("load board");
    }
    phase();
    if BOARD_DISCARDED.load(Ordering::SeqCst) {
        let _ = std::fs::remove_file(&board);
    } else {
        nonvolatile::save(&board).expect("save board");
    }
}

fn run_every_power_on(test_path: &str, phases: usize) {
    let test = test_path
        .split_once("::")
        .map_or(test_path, |(_, path)| path);
    let board = env::temp_dir().join(format!(
        "kaskold-sim-{}-{}.board",
        std::process::id(),
        test.replace("::", "-")
    ));
    for phase in 0..phases {
        run_power_on(test, phase, &board);
    }
    remove_board_and_notes(&board);
}

fn remove_board_and_notes(board: &Path) {
    let stem = board
        .file_name()
        .expect("board name")
        .to_string_lossy()
        .into_owned();
    for entry in std::fs::read_dir(board.parent().expect("board folder"))
        .expect("board folder")
        .flatten()
    {
        if entry.file_name().to_string_lossy().starts_with(&stem) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
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
