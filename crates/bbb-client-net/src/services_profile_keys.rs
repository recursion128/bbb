#[cfg(not(test))]
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bbb_protocol::packets::GameProfilePropertySummary;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{types::NetEvent, NetEventSender};

#[cfg(not(test))]
const SERVICES_PUBLIC_KEYS_URL: &str = "https://api.minecraftservices.com/publickeys";
const SERVICES_PUBLIC_KEYS_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_SERVICES_KEY_RESPONSE_BYTES: u64 = 1024 * 1024;
const MAX_SERVICES_KEYS_PER_FAMILY: usize = 16;
const MAX_SERVICES_KEY_BYTES: usize = 1024;
const SERVICES_RSA_KEY_BITS: usize = 4096;
const MAX_PROFILE_TEXTURE_SIGNATURE_BYTES: usize = 1024;

pub fn verify_services_rsa_sha1_signature(
    services_public_keys: &[Vec<u8>],
    payload: &[u8],
    signature: &[u8],
) -> bool {
    bbb_protocol::verify_services_rsa_sha1_signature(services_public_keys, payload, signature)
}

pub fn verify_profile_texture_signature(
    properties: &[GameProfilePropertySummary],
    profile_property_keys: &[Vec<u8>],
) -> bool {
    if profile_property_keys.is_empty() {
        return false;
    }
    let Some(property) = properties
        .iter()
        .find(|property| property.name == "textures")
    else {
        return false;
    };
    let Some(encoded_signature) = property.signature.as_ref() else {
        return false;
    };
    let Ok(encoded_signature) = String::from_utf16(encoded_signature.units()) else {
        return false;
    };
    let Ok(signature) = STANDARD.decode(encoded_signature) else {
        return false;
    };
    if signature.len() > MAX_PROFILE_TEXTURE_SIGNATURE_BYTES {
        return false;
    }
    verify_services_rsa_sha1_signature(
        profile_property_keys,
        &property.value.to_jdk_utf8(),
        &signature,
    )
}

#[cfg(not(test))]
static CACHED_SERVICES_PUBLIC_KEYS: OnceLock<ServicesPublicKeys> = OnceLock::new();

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ServicesPublicKeys {
    pub(crate) profile_property_keys: Vec<Vec<u8>>,
    pub(crate) player_certificate_keys: Vec<Vec<u8>>,
}

pub(crate) struct ServicesKeysFetchGuard {
    #[cfg(not(target_arch = "wasm32"))]
    handle: Option<tokio::task::JoinHandle<()>>,
    #[cfg(target_arch = "wasm32")]
    abort: Option<crate::wasm_fetch::WasmAbortHandle>,
}

impl Drop for ServicesKeysFetchGuard {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(abort) = self.abort.take() {
            abort.abort();
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServicesKeySetResponse {
    profile_property_keys: Option<Vec<ServicesKeyData>>,
    player_certificate_keys: Option<Vec<ServicesKeyData>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServicesKeyData {
    public_key: String,
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) async fn spawn_default_services_public_keys_fetch(
    timeout: Duration,
    events: &NetEventSender,
) -> Result<Option<ServicesKeysFetchGuard>> {
    if bbb_client_clock::is_virtual() {
        return Ok(None);
    }
    if let Some(cached) = CACHED_SERVICES_PUBLIC_KEYS.get().cloned() {
        events
            .send(NetEvent::ServicesPublicKeys {
                profile_property_keys: cached.profile_property_keys,
                player_certificate_keys: cached.player_certificate_keys,
            })
            .await?;
        return Ok(None);
    }
    Ok(Some(spawn_services_public_keys_fetch_at(
        timeout,
        SERVICES_PUBLIC_KEYS_URL.to_string(),
        events.clone(),
        true,
    )))
}

#[cfg(all(not(test), target_arch = "wasm32"))]
pub(crate) async fn spawn_default_services_public_keys_fetch(
    timeout: Duration,
    events: &NetEventSender,
) -> Result<Option<ServicesKeysFetchGuard>> {
    if bbb_client_clock::is_virtual() {
        return Ok(None);
    }
    if let Some(cached) = CACHED_SERVICES_PUBLIC_KEYS.get().cloned() {
        events
            .send(NetEvent::ServicesPublicKeys {
                profile_property_keys: cached.profile_property_keys,
                player_certificate_keys: cached.player_certificate_keys,
            })
            .await?;
        return Ok(None);
    }
    let request_timeout = timeout.min(SERVICES_PUBLIC_KEYS_TIMEOUT);
    let abort = crate::wasm_fetch::WasmAbortHandle::new();
    let guard_abort = abort.clone();
    let events = events.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let encoded: String = js_sys::encode_uri_component(SERVICES_PUBLIC_KEYS_URL).into();
        let proxy_url = format!("/proxy/skin?url={encoded}");
        let result = async {
            let response = crate::wasm_fetch::fetch_bounded_bytes(
                &proxy_url,
                "Services public keys",
                request_timeout,
                MAX_SERVICES_KEY_RESPONSE_BYTES,
                abort,
                |_| {},
            )
            .await?;
            ensure!(
                response.status == 200,
                "Services public-key endpoint returned HTTP {}",
                response.status
            );
            decode_services_public_keys(&response.body)
        }
        .await;
        match result {
            Ok(keys) => {
                let _ = CACHED_SERVICES_PUBLIC_KEYS.set(keys.clone());
                let _ = events
                    .send(NetEvent::ServicesPublicKeys {
                        profile_property_keys: keys.profile_property_keys,
                        player_certificate_keys: keys.player_certificate_keys,
                    })
                    .await;
            }
            Err(error) => {
                tracing::warn!(%error, "Services public keys are unavailable");
            }
        }
    });
    Ok(Some(ServicesKeysFetchGuard { abort: guard_abort }))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn_services_public_keys_fetch_at(
    timeout: Duration,
    endpoint: String,
    events: NetEventSender,
    cache_in_process: bool,
) -> ServicesKeysFetchGuard {
    let request_timeout = timeout.min(SERVICES_PUBLIC_KEYS_TIMEOUT);
    let handle = tokio::spawn(async move {
        match fetch_services_public_keys_at(request_timeout, &endpoint).await {
            Ok(keys) => {
                #[cfg(not(test))]
                if cache_in_process {
                    let _ = CACHED_SERVICES_PUBLIC_KEYS.set(keys.clone());
                }
                #[cfg(test)]
                let _ = cache_in_process;
                let _ = events
                    .send(NetEvent::ServicesPublicKeys {
                        profile_property_keys: keys.profile_property_keys,
                        player_certificate_keys: keys.player_certificate_keys,
                    })
                    .await;
            }
            Err(error) => {
                tracing::warn!(%error, "Services public keys are unavailable");
            }
        }
    });
    ServicesKeysFetchGuard {
        handle: Some(handle),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_services_public_keys_at(
    timeout: Duration,
    endpoint: &str,
) -> Result<ServicesPublicKeys> {
    crate::install_http_crypto_provider();
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
    decode_services_public_keys(&body)
}

fn decode_services_public_keys(body: &[u8]) -> Result<ServicesPublicKeys> {
    let response: ServicesKeySetResponse =
        serde_json::from_slice(body).context("decode Services public-key response")?;
    Ok(ServicesPublicKeys {
        profile_property_keys: decode_services_key_family(
            response.profile_property_keys.unwrap_or_default(),
            "profile-property",
        )?,
        player_certificate_keys: decode_services_key_family(
            response.player_certificate_keys.unwrap_or_default(),
            "player-certificate",
        )?,
    })
}

fn decode_services_key_family(keys: Vec<ServicesKeyData>, label: &str) -> Result<Vec<Vec<u8>>> {
    ensure!(
        keys.len() <= MAX_SERVICES_KEYS_PER_FAMILY,
        "Services {label} key count exceeds {MAX_SERVICES_KEYS_PER_FAMILY}"
    );
    keys.into_iter()
        .map(|key| {
            let der = STANDARD
                .decode(key.public_key)
                .with_context(|| format!("decode Services {label} public key from base64"))?;
            ensure!(
                der.len() <= MAX_SERVICES_KEY_BYTES,
                "Services {label} public key exceeds {MAX_SERVICES_KEY_BYTES} bytes"
            );
            let public_key = bbb_protocol::parse_cached_services_rsa_public_key(&der)
                .with_context(|| format!("decode Services {label} public key from X.509 DER"))?;
            ensure!(
                public_key.modulus_bits() == SERVICES_RSA_KEY_BITS,
                "Services {label} public key is not RSA-{SERVICES_RSA_KEY_BITS}"
            );
            Ok(der)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::{pkcs8::EncodePublicKey, BoxedUint, Pkcs1v15Sign, PrivateKey, PublicKey};
    use sha1::{Digest, Sha1};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    struct TestRsaRng(u64);

    impl TestRsaRng {
        fn seeded(seed: u64) -> Self {
            Self(seed)
        }

        fn step_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }
    }

    impl rsa::rand_core::TryRng for TestRsaRng {
        type Error = std::convert::Infallible;

        fn try_next_u32(&mut self) -> std::result::Result<u32, Self::Error> {
            Ok((self.step_u64() >> 32) as u32)
        }

        fn try_next_u64(&mut self) -> std::result::Result<u64, Self::Error> {
            Ok(self.step_u64())
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> std::result::Result<(), Self::Error> {
            for chunk in dest.chunks_mut(8) {
                let bytes = self.step_u64().to_le_bytes();
                chunk.copy_from_slice(&bytes[..chunk.len()]);
            }
            Ok(())
        }
    }

    impl rsa::rand_core::TryCryptoRng for TestRsaRng {}

    #[test]
    fn profile_texture_property_trust_requires_a_valid_services_signature() {
        let mut random = TestRsaRng::seeded(0x5a17_cafe);
        let private_key = PrivateKey::new(&mut random, 1024).unwrap();
        let public_key = private_key.to_public_key();
        let public_der = public_key.to_public_key_der().unwrap();
        let value = "packed-profile-textures";
        let signature = private_key
            .sign(Pkcs1v15Sign::new::<Sha1>(), &Sha1::digest(value.as_bytes()))
            .unwrap();
        let signed = GameProfilePropertySummary {
            name: "textures".into(),
            value: value.to_string().into(),
            signature: Some(STANDARD.encode(signature).into()),
        };

        assert!(verify_profile_texture_signature(
            std::slice::from_ref(&signed),
            &[public_der.as_bytes().to_vec()],
        ));
        assert!(!verify_profile_texture_signature(
            std::slice::from_ref(&signed),
            &[]
        ));

        let mut unsigned = signed.clone();
        unsigned.signature = None;
        assert!(!verify_profile_texture_signature(&[unsigned], &[]));
        let mut invalid = signed;
        invalid.signature = Some("not base64".into());
        assert!(!verify_profile_texture_signature(
            &[invalid],
            &[public_der.as_bytes().to_vec()]
        ));
    }

    fn synthetic_rsa_modulus(bits: usize) -> BoxedUint {
        let mut bytes = vec![0u8; bits.div_ceil(8)];
        bytes[0] = 1u8 << ((bits - 1) % 8);
        *bytes.last_mut().unwrap() |= 1;
        BoxedUint::from_be_slice_vartime(&bytes)
    }

    fn rsa_4096_public_key_der() -> Vec<u8> {
        PublicKey::new(synthetic_rsa_modulus(4096), BoxedUint::from(65_537u32))
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
            decode_services_public_keys(body.to_string().as_bytes()).unwrap(),
            ServicesPublicKeys {
                profile_property_keys: Vec::new(),
                player_certificate_keys: vec![der],
            }
        );

        let short_key = PublicKey::new(synthetic_rsa_modulus(2048), BoxedUint::from(65_537u32))
            .unwrap()
            .to_public_key_der()
            .unwrap();
        let body = serde_json::json!({
            "playerCertificateKeys": [{"publicKey": STANDARD.encode(short_key.as_bytes())}],
        });
        assert!(decode_services_public_keys(body.to_string().as_bytes())
            .unwrap_err()
            .to_string()
            .contains("not RSA-4096"));
    }

    #[test]
    fn decodes_both_key_families_and_enforces_family_budget() {
        let der = rsa_4096_public_key_der();
        let encoded = STANDARD.encode(&der);
        let body = serde_json::json!({
            "profilePropertyKeys": [{"publicKey": encoded}],
            "playerCertificateKeys": [{"publicKey": STANDARD.encode(&der)}],
        });
        assert_eq!(
            decode_services_public_keys(body.to_string().as_bytes()).unwrap(),
            ServicesPublicKeys {
                profile_property_keys: vec![der.clone()],
                player_certificate_keys: vec![der],
            }
        );

        let body = serde_json::json!({
            "profilePropertyKeys": (0..=MAX_SERVICES_KEYS_PER_FAMILY)
                .map(|_| serde_json::json!({"publicKey": encoded}))
                .collect::<Vec<_>>(),
        });
        assert!(decode_services_public_keys(body.to_string().as_bytes())
            .unwrap_err()
            .to_string()
            .contains("key count exceeds"));
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
            // Read to the end of the headers rather than once: a request that
            // arrives split across two segments is a busy machine, not a
            // client that asked for something else.
            let mut request = Vec::new();
            let mut chunk = [0u8; 4096];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = socket.read(&mut chunk).await.unwrap();
                assert!(read > 0, "the request ended before its headers did");
                request.extend_from_slice(&chunk[..read]);
            }
            assert!(String::from_utf8_lossy(&request).starts_with("GET /publickeys "));
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
            fetch_services_public_keys_at(Duration::from_secs(2), &endpoint)
                .await
                .unwrap(),
            ServicesPublicKeys {
                profile_property_keys: Vec::new(),
                player_certificate_keys: vec![der],
            }
        );
        server.await.unwrap();
    }
}
