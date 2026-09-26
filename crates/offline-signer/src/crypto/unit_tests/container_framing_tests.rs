use super::*;

fn current_backup(kind: u8, payload_len: usize) -> std::vec::Vec<u8> {
    let total = BACKUP_CURRENT_HEADER_SIZE + payload_len + TAG_SIZE;
    let mut bytes = std::vec![0u8; total];
    bytes[..8].copy_from_slice(&BACKUP_CURRENT_MAGIC);
    bytes[8] = BACKUP_CURRENT_VERSION;
    bytes[9] = kind;
    bytes[10] = KDF_ID_DEVICE_HMAC_SHA256;
    bytes[11] = CredentialKind::Password as u8;
    bytes[12..14].copy_from_slice(&(payload_len as u16).to_le_bytes());
    bytes[16..16 + METADATA_SIZE]
        .copy_from_slice(&password_kdf::encode_metadata(PasswordKdfParams::current()).unwrap());
    bytes[28..44].fill(0x31);
    bytes[44..56].fill(0x42);
    bytes[BACKUP_CURRENT_HEADER_SIZE..BACKUP_CURRENT_HEADER_SIZE + payload_len].fill(0x77);
    bytes[BACKUP_CURRENT_HEADER_SIZE + payload_len..].fill(0x88);
    bytes
}

fn current_transport(data_len: usize) -> std::vec::Vec<u8> {
    let total = TRANSPORT_CURRENT_CIPHERTEXT_START + data_len + TRANSPORT_TAG_SIZE;
    let mut bytes = std::vec![0u8; total];
    bytes[..4].copy_from_slice(&TRANSPORT_CURRENT_MAGIC);
    bytes[4..6].copy_from_slice(&(data_len as u16).to_le_bytes());
    bytes[6..6 + METADATA_SIZE]
        .copy_from_slice(&password_kdf::encode_metadata(PasswordKdfParams::current()).unwrap());
    let salt_start = 6 + METADATA_SIZE;
    let nonce_start = salt_start + SALT_SIZE;
    bytes[salt_start..nonce_start].fill(0x51);
    bytes[nonce_start..TRANSPORT_CURRENT_HEADER_SIZE].fill(0x62);
    bytes[TRANSPORT_CURRENT_CIPHERTEXT_START..TRANSPORT_CURRENT_CIPHERTEXT_START + data_len]
        .fill(0x73);
    bytes[TRANSPORT_CURRENT_CIPHERTEXT_START + data_len..].fill(0x84);
    bytes
}

#[test]
fn backup_payload_kind_codes_are_stable() {
    assert_eq!(BackupPayloadKind::Seed.code(), 1);
    assert_eq!(BackupPayloadKind::Xprv.code(), 2);
}

#[test]
fn transport_capacity_constants_are_exact() {
    assert_eq!(
        TRANSPORT_CURRENT_HEADER_SIZE + TRANSPORT_CURRENT_MAX_DATA_LEN + TRANSPORT_TAG_SIZE,
        1024
    );
}

#[test]
fn framing_minimum_validators_distinguish_exact_boundaries() {
    let backup_minimum = BACKUP_CURRENT_HEADER_SIZE + TAG_SIZE;
    assert_eq!(
        validate_backup_minimum(backup_minimum - 1),
        Err(FramingError::InvalidLength)
    );
    assert_eq!(validate_backup_minimum(backup_minimum), Ok(()));

    let transport_minimum = TRANSPORT_CURRENT_HEADER_SIZE + 1 + TRANSPORT_TAG_SIZE;
    assert_eq!(
        validate_transport_minimum(transport_minimum - 1),
        Err(FramingError::InvalidLength)
    );
    assert_eq!(validate_transport_minimum(transport_minimum), Ok(()));
}

#[test]
fn framing_length_validators_accept_maxima_and_reject_zero() {
    let backup_max = current_backup(1, BACKUP_MAX_PLAINTEXT);
    assert_eq!(
        parse_backup_header(&backup_max).unwrap().payload_len,
        BACKUP_MAX_PLAINTEXT
    );

    let zero_transport_total = TRANSPORT_CURRENT_CIPHERTEXT_START + TRANSPORT_TAG_SIZE;
    assert_eq!(
        validate_transport_length(0, zero_transport_total),
        Err(FramingError::InvalidLength)
    );
    let max_transport_total =
        TRANSPORT_CURRENT_CIPHERTEXT_START + TRANSPORT_CURRENT_MAX_DATA_LEN + TRANSPORT_TAG_SIZE;
    assert_eq!(
        validate_transport_length(TRANSPORT_CURRENT_MAX_DATA_LEN, max_transport_total),
        Ok(())
    );
}

#[test]
fn current_backup_header_parses_argon2_metadata() {
    let bytes = current_backup(1, 16);
    let parsed = parse_backup_header(&bytes).unwrap();
    assert_eq!(parsed.kind, BackupPayloadKind::Seed);
    assert_eq!(parsed.payload_len, 16);
    assert_eq!(parsed.header_size, BACKUP_CURRENT_HEADER_SIZE);
    assert_eq!(parsed.parameters, PasswordKdfParams::current());
    assert_eq!(parsed.salt, [0x31; SALT_SIZE]);
    assert_eq!(parsed.nonce, [0x42; NONCE_SIZE]);
}

#[test]
fn current_backup_xprv_kind_parses() {
    let parsed = parse_backup_header(&current_backup(2, 9)).unwrap();
    assert_eq!(parsed.kind, BackupPayloadKind::Xprv);
}

#[test]
fn historical_pbkdf2_backup_magic_is_rejected() {
    let mut bytes = current_backup(1, 8);
    bytes[..8].copy_from_slice(b"KASDB004");
    assert_eq!(
        parse_backup_header(&bytes),
        Err(FramingError::InvalidFormat)
    );
}

#[test]
fn backup_header_rejects_bad_version_kind_kdf_and_reserved_fields() {
    let baseline = current_backup(1, 8);
    for (offset, value, expected) in [
        (8usize, 0xff, FramingError::UnsupportedFormat),
        (9, 0xff, FramingError::WrongPurpose),
        (10, 0xff, FramingError::InvalidFormat),
        (11, 0xff, FramingError::InvalidFormat),
        (14, 1, FramingError::InvalidFormat),
        (15, 1, FramingError::InvalidFormat),
        (56, 1, FramingError::InvalidFormat),
    ] {
        let mut bytes = baseline.clone();
        bytes[offset] = value;
        assert_eq!(parse_backup_header(&bytes), Err(expected));
    }
}

#[test]
fn backup_header_rejects_zero_and_oversize_payload_lengths() {
    let zero = current_backup(1, 0);
    assert_eq!(parse_backup_header(&zero), Err(FramingError::InvalidLength));

    let oversize = current_backup(1, BACKUP_MAX_PLAINTEXT + 1);
    assert_eq!(
        parse_backup_header(&oversize),
        Err(FramingError::InvalidLength)
    );
}

#[test]
fn backup_header_rejects_unsupported_argon2_metadata() {
    let mut bytes = current_backup(1, 8);
    bytes[16] = 0xff;
    assert_eq!(
        parse_backup_header(&bytes),
        Err(FramingError::UnsupportedKdf)
    );
}

#[test]
fn backup_header_rejects_zero_salt_nonce_and_bad_lengths() {
    let baseline = current_backup(1, 8);
    let mut zero_salt = baseline.clone();
    zero_salt[28..44].fill(0);
    assert_eq!(
        parse_backup_header(&zero_salt),
        Err(FramingError::InvalidFormat)
    );

    let mut zero_nonce = baseline.clone();
    zero_nonce[44..56].fill(0);
    assert_eq!(
        parse_backup_header(&zero_nonce),
        Err(FramingError::InvalidFormat)
    );

    assert_eq!(
        parse_backup_header(&baseline[..baseline.len() - 1]),
        Err(FramingError::InvalidLength)
    );
    assert_eq!(
        parse_backup_header(&[0u8; 7]),
        Err(FramingError::InvalidLength)
    );
}

#[test]
fn current_transport_header_parses_argon2_metadata() {
    let bytes = current_transport(12);
    let parsed = parse_transport_header(&bytes, bytes.len()).unwrap();
    assert_eq!(parsed.data_len, 12);
    assert_eq!(parsed.header_len, TRANSPORT_CURRENT_HEADER_SIZE);
    assert_eq!(parsed.ciphertext_start, TRANSPORT_CURRENT_CIPHERTEXT_START);
    assert_eq!(parsed.tag_start, TRANSPORT_CURRENT_CIPHERTEXT_START + 12);
    assert_eq!(parsed.parameters, PasswordKdfParams::current());
    assert_eq!(parsed.salt, [0x51; SALT_SIZE]);
    assert_eq!(parsed.nonce, [0x62; TRANSPORT_NONCE_SIZE]);
}

#[test]
fn historical_pbkdf2_transport_magic_is_rejected() {
    let mut bytes = current_transport(8);
    bytes[..4].copy_from_slice(b"KAS\x03");
    assert_eq!(
        parse_transport_header(&bytes, bytes.len()),
        Err(FramingError::InvalidFormat)
    );
}

#[test]
fn transport_rejects_unsupported_argon2_metadata_and_zero_material() {
    let baseline = current_transport(8);
    let mut bad_kdf = baseline.clone();
    bad_kdf[6] = 0xff;
    assert_eq!(
        parse_transport_header(&bad_kdf, bad_kdf.len()),
        Err(FramingError::UnsupportedKdf)
    );

    let salt_start = 6 + METADATA_SIZE;
    let nonce_start = salt_start + SALT_SIZE;
    let mut zero_salt = baseline.clone();
    zero_salt[salt_start..nonce_start].fill(0);
    assert_eq!(
        parse_transport_header(&zero_salt, zero_salt.len()),
        Err(FramingError::InvalidFormat)
    );

    let mut zero_nonce = baseline.clone();
    zero_nonce[nonce_start..TRANSPORT_CURRENT_HEADER_SIZE].fill(0);
    assert_eq!(
        parse_transport_header(&zero_nonce, zero_nonce.len()),
        Err(FramingError::InvalidFormat)
    );
}

#[test]
fn transport_rejects_zero_oversize_and_mismatched_lengths() {
    let mut zero = current_transport(1);
    zero[4..6].copy_from_slice(&0u16.to_le_bytes());
    assert_eq!(
        parse_transport_header(&zero, zero.len()),
        Err(FramingError::InvalidLength)
    );

    let too_long = current_transport(TRANSPORT_CURRENT_MAX_DATA_LEN + 1);
    assert_eq!(
        parse_transport_header(&too_long, too_long.len()),
        Err(FramingError::InvalidLength)
    );

    let valid = current_transport(8);
    assert_eq!(
        parse_transport_header(&valid, valid.len() - 1),
        Err(FramingError::InvalidLength)
    );
}

#[test]
fn transport_parser_rejects_unknown_magic_and_out_of_bounds_file_len() {
    let mut bytes = current_transport(8);
    bytes[..4].copy_from_slice(b"NOPE");
    assert_eq!(
        parse_transport_header(&bytes, bytes.len()),
        Err(FramingError::InvalidFormat)
    );
    assert_eq!(
        parse_transport_header(&bytes, bytes.len() + 1),
        Err(FramingError::InvalidLength)
    );
}

#[test]
fn copy_nonzero_requires_exact_nonzero_input() {
    assert_eq!(
        copy_nonzero::<4>(&[1, 2, 3]),
        Err(FramingError::InvalidLength)
    );
    assert_eq!(
        copy_nonzero::<4>(&[1, 2, 3, 4, 5]),
        Err(FramingError::InvalidLength)
    );
    assert_eq!(copy_nonzero::<4>(&[1, 2, 3, 4]), Ok([1, 2, 3, 4]));
    assert_eq!(
        copy_nonzero::<4>(&[0, 0, 0, 0]),
        Err(FramingError::InvalidFormat)
    );
}

#[test]
fn current_backup_rejects_exact_short_header_boundary() {
    let mut truncated_current = std::vec![0u8; BACKUP_CURRENT_HEADER_SIZE + TAG_SIZE - 1];
    truncated_current[..8].copy_from_slice(&BACKUP_CURRENT_MAGIC);
    assert_eq!(
        parse_backup_header(&truncated_current),
        Err(FramingError::InvalidLength)
    );
}
