//! Recovery-material import shared by software Vault shells.

use shared_signer::seed_qr::{decode_compact_seedqr, decode_seedqr};
use zeroize::Zeroize;

use super::{phrase_from_indices, HotWallet, HotWalletError};

impl HotWallet {
    /// Restore standard SeedQR, CompactSeedQR, or UTF-8 BIP39 recovery words.
    pub fn restore_recovery_material(
        data: &[u8],
        passphrase: &str,
    ) -> Result<Self, HotWalletError> {
        let mut indices = [0u16; 24];
        let word_count = if matches!(data.len(), 48 | 96) && data.iter().all(u8::is_ascii_digit) {
            decode_seedqr(data, &mut indices)
        } else if matches!(data.len(), 16 | 32) {
            decode_compact_seedqr(data, &mut indices)
        } else {
            let phrase =
                core::str::from_utf8(data).map_err(|_| HotWalletError::InvalidToolInput)?;
            return Self::restore(phrase, passphrase);
        };
        if word_count == 0 {
            return Err(HotWalletError::InvalidToolInput);
        }
        let mut phrase = phrase_from_indices(&indices[..usize::from(word_count)]);
        indices.zeroize();
        let wallet = Self::restore(&phrase, passphrase);
        phrase.zeroize();
        wallet
    }
}
