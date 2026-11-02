//! Recovery phrases of both lengths: what is accepted, what is refused, what comes back.
use crate::harness::{app_state, discard_board, note, recall, Device};
use crate::runtime::input::AppState;
use crate::scenarios::journeys::known::{self, PIN};
use crate::scenarios::journeys::walk;

const WORDS_IN_LONG_PHRASE: usize = 24;

journey!(a_24_word_phrase_restores_its_known_wallet: restore_the_24_word_phrase);
journey!(a_24_word_phrase_with_a_bad_checksum_is_refused: refuse_a_bad_checksum);
journey!(
    a_created_24_word_wallet_can_be_restored_from_its_words:
    create_a_24_word_wallet,
    restore_the_written_24_words,
);

fn restore_the_24_word_phrase() {
    let device = Device::boot();
    device.tap("Restore Wallet");
    device.tap("Words");
    walk::type_words(&device, &known::TWENTY_FOUR_WORDS);
    device.tap("No Passphrase");
    walk::save_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::TWENTY_FOUR_WORD_RECEIVE_0);
}

fn refuse_a_bad_checksum() {
    let device = Device::boot();
    device.tap("Restore Wallet");
    device.tap("Words");
    walk::type_words(&device, &["abandon"; WORDS_IN_LONG_PHRASE]);
    device.wait_for_text("RESTORE WALLET");
    assert_eq!(
        app_state().navigation.app.state,
        AppState::StorageSeedSourceChoice
    );
    assert!(
        !app_state().wallet.seeds.seed_loaded,
        "a phrase failing its checksum loaded a wallet"
    );
    assert_eq!(app_state().wallet.seeds.word_count, 0);
}

fn create_a_24_word_wallet() {
    let device = Device::boot();
    device.tap("Create Wallet");
    walk::name_wallet(&device);
    device.tap("24 Words");
    device.tap("No Dice");
    device.tap("No Touch Entropy");
    device.tap("No Passphrase");
    let words = walk::write_down_words(&device, WORDS_IN_LONG_PHRASE);
    device.tap("I BACKED UP MY WORDS");
    walk::protect_with_pin(&device, PIN);
    let words = words.join(" ");
    walk::open_receive(&device);
    let address = known::address_of(&words);
    walk::expect_address(&device, &address);
    note("words", &words);
    note("address", &address);
    discard_board();
}

fn restore_the_written_24_words() {
    let device = Device::boot();
    device.tap("Restore Wallet");
    device.tap("Words");
    let words = recall("words");
    let words: Vec<&str> = words.split(' ').collect();
    walk::type_words(&device, &words);
    device.tap("No Passphrase");
    walk::save_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, &recall("address"));
}
