use crate::backup::stego_picture::{capacity_bits, embed, extract};

const NOISE_JPEG: &[u8] = include_bytes!(
    "../../../../apps/kaskold-hardware/src/runtime/workflow_tests/fixtures/stego_noise.jpg"
);
const KEY: &[u8] = b"KasKold hardware shim coverage";

#[test]
fn hardware_shim_round_trips_with_shared_codec_in_both_directions() {
    let payload = [0x5au8; 179];

    let mut encoded = std::vec![0u8; NOISE_JPEG.len() * 2 + 4_096];
    let encoded_len = embed(NOISE_JPEG, &payload, KEY, &mut encoded).expect("shim embed");
    let mut decoded = [0u8; 179];
    let decoded_len = shared_signer::stego_picture::extract(
        &encoded[..encoded_len],
        KEY,
        &mut decoded,
    )
    .expect("shared extract of shim output");
    assert_eq!(decoded_len, payload.len());
    assert_eq!(decoded, payload);

    let mut shared_encoded = std::vec![0u8; NOISE_JPEG.len() * 2 + 4_096];
    let shared_len = shared_signer::stego_picture::embed(
        NOISE_JPEG,
        &payload,
        KEY,
        &mut shared_encoded,
    )
    .expect("shared embed");
    let mut shim_decoded = [0u8; 179];
    let shim_decoded_len = extract(&shared_encoded[..shared_len], KEY, &mut shim_decoded)
        .expect("shim extract of shared output");
    assert_eq!(shim_decoded_len, payload.len());
    assert_eq!(shim_decoded, payload);
}

#[test]
fn malformed_and_truncated_picture_inputs_fail_without_panicking() {
    let key = b"stego-picture-property";
    let mut output = [0u8; 256];
    let corpus: [&[u8]; 8] = [
        b"",
        b"\xff",
        b"\xff\xd8",
        b"\xff\xd8\xff\xd9",
        b"not-a-jpeg",
        b"\xff\xd8\xff\xda\x00\x00",
        b"\xff\xd8\xff\xc0\xff\xff",
        b"\xff\xd8\xff\xc4\xff\xff",
    ];
    for input in corpus {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = capacity_bits(input, key);
            let _ = extract(input, key, &mut output);
        }));
        assert!(
            result.is_ok(),
            "stego parser panicked for malformed corpus input"
        );
    }
}

#[test]
fn deterministic_external_input_sweep_is_panic_free() {
    let key = b"stego-picture-sweep";
    let mut data = [0u8; 512];
    let mut output = [0u8; 256];
    let mut state = 0x6d2b_79f5u32;
    for length in 0..=data.len() {
        for byte in data.iter_mut().take(length) {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }
        let input = &data[..length];
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = capacity_bits(input, key);
            let _ = extract(input, key, &mut output);
        }));
        assert!(
            result.is_ok(),
            "stego parser panicked at deterministic length {length}"
        );
    }
}
