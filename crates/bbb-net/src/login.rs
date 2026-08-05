use anyhow::{bail, ensure, Result};
use bbb_protocol::packets;
use uuid::Uuid;

use crate::connection::RawConnection;

pub(crate) async fn send_login_hello(
    conn: &mut RawConnection,
    username: &str,
    profile_id: Uuid,
) -> Result<()> {
    ensure!(
        !conn.login_hello_sent(),
        "login hello was already sent on this connection"
    );
    let (packet_id, payload) = packets::encode_login_hello(username, profile_id)?;
    conn.send_packet(packet_id, &payload).await?;
    conn.mark_login_hello_sent()
}

/// Online mode is a declared non-goal: this client keeps no Mojang session
/// credentials and no transport cipher, so a server that opens the vanilla
/// encryption handshake gets a clean, attributable failure instead of a
/// protocol stall.
pub(crate) fn reject_encryption_request() -> anyhow::Error {
    anyhow::anyhow!(
        "server requested login encryption: online-mode servers are unsupported by this \
         offline-only client"
    )
}

pub(crate) fn ensure_offline_login(conn: &RawConnection) -> Result<()> {
    if !conn.login_hello_sent() {
        bail!("received login encryption request before sending login hello");
    }
    Err(reject_encryption_request())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::ids;
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn login_hello_is_sent_once_and_encryption_requests_are_rejected() {
        let (mut client, mut server) = raw_connection_pair().await;
        let profile_id = Uuid::from_u128(7);
        send_login_hello(&mut client, "bbb-client", profile_id)
            .await
            .unwrap();
        let (packet_id, _) = server.read_packet().await.unwrap();
        assert_eq!(packet_id, ids::login::SERVERBOUND_HELLO);

        let duplicate = send_login_hello(&mut client, "bbb-client", profile_id)
            .await
            .unwrap_err();
        assert!(duplicate.to_string().contains("already sent"));

        let error = ensure_offline_login(&client).unwrap_err();
        assert!(error.to_string().contains("offline-only"));
        assert!(error.to_string().contains("online-mode servers"));
    }

    #[tokio::test]
    async fn encryption_request_before_hello_stays_a_protocol_order_error() {
        let (client, _server) = raw_connection_pair().await;
        let error = ensure_offline_login(&client).unwrap_err();
        assert!(error.to_string().contains("before sending login hello"));
    }

    async fn raw_connection_pair() -> (RawConnection, RawConnection) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        (
            RawConnection::from_stream(client, None),
            RawConnection::from_stream(server, None),
        )
    }
}
