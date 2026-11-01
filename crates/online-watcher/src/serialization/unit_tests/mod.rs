use serde::{Deserialize, Serialize};

use super::decimal_u64;

#[derive(Debug, Deserialize)]
struct Wrapper {
    #[serde(with = "decimal_u64")]
    value: u64,
}

#[test]
fn accepts_canonical_decimal_string() {
    let parsed: Wrapper = serde_json::from_str(r#"{"value":"18446744073709551615"}"#).unwrap();
    assert_eq!(parsed.value, u64::MAX);
}

#[test]
fn rejects_a_json_number_at_any_size() {
    for encoded in [r#"{"value":1}"#, r#"{"value":9007199254740992}"#] {
        let error = serde_json::from_str::<Wrapper>(encoded).unwrap_err();
        assert!(error.to_string().contains("expected a string"), "{error}");
    }
}

#[test]
fn rejects_noncanonical_decimal_string() {
    assert_eq!(
        serde_json::from_str::<Wrapper>(r#"{"value":"01"}"#)
            .err()
            .map(|error| format!("{error:?}"))
            .as_deref(),
        Some(
            r#"Error("decimal string must be canonical (no leading zeroes)", line: 1, column: 14)"#
        )
    );
}

#[derive(Debug, Serialize)]
struct SerializeWrapper {
    #[serde(with = "decimal_u64")]
    value: u64,
}

#[test]
fn serializes_consensus_u64_as_decimal_string() {
    let encoded = serde_json::to_string(&SerializeWrapper { value: u64::MAX }).unwrap();
    assert_eq!(encoded, r#"{"value":"18446744073709551615"}"#);
}

#[test]
fn rejects_empty_nondigit_and_overflow_decimal_strings_independently() {
    for (encoded, expected) in [
        (
            r#"{"value":""}"#,
            r#"Error("expected an unsigned decimal string", line: 1, column: 12)"#,
        ),
        (
            r#"{"value":"12x"}"#,
            r#"Error("expected an unsigned decimal string", line: 1, column: 15)"#,
        ),
        (
            r#"{"value":"18446744073709551616"}"#,
            r#"Error("decimal string exceeds u64", line: 1, column: 32)"#,
        ),
    ] {
        assert_eq!(
            serde_json::from_str::<Wrapper>(encoded)
                .err()
                .map(|error| format!("{error:?}"))
                .as_deref(),
            Some(expected)
        );
    }
}
