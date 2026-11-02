//! Journeys through what the device signs.
mod anti_klepto;
pub(in crate::scenarios::journeys) mod covenant;
pub(in crate::scenarios::journeys) mod multisig;
mod policies;
mod private_swap;
mod secrets;
mod shared_wallet_files;
pub(in crate::scenarios::journeys) mod signing;
mod stealth;
