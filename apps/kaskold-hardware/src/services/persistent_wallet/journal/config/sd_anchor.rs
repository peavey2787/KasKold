//! Current-format SD-card storage anchor codec.

use crate::services::credential_policy::SALT_SIZE;

use super::super::{CredentialKdf, CredentialKind, DuressPolicy, FlashError, SdAnchor, CONFIG_SIZE};

pub(super) fn encode_sd_anchor(anchor: SdAnchor, out: &mut [u8; CONFIG_SIZE]) {
    out[10] = anchor.credential_kind as u8;
    out[11] = anchor.active_slot;
    out[16..20].copy_from_slice(&anchor.wallet_sequence.to_le_bytes());
    out[20..20 + SALT_SIZE].copy_from_slice(&anchor.salt);
    if anchor.duress.enabled {
        out[36] = 1;
        out[37] = anchor.duress.kind as u8;
        out[38] = anchor.duress.key_slot;
        out[40..40 + SALT_SIZE].copy_from_slice(&anchor.duress.salt);
        out[56..88].copy_from_slice(&anchor.duress.verifier);
    }
}

pub(super) fn decode_sd_anchor(record: &[u8; CONFIG_SIZE]) -> Result<Option<SdAnchor>, FlashError> {
    let Some(credential_kind) = CredentialKind::from_byte(record[10]) else {
        return Ok(None);
    };
    if record[11] > 1 || record[36] > 1 {
        return Ok(None);
    }
    let wallet_sequence = u32::from_le_bytes(record[16..20].try_into().map_err(|_| FlashError::Read)?);
    if wallet_sequence == 0 {
        return Ok(None);
    }
    let mut salt = [0u8; SALT_SIZE];
    salt.copy_from_slice(&record[20..20 + SALT_SIZE]);
    let duress = if record[36] == 0 {
        if record[37] != 0 || record[38] != 0 || record[40..88].iter().any(|byte| *byte != 0) {
            return Ok(None);
        }
        DuressPolicy::disabled()
    } else {
        let Some(kind) = CredentialKind::from_byte(record[37]) else {
            return Ok(None);
        };
        let key_slot = record[38];
        if !(3..=5).contains(&key_slot) {
            return Ok(None);
        }
        let mut duress_salt = [0u8; SALT_SIZE];
        duress_salt.copy_from_slice(&record[40..40 + SALT_SIZE]);
        let mut verifier = [0u8; 32];
        verifier.copy_from_slice(&record[56..88]);
        DuressPolicy { enabled: true, kind, key_slot, salt: duress_salt, verifier }
    };
    Ok(Some(SdAnchor {
        credential_kind,
        salt,
        active_slot: record[11],
        wallet_sequence,
        duress,
        credential_kdf: CredentialKdf::current(),
    }))
}
