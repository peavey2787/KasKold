//! Adding dice and finger movement to a new wallet, and changing your mind on the way.
use crate::harness::{app_state, discard_board, note, recall, Device};
use crate::runtime::input::AppState;
use crate::scenarios::journeys::known::{self, PASSPHRASE, PIN};
use crate::scenarios::journeys::walk;

const ROLLS: u16 = 25;
const FIRST_DIE: (u16, u16) = (60, 100);
const LAST_DIE: (u16, u16) = (260, 165);
const UNDO: (u16, u16) = (160, 215);
const CANVAS_COLUMNS: std::ops::Range<u16> = 30..290;
const CANVAS_ROWS: std::ops::Range<u16> = 85..205;

journey!(
    a_wallet_made_with_dice_and_touch_keeps_its_words:
    make_a_wallet_with_dice_and_touch,
    the_address_follows_the_words,
);
journey!(
    a_wallet_made_with_a_passphrase_keeps_using_it:
    make_a_wallet_with_a_passphrase,
    the_address_follows_the_words_and_passphrase,
);
journey!(leaving_the_dice_and_touch_screens_starts_nothing: back_out_of_the_extra_entropy);

fn roll(device: &Device, die: (u16, u16)) {
    device.tap_at(die.0, die.1);
}

fn roll_the_dice(device: &Device) {
    device.tap("Add Dice Rolls");
    device.tap("25 Rolls");
    device.expect_text("DICE 0/25");
    roll(device, FIRST_DIE);
    roll(device, LAST_DIE);
    device.expect_text("DICE 2/25");
    device.tap_at(UNDO.0, UNDO.1);
    device.expect_text("DICE 1/25");
    for _ in 1..ROLLS {
        roll(device, FIRST_DIE);
    }
}

fn draw_on_the_canvas(device: &Device) {
    device.expect_text("Draw here. Keep moving.");
    let zigzag = CANVAS_ROWS
        .flat_map(|y| CANVAS_COLUMNS.map(move |x| (x, y)))
        .cycle();
    device.draw(zigzag, || {
        app_state().navigation.app.state == AppState::TouchEntropy
    });
}

fn make_a_wallet_with_dice_and_touch() {
    let device = Device::boot();
    device.tap("Create Wallet");
    walk::name_wallet(&device);
    device.tap("12 Words");
    roll_the_dice(&device);
    device.tap("Add Touch Entropy");
    draw_on_the_canvas(&device);
    device.tap("No Passphrase");
    let words = walk::write_down_words(&device, 12);
    device.tap("I BACKED UP MY WORDS");
    walk::protect_with_pin(&device, PIN);
    note("words", &words.join(" "));
}

fn the_address_follows_the_words() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_receive(&device);
    let words = recall("words");
    walk::expect_address(&device, &known::address_of(&words));
    discard_board();
}

fn make_a_wallet_with_a_passphrase() {
    let device = Device::boot();
    device.tap("Create Wallet");
    walk::name_wallet(&device);
    device.tap("12 Words");
    device.tap("No Dice");
    device.tap("No Touch Entropy");
    device.tap("Use Passphrase");
    device.type_text(PASSPHRASE);
    walk::press_full_ok(&device);
    let words = walk::write_down_words(&device, 12);
    device.tap("I BACKED UP MY WORDS");
    walk::protect_with_pin(&device, PIN);
    note("words", &words.join(" "));
}

fn the_address_follows_the_words_and_passphrase() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_receive(&device);
    let words = recall("words");
    let expected = known::address_with_passphrase(&words, PASSPHRASE);
    assert_ne!(expected, known::address_of(&words));
    walk::expect_address(&device, &expected);
    discard_board();
}

fn back_out_of_the_extra_entropy() {
    let device = Device::boot();
    device.tap("Create Wallet");
    walk::name_wallet(&device);
    device.tap("12 Words");
    device.tap("Add Dice Rolls");
    device.tap("25 Rolls");
    roll(&device, FIRST_DIE);
    walk::go_back(&device);
    device.expect_text("25 Rolls");
    walk::go_back(&device);
    device.expect_text("Add Dice Rolls");
    device.tap("No Dice");
    device.tap("Add Touch Entropy");
    walk::go_back(&device);
    device.expect_text("Add Touch Entropy");
}
