use std::time::Duration;

use anyhow::{ensure, Context, Result};
use bbb_protocol::packets::{self, EncryptionRequest, LoginKey};
use num_bigint::BigInt;
use rand::{rngs::OsRng, RngCore};
use reqwest::{header::CONTENT_TYPE, StatusCode};
use rsa::{pkcs8::DecodePublicKey, Pkcs1v15Encrypt, RsaPublicKey};
use serde::Serialize;
use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::{connection::RawConnection, types::SessionCredentials};

const SESSION_JOIN_URL: &str = "https://sessionserver.mojang.com/session/minecraft/join";

struct PreparedEncryption {
    login_key: LoginKey,
    secret: [u8; 16],
    server_hash: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JoinServerRequest<'a> {
    access_token: &'a str,
    selected_profile: String,
    server_id: &'a str,
}

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

pub(crate) async fn handle_encryption_request(
    conn: &mut RawConnection,
    request: &EncryptionRequest,
    credentials: Option<&SessionCredentials>,
    timeout: Duration,
) -> Result<()> {
    ensure!(
        conn.login_hello_sent(),
        "received login encryption request before sending login hello"
    );
    ensure!(
        !conn.encryption_enabled(),
        "received duplicate login encryption request"
    );

    let prepared = prepare_encryption(request)?;
    if request.should_authenticate {
        let credentials = credentials.context(
            "server requires session authentication but no access token was supplied at startup",
        )?;
        authenticate_server(credentials, &prepared.server_hash, timeout).await?;
    }

    let (packet_id, payload) = packets::encode_login_key(&prepared.login_key)?;
    conn.send_packet(packet_id, &payload)
        .await
        .context("send plaintext login key")?;
    conn.enable_encryption(prepared.secret)
}

fn prepare_encryption(request: &EncryptionRequest) -> Result<PreparedEncryption> {
    let public_key = RsaPublicKey::from_public_key_der(&request.public_key)
        .context("decode login RSA public key from X.509 DER")?;
    let mut secret = [0u8; 16];
    OsRng.fill_bytes(&mut secret);

    let server_hash = minecraft_server_hash(&request.server_id, &secret, &request.public_key);
    let mut rng = OsRng;
    let encrypted_secret = public_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, &secret)
        .context("encrypt login shared secret")?;
    let encrypted_challenge = public_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, &request.challenge)
        .context("encrypt login challenge")?;

    Ok(PreparedEncryption {
        login_key: LoginKey {
            encrypted_secret,
            encrypted_challenge,
        },
        secret,
        server_hash,
    })
}

fn minecraft_server_hash(server_id: &str, secret: &[u8; 16], public_key_der: &[u8]) -> String {
    let mut digest = Sha1::new();
    digest.update(iso_8859_1_bytes(server_id));
    digest.update(secret);
    digest.update(public_key_der);
    BigInt::from_signed_bytes_be(&digest.finalize()).to_str_radix(16)
}

fn iso_8859_1_bytes(value: &str) -> Vec<u8> {
    value
        .chars()
        .map(|character| u8::try_from(character as u32).unwrap_or(b'?'))
        .collect()
}

async fn authenticate_server(
    credentials: &SessionCredentials,
    server_hash: &str,
    timeout: Duration,
) -> Result<()> {
    authenticate_server_at(credentials, server_hash, timeout, SESSION_JOIN_URL).await
}

async fn authenticate_server_at(
    credentials: &SessionCredentials,
    server_hash: &str,
    timeout: Duration,
    endpoint: &str,
) -> Result<()> {
    let request = JoinServerRequest {
        access_token: &credentials.access_token,
        selected_profile: credentials.profile_id.simple().to_string(),
        server_id: server_hash,
    };
    let body = serde_json::to_vec(&request).context("encode session join request")?;
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .context("build session authentication client")?;
    let response = client
        .post(endpoint)
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .context("send session join request")?;
    ensure!(
        response.status() == StatusCode::NO_CONTENT,
        "session server rejected join request with HTTP {}",
        response.status()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        codec::{Decoder, Encoder},
        ids,
    };
    use rsa::{pkcs8::EncodePublicKey, RsaPrivateKey};
    use serde_json::Value;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        task::JoinHandle,
        time::timeout,
    };

    #[test]
    fn encryption_preparation_matches_vanilla_rsa_inputs_and_hash_order() {
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 1024).unwrap();
        let public_key = RsaPublicKey::from(&private_key);
        let public_key_der = public_key.to_public_key_der().unwrap();
        let challenge = b"vanilla-challenge".to_vec();
        let prepared = prepare_encryption(&EncryptionRequest {
            server_id: String::new(),
            public_key: public_key_der.as_bytes().to_vec(),
            challenge: challenge.clone(),
            should_authenticate: true,
        })
        .unwrap();

        assert_eq!(prepared.secret.len(), 16);
        assert_eq!(
            private_key
                .decrypt(Pkcs1v15Encrypt, &prepared.login_key.encrypted_secret)
                .unwrap(),
            prepared.secret
        );
        assert_eq!(
            private_key
                .decrypt(Pkcs1v15Encrypt, &prepared.login_key.encrypted_challenge)
                .unwrap(),
            challenge
        );
        assert_eq!(
            prepared.server_hash,
            minecraft_server_hash("", &prepared.secret, public_key_der.as_bytes())
        );

        let ordered_secret = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        assert_eq!(
            minecraft_server_hash("é雪", &ordered_secret, &[0x30, 0x03, 0x01, 0x02, 0x03]),
            "40c43bddfce7cf729de3c329ecf7642ccf1aab49"
        );
        assert_eq!(BigInt::from_signed_bytes_be(&[0xff]).to_str_radix(16), "-1");
    }

    #[tokio::test]
    async fn session_join_posts_undashed_profile_and_requires_no_content() {
        let credentials = SessionCredentials {
            profile_id: Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678),
            access_token: "test-access-token".to_string(),
        };
        let (endpoint, request) = spawn_session_server("204 No Content").await;

        authenticate_server_at(&credentials, "-1234abcd", Duration::from_secs(1), &endpoint)
            .await
            .unwrap();

        let request = request.await.unwrap();
        assert!(request.starts_with("POST /session/minecraft/join HTTP/1.1\r\n"));
        assert!(request
            .to_ascii_lowercase()
            .contains("content-type: application/json"));
        let body = request.split_once("\r\n\r\n").unwrap().1;
        let body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["accessToken"], "test-access-token");
        assert_eq!(body["selectedProfile"], "123456781234567890abcdef12345678");
        assert_eq!(body["serverId"], "-1234abcd");

        let (endpoint, _request) = spawn_session_server("403 Forbidden").await;
        let error =
            authenticate_server_at(&credentials, "digest", Duration::from_secs(1), &endpoint)
                .await
                .unwrap_err();
        assert!(error.to_string().contains("HTTP 403 Forbidden"));
        assert!(!error.to_string().contains("test-access-token"));
    }

    #[tokio::test]
    async fn login_key_is_plaintext_then_transport_switches_to_encrypted_stream() {
        let (mut client, mut server) = raw_connection_pair().await;
        let profile_id = Uuid::from_u128(7);
        send_login_hello(&mut client, "bbb-client", profile_id)
            .await
            .unwrap();
        let (packet_id, _) = server.read_packet().await.unwrap();
        assert_eq!(packet_id, ids::login::SERVERBOUND_HELLO);

        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 1024).unwrap();
        let public_key_der = RsaPublicKey::from(&private_key)
            .to_public_key_der()
            .unwrap();
        let challenge = b"challenge".to_vec();
        let request = EncryptionRequest {
            server_id: String::new(),
            public_key: public_key_der.as_bytes().to_vec(),
            challenge: challenge.clone(),
            should_authenticate: false,
        };
        handle_encryption_request(&mut client, &request, None, Duration::from_secs(1))
            .await
            .unwrap();
        assert!(client.encryption_enabled());

        let (packet_id, payload) = server.read_packet().await.unwrap();
        assert_eq!(packet_id, ids::login::SERVERBOUND_KEY);
        let mut decoder = Decoder::new(&payload);
        let encrypted_secret_len = decoder.read_var_i32().unwrap() as usize;
        let encrypted_secret = decoder
            .read_exact(encrypted_secret_len, "encrypted secret")
            .unwrap();
        let encrypted_challenge_len = decoder.read_var_i32().unwrap() as usize;
        let encrypted_challenge = decoder
            .read_exact(encrypted_challenge_len, "encrypted challenge")
            .unwrap();
        assert!(decoder.is_empty());
        let secret = private_key
            .decrypt(Pkcs1v15Encrypt, encrypted_secret)
            .unwrap();
        assert_eq!(
            private_key
                .decrypt(Pkcs1v15Encrypt, encrypted_challenge)
                .unwrap(),
            challenge
        );
        let secret: [u8; 16] = secret.try_into().unwrap();
        server.enable_encryption(secret).unwrap();

        client.send_packet(0x2a, b"client encrypted").await.unwrap();
        assert_eq!(
            server.read_packet().await.unwrap(),
            (0x2a, b"client encrypted".to_vec())
        );
        let mut login_finished = Encoder::new();
        login_finished.write_uuid(profile_id);
        login_finished.write_string("bbb-client");
        login_finished.write_var_i32(0);
        server
            .send_packet(
                ids::login::CLIENTBOUND_LOGIN_FINISHED,
                &login_finished.into_inner(),
            )
            .await
            .unwrap();
        let (packet_id, payload) = client.read_packet().await.unwrap();
        assert!(matches!(
            packets::decode_login_clientbound(packet_id, &payload).unwrap(),
            packets::LoginClientbound::LoginFinished { profile }
                if profile.uuid == profile_id && profile.name == "bbb-client"
        ));

        let error = handle_encryption_request(&mut client, &request, None, Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("duplicate"));
    }

    #[tokio::test]
    async fn encryption_request_requires_a_sent_hello_and_required_credentials() {
        let (mut client, _server) = raw_connection_pair().await;
        let request = EncryptionRequest {
            server_id: String::new(),
            public_key: Vec::new(),
            challenge: Vec::new(),
            should_authenticate: true,
        };
        let error = handle_encryption_request(&mut client, &request, None, Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("before sending login hello"));

        send_login_hello(&mut client, "bbb-client", Uuid::from_u128(1))
            .await
            .unwrap();
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 1024).unwrap();
        let public_key_der = RsaPublicKey::from(&private_key)
            .to_public_key_der()
            .unwrap();
        let request = EncryptionRequest {
            public_key: public_key_der.as_bytes().to_vec(),
            ..request
        };
        let error = handle_encryption_request(&mut client, &request, None, Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("no access token"));
        assert!(!client.encryption_enabled());
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

    async fn spawn_session_server(status: &str) -> (String, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "http://{}/session/minecraft/join",
            listener.local_addr().unwrap()
        );
        let status = status.to_string();
        let request = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut stream).await;
            let response = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\n\r\n");
            stream.write_all(response.as_bytes()).await.unwrap();
            request
        });
        (endpoint, request)
    }

    async fn read_http_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        loop {
            let mut chunk = [0u8; 1024];
            let read = timeout(Duration::from_secs(1), stream.read(&mut chunk))
                .await
                .unwrap()
                .unwrap();
            assert!(read > 0);
            request.extend_from_slice(&chunk[..read]);
            let Some(headers_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let body_start = headers_end + 4;
            let headers = String::from_utf8_lossy(&request[..headers_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            if request.len() >= body_start + content_length {
                request.truncate(body_start + content_length);
                return String::from_utf8(request).unwrap();
            }
        }
    }
}
