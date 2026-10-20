//! Account extended-private-key slot insertion, including RAM-only imports.

use super::SeedManager;

impl SeedManager {
    /// Store an imported account extended-private key in the next free slot.
    /// Existing fingerprints are reused so duplicate imports stay idempotent.
    pub fn store_account_key(
        &mut self,
        raw: &[u8; 65],
        parent_fingerprint: [u8; 4],
        fingerprint: [u8; 4],
    ) -> Option<usize> {
        self.store_account_key_with_mode(raw, parent_fingerprint, fingerprint, false)
    }

    pub fn store_account_key_transient(
        &mut self,
        raw: &[u8; 65],
        parent_fingerprint: [u8; 4],
        fingerprint: [u8; 4],
    ) -> Option<usize> {
        self.store_account_key_with_mode(raw, parent_fingerprint, fingerprint, true)
    }
    pub fn find_matching_account_key(&self, raw: &[u8; 65], parent_fingerprint: &[u8; 4]) -> Option<usize> {
        self.slots
            .iter()
            .position(|slot| super::matching::account_key_matches(slot, raw, parent_fingerprint, self.selected_network))
    }
    fn store_account_key_with_mode(
        &mut self,
        raw: &[u8; 65],
        parent_fingerprint: [u8; 4],
        fingerprint: [u8; 4],
        transient: bool,
    ) -> Option<usize> {
        if let Some(existing) = self.find_matching_account_key(raw, &parent_fingerprint) {
            return Some(existing);
        }
        let slot_index = self.find_free()?;
        self.slots[slot_index].set_account_key_raw(raw, parent_fingerprint, fingerprint);
        self.slots[slot_index].network = self.selected_network;
        self.slots[slot_index].transient = transient;
        if !transient {
            self.mark_changed();
        }
        Some(slot_index)
    }
}
