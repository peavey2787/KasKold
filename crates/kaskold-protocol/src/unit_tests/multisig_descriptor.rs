use crate::wire::multisig_descriptor::{
    parse_multisig_descriptor, MultisigDescriptorError, MAX_DESCRIPTOR_PARTICIPANTS,
};

fn hd44_participant(prefix: u8, fill: u8) -> String {
    let mut raw = [fill; 65];
    raw[0] = prefix;
    hex::encode(raw)
}

#[test]
fn signer_binding_enforces_kaskold_capacity_thresholds_and_duplicates() {
    let first = hd44_participant(0x02, 0x31);
    let second = hd44_participant(0x03, 0x52);
    assert_eq!(
        parse_multisig_descriptor(format!("multi_hd(0,{first},{second})").as_bytes()),
        Err(MultisigDescriptorError::InvalidThreshold),
    );
    assert_eq!(
        parse_multisig_descriptor(format!("multi_hd(1,{first},{first})").as_bytes()),
        Err(MultisigDescriptorError::DuplicateParticipant),
    );

    let entries = (0..=MAX_DESCRIPTOR_PARTICIPANTS)
        .map(|index| {
            hd44_participant(
                if index.is_multiple_of(2) { 0x02 } else { 0x03 },
                index as u8 + 1,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(
        parse_multisig_descriptor(format!("multi_hd(1,{entries})").as_bytes()),
        Err(MultisigDescriptorError::TooManyParticipants),
    );
}
