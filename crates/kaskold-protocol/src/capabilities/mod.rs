//! Public capabilities of the KasKold v2 reference hardware signer.
//!
//! These limits are protocol-facing compatibility information, not wallet coin-
//! selection policy. Host wallets should query the SDK instead of inventing
//! larger transaction/QR limits that the hardware cannot accept.

#[cfg(feature = "host")]
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[cfg_attr(feature = "host", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "host", serde(rename_all = "camelCase"))]
pub struct SignerCapabilities {
    pub kspt_generation: u8,
    pub max_inputs: u16,
    pub max_outputs: u8,
    pub max_script_bytes: u16,
    pub max_redeem_script_bytes: u16,
    pub max_payload_bytes: u16,
    pub max_signatures_per_input: u8,
    pub max_multisig_keys: u8,
    pub max_multisig_wallets: u8,
    pub qr_max_frames: u8,
    pub qr_multi_frame_fragment_bytes: u16,
    pub qr_single_frame_payload_bytes: u16,
    pub qr_session_frame_version: u8,
    pub qr_session_binding: bool,
}

pub const QR_MULTI_FRAME_FRAGMENT_BYTES: usize = 91;
pub const QR_SINGLE_FRAME_PAYLOAD_BYTES: usize = 134;

pub const SIGNER_CAPABILITIES: SignerCapabilities = SignerCapabilities {
    kspt_generation: crate::wire::kspt::GENERATION_CURRENT,
    max_inputs: crate::wire::kspt::MAX_INPUTS as u16,
    max_outputs: crate::wire::kspt::MAX_OUTPUTS,
    max_script_bytes: crate::wire::kspt::MAX_SCRIPT_SIZE as u16,
    max_redeem_script_bytes: crate::wire::kspt::MAX_REDEEM_SIZE as u16,
    max_payload_bytes: crate::wire::kspt::MAX_PAYLOAD_SIZE as u16,
    max_signatures_per_input: crate::wire::kspt::MAX_SIGNATURE_RECORDS as u8,
    max_multisig_keys: 5,
    max_multisig_wallets: 2,
    qr_max_frames: shared_signer::qr_frame::MAX_FRAMES as u8,
    qr_multi_frame_fragment_bytes: QR_MULTI_FRAME_FRAGMENT_BYTES as u16,
    qr_single_frame_payload_bytes: QR_SINGLE_FRAME_PAYLOAD_BYTES as u16,
    qr_session_frame_version: shared_signer::qr_frame::FRAME_VERSION,
    qr_session_binding: true,
};

#[must_use]
pub const fn limits() -> SignerCapabilities {
    SIGNER_CAPABILITIES
}
