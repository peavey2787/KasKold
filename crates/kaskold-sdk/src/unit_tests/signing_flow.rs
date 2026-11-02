//! A host wallet's whole journey against a real software signer: pair with the
//! exported account, prepare a transaction, carry it over QR, merge the signed
//! response and finalize.

use hot_wallet::HotWallet;
use kaskold_protocol::companion::encode_pskt_json_body;
use serde_json::{json, Value};

use super::*;

const PHRASE: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn script_hex(address: &DerivedAddress) -> String {
    let script = protocol::address_to_script_pubkey(&address.address).expect("address script");
    format!("0000{}", hex::encode(script))
}

/// A wallet-built transaction spending `source` and paying `amount` to `target`.
fn unsigned_pskt(source: &DerivedAddress, target: &DerivedAddress, amount: u64) -> String {
    let document = json!({
        "global": {
            "version": 0,
            "txVersion": 1,
            "inputCount": 1,
            "outputCount": 1,
            "fallbackLockTime": "0",
            "subnetworkId": "0000000000000000000000000000000000000000",
            "gas": "0",
            "txPayload": ""
        },
        "inputs": [{
            "previousOutpoint": { "transactionId": hex::encode([0x77u8; 32]), "index": 0 },
            "utxoEntry": { "amount": "100000", "scriptPublicKey": script_hex(source) },
            "sequence": "18446744073709551615",
            "sigOpCount": 1,
            "sighashType": 1,
            "partialSigs": {},
            "proprietaries": {}
        }],
        "outputs": [{
            "amount": amount.to_string(),
            "scriptPublicKey": script_hex(target),
            "proprietaries": {}
        }]
    });
    let body = encode_pskt_json_body(&json!([document])).expect("canonical body");
    let mut wire = b"PSKB".to_vec();
    wire.extend_from_slice(&body);
    let pskt = hex::encode(wire);
    let pskt = attach_input_derivation(&pskt, 0, source.branch, source.index).expect("input hint");
    attach_output_derivation(&pskt, 0, target.branch, target.index).expect("output hint")
}

struct Paired {
    signer: HotWallet,
    sdk: KasKold,
    account: PairedAccount,
}

fn paired() -> Paired {
    let signer = HotWallet::restore(PHRASE, "").expect("wallet");
    let mut sdk = KasKold::default();
    let account = sdk
        .pair_normal(&signer.export_kpub().expect("kpub"), Network::Mainnet)
        .expect("paired");
    Paired {
        signer,
        sdk,
        account,
    }
}

fn prepared(paired: &Paired, amount: u64) -> SigningRequest {
    let pskt = unsigned_pskt(
        &paired.account.receive_addresses[0],
        &paired.account.change_addresses[0],
        amount,
    );
    paired
        .sdk
        .prepare(&pskt, Network::Mainnet)
        .expect("request")
}

fn signed_by_device(paired: &Paired, request: &SigningRequest) -> String {
    let wire = hex::decode(&request.kspt_hex).expect("KSPT hex");
    hex::encode(paired.signer.sign_compact_kspt(&wire).expect("signed"))
}

#[test]
fn pairing_keeps_the_account_the_signer_exported() {
    let paired = paired();
    assert_eq!(paired.account.mode, PairingMode::Descriptor);
    assert_eq!(paired.account.network, Network::Mainnet);
    assert_eq!(
        paired.sdk.account_fingerprint(),
        Some(paired.account.account_fingerprint.as_str())
    );
    assert_eq!(
        paired.account.account_kpub.as_deref(),
        Some(paired.signer.export_kpub().expect("kpub").as_str())
    );
    assert!(paired.account.receive_addresses[0]
        .address
        .starts_with("kaspa:q"));
    assert_ne!(
        paired.account.receive_addresses[0].address,
        paired.account.change_addresses[0].address
    );
    assert_eq!(
        KasKold::new()
            .pair_normal("00", Network::Mainnet)
            .unwrap_err()
            .kind(),
        SdkErrorKind::Decoding
    );
}

#[test]
fn a_prepared_transaction_crosses_the_air_gap_unchanged() {
    let paired = paired();
    let request = prepared(&paired, 90_000);
    assert_eq!(request.network, Network::Mainnet);

    let mut scanner = KasKold::new();
    let mut scanned = None;
    for frame in request.qr_frames() {
        scanned = scanner.accept_qr_frame(&frame.payload).expect("frame");
    }
    assert_eq!(
        hex::encode(scanned.expect("whole payload")),
        request.kspt_hex
    );
}

#[test]
fn a_signed_response_merges_and_finalizes_into_the_reviewed_transaction() {
    let paired = paired();
    let request = prepared(&paired, 90_000);
    assert_eq!(
        protocol::pskt_is_complete(&request.original_pskt_hex, Network::Mainnet),
        Ok(false)
    );

    let signed = paired
        .sdk
        .complete(&request, &signed_by_device(&paired, &request))
        .expect("merged");
    assert_eq!(signed.network, Network::Mainnet);
    assert_eq!(
        protocol::pskt_is_complete(&signed.pskt_hex, Network::Mainnet),
        Ok(true)
    );

    let finalized: Value =
        serde_json::from_str(&paired.sdk.finalize(&signed).expect("final")).expect("final JSON");
    let script = finalized["inputs"][0]["signatureScript"]
        .as_str()
        .expect("signature script");
    assert!(!script.is_empty());
    assert_eq!(finalized["outputs"].as_array().expect("outputs").len(), 1);
    assert_eq!(finalized["outputs"][0]["amount"], "90000");
}

#[test]
fn a_response_for_another_transaction_is_refused() {
    let paired = paired();
    let request = prepared(&paired, 90_000);
    let other = prepared(&paired, 80_000);
    let error = paired
        .sdk
        .complete(&request, &signed_by_device(&paired, &other))
        .unwrap_err();
    assert_eq!(error.kind(), SdkErrorKind::TransactionMismatch);
}

#[test]
fn an_unsigned_transaction_does_not_finalize() {
    let paired = paired();
    let request = prepared(&paired, 90_000);
    let unsigned = SignedPskt {
        network: Network::Mainnet,
        pskt_hex: request.original_pskt_hex,
    };
    assert_eq!(
        paired.sdk.finalize(&unsigned).unwrap_err().kind(),
        SdkErrorKind::Finalization
    );
}

#[test]
fn resetting_the_decoder_forgets_a_partial_scan() {
    let frames = protocol::encode_qr_frames(&[0x66u8; 220]).expect("frames");
    let mut scanner = KasKold::new();
    assert!(scanner
        .accept_qr_frame(&frames[0].payload)
        .expect("frame")
        .is_none());
    assert_eq!(scanner.qr_decoder_progress().received, 1);
    scanner.reset_qr_decoder();
    assert_eq!(scanner.qr_decoder_progress().received, 0);
}
