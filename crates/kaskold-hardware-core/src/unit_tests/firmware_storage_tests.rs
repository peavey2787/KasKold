use crate::storage::fat32_lfn::{classify_directory_entry, DirectoryEntryKind, LfnAccumulator};
use kaskold_protocol::wire::multisig_descriptor::{
    parse_multisig_descriptor, MAX_DESCRIPTOR_PARTICIPANTS,
};
use std::{format, vec::Vec};

const PARTICIPANT_HEX_LEN: usize = 130;

fn participant(prefix: u8, fill: u8) -> Vec<u8> {
    let mut raw = [fill; 65];
    raw[0] = prefix;
    let mut output = Vec::with_capacity(PARTICIPANT_HEX_LEN);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in raw {
        output.push(HEX[(byte >> 4) as usize]);
        output.push(HEX[(byte & 0x0f) as usize]);
    }
    output
}

fn descriptor(threshold: u8, entries: &[Vec<u8>]) -> Vec<u8> {
    let mut output = b"multi_hd(".to_vec();
    output.push(b'0' + threshold);
    for entry in entries {
        output.push(b',');
        output.extend_from_slice(entry);
    }
    output.push(b')');
    output
}

#[test]
fn descriptor_parser_accepts_valid_hd_descriptors() {
    let mut input = descriptor(2, &[participant(2, 0x11), participant(3, 0x22)]);
    input.extend_from_slice(b"\r\n ");
    let parsed = parse_multisig_descriptor(&input).unwrap();
    assert_eq!(parsed.threshold, 2);
    assert_eq!(parsed.participant_count, 2);
    assert_eq!(parsed.public_keys[0][0], 2);
    assert_eq!(parsed.public_keys[1][0], 3);
    assert_eq!(parsed.chain_codes[0], [0x11; 32]);
}

#[test]
fn descriptor_parser_rejects_invalid_shapes_and_data() {
    assert!(parse_multisig_descriptor(b"multi(1,x)").is_err());
    assert!(parse_multisig_descriptor(b"multi_hd(0,x)").is_err());
    assert!(parse_multisig_descriptor(&descriptor(2, &[participant(2, 1)])).is_err());
    assert!(parse_multisig_descriptor(&descriptor(1, &[participant(4, 1)])).is_err());
    let mut bad_hex = descriptor(1, &[participant(2, 1)]);
    bad_hex[20] = b'z';
    assert!(parse_multisig_descriptor(&bad_hex).is_err());
    let entries: Vec<_> = (0..=MAX_DESCRIPTOR_PARTICIPANTS)
        .map(|index| participant(if index.is_multiple_of(2) { 2 } else { 3 }, index as u8))
        .collect();
    assert!(parse_multisig_descriptor(&descriptor(1, &entries)).is_err());
}

fn lfn_entry(sequence: u8, text: &[u8]) -> [u8; 32] {
    let mut raw = [0xFF; 32];
    raw[0] = sequence;
    raw[11] = 0x0F;
    let slots = [1usize, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30];
    for (index, byte) in text.iter().copied().enumerate() {
        raw[slots[index]] = byte;
        raw[slots[index] + 1] = 0;
    }
    if text.len() < slots.len() {
        raw[slots[text.len()]] = 0;
        raw[slots[text.len()] + 1] = 0;
    }
    raw
}

#[test]
fn descriptor_parser_covers_body_threshold_and_low_nibble_boundaries() {
    assert!(parse_multisig_descriptor(b"multi_hd(1,").is_err());
    assert!(parse_multisig_descriptor(b"multi_hd()").is_err());
    assert!(parse_multisig_descriptor(b"multi_hd(a,x)").is_err());
    assert!(parse_multisig_descriptor(b"multi_hd(1x)").is_err());
    assert!(parse_multisig_descriptor(b"multi_hd(1,)").is_err());

    let mut low_nibble = descriptor(1, &[participant(2, 1)]);
    // Participant hex begins after "multi_hd(1,"; offset 1 is the low nibble
    // of the compressed-key prefix byte, complementing the existing
    // high-nibble malformed-hex case.
    let participant_start = b"multi_hd(1,".len();
    low_nibble[participant_start + 1] = b'z';
    assert!(parse_multisig_descriptor(&low_nibble).is_err());
}

#[test]
fn lfn_state_machine_classifies_and_assembles_entries() {
    let mut raw = [0u8; 32];
    assert_eq!(classify_directory_entry(&raw), DirectoryEntryKind::End);
    raw[0] = 0xE5;
    assert_eq!(classify_directory_entry(&raw), DirectoryEntryKind::Deleted);
    raw[0] = b'A';
    raw[11] = 0x0F;
    assert_eq!(classify_directory_entry(&raw), DirectoryEntryKind::LongName);
    raw[11] = 0x08;
    assert_eq!(classify_directory_entry(&raw), DirectoryEntryKind::Volume);
    raw[11] = 0x20;
    assert_eq!(classify_directory_entry(&raw), DirectoryEntryKind::Regular);

    let mut names = LfnAccumulator::new();
    names.record(&lfn_entry(2, b"World"));
    names.record(&lfn_entry(1, b"Hello "));
    let (display, length) = names.display_name(b"HELLO   TXT");
    assert_eq!(&display[..length], b"Hello World");
}

#[test]
fn lfn_state_machine_falls_back_resets_and_maps_characters() {
    let mut names = LfnAccumulator::new();
    let (display, length) = names.display_name(b"README  TXT");
    assert_eq!(&display[..length], b"README.TXT");
    names.record(&lfn_entry(1, b"Long"));
    names.reset();
    let (display, length) = names.display_name(b"SHORT      ");
    assert_eq!(&display[..length], b"SHORT");

    let mut entry = lfn_entry(1, &[0xE9, b' ', 0, b'A']);
    entry[6] = 1;
    names.record(&entry);
    let (display, length) = names.display_name(b"FALLBACKTXT");
    assert_eq!(&display[..length], b"e _A");
}

#[test]
fn lfn_state_machine_ignores_invalid_sequences_and_bounds_output() {
    let mut names = LfnAccumulator::new();
    names.record(&lfn_entry(0, b"ignored"));
    names.record(&lfn_entry(5, b"ignored"));
    let (display, length) = names.display_name(b"VALID   TXT");
    assert_eq!(&display[..length], b"VALID.TXT");

    let mut names = LfnAccumulator::new();
    for sequence in 1..=4 {
        names.record(&lfn_entry(sequence, b"abcdefghijklm"));
    }
    let (display, length) = names.display_name(b"FALLBACKTXT");
    assert_eq!(length, 52);
    assert_eq!(&display[..13], b"abcdefghijklm");
}

#[test]
fn descriptor_parser_rejects_missing_and_repeated_separators() {
    let first = participant(2, 1);
    let second = participant(3, 2);
    let mut missing = b"multi_hd(1,".to_vec();
    missing.extend_from_slice(&first);
    missing.extend_from_slice(&second);
    missing.push(b')');
    assert!(parse_multisig_descriptor(&missing).is_err());

    let mut repeated = b"multi_hd(1,".to_vec();
    repeated.extend_from_slice(&first);
    repeated.extend_from_slice(b",,");
    repeated.extend_from_slice(&second);
    repeated.push(b')');
    assert!(parse_multisig_descriptor(&repeated).is_err());
}

#[test]
fn lfn_latin1_mapping_covers_every_supported_range_and_fallback() {
    let cases: &[(u8, u8)] = &[
        (0xC0, b'A'),
        (0xC5, b'A'),
        (0xC7, b'C'),
        (0xC8, b'E'),
        (0xCB, b'E'),
        (0xCC, b'I'),
        (0xCF, b'I'),
        (0xD1, b'N'),
        (0xD2, b'O'),
        (0xD6, b'O'),
        (0xD9, b'U'),
        (0xDC, b'U'),
        (0xE0, b'a'),
        (0xE5, b'a'),
        (0xE7, b'c'),
        (0xE8, b'e'),
        (0xEB, b'e'),
        (0xEC, b'i'),
        (0xEF, b'i'),
        (0xF1, b'n'),
        (0xF2, b'o'),
        (0xF6, b'o'),
        (0xF9, b'u'),
        (0xFC, b'u'),
        (0xA0, b' '),
        (0x80, b'_'),
        (0xFF, b'_'),
    ];
    for &(input, expected) in cases {
        let mut names = LfnAccumulator::new();
        names.record(&lfn_entry(1, &[input]));
        let (display, length) = names.display_name(b"FALLBACKTXT");
        assert_eq!(length, 1, "input {input:#04x}");
        assert_eq!(display[0], expected, "input {input:#04x}");
    }
}

#[test]
fn lfn_accumulator_default_matches_empty_constructor() {
    let mut names = LfnAccumulator::default();
    let short = *b"README  TXT";
    let (display, length) = names.display_name(&short);
    assert_eq!(&display[..length], b"README.TXT");
}

#[test]
fn v106_hd45_descriptor_round_trip_parser_is_order_canonical_and_header_tolerant() {
    // Exact v1.0.6 2-of-5 cross-implementation cosigner parents. v1.0.6
    // canonicalizes multi_hd45 by sorting the parent kpub strings before any
    // /cosigner/chain/index child derivation.
    const KPUBS: [&str; 5] = [
        "kpub1:038f332e03405ab68380000000f0453f0894cc8c84ebf6e6208e0c7916e9ddbd14919f9bbb92b0690b4e353392020327c7136972883eab5a7722ec3d4302f888804ecce61658ae962a2c56bb7571",
        "kpub1:038f332e03a7457270800000002908be01d75735944f29befbdbcd173ab00df2d44c6d5ab51a839413fda90cbf035b986b584de244f5d6a1939192f676a9f2992a63b0f43cdc452dcb40d9dd7081",
        "kpub1:038f332e037a262d628000000037234957045cdffdb77c3fdcb25649de3326bd8eb6459276a96ba0b14b99cf05034cf53938d64f4a3d4554e18e9ec0d113b251be5b974386807e95a58530e837e1",
        "kpub1:038f332e03206bbc9880000000d67b1d630674ca46e41e4bc5f6fc953a832efc679d85f80f7c01e494d993684402c0b5ff5ef462947cef268431e0ea913e6b5f51c5d8a572b10ba25ebbbca11440",
        "kpub1:038f332e03d18681ae80000000524d81044c2fad73c8d8e07cf3ec0d21b1b0bfb9163b78b900f4b9ba60f7658a036c8c95588515593aa406c778888410de0a7df460483529d10ae241adf6e2a19f",
    ];
    let unsorted = format!(
        "# KasKold multisig descriptor\r\nmulti_hd45(2,{},{},{},{},{})\r\n",
        KPUBS[0], KPUBS[1], KPUBS[2], KPUBS[3], KPUBS[4],
    );
    let mut sorted = KPUBS;
    sorted.sort_unstable();
    let canonical = format!(
        "multi_hd45(2,{},{},{},{},{})",
        sorted[0], sorted[1], sorted[2], sorted[3], sorted[4],
    );
    let parsed = parse_multisig_descriptor(unsorted.as_bytes()).expect("v1.0.6 descriptor");
    let canonical_parsed =
        parse_multisig_descriptor(canonical.as_bytes()).expect("canonical descriptor");
    assert!(parsed.is_hd45());
    assert_eq!(parsed.threshold, 2);
    assert_eq!(parsed.participant_count, 5);
    assert_eq!(parsed, canonical_parsed);
    assert!(parsed.depths[..5].iter().all(|depth| *depth == 3));
    assert!(parsed.child_numbers[..5]
        .iter()
        .all(|child| *child == [0x80, 0, 0, 0]));
}
