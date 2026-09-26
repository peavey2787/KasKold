use super::*;
use serde_json::json;

#[test]
fn value_kind_validation_covers_every_kind_and_type_failure() {
    assert!(validate_kind(&json!({}), ValueKind::Object, "v").is_ok());
    assert!(validate_kind(&json!([]), ValueKind::Array, "v").is_ok());
    assert!(validate_kind(&json!(true), ValueKind::Boolean, "v").is_ok());
    assert!(validate_kind(&json!("text"), ValueKind::String, "v").is_ok());
    assert!(validate_kind(&json!(7), ValueKind::ExactUnsigned, "v").is_ok());
    assert!(validate_kind(&json!("7"), ValueKind::ExactUnsigned, "v").is_ok());
    assert!(validate_kind(&json!("00ff"), ValueKind::HexString, "v").is_ok());
    assert!(validate_kind(&json!("0000"), ValueKind::ScriptPublicKey, "v").is_ok());

    assert!(validate_kind(&json!([]), ValueKind::Object, "v").is_err());
    assert!(validate_kind(&json!({}), ValueKind::Array, "v").is_err());
    assert!(validate_kind(&json!(0), ValueKind::Boolean, "v").is_err());
    assert!(validate_kind(&json!(0), ValueKind::String, "v").is_err());
    assert!(validate_kind(&json!(-1), ValueKind::ExactUnsigned, "v").is_err());
    assert!(validate_kind(&json!("01"), ValueKind::ExactUnsigned, "v").is_err());
    assert!(validate_kind(
        &json!(9_007_199_254_740_992_u64),
        ValueKind::ExactUnsigned,
        "v"
    )
    .is_err());
    assert!(validate_kind(&json!(0), ValueKind::HexString, "v").is_err());
    assert!(validate_kind(&json!("0"), ValueKind::HexString, "v").is_err());
    assert!(validate_kind(&json!(0), ValueKind::ScriptPublicKey, "v").is_err());
    assert!(validate_kind(&json!("00"), ValueKind::ScriptPublicKey, "v").is_err());
}

#[test]
fn object_helpers_cover_required_optional_and_null_boundaries() {
    let object = json!({"o": {}, "a": [], "n": null, "x": 1});
    let map = object.as_object().expect("object");
    assert!(required_object(map, "o", "o").is_ok());
    assert!(required_object(map, "missing", "missing").is_err());
    assert!(required_array(map, "a", "a").is_ok());
    assert!(required_array(map, "x", "x").is_err());
    assert!(optional_object(None, "o").unwrap().is_none());
    assert!(optional_object(map.get("n"), "n").unwrap().is_none());
    assert!(optional_object(map.get("o"), "o").unwrap().is_some());
    assert!(optional_object(map.get("x"), "x").is_err());
}
