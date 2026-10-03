//! Signature-script materialization from a verified witness plan.

use super::{VerifiedSignature, VerifiedWitnessPlan};

impl VerifiedWitnessPlan {
    /// Serialize exactly the witness plan that was authorized. No source JSON or
    /// KSPT bytes are consulted here.
    pub fn materialize_signature_script(&self) -> Result<Vec<u8>, String> {
        match self {
            Self::P2pk { signature } => materialize_p2pk(signature),
            Self::Multisig {
                threshold,
                signatures,
                redeem_script,
            } => materialize_multisig(*threshold, signatures, redeem_script),
            Self::Covenant {
                signature,
                redeem_script,
                supplied_mask,
                supplied_true_mask,
            } => materialize_covenant(
                signature,
                redeem_script,
                *supplied_mask,
                *supplied_true_mask,
            ),
            Self::SpecializedCovenant {
                signature_script,
                supplied_mask,
                supplied_true_mask,
                ..
            } => materialize_specialized(signature_script, *supplied_mask, *supplied_true_mask),
        }
    }
}

pub(crate) fn materialize_p2pk(signature: &VerifiedSignature) -> Result<Vec<u8>, String> {
    let mut script = Vec::with_capacity(66);
    push_signature(&mut script, signature)?;
    Ok(script)
}

pub(crate) fn materialize_multisig(
    threshold: u8,
    signatures: &[VerifiedSignature],
    redeem_script: &[u8],
) -> Result<Vec<u8>, String> {
    if signatures.len() != usize::from(threshold) {
        return Err("verified multisig witness cardinality changed".to_string());
    }
    let mut script = Vec::new();
    for signature in signatures {
        push_signature(&mut script, signature)?;
    }
    push_data(&mut script, redeem_script)?;
    Ok(script)
}

pub(crate) fn materialize_covenant(
    signature: &VerifiedSignature,
    redeem_script: &[u8],
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<Vec<u8>, String> {
    if supplied_mask != 0b1 || supplied_true_mask & !supplied_mask != 0 {
        return Err("verified generic covenant selector plan is invalid".to_string());
    }
    let mut script = Vec::new();
    push_signature(&mut script, signature)?;
    script.push(if supplied_true_mask & 1 != 0 {
        0x51
    } else {
        0x00
    });
    push_data(&mut script, redeem_script)?;
    Ok(script)
}

pub(crate) fn materialize_specialized(
    signature_script: &[u8],
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<Vec<u8>, String> {
    if supplied_true_mask & !supplied_mask != 0 {
        return Err("verified specialized covenant selector plan is invalid".to_string());
    }
    Ok(signature_script.to_vec())
}

pub(crate) fn push_signature(
    script: &mut Vec<u8>,
    signature: &VerifiedSignature,
) -> Result<(), String> {
    if signature.sighash != crate::wire::pskt_schema::SIGHASH_ALL {
        return Err("verified witness contains unsupported sighash".to_string());
    }
    script.push(65);
    script.extend_from_slice(&signature.bytes);
    script.push(signature.sighash);
    Ok(())
}

pub(crate) fn push_data(script: &mut Vec<u8>, data: &[u8]) -> Result<(), String> {
    match data.len() {
        0..=75 => script.push(data.len() as u8),
        76..=255 => script.extend_from_slice(&[0x4c, data.len() as u8]),
        256..=65535 => {
            script.push(0x4d);
            script.extend_from_slice(&(data.len() as u16).to_le_bytes());
        }
        _ => return Err("verified redeem script exceeds push-data limit".to_string()),
    }
    script.extend_from_slice(data);
    Ok(())
}
