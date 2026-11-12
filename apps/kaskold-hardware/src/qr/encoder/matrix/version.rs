#[cfg(not(feature = "qemu"))]
use super::super::constants::NUMERIC_CAPACITY;
use super::super::{constants::BYTE_CAPACITY, error::QrError};

/// Select the minimum QR version for byte-mode data at ECC level L.
pub fn select_version(data_len: usize) -> Result<u8, QrError> {
    select_from_capacities(data_len, &BYTE_CAPACITY)
}

/// Select the minimum QR version for numeric-mode data at ECC level L.
#[cfg(not(feature = "qemu"))]
pub fn select_version_numeric(digit_count: usize) -> Result<u8, QrError> {
    select_from_capacities(digit_count, &NUMERIC_CAPACITY)
}

fn select_from_capacities(length: usize, capacities: &[usize; 6]) -> Result<u8, QrError> {
    capacities
        .iter()
        .zip(1u8..)
        .find_map(|(capacity, version)| (length <= *capacity).then_some(version))
        .ok_or(QrError::DataTooLong)
}
