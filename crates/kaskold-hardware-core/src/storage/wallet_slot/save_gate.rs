//! What must hold before a wallet may be written to internal storage.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveRefusal {
    RecoveryWordsNotAcknowledged,
    SdStorageActive,
}

/// Recovery words are the only portable backup, so nothing is saved until the
/// user confirms them. SD storage owns the wallet once it is active.
pub const fn check_save(
    recovery_words_acknowledged: bool,
    sd_storage_active: bool,
) -> Result<(), SaveRefusal> {
    if !recovery_words_acknowledged {
        return Err(SaveRefusal::RecoveryWordsNotAcknowledged);
    }
    if sd_storage_active {
        return Err(SaveRefusal::SdStorageActive);
    }
    Ok(())
}
