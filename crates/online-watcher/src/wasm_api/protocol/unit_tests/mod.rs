mod kspt_relay;
#[test]
fn wasm_qr_boundaries_cover_generation_decoding_progress_and_version() {
    use super::qr::{
        decode_qr_frame, decoder_progress, generate_qr_frames, generate_qr_svg_text,
        reset_qr_decoder, version,
    };

    reset_qr_decoder();
    assert_eq!(decoder_progress(), "0/0");
    assert_eq!(version(), "KasKold Companion Web");

    let frames = crate::wasm_api::test_support::expect_js(
        generate_qr_frames(&hex::encode([0x41; 32])),
        "QR frames",
    );
    let frames: serde_json::Value = serde_json::from_str(&frames).unwrap();
    assert_eq!(frames.as_array().unwrap().len(), 1);
    let frame_hex = frames[0]["frame_hex"].as_str().expect("frame hex");
    assert_eq!(
        hex::decode(frame_hex).expect("valid frame hex"),
        vec![0x41; 32]
    );
    assert!(
        crate::wasm_api::test_support::expect_js(generate_qr_svg_text("hello"), "QR SVG")
            .starts_with("<svg")
    );

    // KasKold emits small payloads as raw single-frame QR binary rather than
    // wrapping them in KQ session framing. The watcher must accept that exact
    // path (anti-klepto v2 commitments are typically ~129 bytes for one input).
    let raw_single = b"KAKP\x02\x02single-frame-anti-klepto";
    assert_eq!(
        crate::wasm_api::test_support::expect_js(
            decode_qr_frame(&hex::encode(raw_single)),
            "raw QR frame"
        ),
        hex::encode(raw_single),
    );
    assert_eq!(decoder_progress(), "0/0");

    let payload = b"two-frame-boundary";
    let session = shared_signer::qr_frame::session_id(payload);
    let mut first = [0u8; 64];
    let mut second = [0u8; 64];
    let first_len =
        shared_signer::qr_frame::encode_frame(&session, 0, 2, &payload[..8], &mut first).unwrap();
    let second_len =
        shared_signer::qr_frame::encode_frame(&session, 1, 2, &payload[8..], &mut second).unwrap();
    assert_eq!(
        crate::wasm_api::test_support::expect_js(
            decode_qr_frame(&hex::encode(&first[..first_len])),
            "first QR frame"
        ),
        ""
    );
    assert!(decoder_progress().contains("\"count\":1"));
    assert_eq!(
        crate::wasm_api::test_support::expect_js(
            decode_qr_frame(&hex::encode(&second[..second_len])),
            "second QR frame"
        ),
        hex::encode(payload),
    );
    assert_eq!(decoder_progress(), "0/0");
}

#[test]
fn pskt_and_payload_native_cores_reject_malformed_inputs_without_jsvalue() {
    use super::pskt::{
        pskt_detect, pskt_finalize_and_broadcast_string, pskt_merge_signed_kspt_string,
        pskt_relay_to_kspt_string, pskt_summary_string,
    };
    use crate::wasm_api::{
        contracts::payload::{build_covenant_payload_string, parse_covenant_payload_string},
        test_support::ready,
    };

    assert_eq!(pskt_detect("00"), "unknown");
    assert!(pskt_summary_string("00", "mainnet").is_err());
    assert!(pskt_relay_to_kspt_string("00", "mainnet").is_err());
    assert!(pskt_merge_signed_kspt_string("00", "00").is_err());
    assert!(ready(pskt_finalize_and_broadcast_string("00", "ws://unused")).is_err());

    let payload = build_covenant_payload_string(7, "aabb").expect("payload");
    assert_eq!(payload, "0107aabb");
    let decoded = parse_covenant_payload_string(&payload).expect("decoded payload");
    assert!(decoded.contains("\"covenant_type\":7"));
    assert!(build_covenant_payload_string(1, "zz").is_err());
    assert!(parse_covenant_payload_string("00").is_err());
}

mod anti_klepto_transcript;
