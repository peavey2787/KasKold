// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Saturating conversions from buffer indices and counts to display geometry.
//!
//! Screen geometry is a few hundred pixels, so every index that reaches these
//! helpers fits; saturating keeps an out-of-range value off-screen instead of
//! wrapping it back onto the panel.

/// A `usize` index or count as a signed pixel coordinate.
pub(crate) fn coord(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// A `usize` length as an unsigned pixel extent.
pub(crate) fn extent(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
