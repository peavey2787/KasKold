use kaspa_portal::network::queries::blocks;

pub async fn get_raw(websocket_url: &str, hash: &[u8; 32]) -> Result<Vec<u8>, String> {
    blocks::get_raw(&super::super::client(websocket_url)?, hash).await
}
