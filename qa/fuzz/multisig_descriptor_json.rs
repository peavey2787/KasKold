#![no_main]

use kaskold_protocol::wire::multisig_descriptor as descriptor;
use libfuzzer_sys::fuzz_target;

/// Parse the descriptor every surface sees, whether it arrived bare or inside
/// the JSON record a wallet export carries it in.
fn check(text: &[u8]) {
    let Ok(parsed) = descriptor::parse_multisig_descriptor(text) else {
        return;
    };
    assert!(
        parsed.threshold >= 1,
        "a zero threshold spends without a signature"
    );
    assert!(parsed.threshold <= parsed.participant_count);
    assert!(usize::from(parsed.participant_count) <= descriptor::MAX_DESCRIPTOR_PARTICIPANTS);
    assert_eq!(parsed.is_hd(), descriptor::looks_like_descriptor(text));
}

fuzz_target!(|data: &[u8]| {
    check(data);
    if let Ok(serde_json::Value::Object(record)) = serde_json::from_slice(data) {
        if let Some(serde_json::Value::String(text)) = record.get("descriptor") {
            check(text.as_bytes());
        }
    }
});
