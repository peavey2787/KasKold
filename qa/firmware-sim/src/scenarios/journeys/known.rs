//! Answers worked out away from the firmware, so a journey cannot agree with itself.
//!
//! The addresses come from a separate stdlib-only derivation of BIP39, BIP32 and
//! the Kaspa address format at m/44'/111111'/0'/chain/index.
use kaspa_portal::primitives::address::{encode_p2pk, KaspaNetwork, MAX_ADDR_LEN};
use kaspa_portal::transaction::interchange::kspt::serialize_compact_kspt_vec;
use kaspa_portal::transaction::model::{ScriptPublicKey, SigHashType, Transaction};
use kaspa_portal::wallet::derivation::bip32::{
    derive_account_key, derive_address_key, derive_change_key,
};
use kaspa_portal::wallet::mnemonic::bip39::{seed_from_mnemonic_12, word_to_index, Mnemonic12};

pub(super) const ABOUT_WORDS: &[&str] = &[
    "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon",
    "abandon", "abandon", "abandon", "about",
];
pub(super) const PIN: &str = "483920";
pub(super) const PASSWORD: &str = "correct4horse";
pub(super) const PASSPHRASE: &str = "TREZOR";
/// The x coordinate of the secp256k1 generator: a valid key this wallet does not own.
const STRANGER_KEY: [u8; 32] = [
    0x79, 0xBE, 0x66, 0x7E, 0xF9, 0xDC, 0xBB, 0xAC, 0x55, 0xA0, 0x62, 0x95, 0xCE, 0x87, 0x0B, 0x07,
    0x02, 0x9B, 0xFC, 0xDB, 0x2D, 0xCE, 0x28, 0xD9, 0x59, 0xF2, 0x81, 0x5B, 0x16, 0xF8, 0x17, 0x98,
];

pub(super) const RECEIVE_0: &str =
    "kaspa:qqd6e65yefepe9wk0m9vuxdufxd80sphy67gwwd0vdaumzdt4tc9s3qt0lqeh";
pub(super) const RECEIVE_1: &str =
    "kaspa:qp6r0d88yj4fazlj057wc35245jfgs87n9jn6nahfg223996dfukvgpgq6pcp";
pub(super) const CHANGE_0: &str =
    "kaspa:qqnkyg4hha30lnjystpugz42yh3paurdylxfrsshk3jfstgy95elu2a8l3jcs";
pub(super) const PASSPHRASE_RECEIVE_0: &str =
    "kaspa:qzmzhr957he0n3tsjs2sev9pa3kcshc9jg2zvslvv82mv559wg2quxs9qrlys";

/// The first receive address of a wallet whose words were read off a screen, from
/// Portal because those words are different on every run.
pub(super) fn address_of(words: &str) -> String {
    let mut indices = [0u16; 12];
    for (slot, word) in indices.iter_mut().zip(words.split(' ')) {
        *slot = word_to_index(word).expect("a recovery word");
    }
    let seed = seed_from_mnemonic_12(&Mnemonic12 { indices }, "");
    let account = derive_account_key(&seed.bytes).expect("account key");
    let key = derive_address_key(&account, 0).expect("address key");
    let public_key = key.public_key_x_only().expect("public key");
    let mut text = [0u8; MAX_ADDR_LEN];
    let length = encode_p2pk(&public_key, &mut text);
    String::from_utf8(text[..length].to_vec()).expect("ascii address")
}

/// The x-only keys of the known wallet at one chain position, from Portal.
pub(super) fn about_key(chain_is_change: bool, index: u32) -> [u8; 32] {
    let mut indices = [0u16; 12];
    for (slot, word) in indices.iter_mut().zip(ABOUT_WORDS) {
        *slot = word_to_index(word).expect("a recovery word");
    }
    let seed = seed_from_mnemonic_12(&Mnemonic12 { indices }, "");
    let account = derive_account_key(&seed.bytes).expect("account key");
    let derive = if chain_is_change {
        derive_change_key
    } else {
        derive_address_key
    };
    derive(&account, index)
        .expect("address key")
        .public_key_x_only()
        .expect("public key")
}

/// Spends two of the known wallet's coins: 1.5 KAS to a stranger, 0.49 back as change.
pub(super) fn unsigned_transaction() -> Vec<u8> {
    transaction_spending([about_key(false, 0), about_key(true, 0)])
}

/// The same spend, of coins this wallet has no key for.
pub(super) fn transaction_spending_a_stranger() -> Vec<u8> {
    transaction_spending([STRANGER_KEY, STRANGER_KEY])
}

fn transaction_spending(coins: [[u8; 32]; 2]) -> Vec<u8> {
    let mut tx = Transaction::try_new().expect("transaction");
    tx.version = 0;
    tx.network = KaspaNetwork::Mainnet;
    tx.ensure_input_slots(2).expect("inputs");
    tx.num_inputs = 2;
    tx.num_outputs = 2;
    spend(&mut tx, 0, coins[0], [0x41; 32], 100_000_000);
    spend(&mut tx, 1, coins[1], [0x42; 32], 100_000_000);
    tx.outputs[0].value = 150_000_000;
    set_p2pk(&mut tx.outputs[0].script_public_key, STRANGER_KEY);
    tx.outputs[1].value = 49_000_000;
    set_p2pk(&mut tx.outputs[1].script_public_key, about_key(true, 1));
    tx.outputs[1].has_derivation_hint = true;
    tx.outputs[1].derivation_branch = 1;
    tx.outputs[1].derivation_index = 1;
    serialize_compact_kspt_vec(&tx).expect("serialize")
}

fn spend(tx: &mut Transaction, index: usize, key: [u8; 32], txid: [u8; 32], amount: u64) {
    let input = &mut tx.inputs[index];
    input.previous_outpoint.transaction_id = txid;
    input.previous_outpoint.index = index as u32;
    input.utxo_entry.amount = amount;
    input.sequence = u64::MAX;
    input.sig_op_count = 1;
    input.sighash_type = SigHashType::All.to_byte();
    set_p2pk(&mut input.utxo_entry.script_public_key, key);
}

fn set_p2pk(script: &mut ScriptPublicKey, key: [u8; 32]) {
    script.version = 0;
    script.script[0] = 0x20;
    script.script[1..33].copy_from_slice(&key);
    script.script[33] = 0xac;
    script.script_len = 34;
}
