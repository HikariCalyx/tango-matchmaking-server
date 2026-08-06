use axum::extract::ws::{WebSocket, Message as WsMessage};
use crate::handlers::messages::{handle_answer, handle_ice_candidate, handle_start};
use crate::hub::{Connection, MatchmakingHub, OutgoingMessage};
use crate::ice::get_ice_servers;
use crate::pb::{Packet, packet};
use crate::models::SessionAttachment;
use futures::{SinkExt, StreamExt};
use prost::Message;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{info, warn, debug};

pub async fn handle(
    ws: WebSocket,
    session_id: String,
    protocol_version: Option<u32>,
    hub: Arc<MatchmakingHub>,
    config: Arc<crate::config::Config>,
) {
    // Split the WebSocket into sender and receiver
    let (mut sender, mut receiver) = ws.split();

    // Reject clients whose signaling protocol version falls outside the range
    // this server is configured to matchmake for. The client reads this Abort
    // in place of the Hello and surfaces "update Tango" / "server is out of
    // date" accordingly.
    if let Some(reason) = config.protocol_version_abort_reason(protocol_version) {
        warn!(
            "[{}] Rejecting client (protocol_version={:?}): {}",
            session_id,
            protocol_version,
            reason.as_str_name()
        );
        let mut packet = Packet::default();
        packet.which = Some(packet::Which::Abort(crate::pb::Abort {
            reason: reason as i32,
        }));
        let _ = sender.send(WsMessage::Binary(packet.encode_to_vec())).await;
        let _ = sender.close().await;
        return;
    }

    // Create a message channel for sending data to this connection
    let (tx, mut rx) = mpsc::unbounded_channel();

    let connection_id = uuid::Uuid::new_v4();

    let mut attachment = SessionAttachment::new(session_id.clone());
    attachment.protocol_version = protocol_version;

    let connection = Arc::new(Connection {
        id: connection_id,
        tx: tx.clone(),
        attachment: Arc::new(RwLock::new(attachment)),
    });

    info!("Client connected to session {}", session_id);

    hub.add_connection(session_id.clone(), Arc::clone(&connection))
        .await;

    // Get ICE servers and send hello packet
    let ice_servers = get_ice_servers(&config).await;

    if let Err(e) = send_hello_packet(&connection, &ice_servers).await {
        warn!("[{}] Failed to send hello packet: {}", session_id, e);
        hub.remove_connection(&session_id, connection_id).await;
        return;
    }

    debug!("[{}] Sent hello", session_id);

    // Spawn a task to forward messages from the channel to the WebSocket sender
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                OutgoingMessage::Binary(data) => {
                    if sender.send(WsMessage::Binary(data)).await.is_err() {
                        break;
                    }
                }
                OutgoingMessage::Close => {
                    // Flush a polite close frame, then stop forwarding. This is
                    // how the server closes a peer's socket after the SDP
                    // exchange for legacy (non-trickle) clients.
                    let _ = sender.close().await;
                    break;
                }
            }
        }
    });

    // Main message loop
    loop {
        match receiver.next().await {
            Some(Ok(msg)) => {
                if let Err(e) = handle_ws_message(&connection, &hub, &session_id, msg).await {
                    debug!("[{}] Error handling message: {}", session_id, e);
                    break;
                }
            }
            Some(Err(_)) => {
                break;
            }
            None => {
                break;
            }
        }
    }

    info!("[{}] Client disconnected", session_id);
    hub.remove_connection(&session_id, connection_id).await;
    send_task.abort();
}

async fn handle_ws_message(
    connection: &Arc<Connection>,
    hub: &Arc<MatchmakingHub>,
    session_id: &str,
    msg: WsMessage,
) -> anyhow::Result<()> {
    match msg {
        WsMessage::Binary(data) => {
            handle_packet(connection, hub, session_id, &data).await?;
        }
        WsMessage::Close(_) => {
            return Err(anyhow::anyhow!("Connection closed"));
        }
        _ => {}
    }
    Ok(())
}

async fn handle_packet(
    connection: &Arc<Connection>,
    hub: &Arc<MatchmakingHub>,
    session_id: &str,
    data: &[u8],
) -> anyhow::Result<()> {
    let packet = Packet::decode(data)?;

    if packet.is_server_only() {
        warn!(
            "[{}] Unexpected server-only packet type from client",
            session_id
        );
        send_abort_packet(connection, 1).await?;
        return Err(anyhow::anyhow!("Unexpected packet type from client"));
    }

    match &packet.which {
        Some(packet::Which::Start(start)) => {
            // Do NOT hold a lock on this connection's attachment here.
            // handle_start calls find_offerer, which reads every connection's
            // attachment (including this one). Pre-locking it would self-deadlock.
            handle_start(connection, hub, session_id, start).await?;
        }
        Some(packet::Which::Answer(answer)) => {
            handle_answer(connection, hub, session_id, answer).await?;
        }
        Some(packet::Which::Ping(_)) => {
            send_ping_packet(connection).await?;
        }
        Some(packet::Which::IceCandidate(ice_candidate)) => {
            handle_ice_candidate(connection, hub, session_id, ice_candidate).await?;
        }
        _ => {
            debug!("[{}] Unknown or unhandled packet type, ignoring", session_id);
        }
    }

    Ok(())
}

pub async fn send_hello_packet(
    connection: &Arc<Connection>,
    ice_servers: &[crate::models::IceServer],
) -> anyhow::Result<()> {
    let mut packet = Packet::default();

    let ice_servers_pb: Vec<_> = ice_servers
        .iter()
        .map(|s| crate::pb::packet::hello::IceServer {
            urls: s.urls.clone(),
            username: s.username.clone(),
            credential: s.credential.clone(),
        })
        .collect();

    packet.which = Some(packet::Which::Hello(crate::pb::packet::Hello { 
        ice_servers: ice_servers_pb 
    }));

    let encoded = packet.encode_to_vec();
    debug!("Sending hello packet with {} ICE servers ({} bytes)", ice_servers.len(), encoded.len());
    connection.send_binary(encoded)?;

    Ok(())
}

pub async fn send_offer_packet(
    connection: &Arc<Connection>,
    sdp: &str,
) -> anyhow::Result<()> {
    let mut packet = Packet::default();
    packet.which = Some(packet::Which::Offer(crate::pb::Offer {
        sdp: sdp.to_string(),
    }));

    let encoded = packet.encode_to_vec();
    debug!("Sending offer packet ({} bytes)", encoded.len());
    connection.send_binary(encoded)?;

    Ok(())
}

pub async fn send_answer_packet(
    connection: &Arc<Connection>,
    sdp: &str,
) -> anyhow::Result<()> {
    let mut packet = Packet::default();
    packet.which = Some(packet::Which::Answer(crate::pb::Answer {
        sdp: sdp.to_string(),
    }));

    let encoded = packet.encode_to_vec();
    debug!("Sending answer packet ({} bytes)", encoded.len());
    connection.send_binary(encoded)?;

    Ok(())
}

pub async fn send_ping_packet(connection: &Arc<Connection>) -> anyhow::Result<()> {
    let mut packet = Packet::default();
    packet.which = Some(packet::Which::Ping(crate::pb::Ping {}));

    let encoded = packet.encode_to_vec();
    debug!("Sending ping packet ({} bytes)", encoded.len());
    connection.send_binary(encoded)?;

    Ok(())
}

pub async fn send_ice_candidate_packet(
    connection: &Arc<Connection>,
    candidate: &str,
) -> anyhow::Result<()> {
    let mut packet = Packet::default();
    packet.which = Some(packet::Which::IceCandidate(crate::pb::IceCandidate {
        candidate: candidate.to_string(),
    }));

    let encoded = packet.encode_to_vec();
    debug!("Sending ICE candidate packet ({} bytes)", encoded.len());
    connection.send_binary(encoded)?;

    Ok(())
}

pub async fn send_abort_packet(
    connection: &Arc<Connection>,
    reason: i32,
) -> anyhow::Result<()> {
    let mut packet = Packet::default();
    packet.which = Some(packet::Which::Abort(crate::pb::Abort { reason }));

    let encoded = packet.encode_to_vec();
    debug!("Sending abort packet ({} bytes) with reason {}", encoded.len(), reason);
    connection.send_binary(encoded)?;

    Ok(())
}
