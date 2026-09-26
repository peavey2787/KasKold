#![no_main]

use kaskold_protocol::{Network, QrDecoder, SigningRequest, SigningResponse};
use libfuzzer_sys::fuzz_target;
use shared_signer::{anti_klepto, covenant_sign, pairing, qr_frame};

fuzz_target!(|data: &[u8]| {
    let bounded = &data[..data.len().min(32_768)];

    // Byte-facing public parsers: always feed arbitrary binary, including empty,
    // truncated, overlong, and non-UTF-8 inputs.
    let _ = qr_frame::parse_frame(bounded);
    let _ = pairing::parse_request(bounded);
    let _ = pairing::parse_response(bounded);
    let _ = anti_klepto::parse_request(bounded);
    let _ = anti_klepto::parse_commitment(bounded);
    let _ = anti_klepto::parse_reveal(bounded);
    let _ = anti_klepto::parse_signed(bounded);
    let _ = covenant_sign::parse_request(bounded);
    let _ = covenant_sign::parse_reveal(bounded);
    let _ = covenant_sign::parse_response(bounded);
    let _ = covenant_sign::private_swap::parse_request(bounded);
    let _ = covenant_sign::private_swap::parse_reveal(bounded);
    let _ = covenant_sign::private_swap::parse_response(bounded);
    let mut decoder = QrDecoder::new();
    let _ = decoder.accept(bounded);

    // Text-facing public parsers: every arbitrary byte string that happens to
    // be valid UTF-8 is exercised as-is, including multibyte code points at
    // every possible byte offset and embedded protocol punctuation.
    if let Ok(text) = core::str::from_utf8(bounded) {
        let _ = kaskold_protocol::decode_address(text);
        let _ = kaskold_protocol::address_to_script_pubkey(text);
        let _ = kaskold_protocol::decode_kpub_text(text);
        let _ = kaskold_protocol::decode_account(text, Network::Mainnet);
        let _ = kaskold_protocol::decode_account(text, Network::Testnet10);
        let _ = kaskold_protocol::import_kpub(text, "kaspa");
        let _ = kaskold_protocol::encode_pskt(text, Network::Mainnet);
        let _ = kaskold_protocol::encode_pskt(text, Network::Testnet10);
        let _ = kaskold_protocol::pskt_is_complete(text, Network::Mainnet);
        let _ = kaskold_protocol::finalize_json(text);
        let _ = SigningRequest::from_pskt(text, Network::Mainnet);
        if let Ok(response) = SigningResponse::decode(text) {
            let _ = response.merge_into(text, Network::Mainnet);
        }
    }
});
