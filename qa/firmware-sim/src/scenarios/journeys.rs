//! Journeys a person walks through by reading the screen and tapping what it shows.
use crate::harness::{across_power_cycles, app_state, Device};
use crate::runtime::input::AppState;
use kaskold_hardware_core::input::keyboard::{KeyAction, KeyboardMode};

/// A journey: each named phase is one power-on of the same device, in order.
macro_rules! journey {
    ($name:ident: $($phase:path),+ $(,)?) => {
        #[test]
        fn $name() {
            across_power_cycles(concat!(module_path!(), "::", stringify!($name)), &[$($phase),+]);
        }
    };
}

journey!(a_new_device_boots_to_the_welcome_screen: welcome_screen);

fn welcome_screen() {
    let device = Device::boot();
    device.expect_text("WELCOME");
    device.expect_text("Create Wallet");
    device.expect_text("Restore Wallet");
    assert_eq!(app_state().navigation.app.state, AppState::StorageModeChoice);
}

/// Replays `KASKOLD_SIM_SCRIPT` and saves a picture after every step, for writing journeys.
#[test]
#[ignore = "authoring tool, run by hand"]
fn explore() {
    let device = Device::boot();
    let script = std::env::var("KASKOLD_SIM_SCRIPT").unwrap_or_default();
    let out = std::env::var("KASKOLD_SIM_OUT").unwrap_or_else(|_| ".".to_owned());
    device.screen().save(&format!("{out}/start.png"));
    for (index, step) in script.split(';').filter(|step| !step.is_empty()).enumerate() {
        match step.split_once(':') {
            Some(("at", place)) => {
                let (x, y) = place.split_once(',').expect("x,y");
                device.tap_at(x.parse().expect("x"), y.parse().expect("y"));
            }
            Some(("idle", polls)) => device.idle(polls.parse().expect("polls")),
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
        println!("{index:02} {step} -> {:?}", app_state().navigation.app.state);
    }
}
