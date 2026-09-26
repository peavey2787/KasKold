use crate::wire::pskt_schema::{validate_canonical_json, JsonSyntaxError};

#[test]
fn canonical_json_rejects_duplicate_keys_recursively_in_every_container_shape() {
    let cases: &[&[u8]] = &[
        br#"{"x":1,"x":2}"#,
        br#"{"outer":{"x":1,"x":2}}"#,
        br#"{"outer":{"deeper":{"x":1,"x":2}}}"#,
        br#"{"items":[{"x":1,"x":2}]}"#,
        br#"[{"outer":{"x":1,"x":2}}]"#,
        br#"{"unknownExtension":{"array":[{"future":1,"future":2}]}}"#,
    ];
    for json in cases {
        assert_eq!(
            validate_canonical_json(json),
            Err(JsonSyntaxError::DuplicateKey),
            "duplicate escaped canonical validation: {}",
            core::str::from_utf8(json).expect("ASCII fixture"),
        );
    }
}

#[test]
fn canonical_json_allows_same_key_name_in_distinct_object_scopes() {
    assert_eq!(
        validate_canonical_json(br#"{"left":{"x":1},"right":{"x":2}}"#),
        Ok(())
    );
}

#[test]
fn canonical_json_value_and_structure_boundaries_cover_each_parser_branch() {
    for valid in [
        br#"{}"#.as_slice(),
        br#"[]"#.as_slice(),
        br#"{ "string" : "ASCII", "zero":0, "number":123, "yes":true, "no":false, "none":null, "array":[0,1,{}], "object":{"nested":2} }"#.as_slice(),
    ] {
        assert_eq!(validate_canonical_json(valid), Ok(()));
    }

    let invalid = [
        (br#"{"x":01}"#.as_slice(), JsonSyntaxError::InvalidNumber),
        (br#"{"x":1.0}"#.as_slice(), JsonSyntaxError::InvalidNumber),
        (br#"{"x":-1}"#.as_slice(), JsonSyntaxError::UnexpectedToken),
        (br#"{"x":tru}"#.as_slice(), JsonSyntaxError::UnexpectedToken),
        (
            br#"{"x":"a\nb"}"#.as_slice(),
            JsonSyntaxError::InvalidString,
        ),
        (
            b"{\"x\":\"unterminated}".as_slice(),
            JsonSyntaxError::InvalidString,
        ),
        (br#"{"x" 1}"#.as_slice(), JsonSyntaxError::UnexpectedToken),
        (
            br#"{"x":1 "y":2}"#.as_slice(),
            JsonSyntaxError::UnexpectedToken,
        ),
        (br#"{"x":1,}"#.as_slice(), JsonSyntaxError::UnexpectedToken),
        (br#"[1 2]"#.as_slice(), JsonSyntaxError::UnexpectedToken),
        (br#"[1,]"#.as_slice(), JsonSyntaxError::UnexpectedToken),
        (br#"{} trailing"#.as_slice(), JsonSyntaxError::TrailingData),
    ];
    for (json, expected) in invalid {
        assert_eq!(validate_canonical_json(json), Err(expected));
    }

    let deeply_nested = format!("{}0{}", "[".repeat(80), "]".repeat(80));
    assert_eq!(
        validate_canonical_json(deeply_nested.as_bytes()),
        Err(JsonSyntaxError::NestingTooDeep),
    );
}
