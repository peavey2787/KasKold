#![no_main]

use kaskold_hardware_core::backup::stego_picture as stego;
use libfuzzer_sys::fuzz_target;

const CARRIERS: [&[u8]; 2] = [
    include_bytes!(
        "../../apps/kaskold-hardware/src/runtime/workflow_tests/fixtures/stego_flat.jpg"
    ),
    include_bytes!(
        "../../apps/kaskold-hardware/src/runtime/workflow_tests/fixtures/stego_noise.jpg"
    ),
];
const MAX_KEY: usize = 32;
const MAX_PAYLOAD: usize = 2048;

fn prefixed_round_trip(carrier: &[u8], key: &[u8], payload: &[u8]) {
    let mut photo = vec![0u8; carrier.len() + 64];
    let Ok(written) = stego::embed(carrier, payload, key, &mut photo) else {
        return;
    };
    let mut recovered = vec![0u8; payload.len()];
    let read = stego::extract(&photo[..written], key, &mut recovered)
        .expect("a photo embed wrote must extract under the same key");
    assert_eq!(&recovered[..read], payload);
}

fn exact_round_trip(carrier: &[u8], key: &[u8], payload: &[u8]) {
    let mut photo = vec![0u8; carrier.len() + 64];
    let Ok(written) = stego::embed_exact(carrier, payload, key, &mut photo) else {
        return;
    };
    let mut recovered = vec![0u8; payload.len()];
    stego::extract_exact(&photo[..written], key, &mut recovered)
        .expect("a photo embed_exact wrote must extract under the same key");
    assert_eq!(recovered, payload);
}

fuzz_target!(|data: &[u8]| {
    let [selector, key_length, rest @ ..] = data else {
        return;
    };
    let carrier = CARRIERS[usize::from(selector & 1)];
    let (key, payload) = rest.split_at(rest.len().min(usize::from(*key_length) % MAX_KEY + 1));
    let payload = &payload[..payload.len().min(MAX_PAYLOAD)];
    prefixed_round_trip(carrier, key, payload);
    exact_round_trip(carrier, key, payload);
});
