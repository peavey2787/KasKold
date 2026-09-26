//! Portable additive seed-entropy mixing shared by hardware and software Vaults.
//!
//! Callers must begin with an independently strong, mandatory entropy pool.
//! User-supplied dice/touch material is additive only and never satisfies the
//! primary entropy requirement by itself.

use sha2::{Digest, Sha256};

pub const MAX_ADDITIVE_DICE_ROLLS: usize = 200;
pub const MAX_TOUCH_TRANSCRIPT_BYTES: usize = 32_768;

/// Mix optional user-entered d6 rolls into an already strong entropy pool.
///
/// The byte-for-byte construction is shared with the M5 hardware workflow so
/// the same rolls added to the same base pool produce the same mixed pool.
pub fn mix_additive_dice(pool: &mut [u8; 32], rolls: &[u8]) -> bool {
    if rolls.is_empty()
        || rolls.len() > MAX_ADDITIVE_DICE_ROLLS
        || rolls.iter().any(|roll| !(1..=6).contains(roll))
    {
        return false;
    }
    let mut hasher = Sha256::new();
    hasher.update(b"KasSigner/additive-dice/v1");
    hasher.update((rolls.len() as u16).to_le_bytes());
    hasher.update(*pool);
    hasher.update(rolls);
    pool.copy_from_slice(&hasher.finalize());
    true
}

/// Mix the M5's completed, hardened 32-byte touch digest into an entropy pool.
pub fn mix_additive_touch(pool: &mut [u8; 32], touch_digest: &mut [u8; 32]) {
    let mut hasher = Sha256::new();
    hasher.update(b"KasSigner/additive-touch/v1");
    hasher.update(*pool);
    hasher.update(*touch_digest);
    pool.copy_from_slice(&hasher.finalize());
    crate::bytes::zeroize_bytes(touch_digest);
}

/// Mix a browser touch transcript into an already strong CSPRNG pool.
///
/// Web Vault cannot claim the M5's hardware touch-timing source or device RNG.
/// Instead, raw pointer/timing events are treated strictly as an optional
/// additive transcript with a distinct domain. The mandatory CSPRNG pool is
/// always collected independently inside Rust before this function is called.
pub fn mix_additive_touch_transcript(pool: &mut [u8; 32], transcript: &[u8]) -> bool {
    if transcript.is_empty() || transcript.len() > MAX_TOUCH_TRANSCRIPT_BYTES {
        return false;
    }
    let mut hasher = Sha256::new();
    hasher.update(b"KasKold/web-additive-touch/v1");
    hasher.update((transcript.len() as u32).to_le_bytes());
    hasher.update(*pool);
    hasher.update(transcript);
    pool.copy_from_slice(&hasher.finalize());
    true
}
