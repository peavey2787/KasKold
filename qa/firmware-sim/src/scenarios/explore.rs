//! Walks the device by hand: the tool journeys are written with.
use crate::harness::{app_state, Device};
use kaskold_hardware_core::input::keyboard::{KeyAction, KeyboardMode};

/// Replays `KASKOLD_SIM_SCRIPT` and saves a picture after every step, for writing journeys.
#[test]
#[ignore = "authoring tool, run by hand"]
fn explore() {
    let kept_board = std::env::var("KASKOLD_SIM_KEEP_BOARD")
        .ok()
        .map(std::path::PathBuf::from);
    if let Some(board) = kept_board.as_ref().filter(|board| board.exists()) {
        esp_hal::board::nonvolatile::load(board).expect("load board");
    }
    if std::env::var("KASKOLD_SIM_CARD").is_ok() {
        esp_hal::board::sd::set_present(true);
    }
    let device = Device::boot();
    let script = std::env::var("KASKOLD_SIM_SCRIPT").unwrap_or_default();
    let out = std::env::var("KASKOLD_SIM_OUT").unwrap_or_else(|_| ".".to_owned());
    device.screen().save(&format!("{out}/start.png"));
    for (index, step) in script
        .split(';')
        .filter(|step| !step.is_empty())
        .enumerate()
    {
        match step.split_once(':') {
            Some(("at", place)) => {
                let (x, y) = place.split_once(',').expect("x,y");
                device.tap_at(x.parse().expect("x"), y.parse().expect("y"));
            }
            Some(("idle", polls)) => device.idle(polls.parse().expect("polls")),
            Some(("wait", text)) => device.wait_for_text(text),
            Some(("word", word)) => device.type_letters(word),
            Some(("words", words)) => words.split(' ').for_each(|word| {
                device.type_letters(word);
                device.press_key(KeyAction::Ok, KeyboardMode::Alpha, 0);
            }),
            Some(("pin", pin)) => device.enter_pin(pin),
            Some(("text", text)) => device.type_text(text),
            Some(("ok", "alpha")) => device.press_key(KeyAction::Ok, KeyboardMode::Alpha, 0),
            Some(("ok", "full")) => device.press_key(KeyAction::Ok, KeyboardMode::Full, 0),
            _ => device.tap(step),
        }
        device.settle();
        device.screen().save(&format!("{out}/{index:02}.png"));
        println!(
            "{index:02} {step} -> {:?}",
            app_state().navigation.app.state
        );
    }
    if let Some(board) = &kept_board {
        esp_hal::board::nonvolatile::save(board).expect("save board");
    }
}
