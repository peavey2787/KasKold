//! Compile-time narrowing of integer constants.
//!
//! A protocol limit is often declared as `usize` for indexing but advertised in
//! a narrower wire field. [`narrow!`](crate::narrow) converts such a constant
//! inside an inline `const` block, so a value that does not fit fails the build
//! instead of truncating silently.

/// Narrow an unsigned constant to `u8`, `u16` or `u32` at compile time.
///
/// The expansion is an inline `const` block, so the range check always runs
/// during compilation and never at run time.
#[macro_export]
macro_rules! narrow {
    (u8, $value:expr) => {
        const { $crate::numeric::to_u8(($value) as u64) }
    };
    (u16, $value:expr) => {
        const { $crate::numeric::to_u16(($value) as u64) }
    };
    (u32, $value:expr) => {
        const { $crate::numeric::to_u32(($value) as u64) }
    };
}

/// `value` as a `u8`; evaluating it on a value that does not fit panics, which
/// inside [`narrow!`](crate::narrow) is a compile error.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the assertion above rules truncation out"
)]
pub const fn to_u8(value: u64) -> u8 {
    assert!(value <= u8::MAX as u64, "constant does not fit in u8");
    value as u8
}

/// `value` as a `u16`; evaluating it on a value that does not fit panics,
/// which inside [`narrow!`](crate::narrow) is a compile error.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the assertion above rules truncation out"
)]
pub const fn to_u16(value: u64) -> u16 {
    assert!(value <= u16::MAX as u64, "constant does not fit in u16");
    value as u16
}

/// `value` as a `u32`; evaluating it on a value that does not fit panics,
/// which inside [`narrow!`](crate::narrow) is a compile error.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the assertion above rules truncation out"
)]
pub const fn to_u32(value: u64) -> u32 {
    assert!(value <= u32::MAX as u64, "constant does not fit in u32");
    value as u32
}
