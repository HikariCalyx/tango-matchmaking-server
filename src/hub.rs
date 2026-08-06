use crate::models::SessionAttachment;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::debug;

pub type WsMessageSender = mpsc::UnboundedSender<Vec<u8>>;

pub struct Connection {
    pub id: uuid::Uuid,
    pub tx: WsMessageSender,
    pub attachment: Arc<RwLock<SessionAttachment>>,
}

pub struct MatchmakingHub {
    // session_id -> Vec<(Connection, attachment)>
    connections: DashMap<String, Vec<Arc<Connection>>>,
}

impl MatchmakingHub {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    pub async fn add_connection(
        &self,
        session_id: String,
        connection: Arc<Connection>,
    ) {
        self.connections
            .entry(session_id.clone())
            .or_insert_with(Vec::new)
            .push(connection);
        debug!("Added connection for session {}", session_id);
    }

    pub async fn remove_connection(&self, session_id: &str, connection_id: uuid::Uuid) {
        if let Some(mut conns) = self.connections.get_mut(session_id) {
            conns.retain(|c| c.id != connection_id);
            if conns.is_empty() {
                drop(conns);
                self.connections.remove(session_id);
            }
        }
        debug!("Removed connection for session {}", session_id);
    }

    pub async fn find_offerer(&self, session_id: &str) -> Option<Arc<Connection>> {
        // Snapshot the connection list and release the DashMap shard guard
        // BEFORE awaiting any attachment locks. Holding a DashMap Ref across an
        // `.await` keeps the shard locked, which blocks add_connection/find_offerer
        // for every other connection whose session hashes to the same shard.
        let conns: Vec<Arc<Connection>> = match self.connections.get(session_id) {
            Some(entry) => entry.iter().cloned().collect(),
            None => return None,
        };

        for conn in conns {
            let att = conn.attachment.read().await;
            if att.offer_sdp.is_some() {
                drop(att);
                return Some(conn);
            }
        }
        None
    }

    /// Look up a specific connection within a session by its id.
    pub async fn find_connection_by_id(
        &self,
        session_id: &str,
        connection_id: uuid::Uuid,
    ) -> Option<Arc<Connection>> {
        self.connections
            .get(session_id)
            .and_then(|conns| conns.iter().find(|c| c.id == connection_id).cloned())
    }

    /// Find the other connection in a session — the one that isn't `exclude_id`.
    /// Used as a fallback for routing trickled ICE candidates when an explicit
    /// peer pairing hasn't been recorded yet. A session only ever holds the two
    /// peers sharing a link code, so "the other one" is unambiguous.
    pub async fn find_other_connection(
        &self,
        session_id: &str,
        exclude_id: uuid::Uuid,
    ) -> Option<Arc<Connection>> {
        self.connections
            .get(session_id)
            .and_then(|conns| conns.iter().find(|c| c.id != exclude_id).cloned())
    }

    pub async fn get_all_connections(&self, session_id: &str) -> Vec<Arc<Connection>> {
        self.connections
            .get(session_id)
            .map(|conns| conns.clone())
            .unwrap_or_default()
    }

    pub fn connections_count(&self, session_id: &str) -> usize {
        self.connections
            .get(session_id)
            .map(|conns| conns.len())
            .unwrap_or(0)
    }
}

impl Default for MatchmakingHub {
    fn default() -> Self {
        Self::new()
    }
}
