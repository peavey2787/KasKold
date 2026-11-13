use kaspa_portal::transaction::{broadcast, consensus::ConsensusTransaction};

pub async fn submit(
    websocket_url: &str,
    transaction: &ConsensusTransaction,
) -> Result<String, String> {
    broadcast::submit(&super::client(websocket_url)?, transaction).await
}

/// Submit `transaction` in place of the unconfirmed transaction it
/// double-spends; the node accepts it only at a higher feerate.
pub async fn replace(
    websocket_url: &str,
    transaction: &ConsensusTransaction,
) -> Result<String, String> {
    broadcast::submit_replacement(&super::client(websocket_url)?, transaction).await
}
