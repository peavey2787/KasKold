//! The Companion encodes these messages in JavaScript and firmware reads them
//! through Portal. Both languages check this one file, so neither can drift alone.
use kaspa_portal::transaction::signing::covenant::protocol::{
    encode_request, encode_response, encode_reveal, parse_request, parse_response, parse_reveal,
    RESPONSE_LEN, REVEAL_LEN,
};
use serde_json::Value;

const VECTORS: &str =
    include_str!("../../../../../../qa/contracts/parity/covenant_wire_vectors.json");

fn vectors() -> Value {
    serde_json::from_str(VECTORS).expect("vectors parse")
}

fn bytes(vector: &Value, field: &str) -> Vec<u8> {
    hex::decode(vector[field].as_str().expect(field)).expect(field)
}

fn number(vector: &Value, field: &str) -> u8 {
    u8::try_from(vector[field].as_u64().expect(field)).expect(field)
}

#[test]
fn portal_reads_and_reproduces_every_request_the_companion_sends() {
    for vector in vectors()["requests"].as_array().expect("requests") {
        let wire = bytes(vector, "wire");
        let request = parse_request(&wire).expect("a request");
        let name = vector["name"].as_str().expect("name");
        assert_eq!(request.kind as u8, number(vector, "kind"), "{name}");
        assert_eq!(request.scheme as u8, number(vector, "scheme"), "{name}");
        assert_eq!(request.binding as u8, number(vector, "binding"), "{name}");
        assert_eq!(
            request.session_id.to_vec(),
            bytes(vector, "session_id"),
            "{name}"
        );
        assert_eq!(
            request.host_commitment.to_vec(),
            bytes(vector, "host_commitment"),
            "{name}"
        );
        assert_eq!(request.key_id.to_vec(), bytes(vector, "key_id"), "{name}");
        assert_eq!(
            request.binding_token.to_vec(),
            bytes(vector, "binding_token"),
            "{name}"
        );
        assert_eq!(
            request.commitment.to_vec(),
            bytes(vector, "commitment"),
            "{name}"
        );
        assert_eq!(request.script.to_vec(), bytes(vector, "script"), "{name}");
        assert_eq!(request.context.to_vec(), bytes(vector, "context"), "{name}");
        let mut encoded = vec![0u8; wire.len()];
        let length = encode_request(&request, &mut encoded).expect("re-encoded request");
        assert_eq!(encoded[..length], wire[..], "{name}");
    }
}

#[test]
fn portal_reads_and_reproduces_the_reveal_the_companion_sends() {
    let vectors = vectors();
    let vector = &vectors["reveal"];
    let wire = bytes(vector, "wire");
    let reveal = parse_reveal(&wire).expect("a reveal");
    assert_eq!(reveal.session_id.to_vec(), bytes(vector, "session_id"));
    assert_eq!(reveal.key_id.to_vec(), bytes(vector, "key_id"));
    assert_eq!(reveal.commitment.to_vec(), bytes(vector, "commitment"));
    assert_eq!(reveal.host_secret.to_vec(), bytes(vector, "host_secret"));
    let mut encoded = [0u8; REVEAL_LEN];
    encode_reveal(&reveal, &mut encoded).expect("re-encoded reveal");
    assert_eq!(encoded.to_vec(), wire);
}

#[test]
fn portal_writes_the_exact_responses_the_companion_reads() {
    for vector in vectors()["responses"].as_array().expect("responses") {
        let wire = bytes(vector, "wire");
        let response = parse_response(&wire).expect("a response");
        assert_eq!(response.kind as u8, number(vector, "kind"));
        assert_eq!(response.session_id.to_vec(), bytes(vector, "session_id"));
        assert_eq!(response.key_id.to_vec(), bytes(vector, "key_id"));
        assert_eq!(response.pubkey_x.to_vec(), bytes(vector, "pubkey_x"));
        assert_eq!(
            response.binding_token.to_vec(),
            bytes(vector, "binding_token")
        );
        assert_eq!(response.commitment.to_vec(), bytes(vector, "commitment"));
        assert_eq!(response.nonce_point.to_vec(), bytes(vector, "nonce_point"));
        assert_eq!(response.signature.to_vec(), bytes(vector, "signature"));
        let mut encoded = [0u8; RESPONSE_LEN];
        encode_response(&response, &mut encoded).expect("re-encoded response");
        assert_eq!(encoded.to_vec(), wire);
    }
}
