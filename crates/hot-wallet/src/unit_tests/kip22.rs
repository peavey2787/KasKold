use super::restored_wallet;
use crate::{HotWalletError, OutputOwnership};
use offline_signer::{
    address::{p2pk_script, p2sh_script, script_hash, KaspaNetwork},
    derivation::kip22::{self, Chain},
    transaction::{kspt::serialize_compact_kspt_vec, model::Transaction},
};

fn wallet_key(chain: Chain, index: u32) -> [u8; 32] {
    let wallet = restored_wallet();
    let account = kip22::derive_account_key(&wallet.seed.bytes, 0).expect("KIP-22 account");
    kip22::derive_address_key(&account, chain, index)
        .expect("KIP-22 key")
        .public_key_x_only()
        .expect("KIP-22 public key")
}

fn kip22_script(xonly: &[u8; 32]) -> [u8; 35] {
    p2sh_script(&kip22::redeem_script_hash(xonly))
}

fn set_script(target: &mut offline_signer::transaction::model::ScriptPublicKey, script: &[u8]) {
    target.script[..script.len()].copy_from_slice(script);
    target.script_len = script.len();
}

/// A KIP-22 spend paying a foreign key, with its second output at `change`.
fn spend(source: &[u8; 32], change: &[u8]) -> Vec<u8> {
    spend_from(&kip22_script(source), &kip22::redeem_script(source), change)
}

/// A spend of the script-hash coin at `coin`, revealing `redeem`.
fn spend_from(coin: &[u8], redeem: &[u8], change: &[u8]) -> Vec<u8> {
    let mut transaction = Transaction::try_new().expect("transaction allocation");
    transaction.version = 1;
    transaction.network = KaspaNetwork::Mainnet;
    transaction.num_inputs = 1;
    transaction.num_outputs = 2;
    transaction.inputs[0].previous_outpoint.transaction_id = [0x11; 32];
    transaction.inputs[0].sequence = u64::MAX;
    transaction.inputs[0].sig_op_count = 1;
    transaction.inputs[0].covenant_execution_present = true;
    transaction.inputs[0].utxo_entry.amount = 100_000;
    set_script(
        &mut transaction.inputs[0].utxo_entry.script_public_key,
        coin,
    );
    transaction
        .store_redeem(0, redeem)
        .expect("store redeem script");
    transaction.outputs[0].value = 30_000;
    set_script(
        &mut transaction.outputs[0].script_public_key,
        &p2pk_script(&[0x44; 32]),
    );
    transaction.outputs[1].value = 69_000;
    set_script(&mut transaction.outputs[1].script_public_key, change);
    serialize_compact_kspt_vec(&transaction).expect("KIP-22 fixture serializes")
}

#[test]
fn own_change_is_not_presented_as_a_payment() {
    let source = wallet_key(Chain::Receive, 2);
    let change = kip22_script(&wallet_key(Chain::Change, 3));
    let review = restored_wallet()
        .review_compact_kspt(&spend(&source, &change))
        .expect("KIP-22 spend reviews");
    assert_eq!(review.outputs[0].ownership, OutputOwnership::External);
    assert_eq!(review.outputs[1].ownership, OutputOwnership::Change);
    assert_eq!(review.change_total, 69_000);
    assert_eq!(review.external_total, 30_000);
}

#[test]
fn a_foreign_script_hash_stays_a_payment() {
    let source = wallet_key(Chain::Receive, 0);
    let review = restored_wallet()
        .review_compact_kspt(&spend(&source, &kip22_script(&[0x55; 32])))
        .expect("KIP-22 spend reviews");
    assert_eq!(review.outputs[1].ownership, OutputOwnership::External);
    assert_eq!(review.external_total, 99_000);
}

#[test]
fn a_script_hash_coin_that_is_not_kip22_is_refused() {
    let redeem = [0x51];
    let change = kip22_script(&wallet_key(Chain::Change, 0));
    let spend = spend_from(&p2sh_script(&script_hash(&redeem)), &redeem, &change);
    assert!(matches!(
        restored_wallet().review_compact_kspt(&spend),
        Err(HotWalletError::InvalidToolInput)
    ));
}

#[test]
fn the_seed_signs_its_own_kip22_coin() {
    let source = wallet_key(Chain::Receive, 2);
    let change = kip22_script(&wallet_key(Chain::Change, 0));
    let signed = restored_wallet()
        .sign_compact_kspt(&spend(&source, &change))
        .expect("KIP-22 spend signs");
    assert_ne!(signed, spend(&source, &change));
}

#[test]
fn a_key_the_seed_does_not_hold_cannot_sign() {
    let change = kip22_script(&wallet_key(Chain::Change, 0));
    assert!(restored_wallet()
        .sign_compact_kspt(&spend(&[0x66; 32], &change))
        .is_err());
}

#[test]
fn addresses_use_the_kip22_path_and_hide_the_key() {
    let wallet = restored_wallet();
    let address = wallet
        .derive_kip22_address(KaspaNetwork::Mainnet, false, 2)
        .expect("KIP-22 address");
    assert_eq!(
        address,
        kip22::address(&wallet_key(Chain::Receive, 2), "kaspa")
    );
    assert_ne!(
        address,
        wallet
            .derive_address(KaspaNetwork::Mainnet, false, 2)
            .expect("BIP44 address")
    );
}

#[test]
fn a_raw_key_wallet_has_no_kip22_path() {
    let wallet =
        crate::HotWallet::import_raw_private_key_hex(&"11".repeat(32)).expect("raw key imports");
    assert!(matches!(
        wallet.derive_kip22_address(KaspaNetwork::Mainnet, false, 0),
        Err(HotWalletError::UnsupportedForWalletType)
    ));
}
