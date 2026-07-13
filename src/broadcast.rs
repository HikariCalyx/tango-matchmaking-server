use crate::config::Config;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

/// JSON body sent to the server broadcast API.
#[derive(serde::Serialize)]
struct BroadcastPayload<'a> {
    server_name: &'a str,
    server_description: &'a str,
    country: &'a str,
    address: &'a str,
}

/// POST to `https://api.example.com/v1/ServerBroadcast` every 10 minutes.
pub async fn broadcast_loop(config: Arc<Config>) {
    // Only start the loop if broadcast is enabled.
    if !config.broadcast_server {
        info!("Broadcast server is disabled; skipping broadcast loop");
        return;
    }

    let client = reqwest::Client::new();
    let version = env!("CARGO_PKG_VERSION");
    let user_agent = format!("trilld/{}", version);

    let broadcast_url = "https://api.hikaricalyx.com/Trill/v5/ServerBroadcast";

    // Send an initial broadcast immediately so we don't wait 10 minutes.
    send_broadcast(&client, &config, broadcast_url, &user_agent).await;

    let mut interval = tokio::time::interval(Duration::from_secs(10 * 60));
    // The first tick fires immediately; skip it because we already sent one.
    interval.tick().await;

    loop {
        interval.tick().await;
        send_broadcast(&client, &config, broadcast_url, &user_agent).await;
    }
}

async fn send_broadcast(
    client: &reqwest::Client,
    config: &Config,
    url: &str,
    user_agent: &str,
) {
    let payload = BroadcastPayload {
        server_name: &config.server_name,
        server_description: &config.server_description,
        country: &config.server_country_code_alpha2,
        address: &config.endpoint_address,
    };

    match client
        .post(url)
        .header("User-Agent", user_agent)
        .json(&payload)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Broadcast sent successfully (status: {})", resp.status());
            } else {
                warn!(
                    "Broadcast returned non-success status: {} — body: {:?}",
                    resp.status(),
                    resp.text().await.unwrap_or_default()
                );
            }
        }
        Err(e) => {
            error!("Failed to send broadcast: {}", e);
        }
    }
}
