use anyhow::{bail, Result};
use bbb_protocol::{
    ids,
    packets::{self, ClientIntent},
};

use crate::{
    connection::RawConnection,
    types::{split_host_port, StatusPing},
};

pub async fn status_ping(address: &str) -> Result<StatusPing> {
    let (host, port) = split_host_port(address)?;
    let mut conn = RawConnection::connect(address, None).await?;

    let (id, payload) = packets::encode_handshake(&host, port, ClientIntent::Status);
    conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_status_request();
    conn.send_packet(id, &payload).await?;

    let started = std::time::Instant::now();
    let (packet_id, payload) = conn.read_packet().await?;
    if packet_id != ids::status::CLIENTBOUND_STATUS_RESPONSE {
        bail!("expected status response packet, got {packet_id}");
    }
    let json = packets::decode_status_response(&payload)?;

    let ping_time = started.elapsed().as_millis() as i64;
    let (id, payload) = packets::encode_ping_request(ping_time);
    conn.send_packet(id, &payload).await?;
    let (packet_id, payload) = conn.read_packet().await?;
    if packet_id != ids::status::CLIENTBOUND_PONG_RESPONSE {
        bail!("expected pong response packet, got {packet_id}");
    }
    let _ = packets::decode_pong_response(&payload)?;

    Ok(StatusPing {
        json,
        latency_ms: started.elapsed().as_millis(),
    })
}
