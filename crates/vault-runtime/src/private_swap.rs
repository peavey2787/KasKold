//! Private Swap v2 state-machine adapter.

use hot_wallet::{HotWallet, PrivateSwapPrepared, PrivateSwapReview, PrivateSwapSession};
use zeroize::Zeroizing;

use crate::VaultRuntimeError;

pub(crate) struct PrivateSwapState {
    session: PrivateSwapSession,
    response: Option<Zeroizing<Vec<u8>>>,
}

pub(crate) enum PrepareResult {
    Review(Box<PrivateSwapReview>),
    Response(Zeroizing<Vec<u8>>),
}

impl PrivateSwapState {
    pub(crate) fn new() -> Self {
        Self {
            session: PrivateSwapSession::new(),
            response: None,
        }
    }

    pub(crate) fn prepare(
        &mut self,
        wallet: &HotWallet,
        wire: &[u8],
    ) -> Result<PrepareResult, VaultRuntimeError> {
        self.response = None;
        match self
            .session
            .prepare_request(wallet, wire)
            .map_err(VaultRuntimeError::Custody)?
        {
            PrivateSwapPrepared::Review(review) => Ok(PrepareResult::Review(review)),
            PrivateSwapPrepared::Response(response) => {
                let response = Zeroizing::new(response);
                self.response = Some(Zeroizing::new(response.to_vec()));
                Ok(PrepareResult::Response(response))
            }
        }
    }

    pub(crate) fn confirm(
        &mut self,
        wallet: &HotWallet,
    ) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        let response = Zeroizing::new(
            self.session
                .confirm(wallet)
                .map_err(VaultRuntimeError::Custody)?,
        );
        self.response = Some(Zeroizing::new(response.to_vec()));
        Ok(response)
    }

    pub(crate) fn reveal(
        &mut self,
        wallet: &HotWallet,
        wire: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, VaultRuntimeError> {
        let response = Zeroizing::new(
            self.session
                .finalize_reveal(wallet, wire)
                .map_err(VaultRuntimeError::Custody)?,
        );
        self.response = Some(Zeroizing::new(response.to_vec()));
        Ok(response)
    }

    pub(crate) fn awaiting_reveal(&self) -> bool {
        self.session.awaiting_reveal()
    }

    pub(crate) fn reset_active(&mut self) {
        self.session.reset_active();
        self.response = None;
    }

    pub(crate) fn clear_all(&mut self) {
        self.session.clear_all();
        self.response = None;
    }
}
