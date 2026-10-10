use super::require_signer_descriptor;

fn static_descriptor(cosigners: u8) -> String {
    let keys = (1..=cosigners)
        .map(|index| format!("{index:02x}").repeat(32))
        .collect::<Vec<_>>()
        .join(",");
    format!("multi(1,{keys})")
}

#[test]
fn signer_policy_accepts_up_to_the_signer_capacity_and_rejects_more() {
    let capacity = kaskold_protocol::SIGNER_CAPABILITIES.max_multisig_keys;
    assert_eq!(require_signer_descriptor(&static_descriptor(2)), Ok(()));
    assert_eq!(
        require_signer_descriptor(&static_descriptor(capacity)),
        Ok(())
    );
    assert_eq!(
        require_signer_descriptor(&static_descriptor(capacity + 1)),
        Err("Descriptor has too many participants".to_string()),
    );
    assert!(require_signer_descriptor("not-a-descriptor").is_err());
}
