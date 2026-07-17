use std::time::Duration;

use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::StatusCode;
use rsa::{pkcs8::DecodePublicKey, traits::PublicKeyParts, RsaPublicKey};
use serde::Deserialize;

#[cfg(not(test))]
const SERVICES_PUBLIC_KEYS_URL: &str = "https://api.minecraftservices.com/publickeys";
#[cfg(not(test))]
const SERVICES_PUBLIC_KEYS_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_SERVICES_KEY_RESPONSE_BYTES: u64 = 1024 * 1024;
const SERVICES_RSA_KEY_BITS: usize = 4096;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServicesKeySetResponse {
    player_certificate_keys: Option<Vec<ServicesKeyData>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServicesKeyData {
    public_key: String,
}

#[cfg(not(test))]
pub(crate) async fn fetch_services_profile_keys(timeout: Duration) -> Result<Vec<Vec<u8>>> {
    fetch_services_profile_keys_at(
        timeout.min(SERVICES_PUBLIC_KEYS_TIMEOUT),
        SERVICES_PUBLIC_KEYS_URL,
    )
    .await
}

async fn fetch_services_profile_keys_at(timeout: Duration, endpoint: &str) -> Result<Vec<Vec<u8>>> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .context("build Services public-key client")?;
    let mut response = client
        .get(endpoint)
        .send()
        .await
        .context("request Services public keys")?;
    ensure!(
        response.status() == StatusCode::OK,
        "Services public-key endpoint returned HTTP {}",
        response.status()
    );
    if let Some(content_length) = response.content_length() {
        ensure!(
            content_length <= MAX_SERVICES_KEY_RESPONSE_BYTES,
            "Services public-key response exceeds {MAX_SERVICES_KEY_RESPONSE_BYTES} bytes"
        );
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .context("read Services public-key response")?
    {
        ensure!(
            body.len().saturating_add(chunk.len()) as u64 <= MAX_SERVICES_KEY_RESPONSE_BYTES,
            "Services public-key response exceeds {MAX_SERVICES_KEY_RESPONSE_BYTES} bytes"
        );
        body.extend_from_slice(&chunk);
    }
    decode_services_profile_keys(&body)
}

fn decode_services_profile_keys(body: &[u8]) -> Result<Vec<Vec<u8>>> {
    let response: ServicesKeySetResponse =
        serde_json::from_slice(body).context("decode Services public-key response")?;
    let keys = response.player_certificate_keys.unwrap_or_default();

    keys.into_iter()
        .map(|key| {
            let der = STANDARD
                .decode(key.public_key)
                .context("decode Services profile public key from base64")?;
            let public_key = RsaPublicKey::from_public_key_der(&der)
                .context("decode Services profile public key from X.509 DER")?;
            ensure!(
                public_key.n().bits() == SERVICES_RSA_KEY_BITS,
                "Services profile public key is not RSA-{SERVICES_RSA_KEY_BITS}"
            );
            Ok(der)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::{pkcs8::EncodePublicKey, BigUint};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    fn rsa_4096_public_key_der() -> Vec<u8> {
        let modulus = (BigUint::from(1u8) << 4095) + BigUint::from(1u8);
        RsaPublicKey::new(modulus, BigUint::from(65_537u32))
            .unwrap()
            .to_public_key_der()
            .unwrap()
            .as_bytes()
            .to_vec()
    }

    #[test]
    fn decodes_player_certificate_keys_and_rejects_wrong_rsa_size() {
        let der = rsa_4096_public_key_der();
        let body = serde_json::json!({
            "profilePropertyKeys": [],
            "playerCertificateKeys": [{"publicKey": STANDARD.encode(&der)}],
        });
        assert_eq!(
            decode_services_profile_keys(body.to_string().as_bytes()).unwrap(),
            vec![der]
        );

        let short_key = RsaPublicKey::new(
            (BigUint::from(1u8) << 2047) + BigUint::from(1u8),
            BigUint::from(65_537u32),
        )
        .unwrap()
        .to_public_key_der()
        .unwrap();
        let body = serde_json::json!({
            "playerCertificateKeys": [{"publicKey": STANDARD.encode(short_key.as_bytes())}],
        });
        assert!(decode_services_profile_keys(body.to_string().as_bytes())
            .unwrap_err()
            .to_string()
            .contains("not RSA-4096"));
    }

    #[tokio::test]
    async fn fetches_services_profile_keys_from_http_endpoint() {
        let der = rsa_4096_public_key_der();
        let body = serde_json::json!({
            "playerCertificateKeys": [{"publicKey": STANDARD.encode(&der)}],
        })
        .to_string();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0u8; 4096];
            let read = socket.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..read]).starts_with("GET /publickeys "));
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });

        let endpoint = format!("http://{address}/publickeys");
        assert_eq!(
            fetch_services_profile_keys_at(Duration::from_secs(2), &endpoint)
                .await
                .unwrap(),
            vec![der]
        );
        server.await.unwrap();
    }
}
