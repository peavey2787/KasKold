use super::*;
use serde_json::json;

#[test]
fn derivation_hint_covers_absent_shape_branch_and_index_boundaries() {
    assert_eq!(parse_derivation(&json!({})), Ok(None));
    assert!(parse_derivation(&json!([])).is_err());
    assert!(parse_derivation(&json!({"kassignerDerivation": null})).is_err());
    assert!(parse_derivation(&json!({"kassignerDerivation": []})).is_err());

    assert!(parse_derivation(&json!({"kassignerDerivation": {"index": 0}})).is_err());
    assert!(parse_derivation(&json!({"kassignerDerivation": {"branch": 0}})).is_err());
    assert!(parse_derivation(&json!({"kassignerDerivation": {"branch": 2, "index": 0}})).is_err());
    assert!(
        parse_derivation(&json!({"kassignerDerivation": {"branch": 256, "index": 0}})).is_err()
    );
    assert!(
        parse_derivation(&json!({"kassignerDerivation": {"branch": "01", "index": 0}})).is_err()
    );

    let soft_limit = shared_signer::pairing::SOFT_INDEX_LIMIT;
    assert!(parse_derivation(&json!({
        "kassignerDerivation": {"branch": 1, "index": soft_limit}
    }))
    .is_err());
    assert!(parse_derivation(&json!({
        "kassignerDerivation": {"branch": 1, "index": u64::from(u32::MAX) + 1}
    }))
    .is_err());

    assert_eq!(
        parse_derivation(&json!({
            "kassignerDerivation": {"branch": 0, "index": 0}
        })),
        Ok(Some((0, 0)))
    );
    assert_eq!(
        parse_derivation(&json!({
            "kassignerDerivation": {"branch": "1", "index": (soft_limit - 1).to_string()}
        })),
        Ok(Some((1, soft_limit - 1)))
    );
}
