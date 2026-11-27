//! Runtime-HIL probe of the durable input-claim ledger on real flash.
//!
//! Every admission reloads the ledger from flash, so a refusal of the
//! contradicting claim proves the earlier claim was journaled and read back.
//! The workflow image keeps its ledger in sectors production never uses.

use super::{begin, fail, pass};
use crate::services::persistent_wallet::{ClaimRefusal, PersistentWallet};
use offline_signer::transaction::model::Transaction;

const NAME: &str = "input-claim-ledger-flash";
/// A key no wallet derives, so the probe's tags never match a wallet's own.
const PROBE_KEY: [u8; 32] = [0x6B; 32];

pub(super) fn probe_input_claims(
    persistent_wallet: &mut PersistentWallet<'_>,
    watchdog_feed: &mut impl FnMut(),
) -> bool {
    begin(NAME);
    watchdog_feed();
    // A fresh outpoint each run, so every run writes a new claim to flash.
    let mut txid = [0u8; 32];
    if crate::crypto::entropy::fill(&mut txid).is_err() {
        return fail(NAME, "entropy unavailable");
    }
    let (Some(honest), Some(understated)) = (spending(txid, 1_000), spending(txid, 999)) else {
        return fail(NAME, "transaction allocation failed");
    };
    let steps = [(&honest, Ok(())), (&understated, Err(ClaimRefusal::Conflict)), (&honest, Ok(()))];
    for (tx, expected) in steps {
        watchdog_feed();
        if persistent_wallet.admit_input_claims(&PROBE_KEY, tx) != expected {
            return fail(NAME, "flash ledger answered a claim wrongly");
        }
    }
    watchdog_feed();
    pass(NAME);
    true
}

/// A one-input transaction claiming `amount` for outpoint `txid:0`.
fn spending(txid: [u8; 32], amount: u64) -> Option<Transaction> {
    let mut tx = Transaction::try_new().ok()?;
    tx.ensure_input_slots(1).ok()?;
    tx.num_inputs = 1;
    tx.inputs[0].previous_outpoint.transaction_id = txid;
    tx.inputs[0].utxo_entry.amount = amount;
    Some(tx)
}
