use crate::hub::{Connection, MatchmakingHub};
use crate::pb::{packet::Answer, packet::IceCandidate, packet::Start};
use crate::models::encode_connection_id;
use std::sync::Arc;
use tracing::{debug, warn};

use super::websocket::{send_answer_packet, send_ice_candidate_packet, send_offer_packet};

pub async fn handle_start(
    connection: &Arc<Connection>,
    hub: &Arc<MatchmakingHub>,
    session_id: &str,
    start: &Start,
) -> anyhow::Result<()> {
    let connection_id = encode_connection_id(&start.connection_id);

    // find_offerer reads every connection's attachment. We must not hold any
    // attachment lock while calling it.
    let offerer = hub.find_offerer(session_id).await;

    if offerer.is_none() {
        // No one waiting — become the offerer. Lock only our own attachment.
        let mut attachment = connection.attachment.write().await;
        attachment.offer_sdp = Some(start.offer_sdp.clone());
        attachment.connection_id = connection_id;
        debug!("[{}] Stored offer, waiting for answerer", session_id);
        return Ok(());
    }

    let offerer_conn = offerer.unwrap();

    // If the offerer we found is this very connection (e.g. a duplicate Start),
    // just refresh our own offer and return — avoids locking the same
    // attachment twice.
    if Arc::ptr_eq(&offerer_conn, connection) {
        let mut attachment = connection.attachment.write().await;
        attachment.offer_sdp = Some(start.offer_sdp.clone());
        attachment.connection_id = connection_id;
        debug!("[{}] Refreshed own offer", session_id);
        return Ok(());
    }

    // Read the offerer's current connection_id without holding the lock longer
    // than necessary.
    let offerer_connection_id = {
        let offerer_att = offerer_conn.attachment.read().await;
        offerer_att.connection_id.clone()
    };

    if let Some(ref conn_id) = connection_id {
        if Some(conn_id) == offerer_connection_id.as_ref() {
            // Same connection_id: offerer is reconnecting with a fresh offer.
            let mut new_att = offerer_conn.attachment.write().await;
            new_att.offer_sdp = Some(start.offer_sdp.clone());
            new_att.connection_id = connection_id;
            debug!(
                "[{}] Replaced stale offer from reconnecting offerer",
                session_id
            );
            return Ok(());
        }
    }

    // Different peer — this connection is the answerer. Pair the two so that
    // trickled ICE candidates can be routed to the correct counterpart, then
    // hand it the offerer's SDP so it can answer.
    let offer_sdp = {
        let offerer_att = offerer_conn.attachment.read().await;
        offerer_att.offer_sdp.clone().unwrap_or_default()
    };

    link_peers(connection, &offerer_conn).await;

    debug!("[{}] Answerer arrived, sending offer SDP", session_id);
    send_offer_packet(connection, &offer_sdp).await?;

    Ok(())
}

/// Record a mutual pairing between two connections. Trickled ICE candidates
/// coming from one are relayed to the other for the rest of the session.
async fn link_peers(a: &Arc<Connection>, b: &Arc<Connection>) {
    {
        let mut att = a.attachment.write().await;
        att.peer_id = Some(b.id);
    }
    {
        let mut att = b.attachment.write().await;
        att.peer_id = Some(a.id);
    }
}

pub async fn handle_answer(
    connection: &Arc<Connection>,
    hub: &Arc<MatchmakingHub>,
    session_id: &str,
    answer: &Answer,
) -> anyhow::Result<()> {
    let offerer = hub.find_offerer(session_id).await;

    if offerer.is_none() {
        warn!("[{}] Unexpected answer — no offerer found", session_id);
        return Err(anyhow::anyhow!("Unexpected answer - no offerer"));
    }

    let offerer_conn = offerer.unwrap();

    // Make sure the pairing is recorded (normally set when the answerer's Start
    // arrived) so both directions of ICE trickle route correctly.
    link_peers(connection, &offerer_conn).await;

    // Send answer to offerer with explicit error handling.
    if let Err(e) = send_answer_packet(&offerer_conn, &answer.sdp).await {
        warn!("[{}] Failed to send answer packet to offerer: {}", session_id, e);
    }

    // With trickle ICE the exchange isn't finished here: both peers keep the
    // socket open and trickle candidates until their connection comes up, then
    // each closes its own socket. The server no longer closes them or sends a
    // completion ping.
    debug!("[{}] Answer relayed, awaiting ICE candidates", session_id);

    Ok(())
}

/// Relay a trickled ICE candidate to the peer this connection is paired with.
/// Falls back to the only other connection in the session if the pairing hasn't
/// been recorded yet (candidates can, in principle, arrive before the answer is
/// processed).
pub async fn handle_ice_candidate(
    connection: &Arc<Connection>,
    hub: &Arc<MatchmakingHub>,
    session_id: &str,
    ice_candidate: &IceCandidate,
) -> anyhow::Result<()> {
    let peer_id = {
        let att = connection.attachment.read().await;
        att.peer_id
    };

    let target = match peer_id {
        Some(pid) => hub.find_connection_by_id(session_id, pid).await,
        None => None,
    };

    let target = match target {
        Some(t) => Some(t),
        None => hub.find_other_connection(session_id, connection.id).await,
    };

    match target {
        Some(target) => {
            if let Err(e) = send_ice_candidate_packet(&target, &ice_candidate.candidate).await {
                warn!(
                    "[{}] Failed to relay ICE candidate to peer: {}",
                    session_id, e
                );
            } else {
                debug!("[{}] Relayed ICE candidate to peer", session_id);
            }
        }
        None => {
            debug!(
                "[{}] Received ICE candidate but no peer to relay to yet",
                session_id
            );
        }
    }

    Ok(())
}
