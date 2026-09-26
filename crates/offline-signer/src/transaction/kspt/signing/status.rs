use crate::transaction::{model::Transaction, signature_verification};

/// True only when every required standard input signature is structurally bound
/// and passes BIP340 verification over the exact transaction SIGHASH_ALL digest.
/// Generic covenant inputs deliberately remain incomplete without execution-branch evidence.
pub fn is_fully_signed(tx: &Transaction) -> bool {
    signature_verification::kspt_is_fully_signed(tx)
}

/// Return `(cryptographically_valid, required)`. Invalid/misbound signatures
/// contribute zero rather than being counted merely because a slot is occupied.
pub fn signature_status(tx: &Transaction) -> (u32, u32) {
    signature_verification::kspt_status(tx)
}
