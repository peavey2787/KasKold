// KasKold Companion Web — PSKT / PSKB wire handling
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

mod envelope;
mod json;

pub(crate) use envelope::{decode_root, encode_root, pskt_from_root_mut};
