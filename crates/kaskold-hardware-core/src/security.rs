//! Device-local signing authorization and seed-entropy invariants.

pub mod boot_fuses;
pub mod credential;

use crate::entropy::frame_noise::CameraEntropyReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningAuthorizationError {
    SeedUnavailable,
    ReviewIncomplete,
    ReviewedTransactionChanged,
    NoInputs,
    InputCountMismatch,
    InputOutOfRange,
}

/// The owner's approval of one exact transaction, named by the digest of what
/// the review showed. It can only be minted by approving a review in progress,
/// so no other code path can claim the owner said yes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewApproval {
    digest: [u8; 32],
}

impl ReviewApproval {
    /// Whether this approval names the transaction whose digest is `digest`.
    #[must_use]
    pub fn covers(&self, digest: &[u8; 32]) -> bool {
        self.digest == *digest
    }
}

/// Where the owner's review of the loaded transaction stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionReview {
    /// No transaction is under review.
    Idle,
    /// The transaction whose digest this is is on screen, not yet approved.
    Showing([u8; 32]),
    /// The owner approved the transaction they were shown.
    Approved(ReviewApproval),
}

impl TransactionReview {
    /// The owner approves what is on screen; nothing else can be approved.
    #[must_use]
    pub const fn approve(self) -> Self {
        match self {
            Self::Showing(digest) => Self::Approved(ReviewApproval { digest }),
            other => other,
        }
    }

    #[must_use]
    pub const fn approval(self) -> Option<ReviewApproval> {
        match self {
            Self::Approved(approval) => Some(approval),
            Self::Idle | Self::Showing(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SigningAuthorization {
    pub seed_loaded: bool,
    pub review: TransactionReview,
    /// Review digest of the transaction about to be signed, as it is now.
    pub transaction_digest: [u8; 32],
    pub reviewed_inputs: usize,
    pub transaction_inputs: usize,
    pub signing_input_index: usize,
}

pub fn authorize_transaction_signing(
    authorization: SigningAuthorization,
) -> Result<(), SigningAuthorizationError> {
    if !authorization.seed_loaded {
        return Err(SigningAuthorizationError::SeedUnavailable);
    }
    let approval = authorization
        .review
        .approval()
        .ok_or(SigningAuthorizationError::ReviewIncomplete)?;
    if !approval.covers(&authorization.transaction_digest) {
        return Err(SigningAuthorizationError::ReviewedTransactionChanged);
    }
    authorize_input(authorization)
}

fn authorize_input(authorization: SigningAuthorization) -> Result<(), SigningAuthorizationError> {
    if authorization.reviewed_inputs == 0 || authorization.transaction_inputs == 0 {
        return Err(SigningAuthorizationError::NoInputs);
    }
    if authorization.reviewed_inputs != authorization.transaction_inputs {
        return Err(SigningAuthorizationError::InputCountMismatch);
    }
    if authorization.signing_input_index >= authorization.transaction_inputs {
        return Err(SigningAuthorizationError::InputOutOfRange);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedEntropyError {
    CameraUnavailable,
    HardwareRngUnavailable,
    DeviceIdentityUnavailable,
    TimingUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeedEntropyEvidence {
    pub camera: CameraEntropyReport,
    pub hardware_rng_healthy: bool,
    pub device_identity_mixed: bool,
    pub timing_mixed: bool,
}

#[must_use]
pub fn timing_observations_usable(first: (u32, u32), second: (u32, u32)) -> bool {
    // Equal observations are unusable, including the all-zero pair. Keeping the
    // predicate to this single comparison also avoids an impossible coverage
    // branch: once the observations differ, they cannot both be `(0, 0)`.
    first != second
}

#[must_use]
pub fn device_identity_words_usable(words: &[u32]) -> bool {
    words.iter().any(|value| *value != 0 && *value != u32::MAX)
}

pub fn validate_seed_entropy(evidence: SeedEntropyEvidence) -> Result<(), SeedEntropyError> {
    if !evidence.camera.healthy() {
        return Err(SeedEntropyError::CameraUnavailable);
    }
    if !evidence.hardware_rng_healthy {
        return Err(SeedEntropyError::HardwareRngUnavailable);
    }
    if !evidence.device_identity_mixed {
        return Err(SeedEntropyError::DeviceIdentityUnavailable);
    }
    if !evidence.timing_mixed {
        return Err(SeedEntropyError::TimingUnavailable);
    }
    Ok(())
}
