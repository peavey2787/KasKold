use kaspa_portal::transaction::{broadcast, consensus::ConsensusTransaction};

pub async fn submit(
    websocket_url: &str,
    transaction: &ConsensusTransaction,
) -> Result<String, String> {
    broadcast::submit(&super::client(websocket_url)?, transaction).await
}
