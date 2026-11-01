use crate::storage::save_gate::{check_save, SaveRefusal};

#[test]
fn nothing_saves_before_the_recovery_words_are_acknowledged() {
    assert_eq!(
        check_save(false, false),
        Err(SaveRefusal::RecoveryWordsNotAcknowledged)
    );
    assert_eq!(
        check_save(false, true),
        Err(SaveRefusal::RecoveryWordsNotAcknowledged)
    );
}

#[test]
fn sd_storage_blocks_an_internal_save() {
    assert_eq!(check_save(true, true), Err(SaveRefusal::SdStorageActive));
}

#[test]
fn an_acknowledged_internal_wallet_saves() {
    assert_eq!(check_save(true, false), Ok(()));
}
