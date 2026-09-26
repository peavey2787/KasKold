//! JPEG DCT-coefficient steganographic carrier.
//!
//! The canonical codec lives in `shared-signer` so hardware and software Vaults
//! use exactly the same parser, capacity rules, permutation, and embedding code.

pub use shared_signer::stego_picture::*;
