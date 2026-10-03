use kaspa_portal::network::queries::chain;

pub async fn virtual_daa_score(websocket_url: &str) -> Result<u64, String> {
    chain::virtual_daa_score(&super::super::client(websocket_url)?).await
}
