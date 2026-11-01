use crate::storage::slot_header::{decode, encode, SlotHeader, SlotHeaderError};

#[test]
fn every_valid_header_round_trips_to_a_distinct_byte() {
    let mut seen = std::collections::BTreeSet::new();
    for source in 1..=4 {
        for network in 0..3 {
            for protection in 1..=3 {
                let header = SlotHeader {
                    source,
                    network,
                    protection,
                };
                let byte = encode(header);
                assert!(seen.insert(byte), "{byte:#04x} is ambiguous");
                assert_eq!(decode(byte), Ok(header));
            }
        }
    }
}

#[test]
fn every_decodable_byte_is_canonical() {
    for value in 0..=u8::MAX {
        if let Ok(header) = decode(value) {
            assert_eq!(encode(header), value, "{value:#04x}");
        }
    }
}

#[test]
fn the_wire_layout_is_pinned() {
    // Mnemonic24 on Testnet-12 with a password: tag 2 + 3 * 3 = 11.
    assert_eq!(
        encode(SlotHeader {
            source: 2,
            network: 2,
            protection: 3
        }),
        0xb2
    );
}

#[test]
fn an_empty_slot_encodes_as_zero_whatever_it_carried() {
    assert_eq!(
        encode(SlotHeader {
            source: 0,
            network: 2,
            protection: 3
        }),
        0
    );
    assert_eq!(
        decode(0),
        Ok(SlotHeader {
            source: 0,
            network: 0,
            protection: 1
        })
    );
}

#[test]
fn malformed_headers_are_rejected() {
    assert_eq!(decode(0x35), Err(SlotHeaderError::Source));
    assert_eq!(decode(0x0f), Err(SlotHeaderError::Source));
    assert_eq!(decode(0x10), Err(SlotHeaderError::Canonical));
    // A record with no protection code (tag below 3) has no defined protection.
    assert_eq!(decode(0x01), Err(SlotHeaderError::Canonical));
    assert_eq!(decode(0x21), Err(SlotHeaderError::Canonical));
    // Tag 12 and above would be protection code 4.
    assert_eq!(decode(0xc1), Err(SlotHeaderError::Canonical));
}
