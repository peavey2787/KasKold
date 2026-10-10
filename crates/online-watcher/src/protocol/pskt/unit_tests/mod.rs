// KasKold Companion Web — organized PSKT subsystem
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use serde_json::Value;

mod exact_json;

fn pskb_wire(document: Value) -> String {
    let document = crate::wasm_api::test_support::canonical_test_pskt(document);
    let json = serde_json::to_vec(&document).unwrap();
    let mut wire = b"PSKB".to_vec();
    wire.extend_from_slice(hex::encode(json).as_bytes());
    hex::encode(wire)
}

mod wire;
