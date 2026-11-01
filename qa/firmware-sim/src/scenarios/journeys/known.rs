//! Answers worked out away from the firmware, so a journey cannot agree with itself.
//!
//! The addresses come from a separate stdlib-only derivation of BIP39, BIP32 and
//! the Kaspa address format at m/44'/111111'/0'/chain/index.
use kaspa_portal::primitives::address::{encode_p2pk, KaspaNetwork, MAX_ADDR_LEN};
use kaspa_portal::primitives::hash::blake2b_256;
use kaspa_portal::transaction::interchange::kspt::serialize_compact_kspt_vec;
use kaspa_portal::transaction::model::{
    Ms45Hint, ScriptPublicKey, SigHashType, Transaction, OP_BLAKE2B, OP_DATA_32, OP_EQUAL,
};
use kaspa_portal::wallet::derivation::bip32::{
    derive_account_key, derive_address_key, derive_change_key, ExtendedPrivKey,
};
use kaspa_portal::wallet::mnemonic::bip39::{
    seed_from_mnemonic_12, seed_from_mnemonic_24, word_to_index, Mnemonic12, Mnemonic24, Seed,
};

pub(super) const ABOUT_WORDS: &[&str] = &[
    "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon",
    "abandon", "abandon", "abandon", "about",
];
/// A standard SeedQR is each word's four-digit BIP39 index: abandon is 0, about is 3.
pub(super) const ABOUT_SEEDQR: &str = "000000000000000000000000000000000000000000000003";
pub(super) const TWENTY_FOUR_WORDS: [&str; 24] = [
    "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon",
    "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon",
    "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "art",
];
pub(super) const PIN: &str = "483920";
pub(super) const PASSWORD: &str = "correct4horse";
pub(super) const PASSPHRASE: &str = "TREZOR";
/// The x coordinate of the secp256k1 generator: a valid key this wallet does not own.
pub(super) const STRANGER_KEY: [u8; 32] = [
    0x79, 0xBE, 0x66, 0x7E, 0xF9, 0xDC, 0xBB, 0xAC, 0x55, 0xA0, 0x62, 0x95, 0xCE, 0x87, 0x0B, 0x07,
    0x02, 0x9B, 0xFC, 0xDB, 0x2D, 0xCE, 0x28, 0xD9, 0x59, 0xF2, 0x81, 0x5B, 0x16, 0xF8, 0x17, 0x98,
];

pub(super) const RECEIVE_0: &str =
    "kaspa:qqd6e65yefepe9wk0m9vuxdufxd80sphy67gwwd0vdaumzdt4tc9s3qt0lqeh";
pub(super) const RECEIVE_1: &str =
    "kaspa:qp6r0d88yj4fazlj057wc35245jfgs87n9jn6nahfg223996dfukvgpgq6pcp";
pub(super) const CHANGE_0: &str =
    "kaspa:qqnkyg4hha30lnjystpugz42yh3paurdylxfrsshk3jfstgy95elu2a8l3jcs";
pub(super) const CHANGE_1: &str =
    "kaspa:qzs0tzq7c6atuvu7zsu3g6ln9lxmqvcxgspn57rrkp5ttcgy88prchay8zt9q";
/// m/22'/111111'/0'/0/0 as a KIP-22 P2SH address over `<key> OP_CHECKSIG`.
pub(super) const HIDDEN_KEY_RECEIVE_0: &str =
    "kaspa:pq02dzckvs0g6hhl9s9c484fv0wsd93sltwct9symfad4yw062w5y4v4mtlxa";
pub(super) const STRANGER_ADDRESS: &str =
    "kaspa:qpumuen7l8wthtz45p3ftn58pvrs9xlumvkuu2xet8egzkcklqtes4ypce9sf";
pub(super) const TWENTY_FOUR_WORD_RECEIVE_0: &str =
    "kaspa:qz20g2ugrrm629fnnvmfjpsjlx9zu2f2zj828wwpv9gvxcnajl0qs36s8hfkl";
pub(super) const PASSPHRASE_RECEIVE_0: &str =
    "kaspa:qzmzhr957he0n3tsjs2sev9pa3kcshc9jg2zvslvv82mv559wg2quxs9qrlys";

/// The seed of a 12 or 24 word phrase, from Portal.
fn seed_of(words: &[&str]) -> Seed {
    let index = |word: &&str| word_to_index(word).expect("a recovery word");
    let indices: Vec<u16> = words.iter().map(index).collect();
    match <[u16; 12]>::try_from(indices.as_slice()) {
        Ok(indices) => seed_from_mnemonic_12(&Mnemonic12 { indices }, ""),
        Err(_) => seed_from_mnemonic_24(
            &Mnemonic24 {
                indices: indices.try_into().expect("12 or 24 words"),
            },
            "",
        ),
    }
}

/// What each cosigner in a 2-of-3 of these wallets must see, worked out with the same
/// separate derivation at m/45'/111111'/0'/cosigner/chain/index for the about wallet.
pub(super) const SHARED_RECEIVE_0: &str =
    "kaspa:pqsyp33pamp4m6zw6jsfqrqr72fc7pgzcte9a5sz294twqwmg8ta7sjaj706p";
pub(super) const SHARED_RECEIVE_1: &str =
    "kaspa:ppa0dumu6ctrztu30vdz3h252h074xqmsh9l0mac2zr2239mmnufw5uktkg4d";
pub(super) const LEGAL_WORDS: &[&str] = &[
    "legal", "winner", "thank", "year", "wave", "sausage", "worth", "useful", "legal", "winner",
    "thank", "yellow",
];

pub(super) fn multisig_seed_of(words: &[&str]) -> Seed {
    seed_of(words)
}

/// A spend of a shared wallet's coins paying a stranger, naming where the signers' keys are.
fn transaction_spending_redeem(redeem: &[u8], hint: Ms45Hint) -> Transaction {
    let mut tx = Transaction::try_new().expect("transaction");
    tx.version = 0;
    tx.network = KaspaNetwork::Mainnet;
    tx.ensure_input_slots(1).expect("inputs");
    tx.num_inputs = 1;
    tx.num_outputs = 1;
    let input = &mut tx.inputs[0];
    input.previous_outpoint.transaction_id = [0x71; 32];
    input.utxo_entry.amount = 100_000_000;
    input.sequence = u64::MAX;
    input.sig_op_count = 3;
    input.sighash_type = SigHashType::All.to_byte();
    input.ms45_hint = hint;
    tx.store_redeem(0, redeem).expect("redeem script");
    let script = &mut tx.inputs[0].utxo_entry.script_public_key;
    script.script[0] = OP_BLAKE2B;
    script.script[1] = OP_DATA_32;
    script.script[2..34].copy_from_slice(&blake2b_256(redeem));
    script.script[34] = OP_EQUAL;
    script.script_len = 35;
    set_p2pk(&mut tx.outputs[0].script_public_key, STRANGER_KEY);
    tx
}

pub(super) fn transaction_spending_shared(redeem: &[u8], hint: Ms45Hint) -> Vec<u8> {
    let mut tx = transaction_spending_redeem(redeem, hint);
    tx.outputs[0].value = 99_000_000;
    serialize_compact_kspt_vec(&tx).expect("serialize")
}

/// The one-input, one-output claim of a private swap whose covenant pays [`STRANGER_KEY`].
pub(super) fn transaction_claiming_a_swap(covenant: &[u8]) -> Vec<u8> {
    let mut tx = transaction_spending_redeem(covenant, Ms45Hint::none());
    tx.outputs[0].value = 99_999_000;
    serialize_compact_kspt_vec(&tx).expect("serialize")
}

fn account_of(words: &[&str]) -> ExtendedPrivKey {
    derive_account_key(&seed_of(words).bytes).expect("account key")
}

/// The first receive address of a wallet whose words were read off a screen, from
/// Portal because those words are different on every run.
pub(super) fn address_of(words: &str) -> String {
    let words: Vec<&str> = words.split(' ').collect();
    let key = derive_address_key(&account_of(&words), 0).expect("address key");
    let public_key = key.public_key_x_only().expect("public key");
    let mut text = [0u8; MAX_ADDR_LEN];
    let length = encode_p2pk(&public_key, &mut text);
    String::from_utf8(text[..length].to_vec()).expect("ascii address")
}

/// The account key of the known wallet, which signs messages rather than coins.
pub(super) fn about_message_key() -> [u8; 32] {
    account_of(ABOUT_WORDS)
        .public_key_x_only()
        .expect("public key")
}

/// The x-only keys of the known wallet at one chain position, from Portal.
pub(super) fn about_key(chain_is_change: bool, index: u32) -> [u8; 32] {
    let account = account_of(ABOUT_WORDS);
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
    transaction_spending(
        [about_key(false, 0), about_key(true, 0)],
        about_key(true, 1),
    )
}

/// The same spend, of coins this wallet has no key for.
pub(super) fn transaction_spending_a_stranger() -> Vec<u8> {
    transaction_spending([STRANGER_KEY, STRANGER_KEY], about_key(true, 1))
}

/// A spend whose "change" claims a place in this wallet but pays a stranger.
pub(super) fn transaction_with_false_change() -> Vec<u8> {
    transaction_spending([about_key(false, 0), about_key(true, 0)], STRANGER_KEY)
}

fn transaction_spending(coins: [[u8; 32]; 2], change: [u8; 32]) -> Vec<u8> {
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
    set_p2pk(&mut tx.outputs[1].script_public_key, change);
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

pub(super) fn p2pk_script(key: [u8; 32]) -> Vec<u8> {
    let mut script = vec![0x20];
    script.extend_from_slice(&key);
    script.push(0xac);
    script
}

fn set_p2pk(script: &mut ScriptPublicKey, key: [u8; 32]) {
    script.version = 0;
    script.script[0] = 0x20;
    script.script[1..33].copy_from_slice(&key);
    script.script[33] = 0xac;
    script.script_len = 34;
}
