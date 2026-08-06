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
    /// Lowest signaling protocol version this server will matchmake for.
    /// Clients advertising an older version are rejected with
    /// `REASON_PROTOCOL_VERSION_TOO_OLD`. `None` disables the lower bound.
    pub min_protocol_version: Option<u32>,
    /// Highest signaling protocol version this server will matchmake for.
    /// Clients advertising a newer version are rejected with
    /// `REASON_PROTOCOL_VERSION_TOO_NEW`. `None` disables the upper bound.
    pub max_protocol_version: Option<u32>,
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
            min_protocol_version: env::var("MIN_PROTOCOL_VERSION")
                .ok()
                .and_then(|v| parse_protocol_version(&v)),
            max_protocol_version: env::var("MAX_PROTOCOL_VERSION")
                .ok()
                .and_then(|v| parse_protocol_version(&v)),
        }
    }

    /// Decide whether a client advertising `protocol_version` should be turned
    /// away, and with which [`Reason`](crate::pb::packet::abort::Reason). Returns
    /// `None` when the client is acceptable (including when no bounds are
    /// configured, or when the client advertised no version at all).
    pub fn protocol_version_abort_reason(
        &self,
        protocol_version: Option<u32>,
    ) -> Option<crate::pb::packet::abort::Reason> {
        use crate::pb::packet::abort::Reason;
        let version = protocol_version?;
        if let Some(min) = self.min_protocol_version {
            if version < min {
                return Some(Reason::ProtocolVersionTooOld);
            }
        }
        if let Some(max) = self.max_protocol_version {
            if version > max {
                return Some(Reason::ProtocolVersionTooNew);
            }
        }
        None
    }
}

/// Parse a configured protocol version. Accepts either a hex value (with or
/// without a `0x` prefix, matching how the client encodes it on the query
/// string, e.g. `56`) or a plain decimal value.
fn parse_protocol_version(raw: &str) -> Option<u32> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        return u32::from_str_radix(hex, 16).ok();
    }
    // Bare value: try hex first (the client's on-wire encoding), then decimal.
    u32::from_str_radix(trimmed, 16)
        .ok()
        .or_else(|| trimmed.parse::<u32>().ok())
}
