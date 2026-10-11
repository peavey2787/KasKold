#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate std;

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{}};
}

/// Kaspa addresses, from Kaspa Portal.
pub mod address {
    pub use kaspa_portal::primitives::address::*;
}
pub mod crypto;
/// Key derivation, mnemonics and account keys, from Kaspa Portal.
pub mod derivation {
    pub use kaspa_portal::wallet::{
        derivation::{bip32, bip85, covenant, hmac},
        key::xpub,
        mnemonic::{bip39, wordlist as bip39_wordlist},
        multisig,
    };
}
pub mod facade;
pub mod privacy;
pub use privacy::pairing as privacy_pairing;
pub use privacy::stealth;
/// Transaction model, sighash, KSPT/PSKT interchange and signing, from Kaspa Portal.
pub mod transaction {
    pub use kaspa_portal::transaction::{
        interchange::{kspt, pskt::standard as std_pskt},
        model, sighash,
        signing::private_swap,
    };
}

/// Power-on known-answer self-tests for the signing core, run at boot.
pub use kaspa_portal::self_test;

pub use facade::{OfflineSigner, TransactionEnvelopeError};
