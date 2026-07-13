use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub turn_addr: Option<String>,
    pub turn_user: Option<String>,
    pub turn_credential: Option<String>,
    /// Shared `static-auth-secret` with the co-located coturn instance, used to
    /// mint time-limited (REST API) credentials when static credentials are disabled.
    pub turn_secret: Option<String>,
    /// TTL, in seconds, for dynamically generated TURN credentials.
    pub turn_credential_ttl: u64,
    pub debug: bool,
    pub use_static_turn_credential: bool,
    pub log_level: String,
    /// Broadcast server to HCT API
    pub broadcast_server: bool,
    pub server_name: String,
    pub server_description: String,
    pub server_country_code_alpha2: String,
    pub endpoint_address: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()
                .unwrap_or(8000),
            turn_addr: env::var("TURN_ADDR").ok(),
            turn_user: env::var("TURN_USER").ok(),
            turn_credential: env::var("TURN_CREDENTIAL").ok(),
            turn_secret: env::var("TURN_SECRET").ok(),
            turn_credential_ttl: env::var("TURN_CREDENTIAL_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            debug: env::var("DEBUG")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),
            use_static_turn_credential: env::var("USE_STATIC_TURN_CREDENTIAL")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string()),
            broadcast_server: env::var("BROADCAST_SERVER")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),
            server_name: env::var("SERVER_NAME").unwrap_or_default(),
            server_description: env::var("SERVER_DESCRIPTION").unwrap_or_default(),
            server_country_code_alpha2: env::var("SERVER_COUNTRY_CODE_ALPHA2")
                .unwrap_or_default(),
            endpoint_address: env::var("ENDPOINT_ADDRESS").unwrap_or_default(),
        }
    }
}
