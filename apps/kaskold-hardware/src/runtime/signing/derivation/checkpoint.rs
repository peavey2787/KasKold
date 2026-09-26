//! Cooperative mnemonic-seed derivation for responsive CoreS3 export flows.

use crate::wallet::seed_manager::SeedSlot;

pub(crate) fn begin_mnemonic_seed(
    slot: &SeedSlot,
) -> Result<offline_signer::derivation::bip39::SeedDerivation, &'static str> {
    let count = slot.mnemonic_word_count().ok_or("Active wallet is not a mnemonic")?;
    if !crate::wallet::mnemonic::validate(&slot.indices, count) { return Err("Invalid mnemonic"); }
    match count {
        12 => {
            let mut words = [0u16; 12];
            words.copy_from_slice(&slot.indices[..12]);
            Ok(offline_signer::derivation::bip39::SeedDerivation::from_mnemonic_12(
                &offline_signer::derivation::bip39::Mnemonic12 { indices: words },
                slot.passphrase_str(),
            ))
        }
        24 => Ok(offline_signer::derivation::bip39::SeedDerivation::from_mnemonic_24(
            &offline_signer::derivation::bip39::Mnemonic24 { indices: slot.indices },
            slot.passphrase_str(),
        )),
        _ => Err("Wallet source is not a mnemonic"),
    }
}
