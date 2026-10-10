mod commitment_cases;
mod fixture;
mod transcript_cases;

use fixture::TranscriptFixture;

#[test]
fn anti_klepto_native_boundary_accepts_complete_p2pk_and_p2sh_transcripts() {
    for fixture in [
        TranscriptFixture::p2pk_two_inputs(),
        TranscriptFixture::p2sh_multisig(),
    ] {
        assert_eq!(
            fixture
                .verify_public()
                .expect("valid anti-klepto transcript"),
            hex::encode(&fixture.signed_tx_wire)
        );
    }
}

#[test]
fn anti_klepto_begin_rejects_structurally_unsafe_compact_transactions() {
    let host_secret = [0x42u8; 32];
    for (label, wire, expected) in [
        (
            "no inputs",
            fixture::minimal_compact_transaction(0, 1, 0),
            "NoInputs",
        ),
        (
            "no outputs",
            fixture::minimal_compact_transaction(1, 0, 0),
            "NoOutputs",
        ),
        (
            "unsupported flags",
            fixture::minimal_compact_transaction(1, 1, 0x02),
            "InvalidFlags",
        ),
    ] {
        let error = crate::wasm_api::protocol::anti_klepto::anti_klepto_begin_with_secret_string(
            &hex::encode(wire),
            &host_secret,
        )
        .expect_err(label);
        assert!(error.contains(expected), "{label}: {error}");
    }
}

#[test]
fn anti_klepto_public_boundary_rejects_malformed_hex_and_secret_width() {
    let fixture = TranscriptFixture::p2pk_two_inputs();
    assert!(
        crate::wasm_api::protocol::anti_klepto::anti_klepto_accept_commitment_string(
            &fixture.request_hex(),
            &fixture.commitment_hex(&fixture.commitment_records),
            "00",
        )
        .is_err()
    );
    assert!(
        crate::wasm_api::protocol::anti_klepto::anti_klepto_verify_signed_string(
            "zz",
            &fixture.commitment_hex(&fixture.commitment_records),
            &fixture.signed_message_hex(&fixture.proofs, &fixture.signed_tx_wire),
            &fixture.host_secret_hex(),
        )
        .is_err()
    );
}

#[test]
fn anti_klepto_public_facade_rejects_malformed_inputs() {
    use crate::wasm_api::protocol::anti_klepto::{
        anti_klepto_accept_commitment, anti_klepto_begin, anti_klepto_verify_signed,
    };

    assert!(anti_klepto_begin("00").is_err());
    assert!(anti_klepto_accept_commitment("zz", "00", &"11".repeat(32)).is_err());
    assert!(anti_klepto_verify_signed("zz", "00", "00", &"11".repeat(32)).is_err());
}

#[test]
fn anti_klepto_public_begin_emits_a_request_bound_to_a_fresh_host_secret() {
    use crate::wasm_api::{protocol::anti_klepto::anti_klepto_begin, test_support::expect_js};

    let fixture = TranscriptFixture::p2pk_two_inputs();
    let begun = expect_js(
        anti_klepto_begin(&hex::encode(fixture.original_transaction())),
        "anti-klepto begin",
    );
    let begun: serde_json::Value = serde_json::from_str(&begun).expect("begin JSON");
    let secret = hex::decode(begun["hostSecretHex"].as_str().expect("secret")).unwrap();
    assert_eq!(secret.len(), 32);
    assert_ne!(secret, vec![0u8; 32]);
    let request = hex::decode(begun["requestHex"].as_str().expect("request")).unwrap();
    let parsed = shared_signer::anti_klepto::parse_request(&request).expect("request parses");
    assert_eq!(parsed.transaction, fixture.original_transaction());
}

#[test]
fn anti_klepto_transcript_rejects_unparseable_request_commitment_and_response() {
    use crate::wasm_api::protocol::anti_klepto::{
        anti_klepto_accept_commitment_string, anti_klepto_verify_signed_string,
    };

    let fixture = TranscriptFixture::p2pk_two_inputs();
    let commitment = fixture.commitment_hex(&fixture.commitment_records);
    let secret = fixture.host_secret_hex();
    let request_error =
        anti_klepto_accept_commitment_string("00", &commitment, &secret).unwrap_err();
    assert!(
        request_error.starts_with("invalid anti-klepto request"),
        "{request_error}"
    );
    let commitment_error =
        anti_klepto_accept_commitment_string(&fixture.request_hex(), "00", &secret).unwrap_err();
    assert!(
        commitment_error.starts_with("invalid signer commitment"),
        "{commitment_error}"
    );
    let response_error =
        anti_klepto_verify_signed_string(&fixture.request_hex(), &commitment, "00", &secret)
            .unwrap_err();
    assert!(
        response_error.starts_with("invalid signed response"),
        "{response_error}"
    );
    assert!(
        anti_klepto_accept_commitment_string(&fixture.request_hex(), &commitment, "zz")
            .unwrap_err()
            .starts_with("invalid host secret hex")
    );
}
