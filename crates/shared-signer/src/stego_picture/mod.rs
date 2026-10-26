//! JPEG DCT-coefficient carrier.

extern crate alloc;

use alloc::vec::Vec;

use codec::{bit_get, bit_set, walk, WalkMode};
use permutation::PositionPermutation;

mod codec;
mod frame;
mod huffman;
mod permutation;

const RANK_WINDOW: u32 = 40_000;
const LENGTH_PREFIX_BYTES: usize = 2;

fn try_zeroed_vec<T: Clone>(len: usize, value: T) -> Result<Vec<T>, PictureError> {
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| PictureError::AllocationFailed)?;
    out.resize(len, value);
    Ok(out)
}

fn try_vec_capacity<T>(capacity: usize) -> Result<Vec<T>, PictureError> {
    let mut out = Vec::new();
    out.try_reserve_exact(capacity)
        .map_err(|_| PictureError::AllocationFailed)?;
    Ok(out)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PictureError {
    Malformed,
    NotBaseline,
    NoCapacity,
    BufferTooSmall,
    Unencodable,
    AllocationFailed,
    WorkLimitExceeded,
}

impl PictureError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::Malformed => "Malformed JPEG",
            Self::NotBaseline => "Progressive JPEG unsupported",
            Self::NoCapacity => "Photo has insufficient capacity",
            Self::BufferTooSmall => "JPEG output buffer too small",
            Self::Unencodable => "JPEG table cannot encode payload",
            Self::AllocationFailed => "Not enough memory for JPEG operation",
            Self::WorkLimitExceeded => "JPEG dimensions exceed work budget",
        }
    }
}

pub fn capacity_bits(jpeg: &[u8], key: &[u8]) -> Result<u32, PictureError> {
    let frame = frame::parse(jpeg)?;
    let positions = frame.positions();
    let permutation = PositionPermutation::new(positions, key).ok_or(PictureError::Malformed)?;
    let rank_window = positions.min(RANK_WINDOW);
    let mut coefficients = try_zeroed_vec(rank_window as usize, 0i16)?;
    let mut present = try_zeroed_vec(bitmap_len(rank_window)?, 0u8)?;
    let mut mode = WalkMode::Collect {
        coefficients: &mut coefficients,
        present: &mut present,
    };
    walk(jpeg, &frame, &permutation, rank_window, &mut mode, None)?;
    let mut capacity = 0u32;
    for rank in 0..rank_window {
        if bit_get(&present, rank) && coefficients.get(rank as usize).copied().unwrap_or(0) != 0 {
            capacity += 1;
        }
    }
    Ok(capacity)
}

/// Embed `payload` behind a two-byte length prefix, for payloads whose length
/// the reader cannot know in advance.
pub fn embed(
    jpeg: &[u8],
    payload: &[u8],
    key: &[u8],
    output: &mut [u8],
) -> Result<usize, PictureError> {
    if payload.is_empty() || payload.len() > u16::MAX as usize {
        return Err(PictureError::NoCapacity);
    }
    let mut stream = try_vec_capacity(LENGTH_PREFIX_BYTES + payload.len())?;
    stream.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    stream.extend_from_slice(payload);
    let embedded = embed_stream(jpeg, &stream, key, output);
    crate::bytes::zeroize_bytes(&mut stream);
    embedded
}

/// Embed exactly `payload` with no length or marker, for formats whose
/// length the reader already knows; see [`extract_exact`].
pub fn embed_exact(
    jpeg: &[u8],
    payload: &[u8],
    key: &[u8],
    output: &mut [u8],
) -> Result<usize, PictureError> {
    if payload.is_empty() {
        return Err(PictureError::NoCapacity);
    }
    embed_stream(jpeg, payload, key, output)
}

fn embed_stream(
    jpeg: &[u8],
    stream: &[u8],
    key: &[u8],
    output: &mut [u8],
) -> Result<usize, PictureError> {
    let frame = frame::parse(jpeg)?;
    let positions = frame.positions();
    let permutation = PositionPermutation::new(positions, key).ok_or(PictureError::Malformed)?;
    let rank_window = positions.min(RANK_WINDOW);
    let (mut coefficients, present) = collect_window(jpeg, &frame, &permutation, rank_window)?;
    let changed = embed_payload_bits(stream, rank_window, &present, &mut coefficients)?;
    let length = write_embedded_jpeg(
        jpeg,
        output,
        &frame,
        &permutation,
        rank_window,
        &coefficients,
        &changed,
    )?;
    if let Err(error) = verify_embedded(&output[..length], stream, key) {
        crate::bytes::zeroize_bytes(&mut output[..length]);
        return Err(error);
    }
    Ok(length)
}

/// Read `stream` back out of a freshly written carrier, so a photo that would
/// not restore is never handed to the user.
fn verify_embedded(carrier: &[u8], stream: &[u8], key: &[u8]) -> Result<(), PictureError> {
    let mut readback = try_zeroed_vec(stream.len(), 0u8)?;
    let verified = extract_exact(carrier, key, &mut readback).and_then(|()| {
        if readback == stream {
            Ok(())
        } else {
            Err(PictureError::Unencodable)
        }
    });
    crate::bytes::zeroize_bytes(&mut readback);
    verified
}

fn collect_window(
    jpeg: &[u8],
    frame: &frame::Frame,
    permutation: &PositionPermutation,
    rank_window: u32,
) -> Result<(Vec<i16>, Vec<u8>), PictureError> {
    let mut coefficients = try_zeroed_vec(rank_window as usize, 0i16)?;
    let mut present = try_zeroed_vec(bitmap_len(rank_window)?, 0u8)?;
    let mut mode = WalkMode::Collect {
        coefficients: &mut coefficients,
        present: &mut present,
    };
    walk(jpeg, frame, permutation, rank_window, &mut mode, None)?;
    Ok((coefficients, present))
}

fn embed_payload_bits(
    stream: &[u8],
    rank_window: u32,
    present: &[u8],
    coefficients: &mut [i16],
) -> Result<Vec<u8>, PictureError> {
    let required_bits = stream.len().checked_mul(8).ok_or(PictureError::Malformed)?;
    let mut changed = try_zeroed_vec(bitmap_len(rank_window)?, 0u8)?;
    let mut payload_bit = 0usize;
    for rank in 0..rank_window {
        if payload_bit >= required_bits {
            break;
        }
        if consume_embedding_rank(
            stream,
            present,
            coefficients,
            &mut changed,
            rank,
            payload_bit,
        ) {
            payload_bit += 1;
        }
    }
    if payload_bit < required_bits {
        return Err(PictureError::NoCapacity);
    }
    Ok(changed)
}

fn consume_embedding_rank(
    stream: &[u8],
    present: &[u8],
    coefficients: &mut [i16],
    changed: &mut [u8],
    rank: u32,
    payload_bit: usize,
) -> bool {
    if !bit_get(present, rank) {
        return false;
    }
    let Some(value) = coefficients.get(rank as usize).copied().map(i32::from) else {
        return false;
    };
    if value == 0 {
        return false;
    }
    let Some(&byte) = stream.get(payload_bit / 8) else {
        return false;
    };
    let desired = (byte >> (7 - payload_bit % 8)) & 1;
    if (value.unsigned_abs() & 1) as u8 == desired {
        return true;
    }
    let changed_value = if value > 0 { value - 1 } else { value + 1 };
    let Some(slot) = coefficients.get_mut(rank as usize) else {
        return false;
    };
    *slot = changed_value as i16;
    bit_set(changed, rank);
    changed_value != 0
}

fn write_embedded_jpeg(
    jpeg: &[u8],
    output: &mut [u8],
    frame: &frame::Frame,
    permutation: &PositionPermutation,
    rank_window: u32,
    coefficients: &[i16],
    changed: &[u8],
) -> Result<usize, PictureError> {
    let (header_length, tail_length, available_scan) = embedded_layout(jpeg, output, frame)?;
    copy_header(jpeg, output, header_length)?;
    let scan_end = header_length
        .checked_add(available_scan)
        .ok_or(PictureError::Malformed)?;
    let scan_output = output
        .get_mut(header_length..scan_end)
        .ok_or(PictureError::BufferTooSmall)?;
    let scan_length = {
        let mut mode = WalkMode::Apply {
            coefficients,
            changed,
        };
        walk(
            jpeg,
            frame,
            permutation,
            rank_window,
            &mut mode,
            Some(scan_output),
        )?
    };
    copy_tail(
        jpeg,
        output,
        frame.scan_end,
        header_length,
        scan_length,
        tail_length,
    )
}

fn embedded_layout(
    jpeg: &[u8],
    output: &[u8],
    frame: &frame::Frame,
) -> Result<(usize, usize, usize), PictureError> {
    let header_length = frame.scan_start;
    let tail_length = jpeg
        .len()
        .checked_sub(frame.scan_end)
        .ok_or(PictureError::Malformed)?;
    let minimum = header_length
        .checked_add(tail_length)
        .ok_or(PictureError::Malformed)?;
    if output.len() < minimum {
        return Err(PictureError::BufferTooSmall);
    }
    let available_scan = output
        .len()
        .checked_sub(minimum)
        .ok_or(PictureError::BufferTooSmall)?;
    Ok((header_length, tail_length, available_scan))
}

fn copy_header(jpeg: &[u8], output: &mut [u8], header_length: usize) -> Result<(), PictureError> {
    let source = jpeg.get(..header_length).ok_or(PictureError::Malformed)?;
    let destination = output
        .get_mut(..header_length)
        .ok_or(PictureError::BufferTooSmall)?;
    destination.copy_from_slice(source);
    Ok(())
}

fn copy_tail(
    jpeg: &[u8],
    output: &mut [u8],
    scan_end: usize,
    header_length: usize,
    scan_length: usize,
    tail_length: usize,
) -> Result<usize, PictureError> {
    let data_end = header_length
        .checked_add(scan_length)
        .ok_or(PictureError::Malformed)?;
    let total = data_end
        .checked_add(tail_length)
        .ok_or(PictureError::Malformed)?;
    let source = jpeg.get(scan_end..).ok_or(PictureError::Malformed)?;
    let destination = output
        .get_mut(data_end..total)
        .ok_or(PictureError::BufferTooSmall)?;
    destination.copy_from_slice(source);
    Ok(total)
}

/// Extract a payload written by [`embed`]; returns its length.
pub fn extract(jpeg: &[u8], key: &[u8], output: &mut [u8]) -> Result<usize, PictureError> {
    decode_payload_bits(&carried_bits(jpeg, key)?, output)
}

/// Fill `output` with the first `output.len()` bytes carried under `key`, as
/// written by [`embed_exact`]. Any key reads some bytes; the payload format
/// must authenticate them.
pub fn extract_exact(jpeg: &[u8], key: &[u8], output: &mut [u8]) -> Result<(), PictureError> {
    let bits = carried_bits(jpeg, key)?;
    if output.is_empty() || output.len().checked_mul(8).ok_or(PictureError::Malformed)? > bits.len()
    {
        return Err(PictureError::Malformed);
    }
    write_payload_bytes(&bits, 0, output)
}

fn carried_bits(jpeg: &[u8], key: &[u8]) -> Result<Vec<u8>, PictureError> {
    let frame = frame::parse(jpeg)?;
    let positions = frame.positions();
    let permutation = PositionPermutation::new(positions, key).ok_or(PictureError::Malformed)?;
    let rank_window = positions.min(RANK_WINDOW);
    collect_payload_bits(jpeg, &frame, &permutation, rank_window)
}

fn collect_payload_bits(
    jpeg: &[u8],
    frame: &frame::Frame,
    permutation: &PositionPermutation,
    rank_window: u32,
) -> Result<Vec<u8>, PictureError> {
    let mut coefficients = try_zeroed_vec(rank_window as usize, 0i16)?;
    let mut present = try_zeroed_vec(bitmap_len(rank_window)?, 0u8)?;
    let mut mode = WalkMode::Collect {
        coefficients: &mut coefficients,
        present: &mut present,
    };
    walk(jpeg, frame, permutation, rank_window, &mut mode, None)?;
    payload_bits_from_window(&coefficients, &present, rank_window)
}

fn payload_bits_from_window(
    coefficients: &[i16],
    present: &[u8],
    rank_window: u32,
) -> Result<Vec<u8>, PictureError> {
    let mut bits = try_vec_capacity(rank_window as usize)?;
    for rank in 0..rank_window {
        if !bit_get(present, rank) {
            continue;
        }
        let value = i32::from(
            coefficients
                .get(rank as usize)
                .copied()
                .ok_or(PictureError::Malformed)?,
        );
        if value != 0 {
            bits.push((value.unsigned_abs() & 1) as u8);
        }
    }
    Ok(bits)
}

fn decode_payload_bits(bits: &[u8], output: &mut [u8]) -> Result<usize, PictureError> {
    let prefix_bits = LENGTH_PREFIX_BYTES
        .checked_mul(8)
        .ok_or(PictureError::Malformed)?;
    if bits.len() < prefix_bits {
        return Err(PictureError::Malformed);
    }
    let length = decoded_payload_length(bits)?;
    let required = LENGTH_PREFIX_BYTES
        .checked_add(length)
        .and_then(|value| value.checked_mul(8))
        .ok_or(PictureError::Malformed)?;
    if length == 0 || required > bits.len() {
        return Err(PictureError::Malformed);
    }
    if output.len() < length {
        return Err(PictureError::BufferTooSmall);
    }
    write_payload_bytes(bits, LENGTH_PREFIX_BYTES, &mut output[..length])?;
    Ok(length)
}

fn decoded_payload_length(bits: &[u8]) -> Result<usize, PictureError> {
    Ok((usize::from(byte_at(bits, 0)?) << 8) | usize::from(byte_at(bits, 1)?))
}

fn write_payload_bytes(bits: &[u8], offset: usize, output: &mut [u8]) -> Result<(), PictureError> {
    for (index, byte) in output.iter_mut().enumerate() {
        let framed_index = offset.checked_add(index).ok_or(PictureError::Malformed)?;
        *byte = byte_at(bits, framed_index)?;
    }
    Ok(())
}

fn byte_at(bits: &[u8], byte_index: usize) -> Result<u8, PictureError> {
    let base = byte_index.checked_mul(8).ok_or(PictureError::Malformed)?;
    let mut byte = 0u8;
    for bit_index in 0..8 {
        let index = base.checked_add(bit_index).ok_or(PictureError::Malformed)?;
        byte = (byte << 1) | bits.get(index).copied().ok_or(PictureError::Malformed)?;
    }
    Ok(byte)
}

fn bitmap_len(rank_window: u32) -> Result<usize, PictureError> {
    (rank_window as usize)
        .checked_add(7)
        .map(|n| n / 8)
        .ok_or(PictureError::Malformed)
}

#[cfg(test)]
mod unit_tests;
