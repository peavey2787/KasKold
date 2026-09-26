//! M5 universal covenant-signing workflow shared by software Vault shells.
//!
//! This mirrors the hardware protocol state machine while keeping all
//! mnemonic-derived key operations inside `HotWallet` custody.

use sha2::{Digest, Sha256};
use shared_signer::covenant_sign::{
    self, BindingHint, CovenantSignRequest, CovenantSignResponse, KnownScheme, RequestKind,
    ResponseKind,
};
use zeroize::Zeroize;

use super::{VaultRuntime, VaultRuntimeError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CovenantMode {
    None,
    KeyInfo,
    BindKnown,
    BindOpaque,
    Known,
    Opaque,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CovenantPhase {
    Idle,
    Prepared,
    AwaitingReveal,
    FinalResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CovenantReview {
    pub mode: CovenantMode,
    pub scheme: KnownScheme,
    pub key_id: [u8; 32],
    pub pubkey_x: [u8; 32],
    pub commitment: [u8; 32],
    pub script_hash: [u8; 32],
    pub context: String,
}

pub(crate) struct CovenantSession {
    mode: CovenantMode,
    phase: CovenantPhase,
    scheme: KnownScheme,
    session_id: [u8; covenant_sign::SESSION_ID_LEN],
    host_commitment: [u8; 32],
    key_id: [u8; 32],
    pubkey_x: [u8; 32],
    binding_token: [u8; 32],
    commitment: [u8; 32],
    script_hash: [u8; 32],
    context: Vec<u8>,
    provisional_signature: [u8; 64],
    nonce_point: [u8; 33],
    response: [u8; covenant_sign::RESPONSE_LEN],
    response_len: usize,
    pending_key_id: [u8; 32],
    pending_pubkey_x: [u8; 32],
}

impl CovenantSession {
    pub(crate) fn new() -> Self {
        Self {
            mode: CovenantMode::None,
            phase: CovenantPhase::Idle,
            scheme: KnownScheme::None,
            session_id: [0; covenant_sign::SESSION_ID_LEN],
            host_commitment: [0; 32],
            key_id: [0; 32],
            pubkey_x: [0; 32],
            binding_token: [0; 32],
            commitment: [0; 32],
            script_hash: [0; 32],
            context: Vec::new(),
            provisional_signature: [0; 64],
            nonce_point: [0; 33],
            response: [0; covenant_sign::RESPONSE_LEN],
            response_len: 0,
            pending_key_id: [0; 32],
            pending_pubkey_x: [0; 32],
        }
    }

    pub(crate) fn reset_active(&mut self) {
        self.session_id.zeroize();
        self.host_commitment.zeroize();
        self.key_id.zeroize();
        self.pubkey_x.zeroize();
        self.binding_token.zeroize();
        self.commitment.zeroize();
        self.script_hash.zeroize();
        self.context.zeroize();
        self.context.clear();
        self.provisional_signature.zeroize();
        self.nonce_point.zeroize();
        self.response.zeroize();
        self.mode = CovenantMode::None;
        self.phase = CovenantPhase::Idle;
        self.scheme = KnownScheme::None;
        self.response_len = 0;
    }

    pub(crate) fn clear_all(&mut self) {
        self.reset_active();
        self.pending_key_id.zeroize();
        self.pending_pubkey_x.zeroize();
    }
}

impl Drop for CovenantSession {
    fn drop(&mut self) {
        self.clear_all();
    }
}

impl VaultRuntime {
    pub fn covenant_prepare(&mut self, wire: &[u8]) -> Result<CovenantReview, VaultRuntimeError> {
        let request = covenant_sign::parse_request(wire)
            .map_err(|_| VaultRuntimeError::InvalidCovenantMessage)?;
        if request.kind == RequestKind::KeyInfo {
            return self.covenant_prepare_key_info();
        }
        self.covenant_prepare_request(&request)
    }

    fn covenant_prepare_request(
        &mut self,
        request: &CovenantSignRequest<'_>,
    ) -> Result<CovenantReview, VaultRuntimeError> {
        let wallet = self.active_wallet()?;
        let pubkey_x = wallet
            .covenant_public_key(&request.key_id)
            .map_err(VaultRuntimeError::Custody)?;
        let script_hash: [u8; 32] = Sha256::digest(request.script).into();

        if request.kind == RequestKind::Bind {
            return self.covenant_prepare_binding(request, pubkey_x, script_hash);
        }
        require_covenant_binding(wallet, request, &script_hash)?;
        validate_request_shape(request, &pubkey_x)?;
        self.covenant.reset_active();
        set_request_state(&mut self.covenant, request, pubkey_x, script_hash);
        self.covenant.binding_token = request.binding_token;
        self.covenant.mode = signing_mode(request.kind)?;
        self.covenant.phase = CovenantPhase::Prepared;
        review_from_state(&self.covenant)
    }

    fn covenant_prepare_key_info(&mut self) -> Result<CovenantReview, VaultRuntimeError> {
        let (key_id, pubkey_x) = self
            .active_wallet()?
            .covenant_allocate_key()
            .map_err(VaultRuntimeError::Custody)?;
        self.covenant.reset_active();
        self.covenant.mode = CovenantMode::KeyInfo;
        self.covenant.phase = CovenantPhase::FinalResponse;
        self.covenant.key_id = key_id;
        self.covenant.pubkey_x = pubkey_x;
        self.covenant.pending_key_id = key_id;
        self.covenant.pending_pubkey_x = pubkey_x;
        self.build_covenant_response(ResponseKind::KeyInfo, [0; 64])?;
        review_from_state(&self.covenant)
    }

    fn covenant_prepare_binding(
        &mut self,
        request: &CovenantSignRequest<'_>,
        pubkey_x: [u8; 32],
        script_hash: [u8; 32],
    ) -> Result<CovenantReview, VaultRuntimeError> {
        if request.key_id != self.covenant.pending_key_id
            || pubkey_x != self.covenant.pending_pubkey_x
        {
            return Err(VaultRuntimeError::CovenantBindingRequired);
        }
        validate_request_shape(request, &pubkey_x)?;
        self.covenant.reset_active();
        set_request_state(&mut self.covenant, request, pubkey_x, script_hash);
        self.covenant.mode = if request.scheme == KnownScheme::None {
            CovenantMode::BindOpaque
        } else {
            CovenantMode::BindKnown
        };
        self.covenant.phase = CovenantPhase::Prepared;
        review_from_state(&self.covenant)
    }

    pub fn covenant_confirm(&mut self) -> Result<Vec<u8>, VaultRuntimeError> {
        match self.covenant.mode {
            CovenantMode::KeyInfo => self.confirm_key_info(),
            CovenantMode::BindKnown | CovenantMode::BindOpaque => self.confirm_binding(),
            CovenantMode::Known | CovenantMode::Opaque => self.confirm_signing(),
            _ => Err(VaultRuntimeError::InvalidCovenantState),
        }
    }

    fn confirm_key_info(&self) -> Result<Vec<u8>, VaultRuntimeError> {
        require_covenant_phase(self.covenant.phase, CovenantPhase::FinalResponse)?;
        self.covenant_response()
    }

    fn confirm_binding(&mut self) -> Result<Vec<u8>, VaultRuntimeError> {
        require_covenant_phase(self.covenant.phase, CovenantPhase::Prepared)?;
        let token = self
            .active_wallet()?
            .covenant_binding_token(&self.covenant.key_id, &self.covenant.script_hash)
            .map_err(VaultRuntimeError::Custody)?;
        self.covenant.binding_token = token;
        self.covenant.phase = CovenantPhase::FinalResponse;
        self.covenant.pending_key_id.zeroize();
        self.covenant.pending_pubkey_x.zeroize();
        self.build_covenant_response(ResponseKind::Binding, [0; 64])?;
        self.covenant_response()
    }

    fn confirm_signing(&mut self) -> Result<Vec<u8>, VaultRuntimeError> {
        require_covenant_phase(self.covenant.phase, CovenantPhase::Prepared)?;
        let provisional = self
            .active_wallet()?
            .covenant_begin_signature(&self.covenant.key_id, &self.covenant.commitment)
            .map_err(VaultRuntimeError::Custody)?;
        self.covenant.provisional_signature = provisional.signature;
        self.covenant.nonce_point = provisional.nonce_point;
        self.covenant.phase = CovenantPhase::AwaitingReveal;
        self.build_covenant_response(ResponseKind::NonceCommitment, [0; 64])?;
        self.covenant_response()
    }

    pub fn covenant_finalize_reveal(&mut self, wire: &[u8]) -> Result<Vec<u8>, VaultRuntimeError> {
        if self.covenant.phase != CovenantPhase::AwaitingReveal {
            return Err(VaultRuntimeError::InvalidCovenantState);
        }
        let mut reveal = covenant_sign::parse_reveal(wire)
            .map_err(|_| VaultRuntimeError::InvalidCovenantMessage)?;
        if !covenant_reveal_matches(&self.covenant, &reveal) {
            reveal.host_secret.zeroize();
            return Err(VaultRuntimeError::CovenantRevealMismatch);
        }
        let signature = self
            .active_wallet()?
            .covenant_finalize_signature(
                &self.covenant.key_id,
                &self.covenant.commitment,
                &self.covenant.provisional_signature,
                &self.covenant.nonce_point,
                &self.covenant.session_id,
                &reveal.host_secret,
            )
            .map_err(VaultRuntimeError::Custody);
        reveal.host_secret.zeroize();
        let signature = signature?;
        self.covenant.phase = CovenantPhase::FinalResponse;
        self.build_covenant_response(ResponseKind::Signature, signature)?;
        self.covenant_response()
    }

    pub fn covenant_response(&self) -> Result<Vec<u8>, VaultRuntimeError> {
        if self.covenant.response_len == 0 {
            return Err(VaultRuntimeError::NoCovenantResponse);
        }
        Ok(self.covenant.response[..self.covenant.response_len].to_vec())
    }

    pub fn covenant_cancel(&mut self) {
        self.covenant.reset_active();
    }

    fn build_covenant_response(
        &mut self,
        kind: ResponseKind,
        signature: [u8; 64],
    ) -> Result<(), VaultRuntimeError> {
        let signing = matches!(
            kind,
            ResponseKind::NonceCommitment | ResponseKind::Signature
        );
        let binding = matches!(
            kind,
            ResponseKind::Binding | ResponseKind::NonceCommitment | ResponseKind::Signature
        );
        let response = CovenantSignResponse {
            kind,
            session_id: if signing {
                self.covenant.session_id
            } else {
                [0; covenant_sign::SESSION_ID_LEN]
            },
            key_id: self.covenant.key_id,
            pubkey_x: self.covenant.pubkey_x,
            binding_token: if binding {
                self.covenant.binding_token
            } else {
                [0; 32]
            },
            commitment: match kind {
                ResponseKind::Binding => self.covenant.script_hash,
                ResponseKind::NonceCommitment | ResponseKind::Signature => self.covenant.commitment,
                ResponseKind::KeyInfo => [0; 32],
            },
            nonce_point: if signing {
                self.covenant.nonce_point
            } else {
                [0; 33]
            },
            signature,
        };
        self.covenant.response_len =
            covenant_sign::encode_response(&response, &mut self.covenant.response)
                .map_err(|_| VaultRuntimeError::InvalidCovenantMessage)?;
        Ok(())
    }
}

fn require_covenant_binding(
    wallet: &hot_wallet::HotWallet,
    request: &CovenantSignRequest<'_>,
    script_hash: &[u8; 32],
) -> Result<(), VaultRuntimeError> {
    let matches = wallet
        .covenant_binding_matches(&request.key_id, script_hash, &request.binding_token)
        .map_err(VaultRuntimeError::Custody)?;
    matches
        .then_some(())
        .ok_or(VaultRuntimeError::CovenantBindingMismatch)
}

fn signing_mode(kind: RequestKind) -> Result<CovenantMode, VaultRuntimeError> {
    match kind {
        RequestKind::Known => Ok(CovenantMode::Known),
        RequestKind::Opaque => Ok(CovenantMode::Opaque),
        _ => Err(VaultRuntimeError::InvalidCovenantMessage),
    }
}

fn covenant_reveal_matches(
    session: &CovenantSession,
    reveal: &covenant_sign::CovenantSignReveal,
) -> bool {
    reveal.session_id == session.session_id
        && reveal.key_id == session.key_id
        && reveal.commitment == session.commitment
        && shared_signer::anti_klepto::verify_host_secret(
            &session.host_commitment,
            &reveal.host_secret,
        )
}

fn require_covenant_phase(
    actual: CovenantPhase,
    expected: CovenantPhase,
) -> Result<(), VaultRuntimeError> {
    (actual == expected)
        .then_some(())
        .ok_or(VaultRuntimeError::InvalidCovenantState)
}

fn set_request_state(
    state: &mut CovenantSession,
    request: &CovenantSignRequest<'_>,
    pubkey_x: [u8; 32],
    script_hash: [u8; 32],
) {
    state.scheme = request.scheme;
    state.session_id = request.session_id;
    state.host_commitment = request.host_commitment;
    state.key_id = request.key_id;
    state.pubkey_x = pubkey_x;
    state.commitment = request.commitment;
    state.script_hash = script_hash;
    state.context.extend_from_slice(request.context);
}

fn validate_request_shape(
    request: &CovenantSignRequest<'_>,
    pubkey_x: &[u8; 32],
) -> Result<(), VaultRuntimeError> {
    match request.kind {
        RequestKind::Known => validate_known_shape(request, pubkey_x),
        RequestKind::Opaque => validate_opaque_shape(request, pubkey_x),
        RequestKind::Bind if request.scheme == KnownScheme::None => {
            validate_opaque_shape(request, pubkey_x)
        }
        RequestKind::Bind => validate_known_shape(request, pubkey_x),
        RequestKind::KeyInfo => Err(VaultRuntimeError::InvalidCovenantMessage),
    }
}

fn validate_known_shape(
    request: &CovenantSignRequest<'_>,
    pubkey_x: &[u8; 32],
) -> Result<(), VaultRuntimeError> {
    if request.scheme == KnownScheme::None
        || request.binding != covenant_sign::expected_known_binding(request.scheme)
    {
        return Err(VaultRuntimeError::InvalidCovenantMessage);
    }
    let recomputed = covenant_sign::recompute_known_commitment(request.scheme, request.context)
        .ok_or(VaultRuntimeError::InvalidCovenantContext)?;
    if recomputed != request.commitment
        || !covenant_sign::known_script_binds(request.scheme, request.script, &recomputed, pubkey_x)
    {
        return Err(VaultRuntimeError::InvalidCovenantContext);
    }
    Ok(())
}

fn validate_opaque_shape(
    request: &CovenantSignRequest<'_>,
    pubkey_x: &[u8; 32],
) -> Result<(), VaultRuntimeError> {
    if request.scheme != KnownScheme::None
        || !matches!(request.binding, BindingHint::None | BindingHint::KeyPresent)
    {
        return Err(VaultRuntimeError::InvalidCovenantMessage);
    }
    if request.binding == BindingHint::KeyPresent
        && !covenant_sign::script_contains_xonly_key(request.script, pubkey_x)
    {
        return Err(VaultRuntimeError::InvalidCovenantContext);
    }
    Ok(())
}

fn review_from_state(state: &CovenantSession) -> Result<CovenantReview, VaultRuntimeError> {
    let context = core::str::from_utf8(&state.context)
        .map_err(|_| VaultRuntimeError::InvalidCovenantContext)?
        .to_owned();
    Ok(CovenantReview {
        mode: state.mode,
        scheme: state.scheme,
        key_id: state.key_id,
        pubkey_x: state.pubkey_x,
        commitment: state.commitment,
        script_hash: state.script_hash,
        context,
    })
}
