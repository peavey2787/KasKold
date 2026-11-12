//! Drives the running firmware the way a person holding the device does.
#[cfg(feature = "production")]
mod app_image;
pub(crate) mod card;
mod keyboard;
mod power;
mod qr;
mod screen;

use crate::runtime::input::AppState;
use esp_hal::board::{
    clock, control, efuse, halt::SoftwareReset, lcd, mapped_flash, rtc, sd, touch,
};
use kaskold_hardware_core::advanced_policy::parse_utc_yyyymmddhhmm;
use kaskold_hardware_core::time::bm8563::encode_bm8563;

#[cfg(feature = "production")]
pub(crate) use app_image::ImageFault;
pub(crate) use power::{across_power_cycles, discard_board, note, recall};
pub(crate) use qr::{hold_up_frames, hold_up_qr, lower_qr, read_qr_on_screen};
pub(crate) use screen::Screen;

const TEST_SIGNING_KEY: &[u8; 32] =
    include_bytes!("../../../tools/build/firmware/dev_test_signing_key.bin");
#[cfg(feature = "production")]
const SIMULATED_RELEASE_SIGNING_KEY: &[u8; 32] = &[0x52; 32];
const FIRMWARE_STACK_BYTES: usize = 8 * 1024 * 1024;
const SETTLE_POLLS: u64 = 8;
const MAX_SETTLE_ROUNDS: usize = 20_000;
const FIRMWARE_PANIC_EXIT: i32 = 101;
const WAIT_POLLS: u64 = 32;
const WAIT_SLICE: std::time::Duration = std::time::Duration::from_millis(10);
/// How long the screen may stay unchanged while the device is still expected to get somewhere.
const WAIT_LIMIT: std::time::Duration = std::time::Duration::from_secs(45);
/// How long any one wait may last in all, however busy the host is.
const WORK_LIMIT: std::time::Duration = std::time::Duration::from_secs(600);
const NO_REVIEW_ROUNDS: usize = 200;
const HOLD_SLICE_MILLIS: u128 = 250;

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

    /// Moves the hardware clock, as time passing or someone opening the case would.
    pub(crate) fn set_clock(&self, utc: &str) {
        let time = parse_utc_yyyymmddhhmm(utc.as_bytes()).expect("a UTC time");
        let encoded = encode_bm8563(time).expect("a time the clock can hold");
        rtc::restore(&[&encoded.time[1..], &encoded.date[1..]].concat());
    }

    /// Runs the real `firmware_main` with a memory card in the slot.
    pub(crate) fn boot_with_card() -> Self {
        sd::set_present(true);
        Self::boot()
    }

    /// Powers on firmware changed after it was signed. A refused image never reads
    /// the touch panel again, so it runs free and is only watched.
    pub(crate) fn boot_tampered() -> Self {
        control::free_run();
        install_signed_firmware();
        mapped_flash::tamper();
        start_firmware();
        Self
    }

    /// Powers on firmware signed by a key the device does not trust.
    pub(crate) fn boot_with_bad_signature() -> Self {
        control::free_run();
        install_signed_firmware();
        mapped_flash::set_signature([0x5A; 64]);
        start_firmware();
        Self
    }

    /// Powers on firmware carrying the repository's public development signature.
    #[cfg(feature = "production")]
    pub(crate) fn boot_with_development_signature() -> Self {
        control::free_run();
        install_signed_firmware();
        mapped_flash::set_signature(
            firmware_tools::sign_test_firmware_hash(TEST_SIGNING_KEY, &mapped_flash::image_hash())
                .expect("test signature"),
        );
        start_firmware();
        Self
    }

    /// Powers on a retail unit that must refuse to boot without the named fuses.
    #[cfg(feature = "production")]
    pub(crate) fn boot_refused_without(unburned: &[&str]) -> Self {
        control::free_run();
        install_signed_firmware();
        efuse::burn_retail_fuses(unburned);
        start_firmware();
        Self
    }

    /// Powers on a chip nobody locked yet, holding a damaged application image.
    #[cfg(feature = "production")]
    pub(crate) fn boot_unlocked_with_damaged_image(fault: ImageFault) -> Self {
        app_image::install_with(fault);
        Self::boot_refused_without(&efuse::RETAIL_FUSES)
    }

    /// Powers on a chip nobody locked yet: it attests the application image it finds in flash.
    #[cfg(feature = "production")]
    pub(crate) fn boot_unlocked() -> Self {
        app_image::install();
        Self::boot_without(&efuse::RETAIL_FUSES)
    }

    /// Powers on a retail unit that still boots without the named fuses.
    #[cfg(feature = "production")]
    pub(crate) fn boot_without(unburned: &[&str]) -> Self {
        install_signed_firmware();
        efuse::burn_retail_fuses(unburned);
        start_firmware();
        control::run_polls(1);
        let device = Self;
        device.settle();
        device
    }

    /// Asserts the device stopped on a refusal naming `reason` and never unlocks.
    pub(crate) fn expect_refusal(&self, reason: &str) {
        if !wait_until(|| self.screen().find(reason).is_some(), || ()) {
            let screen = self.screen();
            panic!(
                "no refusal naming {reason:?}; see {}",
                screen.save_for_failure(reason)
            );
        }
        self.expect_text("NO BOOT");
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
            // The live camera view never holds still, and nothing else draws over it.
            if kaskold_hardware_core::screen_flow::is_scan_state(app_state().navigation.app.state) {
                return;
            }
            if lcd::pixel_writes() == before {
                return;
            }
        }
        panic!(
            "screen never settled on {:?}",
            app_state().navigation.app.state
        );
    }

    /// Presses and lifts a finger at a display coordinate.
    pub(crate) fn tap_at(&self, x: u16, y: u16) {
        touch::press(x, y);
        control::run_polls(2);
        touch::release();
        control::run_polls(2);
        self.settle();
    }

    /// Moves a finger through a point at a time without lifting it, while `keep_going` holds.
    pub(crate) fn draw(
        &self,
        points: impl Iterator<Item = (u16, u16)>,
        keep_going: impl Fn() -> bool,
    ) {
        for (x, y) in points.take_while(|_| keep_going()) {
            touch::press(x, y);
            control::run_polls(1);
        }
        touch::release();
        control::run_polls(2);
        self.settle();
    }

    /// Keeps a finger on a label for as long as a destructive action asks.
    pub(crate) fn hold(&self, label: &str, time: std::time::Duration) {
        let screen = self.screen();
        let Some((x, y)) = screen.find(label) else {
            panic!(
                "{label:?} is not on screen; see {}",
                screen.save_for_failure(label)
            );
        };
        touch::press(x, y);
        control::run_polls(2);
        for _ in 0..time.as_millis() / HOLD_SLICE_MILLIS {
            clock::advance_micros(u64::try_from(HOLD_SLICE_MILLIS * 1_000).unwrap());
            control::run_polls(2);
        }
        touch::release();
        control::run_polls(2);
        self.settle();
    }

    /// Taps a label a person can read, and fails when it is not on the screen.
    pub(crate) fn tap(&self, label: &str) {
        let screen = self.screen();
        let Some((x, y)) = screen.find(label) else {
            panic!(
                "{label:?} is not on screen; see {}",
                screen.save_for_failure(label)
            );
        };
        self.tap_at(x, y);
    }

    pub(crate) fn screen(&self) -> Screen {
        Screen::capture()
    }

    /// Asserts the firmware held `text` on screen long enough for a person to read it.
    pub(crate) fn expect_was_shown(&self, text: &str) {
        let held = Screen::held_for_reading();
        if !held.iter().any(|screen| screen.find(text).is_some()) {
            let saved = held
                .last()
                .map_or_else(String::new, |screen| screen.save_for_failure(text));
            panic!("{text:?} was never held on screen; last held screen: {saved}");
        }
    }

    /// Asserts a person can read `text` right now.
    pub(crate) fn expect_text(&self, text: &str) {
        let screen = self.screen();
        if screen.find(text).is_none() {
            panic!(
                "{text:?} is not on screen; see {}",
                screen.save_for_failure(text)
            );
        }
    }

    /// Asserts a person cannot read `text` right now.
    pub(crate) fn expect_absent(&self, text: &str) {
        let screen = self.screen();
        if screen.find(text).is_some() {
            panic!(
                "{text:?} is on screen; see {}",
                screen.save_for_failure(text)
            );
        }
    }

    /// Waits for work the screen is showing progress on, until `text` appears.
    pub(crate) fn wait_for_text(&self, text: &str) {
        let appeared = wait_until(
            || self.screen().find(text).is_some(),
            || control::run_polls(WAIT_POLLS),
        );
        if !appeared {
            let screen = self.screen();
            panic!(
                "{text:?} never appeared; see {}",
                screen.save_for_failure(text)
            );
        }
    }

    /// Lets wall-clock time pass on the device with nobody touching it.
    pub(crate) fn let_time_pass(&self, time: std::time::Duration) {
        clock::advance_micros(u64::try_from(time.as_micros()).unwrap());
        control::run_polls(WAIT_POLLS);
    }

    /// Waits for the firmware to restart itself, or for `text` to appear without one.
    pub(crate) fn wait_until_stopped_or_text(&self, text: &str) {
        let done = wait_until(
            || self.has_reset() || self.screen().find(text).is_some(),
            || control::run_polls(WAIT_POLLS),
        );
        if !done {
            let screen = self.screen();
            panic!(
                "neither a restart nor {text:?}; see {}",
                screen.save_for_failure(text)
            );
        }
    }

    /// Waits for a QR code on the panel and reads it, as another device's camera would.
    pub(crate) fn wait_for_qr(&self) -> Vec<u8> {
        let mut payload = None;
        let appeared = wait_until(
            || {
                payload = read_qr_on_screen();
                payload.is_some()
            },
            || control::run_polls(WAIT_POLLS),
        );
        match (appeared, payload) {
            (true, Some(payload)) => payload,
            _ => panic!(
                "no QR code appeared; see {}",
                self.screen().save_for_failure("qr")
            ),
        }
    }

    /// Lets the camera run for a while and asserts a transaction was never put up for review.
    pub(crate) fn expect_no_transaction_review(&self) {
        for _ in 0..NO_REVIEW_ROUNDS {
            control::run_polls(WAIT_POLLS);
            std::thread::sleep(WAIT_SLICE);
            let state = app_state().navigation.app.state;
            assert!(
                !matches!(state, AppState::ConfirmTx | AppState::ReviewTx { .. }),
                "reached {state:?}"
            );
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

/// Keeps nudging the device until `done`, and reports false once it has stalled or taken too long.
///
/// The device's work runs on the host's CPU, so a loaded host only slows it down: a
/// wait fails when the screen stops changing, not when a fixed time has passed.
fn wait_until(mut done: impl FnMut() -> bool, nudge: impl Fn()) -> bool {
    let started = std::time::Instant::now();
    let mut progressed = started;
    let mut last = Screen::capture();
    while progressed.elapsed() < WAIT_LIMIT && started.elapsed() < WORK_LIMIT {
        if done() {
            return true;
        }
        nudge();
        std::thread::sleep(WAIT_SLICE);
        let now = Screen::capture();
        if now != last {
            progressed = std::time::Instant::now();
            last = now;
        }
    }
    false
}

fn power_on() {
    install_signed_firmware();
    start_firmware();
}

fn install_signed_firmware() {
    stop_the_run_when_firmware_panics();
    mapped_flash::install();
    mapped_flash::set_signature(sign_for_this_build(&mapped_flash::image_hash()));
    if cfg!(feature = "production") {
        efuse::burn_retail_fuses(&[]);
    }
}

#[cfg(feature = "production")]
fn sign_for_this_build(hash: &[u8; 32]) -> [u8; 64] {
    firmware_tools::sign_firmware_hash(SIMULATED_RELEASE_SIGNING_KEY, hash)
        .expect("release signature")
}

#[cfg(not(feature = "production"))]
fn sign_for_this_build(hash: &[u8; 32]) -> [u8; 64] {
    firmware_tools::sign_test_firmware_hash(TEST_SIGNING_KEY, hash).expect("test signature")
}

fn start_firmware() {
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
        if std::thread::current()
            .name()
            .is_some_and(|name| name.starts_with("firmware-core"))
        {
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
