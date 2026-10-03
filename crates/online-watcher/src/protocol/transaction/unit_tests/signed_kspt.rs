use crate::protocol::transaction::signed_kspt::decode_signed_kspt;

const SIGNED_COMPACT_KSPT: &str = "4b53505401010000010000000100000000000000000000000000000000000000000000000000000000000000000000000000001111111111111111111111111111111111111111111111111111111111111111010000006400000000000000000000000000000001000022204444444444444444444444444444444444444444444444444444444444444444ac0100012222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222200005a00000000000000000022205555555555555555555555555555555555555555555555555555555555555555ac4e01";
#[test]
fn production_decoder_rejects_placeholder_signature_before_consensus_assembly() {
    let error = decode_signed_kspt(SIGNED_COMPACT_KSPT)
        .expect_err("placeholder signature must never cross the production decoder boundary");
    assert!(
        error.contains("signature") || error.contains("complete") || error.contains("verification"),
        "unexpected verification error: {error}"
    );
}

#[test]
fn production_decoder_rejects_unsupported_transaction_version_before_signature_use() {
    let mut bytes = hex::decode(SIGNED_COMPACT_KSPT).expect("fixture hex");
    bytes[6..8].copy_from_slice(&2u16.to_le_bytes());
    let error = decode_signed_kspt(&hex::encode(bytes)).unwrap_err();
    assert!(
        error.to_ascii_lowercase().contains("unsupported")
            && error.to_ascii_lowercase().contains("version"),
        "unexpected unsupported-version error: {error}"
    );
}

#[test]
fn signed_kspt_rejects_non_hex_before_wire_decode() {
    assert!(decode_signed_kspt("zz")
        .unwrap_err()
        .starts_with("Invalid hex:"));
}
