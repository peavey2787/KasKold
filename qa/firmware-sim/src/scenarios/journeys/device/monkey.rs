//! A person poking at the screen with no plan: whatever is tapped, in whatever order,
//! the firmware must never lose its way.
use crate::harness::{app_state, card, Device, Screen};
use crate::scenarios::journeys::known::{self, PIN};
use crate::scenarios::journeys::walk;
use std::time::Duration;

const TAPS_PER_WANDER: usize = 600;
const BACK_ZONE_CENTRE: (u16, u16) = (20, 20);
const BACK_ONE_TAP_IN: u64 = 4;
const PANEL_WIDTH: u64 = 320;
const PANEL_HEIGHT: u64 = 240;
const NAVIGATION_ERROR: &str = "Navigation error";
const BUSY_TITLE: &str = "LOADING";
const BUSY_WAIT: Duration = Duration::from_millis(50);
const TRAIL_KEPT: usize = 12;
const ERROR_TITLE: &str = "ERROR";
const BACK_STAYS_ON: [&str; 2] = ["MainMenu", "StorageModeChoice"];

journey!(tapping_anywhere_on_a_new_device_never_loses_the_way: wander_a_new_device);
journey!(
    tapping_anywhere_with_a_wallet_never_loses_the_way:
    walk::restore_the_known_wallet,
    wander_with_a_wallet,
);

/// A small generator so a failure replays exactly.
struct Taps(u64);

impl Taps {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn next_point(&mut self) -> (u16, u16) {
        if self.next().is_multiple_of(BACK_ONE_TAP_IN) {
            return BACK_ZONE_CENTRE;
        }
        (
            (self.next() % PANEL_WIDTH) as u16,
            (self.next() % PANEL_HEIGHT) as u16,
        )
    }
}

/// Where Back has nowhere to go, or a screen spends it on its own step.
fn back_may_stay_on(state: &str) -> bool {
    BACK_STAYS_ON
        .iter()
        .any(|allowed| state.starts_with(allowed))
}

/// A tap lands on nothing while the device is busy, as it would under a finger.
fn wait_out_the_work(device: &Device) {
    while Screen::capture().find(BUSY_TITLE).is_some() {
        device.let_time_pass(BUSY_WAIT);
    }
}

fn wander(device: &Device, seed: u64) {
    let mut taps = Taps(seed);
    let mut trail = Vec::new();
    for _ in 0..TAPS_PER_WANDER {
        if device.has_reset() {
            break;
        }
        wait_out_the_work(device);
        let (x, y) = taps.next_point();
        let state = format!("{:?}", app_state().navigation.app.state);
        trail.push(format!("{state} @({x},{y})"));
        if trail.len() > TRAIL_KEPT {
            trail.remove(0);
        }
        let error_waiting = Screen::capture().find(ERROR_TITLE).is_some();
        device.tap_at(x, y);
        if Screen::capture().find(NAVIGATION_ERROR).is_some() {
            panic!("lost the way tapping ({x},{y}) on {state}; came through {trail:?}");
        }
        let after = format!("{:?}", app_state().navigation.app.state);
        if (x, y) == BACK_ZONE_CENTRE
            && after == state
            && !error_waiting
            && !back_may_stay_on(&state)
        {
            panic!("Back left {state} in place; came through {trail:?}");
        }
    }
}

fn wander_a_new_device() {
    let device = Device::boot_with_card();
    wander(&device, 0x9E37_79B9_7F4A_7C15);
}

fn wander_with_a_wallet() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::format_card(&device);
    card::put_file(b"WALLET  KSP", &known::unsigned_transaction());
    wander(&device, 0xD1B5_4A32_D192_ED03);
}
