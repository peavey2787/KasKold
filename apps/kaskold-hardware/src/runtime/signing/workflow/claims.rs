//! Input-amount claims: a transaction is signed only while every input amount
//! it claims agrees with what earlier signings claimed for the same outpoint.

use sha2::{Digest, Sha256};

use crate::runtime::data::AppData;
use crate::runtime::presentation::{ErrorSpec, SIGN_CLAIM_CONFLICT, SIGN_CLAIM_STORE, SIGN_KEY};
use crate::services::persistent_wallet::{ClaimRefusal, PersistentWallet};
use crate::wallet::seed_manager::SeedSlot;

const WALLET_KEY_DOMAIN: &[u8] = b"KasKold/input-claims/wallet-key/v1";

/// Record the active transaction's input amounts, or name why signing must stop.
pub(super) fn admit(ad: &AppData, persistence: &mut PersistentWallet<'_>) -> Result<(), ErrorSpec> {
    let slot = ad.wallet.seeds.seed_mgr.active_slot().ok_or(SIGN_KEY)?;
    let mut key = wallet_key(slot);
    let result = persistence.admit_input_claims(&key, &ad.signing.transaction.active);
    shared_signer::bytes::zeroize_bytes(&mut key);
    result.map_err(|refusal| match refusal {
        ClaimRefusal::Conflict => SIGN_CLAIM_CONFLICT,
        ClaimRefusal::Storage => SIGN_CLAIM_STORE,
    })
}

/// A key only the wallet's own secret yields, so the stored claim tags reveal
/// nothing about the wallet's outpoints to anyone without it.
fn wallet_key(slot: &SeedSlot) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(WALLET_KEY_DOMAIN);
    for index in slot.indices {
        hasher.update(index.to_le_bytes());
    }
    let passphrase_len = usize::from(slot.passphrase_len).min(slot.passphrase.len());
    hasher.update(&slot.passphrase[..passphrase_len]);
    hasher.finalize().into()
}
