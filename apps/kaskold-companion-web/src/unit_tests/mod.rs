use super::{kaskold_sdk_limits, parse_sdk_network};
use kaskold_sdk::Network;

#[test]
fn the_shell_reports_the_sdk_limits_unchanged() {
    let expected = serde_json::to_string(&kaskold_sdk::limits()).expect("SDK limits serialize");
    assert_eq!(kaskold_sdk_limits(), Ok(expected));
}

#[test]
fn the_shell_accepts_every_network_name_the_sdk_prints() {
    for network in [
        Network::Mainnet,
        Network::Testnet10,
        Network::Testnet11,
        Network::Testnet12,
        Network::Devnet,
        Network::Simnet,
    ] {
        assert_eq!(parse_sdk_network(&network.to_string()), Ok(network));
    }
}
