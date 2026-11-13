use super::{byte_rows, ARRAY_INDENT, BYTES_PER_ROW, FIRMWARE_MAX_WIDTH};

#[test]
fn byte_rows_fill_the_firmware_width_the_way_rustfmt_does() {
    let rows = byte_rows(&[0xab; 32]);
    let lines: Vec<&str> = rows.lines().collect();
    assert_eq!(BYTES_PER_ROW, 19);
    assert_eq!(lines.len(), 2);
    for line in &lines {
        assert!(line.len() <= FIRMWARE_MAX_WIDTH);
        assert!(line.starts_with(ARRAY_INDENT));
        assert!(line.ends_with(','));
    }
    assert_eq!(
        lines.first().map(|line| line.matches("0xab").count()),
        Some(19)
    );
    assert_eq!(
        lines.last().map(|line| line.matches("0xab").count()),
        Some(13)
    );
}
