//! Integrity-protected configuration journal codec and selection.

use crate::services::credential_policy::{MAX_UNLOCK_FAILURES, SALT_SIZE};
use crate::wallet::seed_manager::{MAX_SLOTS, WALLET_NAME_MAX};
use sha2::{Digest, Sha256};
mod multisig;
mod sd_anchor;

use multisig::{decode_multisig_store, encode_multisig_store};
use sd_anchor::{decode_sd_anchor, encode_sd_anchor};

use super::{
    flash, is_newer, AlignedBytes, ConfigHeader, DeviceFlags, DeviceFlash, DevicePreferences, FlashError, SdAnchor,
    StorageMode, WalletActivationRecord, WalletActivationRecords, WalletLabels, CONFIG_DIGEST_END, CONFIG_DIGEST_START,
    CONFIG_MAGIC, CONFIG_SIZE, CONFIG_VERSION, DEVICE_FLAGS_OFFSET, DEVICE_PREFERENCES_OFFSET, DIM_TIMEOUT_OFFSET,
    MULTISIG_DESCRIPTOR_BYTES, MULTISIG_DESCRIPTOR_END, MULTISIG_DESCRIPTOR_ENTRY_SIZE, MULTISIG_DESCRIPTOR_OFFSET,
    UNLOCK_FAILURES_OFFSET, WALLET_ACTIVATION_OFFSET, WALLET_ACTIVATION_RECORD_SIZE, WALLET_LABELS_END,
    WALLET_LABELS_OFFSET, WALLET_LABEL_RECORD_SIZE,
};

pub(in crate::services::persistent_wallet) fn read_mode(
    device: &mut DeviceFlash<'_>,
) -> Result<Option<StorageMode>, FlashError> {
    let a = read_config(device, flash::CONFIG_A)?;
    let b = read_config(device, flash::CONFIG_B)?;
    Ok(newest_config(a, b).and_then(|header| header.mode))
}

pub(in crate::services::persistent_wallet) fn read_sd_anchor(
    device: &mut DeviceFlash<'_>,
) -> Result<Option<SdAnchor>, FlashError> {
    let a = read_config(device, flash::CONFIG_A)?;
    let b = read_config(device, flash::CONFIG_B)?;
    Ok(newest_config(a, b).and_then(
        |header| {
            if header.mode == Some(StorageMode::SdCard) {
                header.sd_anchor
            } else {
                None
            }
        },
    ))
}

pub(in crate::services::persistent_wallet) fn read_device_flags(
    device: &mut DeviceFlash<'_>,
) -> Result<DeviceFlags, FlashError> {
    Ok(current_config(device)?.map(|header| header.device_flags).unwrap_or_default())
}

pub(in crate::services::persistent_wallet) fn read_device_preferences(
    device: &mut DeviceFlash<'_>,
) -> Result<DevicePreferences, FlashError> {
    Ok(current_config(device)?.map(|header| header.device_preferences).unwrap_or_default())
}

pub(in crate::services::persistent_wallet) fn read_wallet_labels(
    device: &mut DeviceFlash<'_>,
) -> Result<WalletLabels, FlashError> {
    Ok(current_config(device)?.map(|header| header.wallet_labels).unwrap_or_default())
}

pub(in crate::services::persistent_wallet) fn read_wallet_activation(
    device: &mut DeviceFlash<'_>,
) -> Result<WalletActivationRecords, FlashError> {
    Ok(current_config(device)?.map(|header| header.wallet_activation).unwrap_or_default())
}

/// Durable count of consecutive failed unlock attempts.
pub(in crate::services::persistent_wallet) fn read_unlock_failures(
    device: &mut DeviceFlash<'_>,
) -> Result<u8, FlashError> {
    Ok(current_config(device)?.map_or(0, |header| header.unlock_failures))
}

pub(in crate::services::persistent_wallet) fn write_unlock_failures(
    device: &mut DeviceFlash<'_>,
    failures: u8,
) -> Result<(), FlashError> {
    update_config(device, |header| header.unlock_failures = failures)
}

pub(in crate::services::persistent_wallet) fn write_mode(
    device: &mut DeviceFlash<'_>,
    mode: StorageMode,
) -> Result<(), FlashError> {
    update_config(device, |header| {
        header.mode = Some(mode);
        header.sd_anchor = None;
    })
}

pub(in crate::services::persistent_wallet) fn write_wallet_activation(
    device: &mut DeviceFlash<'_>,
    records: WalletActivationRecords,
) -> Result<(), FlashError> {
    update_config(device, |header| header.wallet_activation = records)
}

pub(in crate::services::persistent_wallet) fn write_wallet_labels(
    device: &mut DeviceFlash<'_>,
    labels: WalletLabels,
) -> Result<(), FlashError> {
    update_config(device, |header| header.wallet_labels = labels)
}

#[cfg(feature = "m5stack")]
pub(in crate::services::persistent_wallet) fn write_device_flags(
    device: &mut DeviceFlash<'_>,
    flags: DeviceFlags,
) -> Result<(), FlashError> {
    update_config(device, |header| header.device_flags = flags)
}

pub(in crate::services::persistent_wallet) fn write_device_preferences(
    device: &mut DeviceFlash<'_>,
    preferences: DevicePreferences,
) -> Result<(), FlashError> {
    update_config(device, |header| header.device_preferences = preferences)
}

pub(in crate::services::persistent_wallet) fn write_sd_anchor(
    device: &mut DeviceFlash<'_>,
    anchor: SdAnchor,
) -> Result<(), FlashError> {
    update_config(device, |header| {
        header.mode = Some(StorageMode::SdCard);
        header.sd_anchor = Some(anchor);
    })
}

/// Read-modify-write of the newest configuration record: every field the
/// edit leaves alone is carried forward unchanged.
fn update_config(device: &mut DeviceFlash<'_>, edit: impl FnOnce(&mut ConfigHeader)) -> Result<(), FlashError> {
    let mut header = current_config(device)?.unwrap_or(ConfigHeader::EMPTY);
    edit(&mut header);
    write_config(device, &header)
}

pub(super) fn write_config(device: &mut DeviceFlash<'_>, header: &ConfigHeader) -> Result<(), FlashError> {
    let a = read_config(device, flash::CONFIG_A)?;
    let b = read_config(device, flash::CONFIG_B)?;
    let sequence = newest_config(a, b).map_or(1, |newest| newest.sequence.wrapping_add(1));
    let target = match (a, b) {
        (None, _) => flash::CONFIG_A,
        (_, None) => flash::CONFIG_B,
        (Some(left), Some(right)) if is_newer(left.sequence, right.sequence) => flash::CONFIG_B,
        _ => flash::CONFIG_A,
    };
    let mut record = AlignedBytes::<CONFIG_SIZE>::zeroed();
    record.0[..8].copy_from_slice(&CONFIG_MAGIC);
    record.0[8] = CONFIG_VERSION;
    record.0[9] = header.mode.map_or(0, |value| value as u8);
    record.0[DEVICE_FLAGS_OFFSET] = header.device_flags.0;
    record.0[DEVICE_PREFERENCES_OFFSET] = header.device_preferences.flags;
    record.0[DIM_TIMEOUT_OFFSET] = header.device_preferences.dim_timeout_code;
    record.0[UNLOCK_FAILURES_OFFSET] = header.unlock_failures;
    record.0[12..16].copy_from_slice(&sequence.to_le_bytes());
    if let Some(anchor) = header.sd_anchor {
        encode_sd_anchor(anchor, &mut record.0);
    }
    encode_wallet_labels(header.wallet_labels, &mut record.0);
    encode_wallet_activation(header.wallet_activation, &mut record.0);
    record.0[MULTISIG_DESCRIPTOR_OFFSET..MULTISIG_DESCRIPTOR_END].copy_from_slice(&header.multisig_descriptors);
    let digest = Sha256::digest(&record.0[..CONFIG_DIGEST_START]);
    record.0[CONFIG_DIGEST_START..CONFIG_DIGEST_END].copy_from_slice(&digest);
    device.replace_sector(target, &record)
}

pub(super) fn encode_wallet_labels(labels: WalletLabels, out: &mut [u8; CONFIG_SIZE]) {
    for index in 0..MAX_SLOTS {
        let base = WALLET_LABELS_OFFSET + index * WALLET_LABEL_RECORD_SIZE;
        let len = usize::from(labels.lengths[index]).min(WALLET_NAME_MAX);
        out[base] = len as u8;
        out[base + 1..base + 1 + len].copy_from_slice(&labels.names[index][..len]);
    }
}

pub(super) fn decode_wallet_labels(record: &[u8; CONFIG_SIZE]) -> Option<WalletLabels> {
    let mut labels = WalletLabels::empty();
    for index in 0..MAX_SLOTS {
        let base = WALLET_LABELS_OFFSET + index * WALLET_LABEL_RECORD_SIZE;
        let len = usize::from(record[base]);
        if len > WALLET_NAME_MAX {
            return None;
        }
        let name = &record[base + 1..base + 1 + WALLET_NAME_MAX];
        if name[len..].iter().any(|byte| *byte != 0) || core::str::from_utf8(&name[..len]).is_err() {
            return None;
        }
        labels.lengths[index] = len as u8;
        labels.names[index][..len].copy_from_slice(&name[..len]);
    }
    Some(labels)
}

pub(super) fn encode_wallet_activation(records: WalletActivationRecords, out: &mut [u8; CONFIG_SIZE]) {
    for index in 0..MAX_SLOTS {
        let base = WALLET_ACTIVATION_OFFSET + index * WALLET_ACTIVATION_RECORD_SIZE;
        if !records.present[index] {
            continue;
        }
        out[base] = 1;
        out[base + 1..base + 1 + SALT_SIZE].copy_from_slice(&records.records[index].salt);
        out[base + 1 + SALT_SIZE..base + WALLET_ACTIVATION_RECORD_SIZE]
            .copy_from_slice(&records.records[index].verifier);
    }
}

pub(super) fn decode_wallet_activation(record: &[u8; CONFIG_SIZE]) -> Option<WalletActivationRecords> {
    let mut records = WalletActivationRecords::empty();
    for index in 0..MAX_SLOTS {
        let base = WALLET_ACTIVATION_OFFSET + index * WALLET_ACTIVATION_RECORD_SIZE;
        match record[base] {
            0 => {
                if record[base + 1..base + WALLET_ACTIVATION_RECORD_SIZE].iter().any(|byte| *byte != 0) {
                    return None;
                }
            }
            1 => {
                let mut salt = [0u8; SALT_SIZE];
                salt.copy_from_slice(&record[base + 1..base + 1 + SALT_SIZE]);
                let mut verifier = [0u8; 32];
                verifier.copy_from_slice(&record[base + 1 + SALT_SIZE..base + WALLET_ACTIVATION_RECORD_SIZE]);
                records.present[index] = true;
                records.records[index] = WalletActivationRecord { salt, verifier };
            }
            _ => return None,
        }
    }
    Some(records)
}

pub(in crate::services::persistent_wallet) fn read_multisig_store(
    device: &mut DeviceFlash<'_>,
) -> Result<offline_signer::transaction::model::MultisigStore, FlashError> {
    let bytes =
        current_config(device)?.map(|header| header.multisig_descriptors).unwrap_or([0; MULTISIG_DESCRIPTOR_BYTES]);
    Ok(decode_multisig_store(&bytes).unwrap_or_else(offline_signer::transaction::model::MultisigStore::new))
}

pub(in crate::services::persistent_wallet) fn write_multisig_store(
    device: &mut DeviceFlash<'_>,
    store: &offline_signer::transaction::model::MultisigStore,
) -> Result<(), FlashError> {
    let mut bytes = [0u8; MULTISIG_DESCRIPTOR_BYTES];
    encode_multisig_store(store, &mut bytes);
    update_config(device, |header| header.multisig_descriptors = bytes)
}

pub(super) fn read_config(device: &mut DeviceFlash<'_>, address: u32) -> Result<Option<ConfigHeader>, FlashError> {
    let mut record = AlignedBytes::<CONFIG_SIZE>::zeroed();
    device.read(address, &mut record)?;
    if record.0[..8] != CONFIG_MAGIC || record.0[8] != CONFIG_VERSION {
        return Ok(None);
    }
    read_current_config(&record.0)
}

pub(super) fn contains_unknown_device_flags(device_flags: DeviceFlags) -> bool {
    has_unknown_bits(device_flags.0, DeviceFlags::KNOWN_MASK)
}

pub(super) fn has_unknown_bits(value: u8, known_mask: u8) -> bool {
    value & !known_mask != 0
}

pub(super) fn read_current_config(record: &[u8; CONFIG_SIZE]) -> Result<Option<ConfigHeader>, FlashError> {
    let mode = if record[9] == 0 { None } else { StorageMode::from_byte(record[9]) };
    if record[9] != 0 && mode.is_none() {
        return Ok(None);
    }
    let device_flags = DeviceFlags(record[DEVICE_FLAGS_OFFSET]);
    if contains_unknown_device_flags(device_flags) {
        return Ok(None);
    }

    let Some(device_preferences) = decode_preferences(record) else {
        return Ok(None);
    };
    if !config_digest_valid(record) {
        return Ok(None);
    }
    let Some(wallet_labels) = decode_wallet_labels(record) else {
        return Ok(None);
    };
    let Some(wallet_activation) = decode_wallet_activation(record) else {
        return Ok(None);
    };
    let mut multisig_descriptors = [0u8; MULTISIG_DESCRIPTOR_BYTES];
    multisig_descriptors.copy_from_slice(&record[MULTISIG_DESCRIPTOR_OFFSET..MULTISIG_DESCRIPTOR_END]);
    let unlock_failures = record[UNLOCK_FAILURES_OFFSET];
    if unlock_failures > MAX_UNLOCK_FAILURES {
        return Ok(None);
    }
    let sequence = u32::from_le_bytes(record[12..16].try_into().map_err(|_| FlashError::Read)?);
    let sd_anchor = if mode == Some(StorageMode::SdCard) {
        decode_sd_anchor(record)?
    } else {
        if !empty_config_body_valid(record) {
            return Ok(None);
        }
        None
    };
    Ok(Some(ConfigHeader {
        mode,
        sequence,
        sd_anchor,
        device_flags,
        device_preferences,
        wallet_labels,
        wallet_activation,
        multisig_descriptors,
        unlock_failures,
    }))
}

pub(super) fn decode_preferences(record: &[u8; CONFIG_SIZE]) -> Option<DevicePreferences> {
    let preferences =
        DevicePreferences { flags: record[DEVICE_PREFERENCES_OFFSET], dim_timeout_code: record[DIM_TIMEOUT_OFFSET] };
    let valid = preferences.flags & !DevicePreferences::KNOWN_MASK == 0
        && preferences.dim_timeout_code <= 4
        && preferences.wallet_network_code_valid();
    valid.then_some(preferences)
}

pub(super) fn config_digest_valid(record: &[u8; CONFIG_SIZE]) -> bool {
    let digest = Sha256::digest(&record[..CONFIG_DIGEST_START]);
    digest[..] == record[CONFIG_DIGEST_START..CONFIG_DIGEST_END]
}

/// Without an SD anchor, every byte no field owns must be zero. The labels,
/// activation records and multisig descriptors are real fields in every mode.
pub(super) fn empty_config_body_valid(record: &[u8; CONFIG_SIZE]) -> bool {
    record[10] == 0
        && record[11] == 0
        && all_zero(&record[16..DEVICE_FLAGS_OFFSET])
        && all_zero(&record[DEVICE_FLAGS_OFFSET + 1..DEVICE_PREFERENCES_OFFSET])
        && all_zero(&record[UNLOCK_FAILURES_OFFSET + 1..WALLET_LABELS_OFFSET])
        && all_zero(&record[WALLET_LABELS_END..WALLET_ACTIVATION_OFFSET])
        && all_zero(&record[MULTISIG_DESCRIPTOR_END..CONFIG_DIGEST_START])
}

fn all_zero(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| *byte == 0)
}

pub(super) fn current_config(device: &mut DeviceFlash<'_>) -> Result<Option<ConfigHeader>, FlashError> {
    let a = read_config(device, flash::CONFIG_A)?;
    let b = read_config(device, flash::CONFIG_B)?;
    Ok(newest_config(a, b))
}

pub(super) fn newest_config(a: Option<ConfigHeader>, b: Option<ConfigHeader>) -> Option<ConfigHeader> {
    match (a, b) {
        (Some(left), Some(right)) if is_newer(left.sequence, right.sequence) => Some(left),
        (Some(_), Some(right)) => Some(right),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}
