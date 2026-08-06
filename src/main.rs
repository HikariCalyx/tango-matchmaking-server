mod broadcast;
mod config;
mod handlers;
mod hub;
mod messages;
mod models;
mod pb;
mod ice;

use axum::{
    extract::{ws::WebSocketUpgrade, Query},
    http::StatusCode,
    // response::IntoResponse, // unused
    routing::get,
    Router,
};
// use dashmap::DashMap; // unused
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use crate::config::Config;
use crate::hub::MatchmakingHub;

pub type SharedHub = Arc<MatchmakingHub>;
pub type SharedConfig = Arc<Config>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing_subscriber::filter::LevelFilter::INFO.into()),
        )
        .init();

    // Load configuration
    dotenv::dotenv().ok();
    let config = Arc::new(Config::from_env());

    info!("Starting Trill Signaling Server");
    info!(
        "Listening on {}:{}",
        config.server_host, config.server_port
    );

    // Initialize the matchmaking hub
    let hub = Arc::new(MatchmakingHub::new());

    // Spawn the broadcast background task (runs every 10 min when enabled)
    let broadcast_config = config.clone();
    tokio::spawn(async move {
        broadcast::broadcast_loop(broadcast_config).await;
    });

    // Build router
    let app = Router::new()
        .route("/ok", get(health_check_ok))
        .route("/health", get(health_check_json))
        .route("/", get(websocket_handler))
        .route("/ws", get(websocket_handler))
        .fallback(not_found)
        .with_state((hub, config.clone()));

    // Create socket address
    let addr: SocketAddr = format!("{}:{}", config.server_host, config.server_port)
        .parse()
        .expect("Failed to parse socket address");

    // Build listener
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    info!("Server listening on {}", addr);

    // Run server
    axum::serve(listener, app)
        .await
        .expect("Server error");

    info!("Trill Signaling Server shut down");
    Ok(())
}

async fn health_check_ok() -> String {
    "ok".to_string()
}

async fn health_check_json() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok" }))
}

async fn not_found() -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, "not found".to_string())
}

#[derive(serde::Deserialize)]
pub struct WebSocketQuery {
    session_id: String,
    /// Signaling protocol version, hex-encoded by the client
    /// (e.g. `protocol_version=56`). Optional for backwards compatibility with
    /// older clients that don't advertise it.
    protocol_version: Option<String>,
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WebSocketQuery>,
    axum::extract::State((hub, config)): axum::extract::State<(SharedHub, SharedConfig)>,
) -> impl axum::response::IntoResponse {
    let session_id = params.session_id;
    // The client encodes the protocol version as hex on the query string.
    let protocol_version = params
        .protocol_version
        .as_deref()
        .and_then(|v| u32::from_str_radix(v.trim_start_matches("0x").trim_start_matches("0X"), 16).ok());
    let hub_clone = Arc::clone(&hub);
    let config_clone = Arc::clone(&config);
    ws.on_upgrade(move |socket| async move {
        handlers::websocket::handle(socket, session_id, protocol_version, hub_clone, config_clone).await
    })
}
