use crate::derivation::bip32::derive_account_key;
use crate::privacy::pairing::respond;

#[test]
fn response_round_trips_request_ranges() {
    let account = derive_account_key(&[0x33u8; 64]).expect("account");
    let request = shared_signer::pairing::AddressBatchRequest::new(
        [0x11; shared_signer::pairing::NONCE_LEN],
        7,
        2,
        19,
        1,
    );
    let mut wire = [0u8; shared_signer::pairing::REQUEST_LEN];
    shared_signer::pairing::encode_request(request, &mut wire).expect("request");
    let response = respond(&account, &wire).expect("response");
    let parsed = shared_signer::pairing::parse_response(&response).expect("parse response");
    assert_eq!(parsed.request(), request);
    assert!(parsed.receive_key(0).is_some());
    assert!(parsed.receive_key(1).is_some());
    assert!(parsed.change_key(0).is_some());
}

#[test]
fn pairing_error_messages_cover_every_stable_variant() {
    use crate::privacy::pairing::PrivacyPairingError;
    for error in [
        PrivacyPairingError::InvalidRequest,
        PrivacyPairingError::Derivation,
        PrivacyPairingError::Encoding,
    ] {
        assert!(!error.message().is_empty());
    }
}
