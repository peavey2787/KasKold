use crate::wire::qr_payload::{
    unwrap_v1_raw, wrap_v1_raw, MAX_RAW_LEN, PAYLOAD_V1_RAW, QR_V40_BYTE_CAPACITY,
};

#[test]
fn raw_qr_payload_envelope_rejects_invalid_boundaries_and_round_trips() {
    assert_eq!(unwrap_v1_raw(&[]), None);
    assert_eq!(unwrap_v1_raw(&[PAYLOAD_V1_RAW]), None);
    assert_eq!(unwrap_v1_raw(&[2, 0xaa]), None);
    assert_eq!(unwrap_v1_raw(&[PAYLOAD_V1_RAW, 0xaa]), Some(&[0xaa][..]));

    let mut out = vec![0u8; MAX_RAW_LEN + 1];
    assert_eq!(wrap_v1_raw(&[], &mut out), None);
    assert_eq!(wrap_v1_raw(&vec![0u8; MAX_RAW_LEN + 1], &mut out), None);
    assert_eq!(wrap_v1_raw(&[0x11, 0x22], &mut [0u8; 2]), None);
    assert_eq!(wrap_v1_raw(&[0x11, 0x22], &mut out), Some(3));
    assert_eq!(&out[..3], &[PAYLOAD_V1_RAW, 0x11, 0x22]);
}

#[test]
fn the_largest_framed_payload_fills_one_version_40_qr_exactly() {
    let mut out = vec![0u8; QR_V40_BYTE_CAPACITY + 1];
    assert_eq!(
        wrap_v1_raw(&vec![0x5a; MAX_RAW_LEN], &mut out),
        Some(QR_V40_BYTE_CAPACITY)
    );
    assert_eq!(wrap_v1_raw(&vec![0x5a; MAX_RAW_LEN + 1], &mut out), None);
}
