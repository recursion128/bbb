//! Live profile-key fetch, refresh, and encoder installation scheduling.

use std::{fmt, future::pending, time::Duration};

use tokio::{task::JoinHandle, time::Instant};
use uuid::Uuid;

use super::{
    fetch_profile_key_pair_at, LocalChatSession, ProfileKeyPair, PROFILE_CERTIFICATES_TIMEOUT,
    PROFILE_CERTIFICATES_URL,
};
use crate::types::SessionCredentials;

const MINIMUM_PROFILE_KEY_REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60);

pub(crate) enum ChatSessionAction {
    Install(LocalChatSession),
}

pub(crate) struct ChatSessionRuntime {
    profile_id: Uuid,
    profile_keys: ProfileKeyManager,
    play_encrypted: bool,
    active_session: Option<LocalChatSession>,
}

struct ProfileKeyManager {
    credentials: Option<SessionCredentials>,
    request_timeout: Duration,
    endpoint: String,
    minimum_refresh_interval: Duration,
    pending_fetch: Option<JoinHandle<anyhow::Result<ProfileKeyPair>>>,
    accepted_key: Option<ProfileKeyPair>,
    next_refresh_allowed: Option<Instant>,
    refresh_deadline: Option<Instant>,
}

impl fmt::Debug for ChatSessionRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChatSessionRuntime")
            .field("profile_id", &self.profile_id)
            .field("profile_keys", &self.profile_keys)
            .field("play_encrypted", &self.play_encrypted)
            .field("session_active", &self.active_session.is_some())
            .finish()
    }
}

impl fmt::Debug for ProfileKeyManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileKeyManager")
            .field(
                "credentials",
                &self.credentials.as_ref().map(|_| "<redacted>"),
            )
            .field("request_timeout", &self.request_timeout)
            .field("endpoint", &self.endpoint)
            .field("minimum_refresh_interval", &self.minimum_refresh_interval)
            .field("fetch_pending", &self.pending_fetch.is_some())
            .field("accepted_key", &self.accepted_key.is_some())
            .field("next_refresh_allowed", &self.next_refresh_allowed)
            .field("refresh_deadline", &self.refresh_deadline)
            .finish()
    }
}

impl ChatSessionRuntime {
    pub(crate) fn new(
        profile_id: Uuid,
        credentials: Option<SessionCredentials>,
        request_timeout: Duration,
    ) -> Self {
        Self::with_endpoint_and_refresh_interval(
            profile_id,
            credentials,
            request_timeout,
            PROFILE_CERTIFICATES_URL,
            MINIMUM_PROFILE_KEY_REFRESH_INTERVAL,
        )
    }

    pub(crate) fn with_endpoint_and_refresh_interval(
        profile_id: Uuid,
        credentials: Option<SessionCredentials>,
        request_timeout: Duration,
        endpoint: impl Into<String>,
        minimum_refresh_interval: Duration,
    ) -> Self {
        Self::from_parts(
            profile_id,
            credentials,
            request_timeout,
            endpoint.into(),
            minimum_refresh_interval,
        )
    }

    fn from_parts(
        profile_id: Uuid,
        credentials: Option<SessionCredentials>,
        request_timeout: Duration,
        endpoint: String,
        minimum_refresh_interval: Duration,
    ) -> Self {
        let profile_keys = ProfileKeyManager::new(
            credentials,
            request_timeout,
            endpoint,
            minimum_refresh_interval,
        );
        Self {
            profile_id,
            profile_keys,
            play_encrypted: false,
            active_session: None,
        }
    }

    pub(crate) fn begin_play_session(&mut self, encrypted: bool) {
        self.reset_play_session();
        self.play_encrypted = encrypted;
        if encrypted {
            self.profile_keys.prepare();
        }
    }

    pub(crate) fn reset_play_session(&mut self) {
        self.play_encrypted = false;
        self.active_session = None;
        self.profile_keys.refresh_deadline = None;
    }

    pub(crate) fn active_session_mut(&mut self) -> Option<&mut LocalChatSession> {
        self.active_session.as_mut()
    }

    pub(crate) fn install(&mut self, session: LocalChatSession) {
        self.active_session = Some(session);
        self.profile_keys
            .schedule_refresh_check(self.active_session.as_ref());
    }

    pub(crate) async fn next_action(&mut self) -> ChatSessionAction {
        loop {
            if self.play_encrypted
                && self.active_session.is_none()
                && self.profile_keys.pending_fetch.is_none()
            {
                if let Some(key_pair) = self.profile_keys.accepted_key.clone() {
                    return ChatSessionAction::Install(LocalChatSession::create(
                        self.profile_id,
                        key_pair,
                    ));
                }
            }

            if let Some(fetch) = self.profile_keys.pending_fetch.as_mut() {
                let result = fetch.await;
                self.profile_keys.pending_fetch = None;
                match result {
                    Ok(Ok(key_pair)) => {
                        self.profile_keys.accepted_key = Some(key_pair.clone());
                        if self.play_encrypted
                            && !self.active_session.as_ref().is_some_and(|session| {
                                session.key_pair().same_key_material(&key_pair)
                            })
                        {
                            return ChatSessionAction::Install(LocalChatSession::create(
                                self.profile_id,
                                key_pair,
                            ));
                        }
                    }
                    Ok(Err(error)) => {
                        tracing::warn!(%error, "profile-key preparation failed");
                    }
                    Err(error) => {
                        tracing::warn!(%error, "profile-key preparation task failed");
                    }
                }
                self.profile_keys
                    .schedule_refresh_check(self.active_session.as_ref());
                continue;
            }

            if let Some(deadline) = self.profile_keys.refresh_deadline {
                tokio::time::sleep_until(deadline).await;
                self.profile_keys.refresh_deadline = None;
                if self
                    .profile_keys
                    .refresh_is_due(self.active_session.as_ref())
                {
                    self.profile_keys.prepare();
                } else {
                    self.profile_keys
                        .schedule_refresh_check(self.active_session.as_ref());
                }
                continue;
            }

            pending::<()>().await;
        }
    }
}

impl ProfileKeyManager {
    fn new(
        credentials: Option<SessionCredentials>,
        request_timeout: Duration,
        endpoint: String,
        minimum_refresh_interval: Duration,
    ) -> Self {
        let mut manager = Self {
            credentials,
            request_timeout,
            endpoint,
            minimum_refresh_interval,
            pending_fetch: None,
            accepted_key: None,
            next_refresh_allowed: None,
            refresh_deadline: None,
        };
        manager.prepare();
        manager
    }

    fn prepare(&mut self) {
        self.next_refresh_allowed = Some(Instant::now() + self.minimum_refresh_interval);
        self.refresh_deadline = None;
        let needs_fetch = self
            .accepted_key
            .as_ref()
            .is_none_or(|key_pair| key_pair.due_refresh_at(epoch_millis_now()));
        if self.pending_fetch.is_none() && needs_fetch {
            self.start_fetch();
        }
    }

    fn start_fetch(&mut self) {
        let Some(credentials) = self.credentials.as_ref() else {
            return;
        };
        let access_token = credentials.access_token.clone();
        let timeout = self.request_timeout.min(PROFILE_CERTIFICATES_TIMEOUT);
        let endpoint = self.endpoint.clone();
        self.pending_fetch = Some(tokio::spawn(async move {
            fetch_profile_key_pair_at(&access_token, timeout, &endpoint).await
        }));
    }

    fn refresh_is_due(&self, active_session: Option<&LocalChatSession>) -> bool {
        let allowed = self
            .next_refresh_allowed
            .is_none_or(|deadline| Instant::now() >= deadline);
        let key_due = active_session
            .is_none_or(|session| session.key_pair().due_refresh_at(epoch_millis_now()));
        allowed && key_due
    }

    fn schedule_refresh_check(&mut self, active_session: Option<&LocalChatSession>) {
        let Some(active_session) = active_session else {
            self.refresh_deadline = None;
            return;
        };
        let now = Instant::now();
        let allowed = self.next_refresh_allowed.unwrap_or(now);
        let remaining_millis = active_session
            .key_pair()
            .refreshed_after_epoch_millis()
            .saturating_sub(epoch_millis_now())
            .saturating_add(1);
        let due = now + Duration::from_millis(remaining_millis.max(0) as u64);
        self.refresh_deadline = Some(allowed.max(due));
    }
}

impl Drop for ProfileKeyManager {
    fn drop(&mut self) {
        if let Some(fetch) = self.pending_fetch.take() {
            fetch.abort();
        }
    }
}

fn epoch_millis_now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use bbb_protocol::{codec::Decoder, ids, packets::PlayerPositionState};
    use bbb_world::OutboundLastSeenMessages;
    use rand::{rngs::StdRng, SeedableRng};
    use rsa::{
        pkcs8::{EncodePrivateKey, EncodePublicKey},
        RsaPrivateKey, RsaPublicKey,
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::{mpsc, oneshot},
        time::timeout,
    };

    use super::*;
    use crate::{
        chat_session::PROFILE_KEY_BITS,
        connection::RawConnection,
        driver::{read_packet_or_drive_connection_with_chat_session, ConnectionDrive},
        types::{ConnectionState, NetCommand, OutboundChatMessage},
    };

    fn credentials(profile_id: Uuid, access_token: &str) -> SessionCredentials {
        SessionCredentials {
            profile_id,
            access_token: access_token.to_string(),
        }
    }

    fn fixture_private_key(index: usize) -> RsaPrivateKey {
        static KEYS: OnceLock<[RsaPrivateKey; 2]> = OnceLock::new();
        KEYS.get_or_init(|| {
            [261_u64, 262_u64].map(|seed| {
                let mut random = StdRng::seed_from_u64(seed);
                RsaPrivateKey::new(&mut random, PROFILE_KEY_BITS).unwrap()
            })
        })[index]
            .clone()
    }

    fn pem(label: &str, der: &[u8]) -> String {
        format!(
            "-----BEGIN {label}-----\n{}\n-----END {label}-----\n",
            STANDARD.encode(der)
        )
    }

    fn fixture_response(index: usize) -> Vec<u8> {
        fixture_response_with_refreshed_after(index, "2020-01-01T00:00:00Z")
    }

    fn fresh_fixture_response(index: usize) -> Vec<u8> {
        fixture_response_with_refreshed_after(index, "2030-01-01T00:00:00Z")
    }

    fn fixture_response_with_refreshed_after(index: usize, refreshed_after: &str) -> Vec<u8> {
        let private_key = fixture_private_key(index);
        let private_der = private_key.to_pkcs8_der().unwrap();
        let public_der = RsaPublicKey::from(&private_key)
            .to_public_key_der()
            .unwrap();
        serde_json::to_vec(&serde_json::json!({
            "keyPair": {
                "privateKey": pem("RSA PRIVATE KEY", private_der.as_bytes()),
                "publicKey": pem("RSA PUBLIC KEY", public_der.as_bytes()),
            },
            "publicKeySignatureV2": STANDARD.encode(vec![index as u8 + 1; 512]),
            "expiresAt": "2030-01-02T03:04:05.006Z",
            "refreshedAfter": refreshed_after,
        }))
        .unwrap()
    }

    async fn profile_server(
        responses: Vec<(u16, Vec<u8>)>,
    ) -> (String, mpsc::Receiver<usize>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = mpsc::channel(responses.len().max(1));
        let server = tokio::spawn(async move {
            for (index, (status, body)) in responses.into_iter().enumerate() {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = vec![0_u8; 16 * 1024];
                let read = socket.read(&mut request).await.unwrap();
                let request = String::from_utf8_lossy(&request[..read]);
                assert!(request.starts_with("POST /player/certificates HTTP/1.1\r\n"));
                assert!(request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer runtime-access-token\r\n"));
                request_tx.send(index).await.unwrap();
                let reason = if status == 200 {
                    "OK"
                } else {
                    "Internal Server Error"
                };
                socket
                    .write_all(
                        format!(
                            "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
                socket.write_all(&body).await.unwrap();
            }
        });
        (
            format!("http://{address}/player/certificates"),
            request_rx,
            server,
        )
    }

    async fn raw_connection_pair() -> (RawConnection, RawConnection) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = tokio::spawn(async move {
            RawConnection::connect(&address.to_string(), None)
                .await
                .unwrap()
        });
        let (server_stream, _) = listener.accept().await.unwrap();
        let client = client.await.unwrap();
        (client, RawConnection::from_stream(server_stream, None))
    }

    #[tokio::test]
    async fn preparation_starts_before_play_and_offline_mode_never_fetches() {
        let profile_id = Uuid::from_u128(31);
        let (endpoint, mut requests, server) =
            profile_server(vec![(200, fresh_fixture_response(0))]).await;
        let mut runtime = ChatSessionRuntime::with_endpoint_and_refresh_interval(
            profile_id,
            Some(credentials(profile_id, "runtime-access-token")),
            Duration::from_secs(1),
            endpoint,
            Duration::from_secs(60),
        );

        assert_eq!(requests.recv().await, Some(0));
        runtime.begin_play_session(false);
        assert!(timeout(Duration::from_millis(20), runtime.next_action())
            .await
            .is_err());
        runtime.begin_play_session(true);
        let ChatSessionAction::Install(session) = runtime.next_action().await;
        assert_eq!(session.profile_id(), profile_id);

        let mut offline = ChatSessionRuntime::with_endpoint_and_refresh_interval(
            profile_id,
            None,
            Duration::from_secs(1),
            "http://127.0.0.1:1/player/certificates",
            Duration::from_millis(1),
        );
        offline.begin_play_session(true);
        assert!(timeout(Duration::from_millis(20), offline.next_action())
            .await
            .is_err());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn refresh_waits_for_minimum_interval_and_replaces_changed_key() {
        let profile_id = Uuid::from_u128(32);
        let (endpoint, mut requests, server) =
            profile_server(vec![(200, fixture_response(0)), (200, fixture_response(1))]).await;
        let mut runtime = ChatSessionRuntime::with_endpoint_and_refresh_interval(
            profile_id,
            Some(credentials(profile_id, "runtime-access-token")),
            Duration::from_secs(1),
            endpoint,
            Duration::from_millis(60),
        );

        runtime.begin_play_session(true);
        assert_eq!(requests.recv().await, Some(0));
        let ChatSessionAction::Install(first) = runtime.next_action().await;
        let first_session_id = first.session_id();
        runtime.install(first);
        assert!(timeout(Duration::from_millis(20), runtime.next_action())
            .await
            .is_err());

        let ChatSessionAction::Install(second) =
            timeout(Duration::from_secs(1), runtime.next_action())
                .await
                .unwrap();
        assert_eq!(requests.recv().await, Some(1));
        assert_ne!(second.session_id(), first_session_id);
        assert!(!runtime
            .active_session
            .as_ref()
            .unwrap()
            .key_pair()
            .same_key_material(second.key_pair()));
        runtime.install(second);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn refresh_failure_keeps_active_session_and_debug_redacts_token() {
        let profile_id = Uuid::from_u128(33);
        let (endpoint, mut requests, server) =
            profile_server(vec![(200, fixture_response(0)), (500, Vec::new())]).await;
        let mut runtime = ChatSessionRuntime::with_endpoint_and_refresh_interval(
            profile_id,
            Some(credentials(profile_id, "runtime-access-token")),
            Duration::from_secs(1),
            endpoint,
            Duration::from_millis(20),
        );
        let debug = format!("{runtime:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("runtime-access-token"));

        runtime.begin_play_session(true);
        assert_eq!(requests.recv().await, Some(0));
        let ChatSessionAction::Install(first) = runtime.next_action().await;
        let first_session_id = first.session_id();
        runtime.install(first);
        assert!(timeout(Duration::from_millis(30), runtime.next_action())
            .await
            .is_err());
        assert_eq!(requests.recv().await, Some(1));
        assert_eq!(
            runtime.active_session.as_ref().unwrap().session_id(),
            first_session_id
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn unchanged_refresh_keeps_the_existing_session_chain() {
        let profile_id = Uuid::from_u128(38);
        let response = fixture_response(0);
        let (endpoint, mut requests, server) =
            profile_server(vec![(200, response.clone()), (200, response)]).await;
        let mut runtime = ChatSessionRuntime::with_endpoint_and_refresh_interval(
            profile_id,
            Some(credentials(profile_id, "runtime-access-token")),
            Duration::from_secs(1),
            endpoint,
            Duration::from_millis(20),
        );

        runtime.begin_play_session(true);
        assert_eq!(requests.recv().await, Some(0));
        let ChatSessionAction::Install(first) = runtime.next_action().await;
        let first_session_id = first.session_id();
        runtime.install(first);
        assert!(timeout(Duration::from_millis(30), runtime.next_action())
            .await
            .is_err());
        assert_eq!(requests.recv().await, Some(1));
        assert_eq!(
            runtime.active_session.as_ref().unwrap().session_id(),
            first_session_id
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn reset_drops_only_the_play_local_chain() {
        let profile_id = Uuid::from_u128(34);
        let (endpoint, mut requests, server) =
            profile_server(vec![(200, fixture_response(0))]).await;
        let mut runtime = ChatSessionRuntime::with_endpoint_and_refresh_interval(
            profile_id,
            Some(credentials(profile_id, "runtime-access-token")),
            Duration::from_secs(1),
            endpoint,
            Duration::from_secs(1),
        );
        runtime.begin_play_session(true);
        assert_eq!(requests.recv().await, Some(0));
        let ChatSessionAction::Install(session) = runtime.next_action().await;
        runtime.install(session);
        assert!(runtime.active_session_mut().is_some());

        runtime.reset_play_session();
        assert!(runtime.active_session_mut().is_none());
        assert!(timeout(Duration::from_millis(20), runtime.next_action())
            .await
            .is_err());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn reset_preserves_pending_prepare_for_the_next_play_session() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let endpoint = format!("http://{address}/player/certificates");
        let response = fresh_fixture_response(0);
        let (request_tx, request_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 4096];
            let _ = socket.read(&mut request).await.unwrap();
            request_tx.send(()).unwrap();
            let _ = release_rx.await;
            let _ = socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        response.len()
                    )
                    .as_bytes(),
                )
                .await;
            let _ = socket.write_all(&response).await;
        });
        let profile_id = Uuid::from_u128(35);
        let mut runtime = ChatSessionRuntime::with_endpoint_and_refresh_interval(
            profile_id,
            Some(credentials(profile_id, "runtime-access-token")),
            Duration::from_secs(1),
            endpoint,
            Duration::from_secs(1),
        );

        request_rx.await.unwrap();
        runtime.reset_play_session();
        release_tx.send(()).unwrap();
        server.await.unwrap();
        assert!(timeout(Duration::from_millis(20), runtime.next_action())
            .await
            .is_err());
        assert!(runtime.profile_keys.accepted_key.is_some());

        runtime.begin_play_session(true);
        let ChatSessionAction::Install(session) =
            timeout(Duration::from_millis(20), runtime.next_action())
                .await
                .unwrap();
        assert_eq!(session.profile_id(), profile_id);
    }

    #[tokio::test]
    async fn fresh_accepted_key_is_reused_but_the_play_chain_is_recreated() {
        let profile_id = Uuid::from_u128(39);
        let (endpoint, mut requests, server) = profile_server(vec![
            (200, fresh_fixture_response(0)),
            (200, fresh_fixture_response(1)),
        ])
        .await;
        let mut runtime = ChatSessionRuntime::with_endpoint_and_refresh_interval(
            profile_id,
            Some(credentials(profile_id, "runtime-access-token")),
            Duration::from_secs(1),
            endpoint,
            Duration::from_millis(20),
        );

        assert_eq!(requests.recv().await, Some(0));
        runtime.begin_play_session(true);
        let ChatSessionAction::Install(first) = runtime.next_action().await;
        let first_session_id = first.session_id();
        runtime.install(first);

        runtime.reset_play_session();
        runtime.begin_play_session(true);
        let ChatSessionAction::Install(second) =
            timeout(Duration::from_millis(20), runtime.next_action())
                .await
                .unwrap();
        assert_ne!(second.session_id(), first_session_id);
        assert!(runtime
            .profile_keys
            .accepted_key
            .as_ref()
            .unwrap()
            .same_key_material(second.key_pair()));
        assert!(timeout(Duration::from_millis(20), requests.recv())
            .await
            .is_err());
        server.abort();
    }

    #[tokio::test]
    async fn driver_sends_session_update_before_first_signed_message() {
        let profile_id = Uuid::from_u128(36);
        let (endpoint, mut requests, profile_server) =
            profile_server(vec![(200, fixture_response(0))]).await;
        let mut runtime = ChatSessionRuntime::with_endpoint_and_refresh_interval(
            profile_id,
            Some(credentials(profile_id, "runtime-access-token")),
            Duration::from_secs(1),
            endpoint,
            Duration::from_secs(60),
        );
        runtime.begin_play_session(true);
        let (mut client, mut server) = raw_connection_pair().await;
        let (command_tx, mut commands) = mpsc::channel(2);
        let drive = tokio::spawn(async move {
            let mut position = PlayerPositionState::default();
            let result = read_packet_or_drive_connection_with_chat_session(
                &mut client,
                ConnectionState::Play,
                &mut commands,
                &mut position,
                false,
                &mut runtime,
            )
            .await
            .unwrap();
            assert!(matches!(result, ConnectionDrive::Disconnect));
        });

        assert_eq!(requests.recv().await, Some(0));
        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT_SESSION_UPDATE);
        let mut decoder = Decoder::new(&payload);
        let session_id = decoder.read_uuid().unwrap();
        assert_ne!(session_id, Uuid::nil());
        assert_eq!(decoder.read_i64().unwrap(), 1_893_553_445_006);
        let public_key_len = decoder.read_var_i32().unwrap() as usize;
        assert!(public_key_len > 0);
        decoder
            .read_exact(public_key_len, "profile public key")
            .unwrap();
        assert_eq!(decoder.read_var_i32().unwrap(), 512);
        decoder
            .read_exact(512, "profile public key signature")
            .unwrap();
        assert!(decoder.is_empty());

        command_tx
            .send(NetCommand::ChatMessage(OutboundChatMessage {
                message: "signed after update".to_string(),
                timestamp_millis: 4_000,
                salt: 41,
                last_seen: OutboundLastSeenMessages::default(),
            }))
            .await
            .unwrap();
        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(256).unwrap(), "signed after update");
        assert_eq!(decoder.read_i64().unwrap(), 4_000);
        assert_eq!(decoder.read_i64().unwrap(), 41);
        assert!(decoder.read_bool().unwrap());
        assert_eq!(
            decoder.read_exact(256, "message signature").unwrap().len(),
            256
        );
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(
            decoder.read_exact(3, "last seen bitset").unwrap(),
            &[0, 0, 0]
        );
        assert_eq!(decoder.read_u8().unwrap(), 1);
        assert!(decoder.is_empty());

        command_tx.send(NetCommand::Disconnect).await.unwrap();
        drive.await.unwrap();
        profile_server.await.unwrap();
    }

    #[tokio::test]
    async fn pending_profile_fetch_does_not_block_packet_reads() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let endpoint = format!("http://{address}/player/certificates");
        let (request_tx, request_rx) = oneshot::channel();
        let profile_server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 4096];
            let _ = socket.read(&mut request).await.unwrap();
            request_tx.send(()).unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
        let profile_id = Uuid::from_u128(37);
        let mut runtime = ChatSessionRuntime::with_endpoint_and_refresh_interval(
            profile_id,
            Some(credentials(profile_id, "runtime-access-token")),
            Duration::from_secs(2),
            endpoint,
            Duration::from_secs(60),
        );
        runtime.begin_play_session(true);
        request_rx.await.unwrap();

        let (mut client, mut server) = raw_connection_pair().await;
        server
            .send_packet(0x2a, b"packet while fetching")
            .await
            .unwrap();
        let (_command_tx, mut commands) = mpsc::channel(1);
        let mut position = PlayerPositionState::default();
        let drive = timeout(
            Duration::from_millis(100),
            read_packet_or_drive_connection_with_chat_session(
                &mut client,
                ConnectionState::Play,
                &mut commands,
                &mut position,
                false,
                &mut runtime,
            ),
        )
        .await
        .expect("packet read must win while profile request is pending")
        .unwrap();
        assert!(matches!(
            drive,
            ConnectionDrive::Packet(0x2a, payload) if payload == b"packet while fetching"
        ));
        runtime.reset_play_session();
        profile_server.abort();
    }
}
