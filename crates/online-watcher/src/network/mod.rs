pub(crate) use kaspa_portal::network::codec;

use kaspa_portal::network::client::NetworkClient;

pub(crate) mod queries;
pub(crate) mod submission;

/// Open a Kaspa wRPC client for `websocket_url` over Portal's browser transport.
#[cfg(target_arch = "wasm32")]
pub(crate) fn client(websocket_url: &str) -> Result<NetworkClient, String> {
    kaspa_portal::platform::browser::websocket::BrowserWebSocketTransport::new(websocket_url)
        .map(NetworkClient::new)
        .map_err(String::from)
}

/// The Companion talks to Kaspa only from the browser.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn client(websocket_url: &str) -> Result<NetworkClient, String> {
    Err(format!(
        "browser WebSocket transport is unavailable on native hosts (endpoint={websocket_url})"
    ))
}

#[cfg(test)]
mod unit_tests;
