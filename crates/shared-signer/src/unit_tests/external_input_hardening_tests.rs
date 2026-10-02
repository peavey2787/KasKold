//! The shared wire parsers (anti-klepto, covenant-sign, Private Swap) are
//! covered by Kaspa Portal; KasKold keeps tests for its own formats.

use crate::covenant_backup;

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
