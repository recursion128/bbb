//! Authenticated profile certificates and the local signed-message chain.

use std::{fmt, time::Duration};

use anyhow::{anyhow, ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bbb_protocol::packets::{
    ArgumentSignature, ArgumentSignatures, ChatCommand, ChatCommandSigned, ChatMessage,
    ChatSessionUpdate, MessageSignature,
};
use chrono::DateTime;
use reqwest::StatusCode;
use rsa::{
    pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePublicKey},
    traits::PublicKeyParts,
    Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::types::{OutboundChatCommand, OutboundChatCommandSigning, OutboundChatMessage};

mod runtime;

pub(crate) use runtime::{ChatSessionAction, ChatSessionRuntime};

const PROFILE_CERTIFICATES_URL: &str = "https://api.minecraftservices.com/player/certificates";
const PROFILE_CERTIFICATES_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PROFILE_CERTIFICATES_RESPONSE_BYTES: u64 = 128 * 1024;
const PROFILE_KEY_BITS: usize = 2048;
const MESSAGE_SIGNATURE_BYTES: usize = 256;
const MAX_LAST_SEEN_SIGNATURES: usize = 20;
const MAX_PROFILE_PUBLIC_KEY_BYTES: usize = 512;
const MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES: usize = 4096;

/// A Services-issued profile key pair. Private material deliberately has no
/// accessor, serde implementation, or derived `Debug`.
#[derive(Clone)]
pub(crate) struct ProfileKeyPair {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
    public_key_der: Vec<u8>,
    key_signature: Vec<u8>,
    expires_at_epoch_millis: i64,
    refreshed_after_epoch_millis: i64,
}

impl fmt::Debug for ProfileKeyPair {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileKeyPair")
            .field("public_key_bits", &self.public_key.n().bits())
            .field("public_key_der_len", &self.public_key_der.len())
            .field("key_signature_len", &self.key_signature.len())
            .field("expires_at_epoch_millis", &self.expires_at_epoch_millis)
            .field(
                "refreshed_after_epoch_millis",
                &self.refreshed_after_epoch_millis,
            )
            .finish_non_exhaustive()
    }
}

impl ProfileKeyPair {
    fn refreshed_after_epoch_millis(&self) -> i64 {
        self.refreshed_after_epoch_millis
    }

    fn due_refresh_at(&self, now_epoch_millis: i64) -> bool {
        self.refreshed_after_epoch_millis < now_epoch_millis
    }

    fn same_key_material(&self, other: &Self) -> bool {
        self.public_key == other.public_key
            && self.public_key_der == other.public_key_der
            && self.key_signature == other.key_signature
            && self.private_key == other.private_key
            && self.expires_at_epoch_millis == other.expires_at_epoch_millis
            && self.refreshed_after_epoch_millis == other.refreshed_after_epoch_millis
    }

    fn chat_session_update(&self, session_id: Uuid) -> ChatSessionUpdate {
        ChatSessionUpdate {
            session_id,
            expires_at_epoch_millis: self.expires_at_epoch_millis,
            public_key: self.public_key_der.clone(),
            key_signature: self.key_signature.clone(),
        }
    }
}

/// One local chat session and its single advancing signed-message chain.
pub(crate) struct LocalChatSession {
    profile_id: Uuid,
    session_id: Uuid,
    key_pair: ProfileKeyPair,
    next_link_index: Option<i32>,
}

impl fmt::Debug for LocalChatSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalChatSession")
            .field("profile_id", &self.profile_id)
            .field("session_id", &self.session_id)
            .field("next_link_index", &self.next_link_index)
            .field(
                "expires_at_epoch_millis",
                &self.key_pair.expires_at_epoch_millis,
            )
            .finish_non_exhaustive()
    }
}

impl LocalChatSession {
    fn create(profile_id: Uuid, key_pair: ProfileKeyPair) -> Self {
        Self::with_session_id(profile_id, random_session_id(), key_pair)
    }

    fn with_session_id(profile_id: Uuid, session_id: Uuid, key_pair: ProfileKeyPair) -> Self {
        Self {
            profile_id,
            session_id,
            key_pair,
            next_link_index: Some(0),
        }
    }

    #[cfg(test)]
    fn profile_id(&self) -> Uuid {
        self.profile_id
    }

    #[cfg(test)]
    fn session_id(&self) -> Uuid {
        self.session_id
    }

    fn key_pair(&self) -> &ProfileKeyPair {
        &self.key_pair
    }

    pub(crate) fn session_update(&self) -> ChatSessionUpdate {
        self.key_pair.chat_session_update(self.session_id)
    }

    fn sign(&mut self, body: SignedChatBody<'_>) -> Result<MessageSignature> {
        let index = self
            .next_link_index
            .ok_or_else(|| anyhow!("signed chat chain is exhausted"))?;
        let payload = body.signature_payload(self.profile_id, self.session_id, index)?;
        let signature = self.key_pair.private_key.sign(
            Pkcs1v15Sign::new::<Sha256>(),
            Sha256::digest(&payload).as_slice(),
        )?;
        ensure!(
            signature.len() == MESSAGE_SIGNATURE_BYTES,
            "profile key produced a non-256-byte chat signature"
        );
        self.next_link_index = index.checked_add(1);
        Ok(MessageSignature { bytes: signature })
    }

    #[cfg(test)]
    fn verify(
        &self,
        index: i32,
        body: SignedChatBody<'_>,
        signature: &MessageSignature,
    ) -> Result<bool> {
        let payload = body.signature_payload(self.profile_id, self.session_id, index)?;
        Ok(self
            .key_pair
            .public_key
            .verify(
                Pkcs1v15Sign::new::<Sha256>(),
                Sha256::digest(&payload).as_slice(),
                &signature.bytes,
            )
            .is_ok())
    }
}

fn random_session_id() -> Uuid {
    let mut bytes = rand::random::<[u8; 16]>();
    // `UUID.randomUUID()` sets RFC 4122 variant 1 and version 4 after filling
    // the 128 random bits (OpenJDK UUID.java).
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[derive(Clone, Copy, Debug)]
struct SignedChatBody<'a> {
    content: &'a str,
    timestamp_millis: i64,
    salt: i64,
    last_seen: &'a [MessageSignature],
}

impl<'a> SignedChatBody<'a> {
    fn new(
        content: &'a str,
        timestamp_millis: i64,
        salt: i64,
        last_seen: &'a [MessageSignature],
    ) -> Self {
        Self {
            content,
            timestamp_millis,
            salt,
            last_seen,
        }
    }

    fn signature_payload(
        self,
        profile_id: Uuid,
        session_id: Uuid,
        link_index: i32,
    ) -> Result<Vec<u8>> {
        ensure!(
            link_index >= 0,
            "signed chat link index must be non-negative"
        );
        ensure!(
            self.last_seen.len() <= MAX_LAST_SEEN_SIGNATURES,
            "signed chat body exceeds 20 last-seen messages"
        );
        let content = self.content.as_bytes();
        let content_len =
            i32::try_from(content.len()).context("signed chat content byte length exceeds i32")?;
        let last_seen_len = i32::try_from(self.last_seen.len())
            .context("signed chat last-seen count exceeds i32")?;
        let capacity = 16_usize
            .saturating_add(16)
            .saturating_add(4)
            .saturating_add(8)
            .saturating_add(8)
            .saturating_add(4)
            .saturating_add(content.len())
            .saturating_add(4)
            .saturating_add(self.last_seen.len().saturating_mul(MESSAGE_SIGNATURE_BYTES));
        let mut output = Vec::with_capacity(capacity);
        output.extend_from_slice(profile_id.as_bytes());
        output.extend_from_slice(session_id.as_bytes());
        output.extend_from_slice(&link_index.to_be_bytes());
        output.extend_from_slice(&self.salt.to_be_bytes());
        output.extend_from_slice(&self.timestamp_millis.div_euclid(1000).to_be_bytes());
        output.extend_from_slice(&content_len.to_be_bytes());
        output.extend_from_slice(content);
        output.extend_from_slice(&last_seen_len.to_be_bytes());
        for signature in self.last_seen {
            ensure!(
                signature.bytes.len() == MESSAGE_SIGNATURE_BYTES,
                "last-seen message signature must be exactly 256 bytes"
            );
            output.extend_from_slice(&signature.bytes);
        }
        Ok(output)
    }
}

pub(crate) enum PreparedChatCommand {
    Unsigned(ChatCommand),
    Signed(ChatCommandSigned),
}

pub(crate) fn prepare_outbound_chat_command(
    request: OutboundChatCommand,
    session: Option<&mut LocalChatSession>,
) -> Result<PreparedChatCommand> {
    let Some(OutboundChatCommandSigning {
        timestamp_millis,
        salt,
        arguments,
        last_seen,
    }) = request.signing
    else {
        return Ok(PreparedChatCommand::Unsigned(ChatCommand {
            command: request.command,
        }));
    };
    ensure!(
        !arguments.is_empty(),
        "signed outbound command has no signable arguments"
    );

    let mut entries = Vec::with_capacity(arguments.len());
    if let Some(session) = session {
        for argument in arguments {
            let signature = session.sign(SignedChatBody::new(
                &argument.value,
                timestamp_millis,
                salt,
                &last_seen.signatures,
            ))?;
            entries.push(ArgumentSignature {
                name: argument.name,
                signature,
            });
        }
    }
    Ok(PreparedChatCommand::Signed(ChatCommandSigned {
        command: request.command,
        timestamp_millis,
        salt,
        argument_signatures: ArgumentSignatures { entries },
        last_seen_messages: last_seen.update,
    }))
}

pub(crate) fn prepare_outbound_chat_message(
    request: OutboundChatMessage,
    session: Option<&mut LocalChatSession>,
) -> Result<ChatMessage> {
    let signature = session
        .map(|session| {
            session.sign(SignedChatBody::new(
                &request.message,
                request.timestamp_millis,
                request.salt,
                &request.last_seen.signatures,
            ))
        })
        .transpose()?;
    Ok(ChatMessage {
        message: request.message,
        timestamp_millis: request.timestamp_millis,
        salt: request.salt,
        signature,
        last_seen_messages: request.last_seen.update,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileKeyPairResponse {
    key_pair: Option<ProfileKeyMaterialResponse>,
    #[serde(rename = "publicKeySignatureV2")]
    public_key_signature_v2: Option<String>,
    expires_at: String,
    refreshed_after: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileKeyMaterialResponse {
    private_key: String,
    public_key: String,
}

async fn fetch_profile_key_pair_at(
    access_token: &str,
    timeout: Duration,
    endpoint: &str,
) -> Result<ProfileKeyPair> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .context("build profile-certificate client")?;
    let mut response = client
        .post(endpoint)
        .bearer_auth(access_token)
        .send()
        .await
        .context("request profile certificate")?;
    ensure!(
        response.status() == StatusCode::OK,
        "profile-certificate endpoint returned HTTP {}",
        response.status()
    );
    if let Some(content_length) = response.content_length() {
        ensure!(
            content_length <= MAX_PROFILE_CERTIFICATES_RESPONSE_BYTES,
            "profile-certificate response exceeds {MAX_PROFILE_CERTIFICATES_RESPONSE_BYTES} bytes"
        );
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .context("read profile-certificate response")?
    {
        ensure!(
            body.len().saturating_add(chunk.len()) as u64
                <= MAX_PROFILE_CERTIFICATES_RESPONSE_BYTES,
            "profile-certificate response exceeds {MAX_PROFILE_CERTIFICATES_RESPONSE_BYTES} bytes"
        );
        body.extend_from_slice(&chunk);
    }
    decode_profile_key_pair(&body)
}

fn decode_profile_key_pair(body: &[u8]) -> Result<ProfileKeyPair> {
    let response: ProfileKeyPairResponse =
        serde_json::from_slice(body).context("decode profile-certificate response")?;
    let material = response
        .key_pair
        .context("profile-certificate response has no key pair")?;
    let signature = response
        .public_key_signature_v2
        .context("profile-certificate response has no public-key signature")?;
    let private_key_der = decode_pem_or_base64(
        &material.private_key,
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----END RSA PRIVATE KEY-----",
        "profile private key",
    )?;
    let public_key_der = decode_pem_or_base64(
        &material.public_key,
        "-----BEGIN RSA PUBLIC KEY-----",
        "-----END RSA PUBLIC KEY-----",
        "profile public key",
    )?;
    ensure!(
        public_key_der.len() <= MAX_PROFILE_PUBLIC_KEY_BYTES,
        "profile public key exceeds {MAX_PROFILE_PUBLIC_KEY_BYTES} bytes"
    );
    let private_key = RsaPrivateKey::from_pkcs8_der(&private_key_der)
        .context("decode PKCS#8 profile private key")?;
    let public_key = RsaPublicKey::from_public_key_der(&public_key_der)
        .context("decode X.509 profile public key")?;
    ensure!(
        public_key.n().bits() == PROFILE_KEY_BITS,
        "profile public key is not RSA-{PROFILE_KEY_BITS}"
    );
    ensure!(
        RsaPublicKey::from(&private_key) == public_key,
        "profile public key does not match private key"
    );
    let canonical_public_key_der = public_key
        .to_public_key_der()
        .context("encode canonical profile public key")?
        .as_bytes()
        .to_vec();
    ensure!(
        canonical_public_key_der == public_key_der,
        "profile public key is not canonical X.509 DER"
    );
    let key_signature = STANDARD
        .decode(signature)
        .context("decode profile public-key signature")?;
    ensure!(
        !key_signature.is_empty(),
        "profile public-key signature is empty"
    );
    ensure!(
        key_signature.len() <= MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES,
        "profile public-key signature exceeds {MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES} bytes"
    );
    let expires_at_epoch_millis = parse_epoch_millis(&response.expires_at, "expiresAt")?;
    let refreshed_after_epoch_millis =
        parse_epoch_millis(&response.refreshed_after, "refreshedAfter")?;

    Ok(ProfileKeyPair {
        private_key,
        public_key,
        public_key_der: canonical_public_key_der,
        key_signature,
        expires_at_epoch_millis,
        refreshed_after_epoch_millis,
    })
}

fn decode_pem_or_base64(input: &str, header: &str, footer: &str, what: &str) -> Result<Vec<u8>> {
    let encoded = if let Some(start) = input.find(header) {
        let start = start + header.len();
        let end = input[start..]
            .find(footer)
            .map(|offset| start + offset)
            .with_context(|| format!("{what} PEM footer is missing"))?;
        &input[start..end]
    } else {
        input
    };
    let compact = encoded
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    STANDARD
        .decode(compact)
        .with_context(|| format!("decode {what} base64"))
}

fn parse_epoch_millis(value: &str, field: &str) -> Result<i64> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("profile-certificate {field} is not RFC 3339"))
        .map(|value| value.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use bbb_protocol::packets::LastSeenMessagesUpdate;
    use bbb_world::{OutboundLastSeenMessages, SignableCommandArgument};
    use rand::{rngs::StdRng, SeedableRng};
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    #[test]
    fn profile_key_resolver_is_private_and_not_disk_backed() {
        let source = include_str!("chat_session.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let runtime = include_str!("chat_session/runtime.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let exports = include_str!("exports.rs");
        assert_eq!(
            source
                .matches("async fn fetch_profile_key_pair_at(")
                .count(),
            1
        );
        assert!(!source.contains("pub async fn fetch_profile_key_pair"));
        assert!(!source.contains("profilekeys"));
        assert!(!source.contains("write_profile"));
        assert_eq!(runtime.matches("struct ProfileKeyManager {").count(), 1);
        assert_eq!(
            runtime
                .matches("fetch_profile_key_pair_at(&access_token")
                .count(),
            1
        );
        assert!(!runtime.contains("profilekeys"));
        assert!(!exports.contains("fetch_profile_key_pair"));
        assert!(!exports.contains("LocalChatSession"));
        assert!(!exports.contains("ProfileKeyPair"));
        assert!(!exports.contains("SignedChatBody"));
    }

    fn fixture_private_key() -> RsaPrivateKey {
        static KEY: OnceLock<RsaPrivateKey> = OnceLock::new();
        KEY.get_or_init(|| {
            let mut random = StdRng::seed_from_u64(261);
            RsaPrivateKey::new(&mut random, PROFILE_KEY_BITS).unwrap()
        })
        .clone()
    }

    fn pem(label: &str, der: &[u8]) -> String {
        format!(
            "-----BEGIN {label}-----\n{}\n-----END {label}-----\n",
            STANDARD.encode(der)
        )
    }

    fn fixture_response() -> Vec<u8> {
        let private_key = fixture_private_key();
        let private_der = private_key.to_pkcs8_der().unwrap();
        let public_der = RsaPublicKey::from(&private_key)
            .to_public_key_der()
            .unwrap();
        serde_json::to_vec(&serde_json::json!({
            "keyPair": {
                "privateKey": pem("RSA PRIVATE KEY", private_der.as_bytes()),
                "publicKey": pem("RSA PUBLIC KEY", public_der.as_bytes()),
            },
            "publicKeySignatureV2": STANDARD.encode(vec![0x5a; 512]),
            "expiresAt": "2030-01-02T03:04:05.006Z",
            "refreshedAfter": "2030-01-01T03:04:05.006Z",
        }))
        .unwrap()
    }

    #[test]
    fn decodes_services_key_pair_and_redacts_private_material() {
        let pair = decode_profile_key_pair(&fixture_response()).unwrap();
        assert_eq!(pair.public_key.n().bits(), PROFILE_KEY_BITS);
        assert_eq!(pair.key_signature, vec![0x5a; 512]);
        assert_eq!(pair.expires_at_epoch_millis, 1_893_553_445_006);
        assert_eq!(pair.refreshed_after_epoch_millis, 1_893_467_045_006);
        assert!(!pair.due_refresh_at(pair.refreshed_after_epoch_millis));
        assert!(pair.due_refresh_at(pair.refreshed_after_epoch_millis + 1));
        let debug = format!("{pair:?}");
        assert!(!debug.contains("PRIVATE KEY"));
        assert!(
            !debug.contains(&STANDARD.encode(pair.private_key.to_pkcs8_der().unwrap().as_bytes()))
        );
    }

    #[test]
    fn generated_session_id_has_java_random_uuid_version_and_variant_bits() {
        for _ in 0..32 {
            let session_id = random_session_id();
            let bytes = session_id.as_bytes();
            assert_eq!(bytes[6] & 0xf0, 0x40);
            assert_eq!(bytes[8] & 0xc0, 0x80);
        }
    }

    #[test]
    fn signature_payload_matches_vanilla_field_order_and_epoch_seconds() {
        let profile_id = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
        let session_id = Uuid::from_u128(0xffeeddcc_bbaa_9988_7766_554433221100);
        let last_seen = MessageSignature {
            bytes: vec![0xab; MESSAGE_SIGNATURE_BYTES],
        };
        let payload = SignedChatBody::new("hi", 1_234_567, -7, &[last_seen])
            .signature_payload(profile_id, session_id, 3)
            .unwrap();

        let mut expected = Vec::new();
        expected.extend_from_slice(&profile_id.as_u128().to_be_bytes());
        expected.extend_from_slice(&session_id.as_u128().to_be_bytes());
        expected.extend_from_slice(&3_i32.to_be_bytes());
        expected.extend_from_slice(&(-7_i64).to_be_bytes());
        expected.extend_from_slice(&1_234_i64.to_be_bytes());
        expected.extend_from_slice(&2_i32.to_be_bytes());
        expected.extend_from_slice(b"hi");
        expected.extend_from_slice(&1_i32.to_be_bytes());
        expected.extend_from_slice(&[0xab; MESSAGE_SIGNATURE_BYTES]);
        assert_eq!(payload, expected);
    }

    #[test]
    fn local_chain_signs_each_link_once_and_exhausts_after_i32_max() {
        let pair = decode_profile_key_pair(&fixture_response()).unwrap();
        let profile_id = Uuid::from_u128(1);
        let session_id = Uuid::from_u128(2);
        let mut session = LocalChatSession::with_session_id(profile_id, session_id, pair.clone());
        let first_body = SignedChatBody::new("first", 2_000, 3, &[]);
        let first = session.sign(first_body).unwrap();
        assert_eq!(first.bytes.len(), MESSAGE_SIGNATURE_BYTES);
        assert!(session.verify(0, first_body, &first).unwrap());
        assert!(!session.verify(1, first_body, &first).unwrap());

        let second_body = SignedChatBody::new("second", 3_000, 4, &[]);
        let second = session.sign(second_body).unwrap();
        assert!(session.verify(1, second_body, &second).unwrap());
        assert_ne!(first, second);

        session.next_link_index = Some(i32::MAX);
        let final_body = SignedChatBody::new("final", 4_000, 5, &[]);
        let final_signature = session.sign(final_body).unwrap();
        assert!(session
            .verify(i32::MAX, final_body, &final_signature)
            .unwrap());
        assert!(session
            .sign(final_body)
            .unwrap_err()
            .to_string()
            .contains("exhausted"));
        assert!(pair.same_key_material(session.key_pair()));
    }

    #[test]
    fn outbound_requests_are_unsigned_without_a_session() {
        let last_seen = OutboundLastSeenMessages {
            update: LastSeenMessagesUpdate {
                offset: 3,
                acknowledged: 5,
                checksum: 7,
            },
            signatures: vec![MessageSignature {
                bytes: vec![0x11; MESSAGE_SIGNATURE_BYTES],
            }],
        };
        let message = prepare_outbound_chat_message(
            OutboundChatMessage {
                message: "hello".to_string(),
                timestamp_millis: 2_000,
                salt: 9,
                last_seen: last_seen.clone(),
            },
            None,
        )
        .unwrap();
        assert_eq!(message.signature, None);
        assert_eq!(message.last_seen_messages, last_seen.update);

        let command = prepare_outbound_chat_command(
            OutboundChatCommand::signed(
                "say hello",
                3_000,
                10,
                vec![SignableCommandArgument {
                    name: "message".to_string(),
                    value: "hello".to_string(),
                }],
                last_seen.clone(),
            ),
            None,
        )
        .unwrap();
        let PreparedChatCommand::Signed(command) = command else {
            panic!("signable command must retain the signed-command packet shape");
        };
        assert!(command.argument_signatures.entries.is_empty());
        assert_eq!(command.last_seen_messages, last_seen.update);

        let command =
            prepare_outbound_chat_command(OutboundChatCommand::unsigned("time set day"), None)
                .unwrap();
        let PreparedChatCommand::Unsigned(command) = command else {
            panic!("plain command must use the unsigned command packet");
        };
        assert_eq!(command.command, "time set day");
    }

    #[test]
    fn outbound_requests_sign_content_and_command_arguments_in_wire_order() {
        let pair = decode_profile_key_pair(&fixture_response()).unwrap();
        let profile_id = Uuid::from_u128(21);
        let session_id = Uuid::from_u128(22);
        let mut session = LocalChatSession::with_session_id(profile_id, session_id, pair.clone());
        let last_seen_signature = MessageSignature {
            bytes: vec![0x22; MESSAGE_SIGNATURE_BYTES],
        };
        let last_seen = OutboundLastSeenMessages {
            update: LastSeenMessagesUpdate::default(),
            signatures: vec![last_seen_signature],
        };

        let message = prepare_outbound_chat_message(
            OutboundChatMessage {
                message: "first".to_string(),
                timestamp_millis: 4_000,
                salt: 11,
                last_seen: last_seen.clone(),
            },
            Some(&mut session),
        )
        .unwrap();
        let signature = message.signature.as_ref().unwrap();
        assert!(session
            .verify(
                0,
                SignedChatBody::new("first", 4_000, 11, &last_seen.signatures),
                signature,
            )
            .unwrap());

        let command = prepare_outbound_chat_command(
            OutboundChatCommand::signed(
                "custom alpha beta",
                5_000,
                12,
                vec![
                    SignableCommandArgument {
                        name: "first".to_string(),
                        value: "alpha".to_string(),
                    },
                    SignableCommandArgument {
                        name: "second".to_string(),
                        value: "beta".to_string(),
                    },
                ],
                last_seen.clone(),
            ),
            Some(&mut session),
        )
        .unwrap();
        let PreparedChatCommand::Signed(command) = command else {
            panic!("signable command must use the signed-command packet");
        };
        assert_eq!(command.argument_signatures.entries[0].name, "first");
        assert_eq!(command.argument_signatures.entries[1].name, "second");
        assert!(session
            .verify(
                1,
                SignedChatBody::new("alpha", 5_000, 12, &last_seen.signatures),
                &command.argument_signatures.entries[0].signature,
            )
            .unwrap());
        assert!(session
            .verify(
                2,
                SignedChatBody::new("beta", 5_000, 12, &last_seen.signatures),
                &command.argument_signatures.entries[1].signature,
            )
            .unwrap());
        assert!(pair.same_key_material(session.key_pair()));
    }

    #[test]
    fn rejects_mismatched_or_non_2048_bit_profile_keys() {
        let mut response: serde_json::Value = serde_json::from_slice(&fixture_response()).unwrap();
        let mut random = StdRng::seed_from_u64(262);
        let other = RsaPrivateKey::new(&mut random, PROFILE_KEY_BITS).unwrap();
        let other_public = RsaPublicKey::from(&other).to_public_key_der().unwrap();
        response["keyPair"]["publicKey"] = pem("RSA PUBLIC KEY", other_public.as_bytes()).into();
        assert!(
            decode_profile_key_pair(&serde_json::to_vec(&response).unwrap())
                .unwrap_err()
                .to_string()
                .contains("does not match")
        );

        let short = RsaPrivateKey::new(&mut random, 1024).unwrap();
        let short_private = short.to_pkcs8_der().unwrap();
        let short_public = RsaPublicKey::from(&short).to_public_key_der().unwrap();
        response["keyPair"]["privateKey"] = pem("RSA PRIVATE KEY", short_private.as_bytes()).into();
        response["keyPair"]["publicKey"] = pem("RSA PUBLIC KEY", short_public.as_bytes()).into();
        assert!(
            decode_profile_key_pair(&serde_json::to_vec(&response).unwrap())
                .unwrap_err()
                .to_string()
                .contains("not RSA-2048")
        );
    }

    #[test]
    fn rejects_malformed_pem_timestamp_and_empty_services_signature() {
        let response: serde_json::Value = serde_json::from_slice(&fixture_response()).unwrap();

        let mut malformed = response.clone();
        malformed["keyPair"]["privateKey"] = "-----BEGIN RSA PRIVATE KEY-----\nAAAA".into();
        assert!(
            decode_profile_key_pair(&serde_json::to_vec(&malformed).unwrap())
                .unwrap_err()
                .to_string()
                .contains("footer is missing")
        );

        let mut malformed = response.clone();
        malformed["expiresAt"] = "not-an-instant".into();
        assert!(
            decode_profile_key_pair(&serde_json::to_vec(&malformed).unwrap())
                .unwrap_err()
                .to_string()
                .contains("expiresAt is not RFC 3339")
        );

        let mut malformed = response;
        malformed["publicKeySignatureV2"] = "".into();
        assert!(
            decode_profile_key_pair(&serde_json::to_vec(&malformed).unwrap())
                .unwrap_err()
                .to_string()
                .contains("signature is empty")
        );
    }

    #[tokio::test]
    async fn fetches_profile_certificate_with_bearer_post_and_bounds_response() {
        let body = fixture_response();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0u8; 16 * 1024];
            let read = socket.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("POST /player/certificates HTTP/1.1\r\n"));
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-access-token\r\n"));
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(&body).await.unwrap();
        });

        let endpoint = format!("http://{address}/player/certificates");
        let pair =
            fetch_profile_key_pair_at("test-access-token", Duration::from_secs(2), &endpoint)
                .await
                .unwrap();
        assert_eq!(pair.public_key.n().bits(), PROFILE_KEY_BITS);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn profile_certificate_http_error_redacts_access_token() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0u8; 4096];
            let _ = socket.read(&mut request).await.unwrap();
            socket
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                )
                .await
                .unwrap();
        });
        let endpoint = format!("http://{address}/player/certificates");
        let error =
            fetch_profile_key_pair_at("sensitive-access-token", Duration::from_secs(2), &endpoint)
                .await
                .unwrap_err()
                .to_string();
        assert!(!error.contains("sensitive-access-token"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn rejects_oversized_profile_certificate_from_content_length() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0u8; 4096];
            let _ = socket.read(&mut request).await.unwrap();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        MAX_PROFILE_CERTIFICATES_RESPONSE_BYTES + 1
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let endpoint = format!("http://{address}/player/certificates");
        let error =
            fetch_profile_key_pair_at("test-access-token", Duration::from_secs(2), &endpoint)
                .await
                .unwrap_err()
                .to_string();
        assert!(error.contains("response exceeds"));
        server.await.unwrap();
    }
}
