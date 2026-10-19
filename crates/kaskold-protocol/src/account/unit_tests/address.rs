use super::*;

#[test]
fn address_decode_rejects_noncanonical_lengths_and_unicode() {
    let payload = [0x42; 32];
    let canonical = encode_p2pk_address(&payload, "kaspa");
    assert_eq!(decode_address(&canonical), Ok((0x00, payload)));
    assert_eq!(
        decode_address(&(canonical.clone() + "q"))
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Checksum mismatch")
    );
    assert_eq!(
        decode_address(&canonical[..canonical.len() - 1])
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Checksum mismatch")
    );
    assert_eq!(
        decode_address("kaspa:éqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq")
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Invalid character: 'Ã'")
    );
    assert_eq!(
        decode_address(&canonical.to_uppercase())
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some("Unknown address prefix")
    );
}
#[test]
fn address_encode_rejects_unknown_prefix_and_version() {
    let payload = [0x11; 32];
    assert!(encode_address(&payload, 0x7f, "kaspa").is_empty());
    assert!(encode_address(&payload, 0x00, "bitcoin").is_empty());
    assert!(!encode_address(&payload, 0x00, "kaspa").is_empty());
}
