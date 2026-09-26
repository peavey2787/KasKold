use super::{
    anti_klepto, private_swap, PrivateSwapPrepareResult, ScanResult, VaultReview, VaultRuntime,
    VaultRuntimeError,
};
use kaskold_protocol::{QrFrame, QrProgress};
use zeroize::{Zeroize, Zeroizing};

impl VaultRuntime {
    /// Begin a fresh QR signing session. This invalidates any earlier request
    /// and any signed response that has not yet been displayed/exported.
    pub fn begin_scan(&mut self) -> Result<(), VaultRuntimeError> {
        if self.active_wallet.is_none() {
            return Err(VaultRuntimeError::Locked);
        }
        self.clear_signing_session();
        Ok(())
    }

    /// Load one transaction request directly from a local file for the M5
    /// Wallet -> Recovery -> Transaction workflow. This bypasses QR framing
    /// only; it uses the exact same parser, review, approval, and signing path.
    pub fn load_transaction_file(&mut self, wire: &[u8]) -> Result<VaultReview, VaultRuntimeError> {
        const MAX_LOCAL_TRANSACTION_BYTES: usize = 1_048_576;
        if self.active_wallet.is_none() {
            return Err(VaultRuntimeError::Locked);
        }
        if wire.is_empty() || wire.len() > MAX_LOCAL_TRANSACTION_BYTES {
            return Err(VaultRuntimeError::InvalidLocalTransaction);
        }
        self.clear_signing_session();
        let review = self
            .active_wallet()?
            .review_transaction(wire)
            .map(VaultReview::from)
            .map_err(VaultRuntimeError::Custody)?;
        self.pending_request = Some(Zeroizing::new(wire.to_vec()));
        Ok(review)
    }

    /// Accept one raw QR payload. Session-bound multi-frame assembly is handled
    /// by `kaskold-protocol`; conflicting/mixed sessions fail closed.
    pub fn accept_qr_frame(&mut self, frame: &[u8]) -> Result<ScanResult, VaultRuntimeError> {
        if self.active_wallet.is_none() {
            return Err(VaultRuntimeError::Locked);
        }
        if self.pending_request.is_some() || !self.signed_response_frames.is_empty() {
            return Err(VaultRuntimeError::SigningSessionAlreadyComplete);
        }
        let complete = self
            .decoder
            .accept(frame)
            .map_err(VaultRuntimeError::Protocol)?;
        let Some(mut request) = complete else {
            return Ok(ScanResult::Progress(self.decoder.progress()));
        };
        let review_wire = shared_signer::anti_klepto::parse_request(&request)
            .map(|anti| anti.transaction)
            .unwrap_or(&request);
        let review = self
            .active_wallet()?
            .review_transaction(review_wire)
            .map(VaultReview::from)
            .map_err(VaultRuntimeError::Custody)?;
        self.pending_request = Some(Zeroizing::new(core::mem::take(&mut request)));
        Ok(ScanResult::Ready(review))
    }

    #[must_use]
    pub fn scan_progress(&self) -> QrProgress {
        self.decoder.progress()
    }

    /// Return the current review without signing. Useful when a native screen
    /// is recreated while the app remains in memory.
    pub fn review_transaction(&self) -> Result<VaultReview, VaultRuntimeError> {
        let wallet = self.active_wallet()?;
        let request = self
            .pending_request
            .as_ref()
            .ok_or(VaultRuntimeError::NoPendingReview)?;
        let review_wire = shared_signer::anti_klepto::parse_request(request)
            .map(|anti| anti.transaction)
            .unwrap_or(request);
        wallet
            .review_transaction(review_wire)
            .map(VaultReview::from)
            .map_err(VaultRuntimeError::Custody)
    }

    /// Explicitly approve the reviewed request and generate session-bound QR
    /// response frames. This is the only operation that signs a scanned request.
    pub fn approve(&mut self, now_unix: u64) -> Result<&[QrFrame], VaultRuntimeError> {
        self.check_session_signing_policy(now_unix)?;
        self.ensure_unlocked_for_approval()?;
        let request = self.take_pending_request()?;
        let response = self.sign_approved_request(&request)?;
        self.signed_response_frames = anti_klepto::frames_for_response(&response)?;
        self.signed_response_wire = Some(response);
        Ok(&self.signed_response_frames)
    }

    fn ensure_unlocked_for_approval(&self) -> Result<(), VaultRuntimeError> {
        if self.active_wallet.is_none() {
            return Err(VaultRuntimeError::Locked);
        }
        Ok(())
    }

    fn take_pending_request(&mut self) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        self.pending_request
            .take()
            .ok_or(VaultRuntimeError::NoPendingReview)
    }

    fn sign_approved_request(
        &mut self,
        request: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        if shared_signer::anti_klepto::parse_request(request).is_ok() {
            return self.approve_anti_klepto_request(request);
        }
        let mut signed = self
            .active_wallet()?
            .sign_transaction(request)
            .map_err(VaultRuntimeError::Custody)?;
        Ok(Zeroizing::new(core::mem::take(&mut signed)))
    }

    fn approve_anti_klepto_request(
        &mut self,
        request: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        let index = self.active_wallet.ok_or(VaultRuntimeError::Locked)?;
        let wallet = self.wallets.get(index).ok_or(VaultRuntimeError::Locked)?;
        self.anti_klepto.approve_request(wallet, request)
    }

    #[must_use]
    pub fn anti_klepto_awaiting_reveal(&self) -> bool {
        self.anti_klepto.awaiting_reveal()
    }

    /// Transition from displaying the nonce commitment to scanning the host
    /// reveal while retaining the authenticated provisional signing session.
    pub fn begin_anti_klepto_reveal(&mut self) -> Result<(), VaultRuntimeError> {
        if !self.anti_klepto.awaiting_reveal() {
            return Err(VaultRuntimeError::AntiKleptoRevealNotExpected);
        }
        self.decoder.reset();
        for frame in &mut self.signed_response_frames {
            frame.payload.zeroize();
        }
        self.signed_response_frames.clear();
        self.signed_response_wire = None;
        Ok(())
    }

    pub fn finalize_anti_klepto_reveal(
        &mut self,
        reveal: &[u8],
        now_unix: u64,
    ) -> Result<&[QrFrame], VaultRuntimeError> {
        self.check_session_signing_policy(now_unix)?;
        let index = self.active_wallet.ok_or(VaultRuntimeError::Locked)?;
        let wallet = self.wallets.get(index).ok_or(VaultRuntimeError::Locked)?;
        let response = self.anti_klepto.finalize_reveal(wallet, reveal)?;
        self.signed_response_frames = anti_klepto::frames_for_response(&response)?;
        self.signed_response_wire = Some(response);
        Ok(&self.signed_response_frames)
    }

    /// Reject the current request and erase all assembled/request/response bytes.
    pub fn reject(&mut self) {
        self.clear_signing_session();
    }

    pub fn signed_response_frames(&self) -> Result<&[QrFrame], VaultRuntimeError> {
        if self.signed_response_frames.is_empty() {
            return Err(VaultRuntimeError::NoSignedResponse);
        }
        Ok(&self.signed_response_frames)
    }

    /// Return the signed transaction wire while the approved response session is active.
    /// The bytes are zeroized when the response is rejected, completed, or the Vault locks.
    pub fn signed_response_wire(&self) -> Result<&[u8], VaultRuntimeError> {
        self.signed_response_wire
            .as_ref()
            .map(|wire| wire.as_slice())
            .ok_or(VaultRuntimeError::NoSignedResponse)
    }

    pub fn private_swap_prepare(
        &mut self,
        wire: &[u8],
    ) -> Result<PrivateSwapPrepareResult, VaultRuntimeError> {
        let index = self.active_wallet.ok_or(VaultRuntimeError::Locked)?;
        let wallet = self.wallets.get(index).ok_or(VaultRuntimeError::Locked)?;
        match self.private_swap.prepare(wallet, wire)? {
            private_swap::PrepareResult::Review(review) => {
                Ok(PrivateSwapPrepareResult::Review(review))
            }
            private_swap::PrepareResult::Response(response) => {
                Ok(PrivateSwapPrepareResult::Response(response))
            }
        }
    }

    pub fn private_swap_confirm(&mut self) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        let index = self.active_wallet.ok_or(VaultRuntimeError::Locked)?;
        let wallet = self.wallets.get(index).ok_or(VaultRuntimeError::Locked)?;
        self.private_swap.confirm(wallet)
    }

    pub fn private_swap_reveal(
        &mut self,
        wire: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        let index = self.active_wallet.ok_or(VaultRuntimeError::Locked)?;
        let wallet = self.wallets.get(index).ok_or(VaultRuntimeError::Locked)?;
        self.private_swap.reveal(wallet, wire)
    }

    #[must_use]
    pub fn private_swap_awaiting_reveal(&self) -> bool {
        self.private_swap.awaiting_reveal()
    }

    pub fn private_swap_cancel(&mut self) {
        self.private_swap.reset_active();
    }

    pub(crate) fn clear_signing_session(&mut self) {
        self.decoder.reset();
        self.pending_request = None;
        for frame in &mut self.signed_response_frames {
            frame.payload.zeroize();
        }
        self.signed_response_frames.clear();
        self.signed_response_wire = None;
        self.anti_klepto.clear();
        self.private_swap.reset_active();
        self.covenant.reset_active();
    }
}
