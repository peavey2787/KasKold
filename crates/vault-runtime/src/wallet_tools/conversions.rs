//! Hot-wallet results as the views Vault shells receive.

use hot_wallet::{CommittedSecret, MessageSignature, MultisigView};

use super::{MultisigResult, SecretCommitment, SignedMessage};

impl From<MessageSignature> for SignedMessage {
    fn from(value: MessageSignature) -> Self {
        Self {
            signature: value.signature,
            digest: value.digest,
        }
    }
}

impl From<CommittedSecret> for SecretCommitment {
    fn from(value: CommittedSecret) -> Self {
        Self {
            payload: value.payload,
            commitment: value.commitment,
        }
    }
}

impl From<MultisigView> for MultisigResult {
    fn from(value: MultisigView) -> Self {
        Self {
            descriptor: value.descriptor,
            address: value.address,
            threshold: value.threshold,
            participants: value.participants,
            chain: value.chain,
            index: value.index,
        }
    }
}
