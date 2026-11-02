#![no_main]

use libfuzzer_sys::fuzz_target;
use online_watcher::facade::WatchWallet;
use serde_json::Value;

const NETWORKS: [&str; 3] = ["mainnet", "testnet-10", "devnet"];

fn field<'a>(invite: &'a Value, name: &str) -> &'a str {
    invite.get(name).and_then(Value::as_str).unwrap_or_default()
}

// An invite is untrusted JSON pasted or scanned from someone else's device.
fuzz_target!(|data: &[u8]| {
    let [selector, body @ ..] = data else {
        return;
    };
    let Ok(invite) = serde_json::from_slice::<Value>(body) else {
        return;
    };
    let address = field(&invite, "addr");
    let covenant_type = field(&invite, "ct");
    let network = NETWORKS[usize::from(*selector) % NETWORKS.len()];
    let Ok(terms) = WatchWallet::new().verified_covenant_terms(
        covenant_type,
        field(&invite, "rs"),
        address,
        network,
        field(&invite, "ldi"),
    ) else {
        return;
    };
    let terms: Value = serde_json::from_str(&terms).expect("verified terms are JSON");
    assert_eq!(terms["type"], covenant_type);
    assert_eq!(terms["address"], address);
});
