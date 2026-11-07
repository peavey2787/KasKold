//! Device-bound backup sealing boundary.

use super::super::{PersistError, PersistentWallet};
use offline_signer::crypto::device_bound_storage::{Envelope, StoragePurpose, TAG_SIZE};

impl PersistentWallet<'_> {
    /// Mix an already-stretched backup credential with the non-exportable
    /// device HMAC key and seal a device-bound backup. Password KDF selection
    /// belongs to the authenticated outer backup format, not this hardware
    /// boundary.
    pub(crate) fn seal_backup_key(
        &mut self,
        purpose: StoragePurpose,
        envelope: Envelope<'_>,
        ciphertext: &mut [u8],
    ) -> Result<[u8; TAG_SIZE], PersistError> {
        self.crypto.seal_backup(purpose, envelope, ciphertext)
    }

    pub(crate) fn open_backup_key(
        &mut self,
        purpose: StoragePurpose,
        envelope: Envelope<'_>,
        ciphertext: &mut [u8],
        tag: &[u8; TAG_SIZE],
    ) -> Result<(), PersistError> {
        self.crypto.open_backup(purpose, envelope, ciphertext, tag)
    }
}
