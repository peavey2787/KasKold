//! Secret-bearing values wipe themselves when dropped.

use core::mem::MaybeUninit;
use core::ptr::addr_of;

use super::super::{transaction_codec::ParsedTransaction, RecoveryMaterial};
use offline_signer::transaction::{model::Transaction, std_pskt};

/// Run `value`'s destructor in place and keep its storage, so a test can read
/// the plain-data fields the destructor left behind.
pub(crate) fn dropped<T>(value: T) -> MaybeUninit<T> {
    let mut slot = MaybeUninit::new(value);
    // SAFETY: `slot` holds an initialised `T` that is dropped exactly once
    // here and is never used as a `T` again.
    unsafe { slot.as_mut_ptr().drop_in_place() };
    slot
}

#[test]
fn recovery_material_is_wiped_on_drop() {
    let material = RecoveryMaterial::from_parts(24, [7; 24], "secret").unwrap();
    let slot = dropped(material);
    let dropped = slot.as_ptr();
    // SAFETY: the fields are plain integers and arrays left by `drop`.
    let (word_count, indices, passphrase, passphrase_len) = unsafe {
        (
            addr_of!((*dropped).word_count).read(),
            addr_of!((*dropped).indices).read(),
            addr_of!((*dropped).passphrase).read(),
            addr_of!((*dropped).passphrase_len).read(),
        )
    };
    assert_eq!(word_count, 0);
    assert_eq!(indices, [0; 24]);
    assert!(passphrase.iter().all(|byte| *byte == 0));
    assert_eq!(passphrase_len, 0);
}

#[test]
fn parsed_transaction_scratch_is_wiped_on_drop() {
    let parsed = ParsedTransaction {
        format: std_pskt::DetectedFormat::Unknown,
        transaction: Transaction::try_new().unwrap(),
        scratch: vec![0x5a; 16],
        parsed: shared_signer::pskt::PsktParsed::empty(),
    };
    let slot = dropped(parsed);
    // SAFETY: only the scratch vector's length field is read.
    let length = unsafe { (*addr_of!((*slot.as_ptr()).scratch)).len() };
    assert_eq!(length, 0);
}
