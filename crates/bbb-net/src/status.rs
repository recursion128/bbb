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

    let (id, payload) = packets::encode_handshake(&host, port, ClientIntent::Status)?;
    conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_status_request();
    conn.send_packet(id, &payload).await?;

    let (packet_id, payload) = conn.read_packet().await?;
    if packet_id != ids::status::CLIENTBOUND_STATUS_RESPONSE {
        bail!("expected status response packet, got {packet_id}");
    }
    let json = packets::decode_status_response(&payload)?;

    let started = std::time::Instant::now();
    let ping_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bbb_protocol::{
        codec::{Decoder, Encoder},
        frame::encode_packet,
        PROTOCOL_VERSION,
    };
    use tokio::{io::AsyncWriteExt, net::TcpListener, time::timeout};

    use super::*;

    #[tokio::test]
    async fn status_ping_enforces_handshake_request_response_and_pong_sequence() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut connection = RawConnection::from_stream(stream, None);

            let (packet_id, payload) = connection.read_packet().await.unwrap();
            assert_eq!(packet_id, ids::handshake::SERVERBOUND_CLIENT_INTENTION);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), PROTOCOL_VERSION);
            assert_eq!(decoder.read_string(255).unwrap(), "127.0.0.1");
            assert_eq!(decoder.read_u16().unwrap(), address.port());
            assert_eq!(decoder.read_var_i32().unwrap(), ClientIntent::Status as i32);
            assert!(decoder.is_empty());

            let (packet_id, payload) = connection.read_packet().await.unwrap();
            assert_eq!(packet_id, ids::status::SERVERBOUND_STATUS_REQUEST);
            assert!(payload.is_empty());

            tokio::time::sleep(Duration::from_millis(200)).await;
            let mut response = Encoder::new();
            response.write_string(r#"{"version":{"name":"26.1"}}"#);
            connection
                .stream
                .write_all(&encode_packet(
                    ids::status::CLIENTBOUND_STATUS_RESPONSE,
                    &response.into_inner(),
                ))
                .await
                .unwrap();

            let (packet_id, payload) = connection.read_packet().await.unwrap();
            assert_eq!(packet_id, ids::status::SERVERBOUND_PING_REQUEST);
            assert_eq!(payload.len(), 8);
            connection
                .stream
                .write_all(&encode_packet(
                    ids::status::CLIENTBOUND_PONG_RESPONSE,
                    &payload,
                ))
                .await
                .unwrap();
        });

        let wall_started = std::time::Instant::now();
        let result = timeout(Duration::from_secs(2), status_ping(&address.to_string()))
            .await
            .expect("status ping should not hang")
            .unwrap();
        assert_eq!(result.json, r#"{"version":{"name":"26.1"}}"#);
        let wall_ms = wall_started.elapsed().as_millis();
        assert!(wall_ms >= 200);
        assert!(
            wall_ms.saturating_sub(result.latency_ms) >= 150,
            "wall time was {wall_ms} ms but ping latency was {} ms",
            result.latency_ms
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn status_ping_rejects_packets_outside_the_status_sequence() {
        for wrong_packet_id in [
            ids::status::CLIENTBOUND_PONG_RESPONSE,
            ids::status::CLIENTBOUND_STATUS_RESPONSE,
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut connection = RawConnection::from_stream(stream, None);
                connection.read_packet().await.unwrap();
                connection.read_packet().await.unwrap();

                let payload = if wrong_packet_id == ids::status::CLIENTBOUND_PONG_RESPONSE {
                    Vec::new()
                } else {
                    let mut response = Encoder::new();
                    response.write_string(r#"{}"#);
                    response.into_inner()
                };
                connection
                    .stream
                    .write_all(&encode_packet(wrong_packet_id, &payload))
                    .await
                    .unwrap();

                if wrong_packet_id == ids::status::CLIENTBOUND_STATUS_RESPONSE {
                    connection.read_packet().await.unwrap();
                    connection
                        .stream
                        .write_all(&encode_packet(wrong_packet_id, &payload))
                        .await
                        .unwrap();
                }
            });

            let error = timeout(Duration::from_secs(2), status_ping(&address.to_string()))
                .await
                .expect("status ping should not hang")
                .unwrap_err();
            let expected = if wrong_packet_id == ids::status::CLIENTBOUND_PONG_RESPONSE {
                "expected status response packet"
            } else {
                "expected pong response packet"
            };
            assert!(error.to_string().contains(expected), "{error:#}");
            server.await.unwrap();
        }
    }
}
