//! Authenticated Minecraft Services privacy and abuse-report transport.
//!
//! The schemas and request behavior in this module are pinned to the authlib
//! 7.0.63 dependency shipped with Minecraft Java 26.1.

use std::{collections::BTreeSet, error::Error, fmt, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{Client, Response, StatusCode};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use uuid::Uuid;

const MINECRAFT_SERVICES_BASE_URL: &str = "https://api.minecraftservices.com";
const USER_API_TIMEOUT: Duration = Duration::from_secs(5);
const BLOCK_LIST_REFRESH_COOLDOWN_SECONDS: i64 = 120;
const MAX_BLOCK_LIST_RESPONSE_BYTES: u64 = 1024 * 1024;
const MAX_BLOCKED_PROFILES: usize = 16_384;
const MAX_ABUSE_REPORT_REQUEST_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinecraftUserApiErrorKind {
    ServiceUnavailable,
    Http,
    InvalidResponse,
    InvalidRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinecraftUserApiError {
    kind: MinecraftUserApiErrorKind,
    status: Option<u16>,
    message: &'static str,
}

impl MinecraftUserApiError {
    pub fn kind(&self) -> MinecraftUserApiErrorKind {
        self.kind
    }

    pub fn status(&self) -> Option<u16> {
        self.status
    }

    fn service_unavailable(message: &'static str) -> Self {
        Self {
            kind: MinecraftUserApiErrorKind::ServiceUnavailable,
            status: None,
            message,
        }
    }

    fn http(status: StatusCode) -> Self {
        Self {
            kind: MinecraftUserApiErrorKind::Http,
            status: Some(status.as_u16()),
            message: "Minecraft Services returned an HTTP error",
        }
    }

    fn invalid_response(message: &'static str) -> Self {
        Self {
            kind: MinecraftUserApiErrorKind::InvalidResponse,
            status: None,
            message,
        }
    }

    fn invalid_request(message: &'static str) -> Self {
        Self {
            kind: MinecraftUserApiErrorKind::InvalidRequest,
            status: None,
            message,
        }
    }
}

impl fmt::Display for MinecraftUserApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status {
            Some(status) => write!(formatter, "{} ({status})", self.message),
            None => formatter.write_str(self.message),
        }
    }
}

impl Error for MinecraftUserApiError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserApiBlockList {
    blocked_profiles: Vec<Uuid>,
}

impl UserApiBlockList {
    pub fn blocked_profiles(&self) -> &[Uuid] {
        &self.blocked_profiles
    }

    pub fn is_blocked(&self, profile_id: Uuid) -> bool {
        !profile_id.is_nil() && self.blocked_profiles.binary_search(&profile_id).is_ok()
    }
}

pub struct MinecraftUserApiClient {
    http: Client,
    access_token: String,
    block_list_endpoint: String,
    abuse_report_endpoint: String,
    next_block_list_refresh_epoch_seconds: Option<i64>,
    block_list: Option<UserApiBlockList>,
}

impl fmt::Debug for MinecraftUserApiClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MinecraftUserApiClient")
            .field("access_token", &"<redacted>")
            .field(
                "next_block_list_refresh_epoch_seconds",
                &self.next_block_list_refresh_epoch_seconds,
            )
            .field(
                "cached_blocked_profiles",
                &self
                    .block_list
                    .as_ref()
                    .map(|block_list| block_list.blocked_profiles.len()),
            )
            .finish_non_exhaustive()
    }
}

impl MinecraftUserApiClient {
    pub fn authenticated(
        access_token: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, MinecraftUserApiError> {
        Self::with_endpoints(
            access_token,
            timeout,
            format!("{MINECRAFT_SERVICES_BASE_URL}/privacy/blocklist"),
            format!("{MINECRAFT_SERVICES_BASE_URL}/player/report"),
        )
    }

    fn with_endpoints(
        access_token: impl Into<String>,
        timeout: Duration,
        block_list_endpoint: String,
        abuse_report_endpoint: String,
    ) -> Result<Self, MinecraftUserApiError> {
        let access_token = access_token.into();
        if access_token.is_empty() {
            return Err(MinecraftUserApiError::invalid_request(
                "Minecraft Services require an authenticated access token",
            ));
        }
        let http = Client::builder()
            .timeout(timeout.min(USER_API_TIMEOUT))
            .build()
            .map_err(|_| {
                MinecraftUserApiError::service_unavailable(
                    "could not build Minecraft Services client",
                )
            })?;
        Ok(Self {
            http,
            access_token,
            block_list_endpoint,
            abuse_report_endpoint,
            next_block_list_refresh_epoch_seconds: None,
            block_list: None,
        })
    }

    pub fn cached_block_list(&self) -> Option<&UserApiBlockList> {
        self.block_list.as_ref()
    }

    pub fn next_block_list_refresh_epoch_seconds(&self) -> Option<i64> {
        self.next_block_list_refresh_epoch_seconds
    }

    /// Authlib's explicit `refreshBlockList`: an absent cache always requests;
    /// a populated cache requests only strictly after the 120-second deadline.
    pub async fn refresh_block_list(
        &mut self,
        now_epoch_seconds: i64,
    ) -> Result<Option<UserApiBlockList>, MinecraftUserApiError> {
        let request_due = self.block_list.is_none()
            || self
                .next_block_list_refresh_epoch_seconds
                .is_none_or(|deadline| now_epoch_seconds > deadline);
        if !request_due {
            return Ok(None);
        }

        self.next_block_list_refresh_epoch_seconds =
            Some(now_epoch_seconds.saturating_add(BLOCK_LIST_REFRESH_COOLDOWN_SECONDS));
        let result = self.request_block_list().await;
        match result {
            Ok(block_list) => {
                self.block_list = Some(block_list.clone());
                Ok(Some(block_list))
            }
            Err(error) => {
                // `YggdrasilUserApiService.refreshBlockList` assigns the null
                // fetch result, replacing a previously populated cache.
                self.block_list = None;
                Err(error)
            }
        }
    }

    async fn request_block_list(&self) -> Result<UserApiBlockList, MinecraftUserApiError> {
        let response = self
            .http
            .get(&self.block_list_endpoint)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|_| {
                MinecraftUserApiError::service_unavailable(
                    "could not request Minecraft Services block list",
                )
            })?;
        if response.status().as_u16() >= 400 {
            return Err(MinecraftUserApiError::http(response.status()));
        }
        let body = read_bounded_response(response, MAX_BLOCK_LIST_RESPONSE_BYTES).await?;
        decode_block_list(&body)
    }

    pub async fn submit_abuse_report(
        &self,
        request: &AbuseReportRequest,
    ) -> Result<(), MinecraftUserApiError> {
        let body = serde_json::to_vec(request).map_err(|_| {
            MinecraftUserApiError::invalid_request("could not encode abuse-report request")
        })?;
        if body.len() > MAX_ABUSE_REPORT_REQUEST_BYTES {
            return Err(MinecraftUserApiError::invalid_request(
                "abuse-report request exceeds the client size limit",
            ));
        }
        let response = self
            .http
            .post(&self.abuse_report_endpoint)
            .bearer_auth(&self.access_token)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            )
            .body(body)
            .send()
            .await
            .map_err(|_| {
                MinecraftUserApiError::service_unavailable(
                    "could not submit abuse report to Minecraft Services",
                )
            })?;
        if response.status().as_u16() >= 400 {
            return Err(MinecraftUserApiError::http(response.status()));
        }
        Ok(())
    }
}

fn decode_block_list(body: &[u8]) -> Result<UserApiBlockList, MinecraftUserApiError> {
    let response: BlockListResponse = serde_json::from_slice(body).map_err(|_| {
        MinecraftUserApiError::invalid_response(
            "Minecraft Services block-list response is invalid JSON",
        )
    })?;
    if response.blocked_profiles.len() > MAX_BLOCKED_PROFILES {
        return Err(MinecraftUserApiError::invalid_response(
            "Minecraft Services block-list response has too many profiles",
        ));
    }
    let blocked_profiles = response
        .blocked_profiles
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(UserApiBlockList { blocked_profiles })
}

async fn read_bounded_response(
    mut response: Response,
    max_bytes: u64,
) -> Result<Vec<u8>, MinecraftUserApiError> {
    if response
        .content_length()
        .is_some_and(|content_length| content_length > max_bytes)
    {
        return Err(MinecraftUserApiError::invalid_response(
            "Minecraft Services response exceeds the client size limit",
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        MinecraftUserApiError::service_unavailable("could not read Minecraft Services response")
    })? {
        if body.len().saturating_add(chunk.len()) as u64 > max_bytes {
            return Err(MinecraftUserApiError::invalid_response(
                "Minecraft Services response exceeds the client size limit",
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockListResponse {
    blocked_profiles: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportByteBuffer(pub Vec<u8>);

impl Serialize for ReportByteBuffer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(&self.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AbuseReportType {
    #[serde(rename = "CHAT")]
    Chat,
    #[serde(rename = "SKIN")]
    Skin,
    #[serde(rename = "USERNAME")]
    Username,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbuseReportRequest {
    pub version: i32,
    pub id: Uuid,
    pub report: AbuseReport,
    pub client_info: AbuseReportClientInfo,
    pub third_party_server_info: Option<ThirdPartyServerInfo>,
    pub realm_info: Option<AbuseReportRealmInfo>,
    pub report_type: AbuseReportType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbuseReportClientInfo {
    pub client_version: String,
    pub locale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThirdPartyServerInfo {
    pub address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbuseReportRealmInfo {
    pub realm_id: String,
    pub slot_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbuseReport {
    pub opinion_comments: String,
    pub reason: Option<String>,
    pub evidence: Option<AbuseReportEvidence>,
    pub skin_url: Option<String>,
    pub reported_entity: ReportedEntity,
    pub created_time: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportedEntity {
    pub profile_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AbuseReportEvidence {
    pub messages: Vec<AbuseReportMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbuseReportMessage {
    pub index: i32,
    pub profile_id: Uuid,
    pub session_id: Uuid,
    pub timestamp: String,
    pub salt: i64,
    pub last_seen: Vec<ReportByteBuffer>,
    pub message: String,
    pub signature: Option<ReportByteBuffer>,
    pub message_reported: bool,
}

impl Serialize for AbuseReportMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut value = serializer.serialize_struct("ReportChatMessage", 9)?;
        value.serialize_field("index", &self.index)?;
        value.serialize_field("profileId", &self.profile_id)?;
        value.serialize_field("sessionId", &self.session_id)?;
        value.serialize_field("timestamp", &self.timestamp)?;
        value.serialize_field("salt", &self.salt)?;
        value.serialize_field("lastSeen", &self.last_seen)?;
        value.serialize_field("message", &self.message)?;
        value.serialize_field("signature", &self.signature)?;
        value.serialize_field("messageReported", &self.message_reported)?;
        value.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    async fn read_http_request(socket: &mut TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0u8; 4096];
        let mut expected_len = None;
        loop {
            let read = socket.read(&mut buffer).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if expected_len.is_none() {
                if let Some(header_end) =
                    request.windows(4).position(|window| window == b"\r\n\r\n")
                {
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.split_once(':').and_then(|(name, value)| {
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                        })
                        .unwrap_or(0);
                    expected_len = Some(header_end + 4 + content_length);
                }
            }
            if expected_len.is_some_and(|expected_len| request.len() >= expected_len) {
                break;
            }
        }
        request
    }

    fn test_client(
        token: &str,
        block_list_endpoint: String,
        report_endpoint: String,
    ) -> MinecraftUserApiClient {
        MinecraftUserApiClient::with_endpoints(
            token,
            Duration::from_secs(2),
            block_list_endpoint,
            report_endpoint,
        )
        .unwrap()
    }

    fn fixture_report_request() -> AbuseReportRequest {
        AbuseReportRequest {
            version: 1,
            id: Uuid::from_u128(1),
            report: AbuseReport {
                opinion_comments: "evidence".to_string(),
                reason: Some("HARASSMENT_OR_BULLYING".to_string()),
                evidence: Some(AbuseReportEvidence {
                    messages: vec![AbuseReportMessage {
                        index: 3,
                        profile_id: Uuid::from_u128(2),
                        session_id: Uuid::from_u128(3),
                        timestamp: "2026-07-29T00:00:00Z".to_string(),
                        salt: 7,
                        last_seen: vec![ReportByteBuffer(vec![0, 1, 2])],
                        message: "signed content".to_string(),
                        signature: Some(ReportByteBuffer(vec![3, 4, 5])),
                        message_reported: true,
                    }],
                }),
                skin_url: None,
                reported_entity: ReportedEntity {
                    profile_id: Uuid::from_u128(2),
                },
                created_time: "2026-07-29T00:00:01Z".to_string(),
            },
            client_info: AbuseReportClientInfo {
                client_version: "26.1 (modded)".to_string(),
                locale: "en-US".to_string(),
            },
            third_party_server_info: Some(ThirdPartyServerInfo {
                address: "example.org:25565".to_string(),
            }),
            realm_info: None,
            report_type: AbuseReportType::Chat,
        }
    }

    #[test]
    fn user_api_report_json_matches_authlib_field_names_nulls_and_base64() {
        let value = serde_json::to_value(fixture_report_request()).unwrap();
        assert_eq!(value["version"], 1);
        assert_eq!(value["reportType"], "CHAT");
        assert_eq!(value["realmInfo"], serde_json::Value::Null);
        assert_eq!(value["report"]["skinUrl"], serde_json::Value::Null);
        assert_eq!(
            value["thirdPartyServerInfo"]["address"],
            "example.org:25565"
        );
        assert_eq!(
            value["report"]["evidence"]["messages"][0]["lastSeen"][0],
            "AAEC"
        );
        assert_eq!(
            value["report"]["evidence"]["messages"][0]["signature"],
            "AwQF"
        );
    }

    #[tokio::test]
    async fn user_api_fetches_sorted_block_list_with_bearer_and_strict_cooldown() {
        let first = Uuid::from_u128(2);
        let second = Uuid::from_u128(1);
        let body = serde_json::json!({
            "blockedProfiles": [first, Uuid::nil(), second, first],
        })
        .to_string();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut socket).await;
            let request = String::from_utf8_lossy(&request);
            assert!(request.starts_with("GET /privacy/blocklist HTTP/1.1\r\n"));
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-access-token\r\n"));
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
        let endpoint = format!("http://{address}/privacy/blocklist");
        let mut client = test_client("test-access-token", endpoint.clone(), endpoint);

        let block_list = client.refresh_block_list(100).await.unwrap().unwrap();
        assert_eq!(block_list.blocked_profiles(), &[Uuid::nil(), second, first]);
        assert!(!block_list.is_blocked(Uuid::nil()));
        assert!(block_list.is_blocked(first));
        assert_eq!(client.next_block_list_refresh_epoch_seconds(), Some(220));
        assert_eq!(client.refresh_block_list(220).await.unwrap(), None);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn user_api_rejects_oversized_block_list_before_reading_body() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = read_http_request(&mut socket).await;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        MAX_BLOCK_LIST_RESPONSE_BYTES + 1
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let endpoint = format!("http://{address}/privacy/blocklist");
        let mut client = test_client("test-access-token", endpoint.clone(), endpoint);

        let error = client.refresh_block_list(0).await.unwrap_err();
        assert_eq!(error.kind(), MinecraftUserApiErrorKind::InvalidResponse);
        assert!(error.to_string().contains("size limit"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn user_api_rejects_streamed_block_list_after_crossing_size_limit() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = read_http_request(&mut socket).await;
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nconnection: close\r\n\r\n")
                .await
                .unwrap();
            socket
                .write_all(&vec![b'x'; MAX_BLOCK_LIST_RESPONSE_BYTES as usize + 1])
                .await
                .unwrap();
        });
        let endpoint = format!("http://{address}/privacy/blocklist");
        let mut client = test_client("test-access-token", endpoint.clone(), endpoint);

        let error = client.refresh_block_list(0).await.unwrap_err();
        assert_eq!(error.kind(), MinecraftUserApiErrorKind::InvalidResponse);
        assert!(error.to_string().contains("size limit"));
        server.await.unwrap();
    }

    #[test]
    fn user_api_rejects_invalid_or_excessive_block_list_json() {
        assert_eq!(
            decode_block_list(br#"{"blockedProfiles":"not-an-array"}"#)
                .unwrap_err()
                .kind(),
            MinecraftUserApiErrorKind::InvalidResponse
        );

        let profiles = vec![Uuid::from_u128(1); MAX_BLOCKED_PROFILES + 1];
        let body = serde_json::to_vec(&serde_json::json!({
            "blockedProfiles": profiles,
        }))
        .unwrap();
        let error = decode_block_list(&body).unwrap_err();
        assert_eq!(error.kind(), MinecraftUserApiErrorKind::InvalidResponse);
        assert!(error.to_string().contains("too many profiles"));
    }

    #[test]
    fn user_api_requires_an_access_token() {
        let error = MinecraftUserApiClient::authenticated("", Duration::from_secs(5)).unwrap_err();
        assert_eq!(error.kind(), MinecraftUserApiErrorKind::InvalidRequest);
    }

    #[tokio::test]
    async fn user_api_due_refresh_failure_replaces_previous_block_list() {
        let blocked = Uuid::from_u128(7);
        let body = serde_json::json!({"blockedProfiles": [blocked]}).to_string();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut first_socket, _) = listener.accept().await.unwrap();
            let _ = read_http_request(&mut first_socket).await;
            first_socket
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

            let (mut second_socket, _) = listener.accept().await.unwrap();
            let _ = read_http_request(&mut second_socket).await;
            second_socket
                .write_all(
                    b"HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                )
                .await
                .unwrap();
        });
        let endpoint = format!("http://{address}/privacy/blocklist");
        let mut client = test_client("test-access-token", endpoint.clone(), endpoint);

        assert!(client
            .refresh_block_list(100)
            .await
            .unwrap()
            .unwrap()
            .is_blocked(blocked));
        assert_eq!(
            client.refresh_block_list(221).await.unwrap_err().status(),
            Some(503)
        );
        assert_eq!(client.cached_block_list(), None);
        assert_eq!(client.next_block_list_refresh_epoch_seconds(), Some(341));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn user_api_submits_exact_json_after_bearer_header() {
        let expected = serde_json::to_value(fixture_report_request()).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut socket).await;
            let header_end = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .unwrap();
            let headers = String::from_utf8_lossy(&request[..header_end]);
            assert!(headers.starts_with("POST /player/report HTTP/1.1\r\n"));
            assert!(headers
                .to_ascii_lowercase()
                .contains("authorization: bearer report-access-token\r\n"));
            assert!(headers
                .to_ascii_lowercase()
                .contains("content-type: application/json; charset=utf-8\r\n"));
            let actual: serde_json::Value =
                serde_json::from_slice(&request[header_end + 4..]).unwrap();
            assert_eq!(actual, expected);
            socket
                .write_all(b"HTTP/1.1 204 No Content\r\nconnection: close\r\n\r\n")
                .await
                .unwrap();
        });
        let endpoint = format!("http://{address}/player/report");
        let client = test_client("report-access-token", endpoint.clone(), endpoint);

        client
            .submit_abuse_report(&fixture_report_request())
            .await
            .unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn user_api_rejects_oversized_report_before_network_request() {
        let mut request = fixture_report_request();
        request.report.opinion_comments = "x".repeat(MAX_ABUSE_REPORT_REQUEST_BYTES + 1);
        let client = test_client(
            "test-access-token",
            "http://127.0.0.1:1/privacy/blocklist".to_string(),
            "http://127.0.0.1:1/player/report".to_string(),
        );

        let error = client.submit_abuse_report(&request).await.unwrap_err();
        assert_eq!(error.kind(), MinecraftUserApiErrorKind::InvalidRequest);
        assert!(error.to_string().contains("size limit"));
    }

    #[tokio::test]
    async fn user_api_http_errors_and_debug_never_expose_access_token() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = read_http_request(&mut socket).await;
            socket
                .write_all(
                    b"HTTP/1.1 403 Forbidden\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                )
                .await
                .unwrap();
        });
        let endpoint = format!("http://{address}/privacy/blocklist");
        let mut client = test_client("sensitive-access-token", endpoint.clone(), endpoint);
        let debug = format!("{client:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("sensitive-access-token"));

        let error = client.refresh_block_list(0).await.unwrap_err();
        assert_eq!(error.kind(), MinecraftUserApiErrorKind::Http);
        assert_eq!(error.status(), Some(403));
        assert!(!error.to_string().contains("sensitive-access-token"));
        server.await.unwrap();
    }
}
