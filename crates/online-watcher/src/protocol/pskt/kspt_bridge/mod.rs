// KasKold Companion Web — organized PSKT subsystem
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

mod merger;
mod parser_compact;
mod parser_transaction;
mod relay;

pub use merger::merge_signed_kspt_into_pskb;
pub(crate) use parser_compact::xonly_at_position;
pub(crate) use parser_transaction::parse_compact_kspt_transaction;
#[cfg(test)]
pub(crate) use parser_transaction::require_compact_trailer_progress;
#[cfg(test)]
pub(crate) use parser_transaction::{decode_error_for_test, CompanionSink};
pub use relay::relay_pskb_as_kspt_hex_for_network;
