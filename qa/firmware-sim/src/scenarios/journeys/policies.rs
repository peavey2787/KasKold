//! Permanent signing policies: once set, the hardware clock decides when this wallet may sign.
use super::known::{self, PIN};
use super::walk::{self, restore_the_known_wallet};
use crate::harness::{app_state, hold_up_qr, lower_qr, Device};
use esp_hal::board::rtc;

const CLOCK_ROW: (u16, u16) = (160, 207);
const TIME_LOCK_ROW: (u16, u16) = (160, 93);
const WEEKLY_ROW: (u16, u16) = (160, 131);
/// 1 January 2026 is a Thursday.
const THURSDAY: &str = "202601010000";
const LOCKED_UNTIL: &str = "202601020000";
const AFTER_THE_LOCK: &str = "202601030000";
const BEFORE_THE_LOCK_WAS_SET: &str = "202512310000";
const EARLIER_MONDAY_IN_WINDOW: &str = "202512290930";
const MONDAY_IN_WINDOW: &str = "202601050930";
const MONDAY_AFTER_WINDOW: &str = "202601051800";
const MONDAY_WINDOW: &str = "MON 09:00-17:00";
const REFUSED: &str = "Signing policy rejected";

journey!(
    a_time_lock_holds_through_reboots_until_its_date:
    restore_the_known_wallet,
    lock_until_tomorrow,
    still_locked_after_reboot,
    an_expired_lock_stays_open,
);
journey!(
    a_clock_that_lost_its_battery_cannot_open_a_time_lock:
    restore_the_known_wallet,
    lock_until_tomorrow,
    dead_clock_battery_is_refused,
);
journey!(
    weekly_windows_only_sign_inside_the_window:
    restore_the_known_wallet,
    allow_only_monday_hours,
    sign_only_inside_the_window,
    an_earlier_window_cannot_be_replayed,
);

fn open_security(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    device.tap("Settings");
    device.tap("Security");
    device.tap_at(CLOCK_ROW.0, CLOCK_ROW.1);
    device.enter_on_number_row(THURSDAY);
    device.wait_for_text("SECURITY");
}

fn scan_and_confirm(device: &Device) {
    device.tap("Scan QR");
    hold_up_qr(&known::unsigned_transaction());
    device.wait_for_text("CONFIRM SEND");
    lower_qr();
    device.tap("CONFIRM");
}

fn expect_refused(device: &Device) {
    scan_and_confirm(device);
    device.wait_for_text(REFUSED);
    assert_eq!(app_state().qr.outgoing.length, 0);
    device.tap("OK");
    device.wait_for_text("CONFIRM SEND");
    walk::go_home(device);
}

fn expect_signed(device: &Device) {
    scan_and_confirm(device);
    device.wait_for_text("QR DISPLAY MODE");
    assert_ne!(app_state().qr.outgoing.length, 0);
    walk::go_home(device);
}

fn lock_until_tomorrow() {
    let device = Device::boot();
    open_security(&device);
    device.tap_at(TIME_LOCK_ROW.0, TIME_LOCK_ROW.1);
    walk::accept_warning(&device);
    device.enter_on_number_row(LOCKED_UNTIL);
    walk::accept_warning(&device);
    device.wait_for_text("SECURITY");
    walk::go_home(&device);
    expect_refused(&device);
}

fn still_locked_after_reboot() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    expect_refused(&device);
    device.set_clock(BEFORE_THE_LOCK_WAS_SET);
    expect_refused(&device);
    device.set_clock(AFTER_THE_LOCK);
    expect_signed(&device);
}

/// Once its date has passed on this device, a lock is over and the clock no longer matters.
fn an_expired_lock_stays_open() {
    let device = Device::boot();
    device.set_clock(BEFORE_THE_LOCK_WAS_SET);
    walk::unlock_with_pin(&device, PIN);
    expect_signed(&device);
}

fn dead_clock_battery_is_refused() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.set_clock(AFTER_THE_LOCK);
    rtc::lose_battery();
    expect_refused(&device);
}

fn allow_only_monday_hours() {
    let device = Device::boot();
    open_security(&device);
    device.tap_at(WEEKLY_ROW.0, WEEKLY_ROW.1);
    walk::accept_warning(&device);
    device.type_text(MONDAY_WINDOW);
    walk::press_full_ok(&device);
    walk::accept_warning(&device);
    device.wait_for_text("SECURITY");
}

fn sign_only_inside_the_window() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    expect_refused(&device);
    device.set_clock(MONDAY_IN_WINDOW);
    expect_signed(&device);
    device.set_clock(MONDAY_AFTER_WINDOW);
    expect_refused(&device);
}

fn an_earlier_window_cannot_be_replayed() {
    let device = Device::boot();
    device.set_clock(EARLIER_MONDAY_IN_WINDOW);
    walk::unlock_with_pin(&device, PIN);
    expect_refused(&device);
}
