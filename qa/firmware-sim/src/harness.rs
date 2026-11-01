//! Drives the running firmware the way a person holding the device does.
mod keyboard;
mod power;
mod screen;

use esp_hal::board::{control, halt::SoftwareReset, lcd, mapped_flash, touch};

pub(crate) use power::across_power_cycles;
pub(crate) use screen::Screen;

const TEST_SIGNING_KEY: &[u8; 32] = include_bytes!("../../../tools/build/firmware/dev_test_signing_key.bin");
const FIRMWARE_STACK_BYTES: usize = 8 * 1024 * 1024;
const SETTLE_POLLS: u64 = 8;
const MAX_SETTLE_ROUNDS: usize = 20_000;
const FIRMWARE_PANIC_EXIT: i32 = 101;

pub(crate) struct Device;

impl Device {
    /// Runs the real `firmware_main` until its first screen is drawn.
    pub(crate) fn boot() -> Self {
        power_on();
        control::run_polls(1);
        let device = Self;
        device.settle();
        device
    }

    /// Powers on an image that drives itself and never waits for a person.
    pub(crate) fn boot_self_driving() {
        control::free_run();
        power_on();
    }

    /// Runs until a few event-loop passes in a row leave the screen untouched.
    pub(crate) fn settle(&self) {
        for _ in 0..MAX_SETTLE_ROUNDS {
            if control::is_stopped() {
                return;
            }
            let before = lcd::pixel_writes();
            control::run_polls(SETTLE_POLLS);
            if lcd::pixel_writes() == before {
                return;
            }
        }
        panic!("screen never settled");
    }

    /// Presses and lifts a finger at a display coordinate.
    pub(crate) fn tap_at(&self, x: u16, y: u16) {
        touch::press(x, y);
        control::run_polls(2);
        touch::release();
        control::run_polls(2);
        self.settle();
    }

    /// Taps a label a person can read, and fails when it is not on the screen.
    pub(crate) fn tap(&self, label: &str) {
        let screen = self.screen();
        let Some((x, y)) = screen.find(label) else {
            panic!("{label:?} is not on screen; see {}", screen.save_for_failure(label));
        };
        self.tap_at(x, y);
    }

    pub(crate) fn screen(&self) -> Screen {
        Screen::capture()
    }

    /// Asserts a person can read `text` right now.
    pub(crate) fn expect_text(&self, text: &str) {
        let screen = self.screen();
        if screen.find(text).is_none() {
            panic!("{text:?} is not on screen; see {}", screen.save_for_failure(text));
        }
    }

    /// True once the firmware has asked the chip to restart.
    pub(crate) fn has_reset(&self) -> bool {
        control::is_stopped()
    }

    /// Runs `polls` passes with no input.
    pub(crate) fn idle(&self, polls: u64) {
        control::run_polls(polls);
    }
}

fn power_on() {
    stop_the_run_when_firmware_panics();
    mapped_flash::install();
    mapped_flash::set_signature(
        firmware_tools::sign_test_firmware_hash(TEST_SIGNING_KEY, &mapped_flash::image_hash())
            .expect("test signature"),
    );
    std::thread::Builder::new()
        .name("firmware-core0".into())
        .stack_size(FIRMWARE_STACK_BYTES)
        .spawn(|| crate::firmware_main())
        .expect("firmware thread");
}

/// A panic on either core would stop the chip, so it stops the whole test process.
/// A requested reset is how the firmware restarts, and is not a failure.
fn stop_the_run_when_firmware_panics() {
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if info.payload().is::<SoftwareReset>() {
            return;
        }
        report(info);
        if std::thread::current().name().is_some_and(|name| name.starts_with("firmware-core")) {
            std::process::exit(FIRMWARE_PANIC_EXIT);
        }
    }));
}

/// The state of a stopped device. The firmware thread is parked inside its touch
/// read whenever a test holds control, so this read cannot race it.
pub(crate) fn app_state() -> &'static crate::runtime::data::AppData {
    let root = crate::runtime::secret_state::peek_for_harness();
    // SAFETY: `initialize` stored a live static, and the writer is parked as above.
    unsafe { &*root }
}
