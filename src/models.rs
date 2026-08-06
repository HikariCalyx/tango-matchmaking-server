use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionAttachment {
    pub session_id: String,
    pub offer_sdp: Option<String>,
    pub connection_id: Option<String>, // hex-encoded
    /// The connection this one is paired with once the SDP exchange has begun.
    /// Used to route trickled ICE candidates to the correct peer.
    pub peer_id: Option<uuid::Uuid>,
    /// Signaling protocol version advertised by the client on the query string.
    pub protocol_version: Option<u32>,
}

impl SessionAttachment {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            offer_sdp: None,
            connection_id: None,
            peer_id: None,
            protocol_version: None,
        }
    }
}

pub const DEFAULT_ICE_SERVERS: &[(&str, Option<&str>, Option<&str>)] = &[
    ("stun:stun.l.google.com:19302", None, None),
    ("stun:stun1.l.google.com:19302", None, None),
    ("stun:stun2.l.google.com:19302", None, None),
    ("stun:stun3.l.google.com:19302", None, None),
    ("stun:stun4.l.google.com:19302", None, None),
];

pub fn encode_connection_id(connection_id: &[u8]) -> Option<String> {
    if connection_id.is_empty() {
        return None;
    }
    Some(hex::encode(connection_id))
}

impl From<(&str, Option<&str>, Option<&str>)> for IceServer {
    fn from((url, username, credential): (&str, Option<&str>, Option<&str>)) -> Self {
        IceServer {
            urls: vec![url.to_string()],
            username: username.map(|s| s.to_string()),
            credential: credential.map(|s| s.to_string()),
        }
    }
}
