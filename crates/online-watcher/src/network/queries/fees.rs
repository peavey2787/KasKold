use kaspa_portal::network::{model::fee_estimate::FeeEstimate, queries::fees};

pub async fn get(websocket_url: &str) -> Result<FeeEstimate, String> {
    fees::get(&super::super::client(websocket_url)?).await
}
