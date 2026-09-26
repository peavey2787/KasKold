//! M5-compatible two-round anti-klepto transaction session orchestration.

use hot_wallet::{AntiKleptoSession, HotWallet};
use kaskold_protocol::{encode_qr_frames, QrFrame};
use zeroize::Zeroizing;

use crate::VaultRuntimeError;

pub(crate) struct AntiKleptoState {
    session: Option<AntiKleptoSession>,
}

impl AntiKleptoState {
    pub(crate) const fn new() -> Self {
        Self { session: None }
    }

    pub(crate) fn awaiting_reveal(&self) -> bool {
        self.session.is_some()
    }

    pub(crate) fn clear(&mut self) {
        self.session = None;
    }

    pub(crate) fn approve_request(
        &mut self,
        wallet: &HotWallet,
        request: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        let (session, mut commitment) = wallet
            .prepare_anti_klepto(request)
            .map_err(VaultRuntimeError::Custody)?;
        self.session = Some(session);
        Ok(Zeroizing::new(core::mem::take(&mut commitment)))
    }

    pub(crate) fn finalize_reveal(
        &mut self,
        wallet: &HotWallet,
        reveal: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        let mut session = self
            .session
            .take()
            .ok_or(VaultRuntimeError::AntiKleptoRevealNotExpected)?;
        let mut signed = wallet
            .finalize_anti_klepto(&mut session, reveal)
            .map_err(VaultRuntimeError::Custody)?;
        Ok(Zeroizing::new(core::mem::take(&mut signed)))
    }
}

pub(crate) fn frames_for_response(bytes: &[u8]) -> Result<Vec<QrFrame>, VaultRuntimeError> {
    encode_qr_frames(bytes).map_err(VaultRuntimeError::Protocol)
}
