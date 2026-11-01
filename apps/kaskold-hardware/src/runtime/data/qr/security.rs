//! QR-domain volatile payload and scan scrubbing.

use super::{CameraScanFault, OutgoingQrPurpose, QrScanState, QrState};

impl QrScanState {
    /// Forget the address on the address screen, so the next visit derives its own.
    pub fn clear_address(&mut self) {
        shared_signer::bytes::zeroize_bytes(&mut self.address);
        self.address_length = 0;
        self.address_valid = false;
        self.address_hidden_key = false;
    }

    pub fn clear(&mut self) {
        self.clear_address();
        self.camera_fault = CameraScanFault::None;
        self.camera_reset_requested = false;
    }
}

impl QrState {
    pub fn clear_sensitive(&mut self) {
        self.outgoing.clear();
        self.scan.clear();
        self.presentation.large = false;
        self.presentation.mode = 0;
        self.presentation.via_density = false;
    }
}

impl super::OutgoingQrState {
    pub fn clear(&mut self) {
        shared_signer::bytes::zeroize_bytes(&mut self.buffer);
        self.purpose = OutgoingQrPurpose::None;
        self.length = 0;
        self.frame = 0;
        self.frame_count = 0;
        self.manual_frames = false;
        self.covenant_backup_length = 0;
        self.close_state = None;
    }
}
