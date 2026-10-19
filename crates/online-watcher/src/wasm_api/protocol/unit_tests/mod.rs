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
    assert_eq!(
        pskt_summary_string("00", "mainnet")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Not a PSKT/PSKB payload")
    );
    assert_eq!(
        pskt_relay_to_kspt_string("00", "mainnet")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Encoding: payload too short")
    );
    assert_eq!(
        pskt_merge_signed_kspt_string("00", "00")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("KSPT is truncated")
    );
    assert_eq!(
        ready(pskt_finalize_and_broadcast_string("00", "ws://unused"))
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("PSKT cryptographic verification failed: payload too short")
    );

    let payload = build_covenant_payload_string(7, "aabb").expect("payload");
    assert_eq!(payload, "0107aabb");
    let decoded = parse_covenant_payload_string(&payload).expect("decoded payload");
    assert!(decoded.contains("\"covenant_type\":7"));
    assert_eq!(
        build_covenant_payload_string(1, "zz")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Bad params hex: Invalid character 'z' at position 0")
    );
    assert_eq!(
        parse_covenant_payload_string("00")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Payload too short")
    );
}

#[test]
fn pskb_document_and_output_cores_go_through_portal() {
    use super::pskt::{pskb_append_output_string, pskb_encode_document_string, pskt_detect};

    let document = r#"{"global":{"outputCount":0},"inputs":[],"outputs":[]}"#;
    let wire = pskb_encode_document_string(document).expect("PSKB wire");
    assert_eq!(pskt_detect(&wire), "pskb");
    let appended = pskb_append_output_string(&wire, 7, "000051").expect("appended");
    assert_ne!(appended, wire);
    assert_eq!(
        pskb_encode_document_string("[")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("PSKT JSON: EOF while parsing a list at line 1 column 1")
    );
    assert_eq!(
        pskb_append_output_string(&wire, 7, "010051")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("script public key must be version 0")
    );
    assert_eq!(
        pskb_append_output_string(&wire, 7, "0000zz")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("script hex: Invalid character 'z' at position 0")
    );
}

#[test]
fn pskb_document_and_output_exports_wrap_the_cores() {
    use super::pskt::{pskb_append_output, pskb_encode_document};
    use crate::wasm_api::test_support::expect_js;

    let document = r#"{"global":{"outputCount":0},"inputs":[],"outputs":[]}"#;
    let wire = expect_js(pskb_encode_document(document), "PSKB wire");
    let appended = expect_js(pskb_append_output(&wire, 7, "000051"), "appended");
    assert_ne!(appended, wire);
    assert_eq!(
        pskb_encode_document("[")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("PSKT JSON: EOF while parsing a list at line 1 column 1"))"#)
    );
    assert_eq!(
        pskb_append_output(&wire, 7, "0000zz")
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(r#"JsValue(Some("script hex: Invalid character 'z' at position 0"))"#)
    );
}

mod anti_klepto_transcript;
