use crate::{anti_klepto, covenant_backup, covenant_sign};

#[test]
fn externally_controlled_shared_wire_parsers_are_total_over_truncated_and_noise_inputs() {
    let mut seed = 0x6a09_e667_f3bc_c909u64;
    for len in 0..=600usize {
        let mut bytes = std::vec![0u8; len];
        for byte in &mut bytes {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            *byte = (seed >> 32) as u8;
        }
        let data = bytes.as_slice();
        let result = std::panic::catch_unwind(|| {
            let _ = anti_klepto::parse_request(data);
            let _ = anti_klepto::parse_reveal(data);
            let _ = covenant_sign::parse_request(data);
            let _ = covenant_sign::parse_reveal(data);
            let _ = covenant_sign::private_swap::parse_request(data);
            let _ = covenant_sign::private_swap::parse_reveal(data);
        });
        assert!(
            result.is_ok(),
            "external shared parser panicked at length {len}"
        );
    }
}

#[test]
fn covenant_backup_normalize_covers_raw_hex_capacity_and_format_paths() {
    let raw = b"COVB\x01";
    let mut output = [0u8; 8];
    let len = covenant_backup::normalize(raw, &mut output).expect("raw covenant backup normalizes");
    assert_eq!(len, raw.len());
    assert_eq!(&output[..len], raw);

    let mut short_raw = [0u8; 4];
    assert_eq!(
        covenant_backup::normalize(raw, &mut short_raw),
        Err(covenant_backup::CovenantBackupError::OutputTooSmall)
    );

    let hex = b"434f564201";
    output.fill(0);
    let len = covenant_backup::normalize(hex, &mut output).expect("hex covenant backup normalizes");
    assert_eq!(len, raw.len());
    assert_eq!(&output[..len], raw);

    let mut short_hex = [0u8; 4];
    assert_eq!(
        covenant_backup::normalize(hex, &mut short_hex),
        Err(covenant_backup::CovenantBackupError::OutputTooSmall)
    );

    assert_eq!(
        covenant_backup::normalize(b"not-a-covenant-backup", &mut output),
        Err(covenant_backup::CovenantBackupError::InvalidFormat)
    );
    assert_eq!(
        covenant_backup::normalize(b"434f5642zz", &mut output),
        Err(covenant_backup::CovenantBackupError::InvalidFormat)
    );
}
