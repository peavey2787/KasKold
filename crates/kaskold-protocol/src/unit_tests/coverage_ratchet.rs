
use crate::{decode_account, AddressBranch, Network, ProtocolErrorKind, QrDecoder, SigningRequest};

#[test]
fn stable_enum_names_network_display_and_branch_codes_are_exhaustive() {
    let error_kinds = [
        (ProtocolErrorKind::MalformedRequest, "malformedRequest"),
        (ProtocolErrorKind::WrongNetwork, "wrongNetwork"),
        (
            ProtocolErrorKind::TransactionMismatch,
            "transactionMismatch",
        ),
        (ProtocolErrorKind::PairingMismatch, "pairingMismatch"),
        (ProtocolErrorKind::Qr, "qr"),
        (ProtocolErrorKind::Finalization, "finalization"),
        (ProtocolErrorKind::Derivation, "derivation"),
        (ProtocolErrorKind::Encoding, "encoding"),
        (ProtocolErrorKind::Decoding, "decoding"),
        (ProtocolErrorKind::Unsupported, "unsupported"),
        (ProtocolErrorKind::Internal, "internal"),
    ];
    for (kind, expected) in error_kinds {
        assert_eq!(kind.as_str(), expected);
    }

    let networks = [
        (Network::Mainnet, "mainnet", "kaspa", 1),
        (Network::Testnet10, "testnet-10", "kaspatest", 2),
        (Network::Testnet11, "testnet-11", "kaspatest", 2),
        (Network::Testnet12, "testnet-12", "kaspatest", 2),
        (Network::Devnet, "devnet", "kaspadev", 3),
        (Network::Simnet, "simnet", "kaspasim", 4),
    ];
    for (network, text, prefix, code) in networks {
        assert_eq!(network.to_string(), text);
        assert_eq!(Network::parse(text), Ok(network));
        assert_eq!(network.address_prefix(), prefix);
        assert_eq!(network.kspt_code(), code);
    }
    assert_eq!(AddressBranch::from_code(0), Ok(AddressBranch::Receive));
    assert_eq!(AddressBranch::from_code(1), Ok(AddressBranch::Change));
    assert_eq!(
        AddressBranch::from_code(2).unwrap_err().kind(),
        ProtocolErrorKind::Derivation
    );
}

#[test]
fn account_descriptor_accepts_canonical_raw_and_hex_wrapped_account_keys() {
    let (text, payload) = canonical_account_key();
    let descriptor = decode_account(&text, Network::Mainnet).expect("canonical account");
    assert_eq!(descriptor.receive_addresses.len(), 20);
    assert_eq!(descriptor.change_addresses.len(), 20);
    assert!(descriptor.receive_addresses[0]
        .address
        .starts_with("kaspa:"));

    let raw = decode_account(&hex::encode(payload), Network::Testnet10).expect("raw payload hex");
    assert!(raw.receive_addresses[0].address.starts_with("kaspatest:"));
    let wrapped = decode_account(&hex::encode(text.as_bytes()), Network::Devnet)
        .expect("hex-wrapped canonical text");
    assert!(wrapped.receive_addresses[0]
        .address
        .starts_with("kaspadev:"));
    assert!(decode_account("00", Network::Mainnet).is_err());
}

#[test]
fn qr_raw_paths_and_signing_request_success_are_covered() {
    let mut decoder = QrDecoder::new();
    assert_eq!(
        decoder.accept(b"raw").expect("raw QR"),
        Some(b"raw".to_vec())
    );
    assert!(decoder
        .accept(&shared_signer::qr_frame::FRAME_MAGIC)
        .is_err());

    let frames = crate::encode_qr_frames(&vec![0x55; 240]).expect("multi-frame");
    assert!(decoder
        .accept(&frames[0].payload)
        .expect("session start")
        .is_none());
    assert!(decoder.accept(b"raw while active").is_err());
    decoder.reset();

    let request = SigningRequest::from_pskt(&super::test_pskb([0x11; 32], 0), Network::Mainnet)
        .expect("signing request");
    assert!(!request.kspt_hex.is_empty());
    assert!(!request.qr_frames().is_empty());
}

fn canonical_account_key() -> (
    String,
    [u8; shared_signer::account_key::ACCOUNT_KEY_PAYLOAD_LEN],
) {
    use shared_signer::account_key::{
        encode_account_key_text, ACCOUNT_KEY_CHILD_INDEX, ACCOUNT_KEY_DEPTH,
        ACCOUNT_KEY_PAYLOAD_LEN, ACCOUNT_KEY_TEXT_LEN, ACCOUNT_KEY_VERSION,
    };
    let mut payload = [0u8; ACCOUNT_KEY_PAYLOAD_LEN];
    payload[..4].copy_from_slice(&ACCOUNT_KEY_VERSION);
    payload[4] = ACCOUNT_KEY_DEPTH;
    payload[9..13].copy_from_slice(&ACCOUNT_KEY_CHILD_INDEX.to_be_bytes());
    payload[13..45].fill(0x11);
    payload[45..78].copy_from_slice(&[
        0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87,
        0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16,
        0xf8, 0x17, 0x98,
    ]);
    let mut encoded = [0u8; ACCOUNT_KEY_TEXT_LEN];
    let length = encode_account_key_text(&payload, &mut encoded).expect("canonical key");
    (
        std::str::from_utf8(&encoded[..length]).unwrap().to_string(),
        payload,
    )
}

