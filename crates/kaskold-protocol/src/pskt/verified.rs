//! Typed authorization result used by consumer finalizers.
//!
//! A `VerifiedTransaction` is created only after canonical parsing, semantic
//! validation, branch binding and Schnorr verification have all succeeded.
//! Finalizers must materialize consensus bytes exclusively from this object;
//! reparsing the original PSKT/KSPT after authorization is forbidden.

use super::{compact, relay_fields::parse_multisig_redeem};
mod plan;
mod witness;
pub(crate) use plan::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedSignature {
    position: u8,
    sighash: u8,
    bytes: [u8; 64],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifiedCovenantRoute {
    PrivateSwapClaim,
    OracleV1Claim,
    CommitRevealClaim,
    MerkleClaim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifiedWitnessPlan {
    P2pk {
        signature: VerifiedSignature,
    },
    Multisig {
        threshold: u8,
        signatures: Vec<VerifiedSignature>,
        redeem_script: Vec<u8>,
    },
    /// Generic branch-aware covenant witness. Consumer finalization deliberately
    /// supports one canonical outer selector only. More complex/specialized
    /// covenants must define a typed execution plan before they can cross this
    /// boundary; host routing hints are never consulted after authorization.
    Covenant {
        signature: VerifiedSignature,
        redeem_script: Vec<u8>,
        supplied_mask: u16,
        supplied_true_mask: u16,
    },
    /// Specialized covenant witness whose template, branch selectors and route
    /// metadata were validated before authorization. The exact script is frozen
    /// into the verified model so materialization performs no second parse.
    SpecializedCovenant {
        route: VerifiedCovenantRoute,
        signature_script: Vec<u8>,
        supplied_mask: u16,
        supplied_true_mask: u16,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedInput {
    previous_tx_id: [u8; 32],
    previous_index: u32,
    amount: u64,
    sequence: u64,
    sig_op_count: u8,
    script_version: u16,
    script_public_key: Vec<u8>,
    has_covenant_id: bool,
    witness: VerifiedWitnessPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedOutput {
    amount: u64,
    script_version: u16,
    script_public_key: Vec<u8>,
    covenant: Option<(u16, [u8; 32])>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedTransaction {
    version: u16,
    locktime: u64,
    subnetwork_id: [u8; 20],
    gas: u64,
    payload: Vec<u8>,
    inputs: Vec<VerifiedInput>,
    outputs: Vec<VerifiedOutput>,
}

impl VerifiedSignature {
    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn position(&self) -> u8 {
        self.position
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn sighash(&self) -> u8 {
        self.sighash
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn bytes(&self) -> &[u8; 64] {
        &self.bytes
    }
}

impl VerifiedInput {
    #[must_use]
    pub const fn previous_tx_id(&self) -> &[u8; 32] {
        &self.previous_tx_id
    }

    #[must_use]
    pub const fn previous_index(&self) -> u32 {
        self.previous_index
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn amount(&self) -> u64 {
        self.amount
    }

    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub const fn sig_op_count(&self) -> u8 {
        self.sig_op_count
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn script_version(&self) -> u16 {
        self.script_version
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub fn script_public_key(&self) -> &[u8] {
        &self.script_public_key
    }

    #[cfg(feature = "companion-compat")]
    #[must_use]
    pub const fn has_covenant_id(&self) -> bool {
        self.has_covenant_id
    }

    #[must_use]
    pub const fn witness(&self) -> &VerifiedWitnessPlan {
        &self.witness
    }
}

impl VerifiedOutput {
    #[must_use]
    pub const fn amount(&self) -> u64 {
        self.amount
    }

    #[must_use]
    pub const fn script_version(&self) -> u16 {
        self.script_version
    }

    #[must_use]
    pub fn script_public_key(&self) -> &[u8] {
        &self.script_public_key
    }

    #[must_use]
    pub const fn covenant(&self) -> Option<(u16, [u8; 32])> {
        self.covenant
    }
}

impl VerifiedTransaction {
    #[must_use]
    pub const fn version(&self) -> u16 {
        self.version
    }

    #[must_use]
    pub const fn locktime(&self) -> u64 {
        self.locktime
    }

    #[must_use]
    pub const fn subnetwork_id(&self) -> &[u8; 20] {
        &self.subnetwork_id
    }

    #[must_use]
    pub const fn gas(&self) -> u64 {
        self.gas
    }

    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    #[must_use]
    pub fn inputs(&self) -> &[VerifiedInput] {
        &self.inputs
    }

    #[must_use]
    pub fn outputs(&self) -> &[VerifiedOutput] {
        &self.outputs
    }
}
