#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate std;

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{}};
}

pub mod address;
pub mod crypto;
pub mod derivation;
pub mod facade;
pub mod privacy;
pub use privacy::pairing as privacy_pairing;
pub use privacy::stealth;
pub mod transaction;

pub use facade::{OfflineSigner, TransactionEnvelopeError};
