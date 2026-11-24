use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{broadcast, oneshot},
    time::timeout,
};

use crate::{
    CallRefusal, Caller, HostIdentity, JsonRpcError, McpHost, PendingCall, ResourceChange,
    ResourceDescriptor, Scope, ToolDescriptor, INTERNAL_ERROR, PROTOCOL_VERSION, SESSION_HEADER,
};

type AuthConfig = crate::AuthConfig<Scope>;
type TokenGrant = crate::TokenGrant<Scope>;
type McpServerConfig = crate::McpServerConfig<TestHost>;
type McpServer = crate::McpServer<TestHost>;

const OPERATION_URI_PREFIX: &str = "bbb://test/operation/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Answer,
    Withhold,
    Busy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestOutcome {
    operation_id: u64,
    action: &'static str,
    refused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedCall {
    caller: Caller<Scope>,
    action: &'static str,
    arguments: Value,
}

struct Shared {
    mode: Mode,
    next_operation: AtomicU64,
    calls: Mutex<Vec<RecordedCall>>,
    withheld: Mutex<Vec<(u64, oneshot::Sender<TestOutcome>)>>,
    changes: broadcast::Sender<ResourceChange>,
}

/// A host with one readout and one mutation, answering, withholding or
/// refusing every call as its mode says.
#[derive(Clone)]
pub(super) struct TestHost(Arc<Shared>);

impl TestHost {
    fn new(mode: Mode) -> Self {
        Self(Arc::new(Shared {
            mode,
            next_operation: AtomicU64::new(1),
            calls: Mutex::new(Vec::new()),
            withheld: Mutex::new(Vec::new()),
            changes: broadcast::channel(8).0,
        }))
    }

    pub(super) fn answering() -> Self {
        Self::new(Mode::Answer)
    }

    fn calls(&self) -> Vec<RecordedCall> {
        self.0.calls.lock().expect("calls").clone()
    }

    fn answer_withheld(&self) {
        let withheld = std::mem::take(&mut *self.0.withheld.lock().expect("withheld"));
        for (operation_id, sender) in withheld {
            let _ = sender.send(TestOutcome {
                operation_id,
                action: "status",
                refused: false,
            });
        }
    }

    fn submit(
        &self,
        caller: &Caller<Scope>,
        action: &'static str,
        required: Scope,
        arguments: Value,
    ) -> Result<PendingCall<TestOutcome>, CallRefusal> {
        if self.0.mode == Mode::Busy {
            return Err(CallRefusal::Unavailable {
                detail: "the test host is busy".to_string(),
                retryable: true,
            });
        }
        let operation_id = self.0.next_operation.fetch_add(1, Ordering::Relaxed);
        self.0.calls.lock().expect("calls").push(RecordedCall {
            caller: caller.clone(),
            action,
            arguments,
        });
        let outcome = TestOutcome {
            operation_id,
            action,
            refused: caller.scope < required,
        };
        let (sender, answer) = oneshot::channel();
        match self.0.mode {
            Mode::Answer => {
                let _ = sender.send(outcome);
            }
            Mode::Withhold => self
                .0
                .withheld
                .lock()
                .expect("withheld")
                .push((operation_id, sender)),
            Mode::Busy => unreachable!("a busy host never submits"),
        }
        Ok(PendingCall {
            operation_id,
            action,
            answer,
        })
    }
}

impl McpHost for TestHost {
    type Scopes = Scope;
    type Outcome = TestOutcome;

    fn identity() -> HostIdentity {
        HostIdentity {
            name: "bbb-test",
            title: "A test host",
            version: "0.0.0",
            instructions: Some("Read bbb://test/status, then reset."),
            operation_uri_prefix: OPERATION_URI_PREFIX,
        }
    }

    fn now(&self) -> i64 {
        1_755_000_000_000
    }

    fn tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor {
                name: "status",
                title: "Status",
                summary: "The host's status.",
                input_schema: json!({ "type": "object", "properties": {} }),
                read_only: true,
                destructive: false,
                meta: None,
            },
            ToolDescriptor {
                name: "reset",
                title: "Reset",
                summary: "Resets the host.",
                input_schema: json!({
                    "type": "object",
                    "properties": { "hard": { "type": "boolean" } },
                }),
                read_only: false,
                destructive: true,
                meta: Some(json!({ "bbb/scope": "manage" })),
            },
        ]
    }

    fn resources(&self) -> Vec<ResourceDescriptor> {
        vec![ResourceDescriptor {
            uri: "bbb://test/status",
            name: "status",
            title: "Status",
            summary: "The host's status.",
        }]
    }

    fn call_tool(
        &self,
        caller: &Caller<Scope>,
        name: &str,
        arguments: &Value,
    ) -> Result<PendingCall<TestOutcome>, CallRefusal> {
        match name {
            "status" => self.submit(caller, "status", Scope::Read, arguments.clone()),
            "reset" => {
                if !arguments.get("hard").is_none_or(|value| value.is_boolean()) {
                    return Err(CallRefusal::InvalidArguments(
                        "hard is a boolean".to_string(),
                    ));
                }
                self.submit(caller, "reset", Scope::Manage, arguments.clone())
            }
            _ => Err(CallRefusal::Unknown),
        }
    }

    fn read_resource(
        &self,
        caller: &Caller<Scope>,
        uri: &str,
    ) -> Result<PendingCall<TestOutcome>, CallRefusal> {
        match uri {
            "bbb://test/status" => self.submit(caller, "status", Scope::Read, Value::Null),
            _ => Err(CallRefusal::Unknown),
        }
    }

    fn tool_result(&self, outcome: &TestOutcome) -> Value {
        json!({
            "content": [{ "type": "text", "text": outcome.action }],
            "structuredContent": { "action": outcome.action },
            "isError": outcome.refused,
            "_meta": { "bbb/operationId": outcome.operation_id },
        })
    }

    fn outcome_record(&self, outcome: &TestOutcome) -> Value {
        json!({
            "operationId": outcome.operation_id,
            "action": outcome.action,
            "refused": outcome.refused,
        })
    }

    fn resource_contents(&self, outcome: &TestOutcome) -> Result<Value, JsonRpcError> {
        if outcome.refused {
            return Err(JsonRpcError::new(INTERNAL_ERROR, "refused"));
        }
        Ok(json!({ "status": "up", "operationId": outcome.operation_id }))
    }

    fn changes(&self) -> broadcast::Receiver<ResourceChange> {
        self.0.changes.subscribe()
    }
}

fn loopback() -> SocketAddr {
    "127.0.0.1:52000".parse().expect("address")
}

fn remote() -> SocketAddr {
    "203.0.113.7:52000".parse().expect("address")
}

fn server(host: TestHost) -> Arc<McpServer> {
    McpServer::new(McpServerConfig::default(), host)
}

struct RawResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl RawResponse {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("body is JSON: {error}; body was {:?}", self.body))
    }

    fn result(&self) -> Value {
        let body = self.json();
        assert!(
            body.get("error").is_none(),
            "expected a result, got {}",
            self.body
        );
        body["result"].clone()
    }

    fn error_code(&self) -> i64 {
        self.json()["error"]["code"]
            .as_i64()
            .unwrap_or_else(|| panic!("expected an error, got {}", self.body))
    }
}

async fn exchange(server: &Arc<McpServer>, peer: SocketAddr, raw: String) -> RawResponse {
    let (mut client, service) = tokio::io::duplex(256 * 1024);
    let handler = tokio::spawn(Arc::clone(server).handle_connection(service, peer));
    client.write_all(raw.as_bytes()).await.expect("write");
    let mut received = Vec::new();
    timeout(Duration::from_secs(5), client.read_to_end(&mut received))
        .await
        .expect("the answer arrives")
        .expect("read");
    handler.await.expect("the connection task finishes");
    parse_response(&received)
}

async fn read_head(reader: &mut (impl AsyncRead + Unpin)) -> Vec<u8> {
    timeout(Duration::from_secs(5), async {
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            reader.read_exact(&mut byte).await.expect("response head");
            head.push(byte[0]);
        }
        head
    })
    .await
    .expect("the response head arrives")
}

#[tokio::test]
async fn unfinished_requests_and_idle_keep_alive_connections_release_the_connection_budget() {
    let config = McpServerConfig {
        transport_timeout: Duration::from_secs(1),
        max_connections: 3,
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let (stop, stopped) = oneshot::channel();
    let serving = tokio::spawn(Arc::clone(&server).serve(listener, async {
        let _ = stopped.await;
    }));

    let mut header = TcpStream::connect(address).await.expect("connect header");
    header
        .write_all(b"POST /mcp HTTP/1.1\r\nhost:")
        .await
        .expect("partial header");
    let mut body = TcpStream::connect(address).await.expect("connect body");
    body.write_all(
        b"POST /mcp HTTP/1.1\r\nhost: h\r\ncontent-length: 2\r\nexpect: 100-continue\r\n\r\n{",
    )
    .await
    .expect("partial body");
    assert_eq!(read_head(&mut body).await, b"HTTP/1.1 100 Continue\r\n\r\n");
    let mut idle = TcpStream::connect(address).await.expect("connect idle");
    idle.write_all(b"OPTIONS /mcp HTTP/1.1\r\nhost: h\r\n\r\n")
        .await
        .expect("request");
    assert_eq!(parse_response(&read_head(&mut idle).await).status, 204);

    let mut excess = TcpStream::connect(address).await.expect("connect excess");
    let _ = excess
        .write_all(b"OPTIONS /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n")
        .await;
    let mut byte = [0];
    let refused = timeout(Duration::from_secs(5), excess.read(&mut byte))
        .await
        .expect("refusal");
    assert!(
        matches!(refused, Ok(0) | Err(_)),
        "the three connections exhaust the budget"
    );

    for mut connection in [header, body, idle] {
        let mut remaining = Vec::new();
        timeout(
            Duration::from_secs(5),
            connection.read_to_end(&mut remaining),
        )
        .await
        .expect("the transport deadline closes the connection")
        .expect("read EOF");
    }
    let mut client = TcpStream::connect(address)
        .await
        .expect("connect after deadlines");
    client
        .write_all(b"OPTIONS /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n")
        .await
        .expect("request after deadlines");
    let mut response = Vec::new();
    timeout(Duration::from_secs(5), client.read_to_end(&mut response))
        .await
        .expect("the recovered slot answers")
        .expect("response");
    assert_eq!(parse_response(&response).status, 204);
    stop.send(()).expect("stop server");
    serving.await.expect("server stopped");
}

#[tokio::test]
async fn an_unread_response_cannot_hold_the_connection_open() {
    let config = McpServerConfig {
        transport_timeout: Duration::from_millis(50),
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());
    let (mut client, service) = tokio::io::duplex(16);
    let handler = tokio::spawn(server.handle_connection(service, loopback()));
    client
        .write_all(b"OPTIONS /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n")
        .await
        .expect("request");
    timeout(Duration::from_secs(5), handler)
        .await
        .expect("blocked response deadline")
        .expect("handler");
    let mut partial = Vec::new();
    client
        .read_to_end(&mut partial)
        .await
        .expect("closed response");
    assert!(!partial.is_empty());
    assert!(!partial.ends_with(b"\r\n\r\n"));
}

#[tokio::test]
async fn blocked_event_stream_writes_release_the_session_stream() {
    let config = McpServerConfig {
        transport_timeout: Duration::from_millis(50),
        keep_alive_interval: Duration::from_millis(10),
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());
    let session = initialize(&server).await;
    server
        .sessions()
        .subscribe(&session, "bbb://test/status")
        .expect("subscribe");

    for blocked_write in ["headers", "event", "keep-alive"] {
        let capacity = if blocked_write == "headers" { 16 } else { 256 };
        let (mut client, service) = tokio::io::duplex(capacity);
        let handler = tokio::spawn(Arc::clone(&server).handle_connection(service, loopback()));
        let request = format!("GET /mcp HTTP/1.1\r\nhost: h\r\naccept: text/event-stream\r\n{SESSION_HEADER}: {session}\r\n\r\n");
        client
            .write_all(request.as_bytes())
            .await
            .expect("stream request");
        if blocked_write != "headers" {
            assert_eq!(parse_response(&read_head(&mut client).await).status, 200);
        }
        if blocked_write == "event" {
            assert_eq!(
                server
                    .sessions()
                    .publish("bbb://test/status", &json!({ "data": "x".repeat(2048) })),
                1
            );
        }
        timeout(Duration::from_secs(5), handler)
            .await
            .unwrap_or_else(|_| panic!("blocked {blocked_write} deadline"))
            .expect("handler");
        server
            .sessions()
            .open_stream(&session)
            .expect("the timed-out stream released its session claim");
        server.sessions().close_stream(&session);
    }
}

fn parse_response(bytes: &[u8]) -> RawResponse {
    let text = String::from_utf8_lossy(bytes).to_string();
    let (head, body) = text.split_once("\r\n\r\n").expect("a response head");
    let mut lines = head.split("\r\n");
    let status_line = lines.next().expect("a status line");
    let status = status_line
        .split(' ')
        .nth(1)
        .and_then(|status| status.parse::<u16>().ok())
        .expect("a status code");
    let headers = lines
        .filter_map(|line| line.split_once(": "))
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    RawResponse {
        status,
        headers,
        body: body.to_string(),
    }
}

fn post(body: &Value, session: Option<&str>, extra: &[(&str, &str)]) -> String {
    let body = body.to_string();
    let mut request = String::from("POST /mcp HTTP/1.1\r\nhost: 127.0.0.1\r\n");
    request.push_str("accept: application/json, text/event-stream\r\n");
    request.push_str("content-type: application/json\r\n");
    request.push_str("connection: close\r\n");
    if let Some(session) = session {
        request.push_str(&format!("{SESSION_HEADER}: {session}\r\n"));
    }
    for (name, value) in extra {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str(&format!("content-length: {}\r\n\r\n", body.len()));
    request.push_str(&body);
    request
}

fn call(id: i64, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

async fn initialize_unconfirmed(server: &Arc<McpServer>) -> String {
    let response = exchange(
        server,
        loopback(),
        post(
            &call(
                1,
                "initialize",
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": { "name": "bbb-admin", "version": "0.1.0" },
                }),
            ),
            None,
            &[],
        ),
    )
    .await;
    assert_eq!(response.status, 200, "{}", response.body);
    response
        .header(SESSION_HEADER)
        .expect("a session id")
        .to_string()
}

async fn initialize(server: &Arc<McpServer>) -> String {
    let session = initialize_unconfirmed(server).await;
    let ready = exchange(
        server,
        loopback(),
        post(
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(ready.status, 202, "{}", ready.body);
    session
}

async fn read_record(server: &Arc<McpServer>, session: &str, id: i64, uri: &str) -> Value {
    let response = exchange(
        server,
        loopback(),
        post(
            &call(id, "resources/read", json!({ "uri": uri })),
            Some(session),
            &[],
        ),
    )
    .await;
    serde_json::from_str(
        response.result()["contents"][0]["text"]
            .as_str()
            .expect("text"),
    )
    .expect("the record is JSON")
}

#[tokio::test]
async fn the_handshake_answers_the_pinned_revision_and_the_host_identity() {
    let server = server(TestHost::answering());
    let response = exchange(
        &server,
        loopback(),
        post(
            &call(
                1,
                "initialize",
                json!({ "protocolVersion": "1999-01-01", "capabilities": {} }),
            ),
            None,
            &[],
        ),
    )
    .await;
    let result = response.result();
    assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
    assert_eq!(result["capabilities"]["resources"]["subscribe"], true);
    assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
    assert_eq!(result["serverInfo"]["name"], "bbb-test");
    assert_eq!(result["serverInfo"]["title"], "A test host");
    assert_eq!(result["serverInfo"]["version"], "0.0.0");
    assert_eq!(
        result["instructions"],
        "Read bbb://test/status, then reset."
    );
    assert_eq!(result["_meta"]["bbb/scope"], "manage");
    assert_eq!(result["_meta"]["bbb/principal"], "loopback");
    assert_eq!(
        result["_meta"]["bbb/requestedProtocolVersion"],
        "1999-01-01"
    );
    let session = response.header(SESSION_HEADER).expect("a session id");
    assert_eq!(session.len(), 32, "128 bits of session id");
    assert!(session
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
    assert_eq!(server.sessions().len(), 1);
}

#[tokio::test]
async fn a_session_is_required_known_versioned_and_deletable() {
    let server = server(TestHost::answering());
    let session = initialize(&server).await;

    let missing = exchange(
        &server,
        loopback(),
        post(&call(2, "ping", json!({})), None, &[]),
    )
    .await;
    assert_eq!(missing.status, 400);

    let unknown = exchange(
        &server,
        loopback(),
        post(&call(2, "ping", json!({})), Some("deadbeef"), &[]),
    )
    .await;
    assert_eq!(unknown.status, 404);

    let wrong_version = exchange(
        &server,
        loopback(),
        post(
            &call(2, "ping", json!({})),
            Some(&session),
            &[("mcp-protocol-version", "2024-11-05")],
        ),
    )
    .await;
    assert_eq!(wrong_version.status, 400);

    let accepted = exchange(
        &server,
        loopback(),
        post(
            &call(2, "ping", json!({})),
            Some(&session),
            &[("mcp-protocol-version", PROTOCOL_VERSION)],
        ),
    )
    .await;
    assert_eq!(accepted.result(), json!({}));

    let mut request = String::from("DELETE /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n");
    request.push_str(&format!("{SESSION_HEADER}: {session}\r\n\r\n"));
    let deleted = exchange(&server, loopback(), request).await;
    assert_eq!(deleted.status, 204);
    assert!(server.sessions().is_empty());
    let after = exchange(
        &server,
        loopback(),
        post(&call(3, "ping", json!({})), Some(&session), &[]),
    )
    .await;
    assert_eq!(after.status, 404);
}

#[tokio::test]
async fn a_tool_call_reaches_the_host_with_its_caller_and_answers_its_result() {
    let host = TestHost::answering();
    let server = server(host.clone());
    let session = initialize(&server).await;
    let response = exchange(
        &server,
        loopback(),
        post(
            &call(
                2,
                "tools/call",
                json!({ "name": "reset", "arguments": { "hard": true } }),
            ),
            Some(&session),
            &[],
        ),
    )
    .await;
    let result = response.result();
    assert_eq!(result["isError"], false);
    assert_eq!(result["structuredContent"]["action"], "reset");
    assert_eq!(result["_meta"]["bbb/operationId"], 1);
    let calls = host.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].action, "reset");
    assert_eq!(calls[0].arguments, json!({ "hard": true }));
    assert_eq!(calls[0].caller.principal, "loopback");
    assert_eq!(calls[0].caller.scope, Scope::Manage);
    assert_eq!(calls[0].caller.request_id, format!("{session}:2"));

    // A caller-supplied request id is what makes a retry the same operation.
    for id in 3..5 {
        exchange(
            &server,
            loopback(),
            post(
                &call(
                    id,
                    "tools/call",
                    json!({
                        "name": "status",
                        "_meta": { "bbb/requestId": "restart-1" },
                    }),
                ),
                Some(&session),
                &[],
            ),
        )
        .await
        .result();
    }
    let calls = host.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[1].caller.request_id, "restart-1");
    assert_eq!(calls[2].caller.request_id, "restart-1");
    assert_eq!(calls[2].arguments, json!({}));
}

#[tokio::test]
async fn a_refusal_the_host_decided_is_content_and_a_protocol_fault_is_an_error() {
    let config = McpServerConfig {
        auth: AuthConfig {
            loopback_scope: Some(Scope::Read),
            ..AuthConfig::default()
        },
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());
    let session = initialize(&server).await;

    let refused = exchange(
        &server,
        loopback(),
        post(
            &call(2, "tools/call", json!({ "name": "reset", "arguments": {} })),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(refused.result()["isError"], true);

    let unknown_tool = exchange(
        &server,
        loopback(),
        post(
            &call(3, "tools/call", json!({ "name": "destroy" })),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(unknown_tool.error_code(), i64::from(crate::INVALID_PARAMS));
    assert!(unknown_tool.json()["error"]["message"]
        .as_str()
        .expect("message")
        .contains("\"destroy\" is not a tool"));

    let bad_arguments = exchange(
        &server,
        loopback(),
        post(
            &call(
                4,
                "tools/call",
                json!({ "name": "reset", "arguments": { "hard": "yes" } }),
            ),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(bad_arguments.error_code(), i64::from(crate::INVALID_PARAMS));

    let nameless = exchange(
        &server,
        loopback(),
        post(&call(5, "tools/call", json!({})), Some(&session), &[]),
    )
    .await;
    assert_eq!(nameless.error_code(), i64::from(crate::INVALID_PARAMS));

    let unknown_method = exchange(
        &server,
        loopback(),
        post(&call(6, "prompts/list", json!({})), Some(&session), &[]),
    )
    .await;
    assert_eq!(
        unknown_method.error_code(),
        i64::from(crate::METHOD_NOT_FOUND)
    );
}

#[tokio::test]
async fn the_listings_are_what_the_host_declares() {
    let server = server(TestHost::answering());
    let session = initialize(&server).await;
    let tools = exchange(
        &server,
        loopback(),
        post(&call(2, "tools/list", json!({})), Some(&session), &[]),
    )
    .await;
    let tools = tools.result()["tools"].clone();
    assert_eq!(tools.as_array().expect("tools").len(), 2);
    assert_eq!(tools[0]["name"], "status");
    assert_eq!(tools[0]["annotations"]["readOnlyHint"], true);
    assert_eq!(tools[0]["annotations"]["destructiveHint"], false);
    assert!(tools[0].get("_meta").is_none());
    assert_eq!(tools[1]["name"], "reset");
    assert_eq!(
        tools[1]["inputSchema"]["properties"]["hard"]["type"],
        "boolean"
    );
    assert_eq!(tools[1]["annotations"]["destructiveHint"], true);
    assert_eq!(tools[1]["_meta"]["bbb/scope"], "manage");

    let resources = exchange(
        &server,
        loopback(),
        post(&call(3, "resources/list", json!({})), Some(&session), &[]),
    )
    .await;
    let resources = resources.result()["resources"].clone();
    assert_eq!(resources.as_array().expect("resources").len(), 1);
    assert_eq!(resources[0]["uri"], "bbb://test/status");
    assert_eq!(resources[0]["mimeType"], "application/json");

    let templates = exchange(
        &server,
        loopback(),
        post(
            &call(4, "resources/templates/list", json!({})),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(
        templates.result()["resourceTemplates"][0]["uriTemplate"],
        format!("{OPERATION_URI_PREFIX}{{operationId}}")
    );
}

#[tokio::test]
async fn reading_a_resource_goes_through_the_same_host_call_a_tool_does() {
    let host = TestHost::answering();
    let server = server(host.clone());
    let session = initialize(&server).await;
    let response = exchange(
        &server,
        loopback(),
        post(
            &call(2, "resources/read", json!({ "uri": "bbb://test/status" })),
            Some(&session),
            &[],
        ),
    )
    .await;
    let contents = response.result();
    assert_eq!(contents["contents"][0]["uri"], "bbb://test/status");
    assert_eq!(contents["contents"][0]["mimeType"], "application/json");
    assert!(contents["contents"][0]["text"]
        .as_str()
        .expect("text")
        .contains("\"status\": \"up\""));
    assert_eq!(host.calls()[0].action, "status");

    let missing = exchange(
        &server,
        loopback(),
        post(
            &call(3, "resources/read", json!({ "uri": "bbb://test/secrets" })),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(missing.error_code(), i64::from(crate::RESOURCE_NOT_FOUND));

    let unsubscribable = exchange(
        &server,
        loopback(),
        post(
            &call(
                4,
                "resources/subscribe",
                json!({ "uri": "bbb://test/secrets" }),
            ),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(
        unsubscribable.error_code(),
        i64::from(crate::RESOURCE_NOT_FOUND)
    );
}

#[tokio::test]
async fn a_deadline_leaves_the_work_running_and_the_answer_readable_by_id() {
    let host = TestHost::new(Mode::Withhold);
    let config = McpServerConfig {
        request_timeout: Duration::from_millis(50),
        transport_timeout: Duration::from_millis(10),
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, host.clone());
    let session = initialize(&server).await;
    let deferred = exchange(
        &server,
        loopback(),
        post(
            &call(2, "tools/call", json!({ "name": "status" })),
            Some(&session),
            &[],
        ),
    )
    .await;
    let result = deferred.result();
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["_meta"]["bbb/deferred"]["operationUri"],
        format!("{OPERATION_URI_PREFIX}1")
    );

    let record = read_record(&server, &session, 3, &format!("{OPERATION_URI_PREFIX}1")).await;
    assert_eq!(record["state"], "pending");
    assert_eq!(record["action"], "status");
    assert_eq!(record["requestId"], format!("{session}:2"));

    host.answer_withheld();
    tokio::task::yield_now().await;
    let record = read_record(&server, &session, 4, &format!("{OPERATION_URI_PREFIX}1")).await;
    assert_eq!(record["state"], "applied");
    assert_eq!(record["outcome"]["operationId"], 1);
    assert_eq!(record["outcome"]["refused"], false);

    // A record is readable only by the principal that submitted it, and a
    // resource read that misses its deadline is an error naming the record.
    let missing = exchange(
        &server,
        loopback(),
        post(
            &call(
                5,
                "resources/read",
                json!({ "uri": format!("{OPERATION_URI_PREFIX}99") }),
            ),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(missing.error_code(), i64::from(crate::RESOURCE_NOT_FOUND));
    let deferred_read = exchange(
        &server,
        loopback(),
        post(
            &call(6, "resources/read", json!({ "uri": "bbb://test/status" })),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(deferred_read.error_code(), i64::from(INTERNAL_ERROR));
    assert_eq!(
        deferred_read.json()["error"]["data"]["operationUri"],
        format!("{OPERATION_URI_PREFIX}2")
    );
}

#[tokio::test]
async fn a_busy_host_is_an_attributable_refusal_rather_than_a_wait() {
    let server = server(TestHost::new(Mode::Busy));
    let session = initialize(&server).await;
    let response = exchange(
        &server,
        loopback(),
        post(
            &call(2, "tools/call", json!({ "name": "status" })),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(response.error_code(), i64::from(INTERNAL_ERROR));
    assert_eq!(response.json()["error"]["data"]["retryable"], true);
    assert_eq!(response.json()["error"]["message"], "the test host is busy");

    let read = exchange(
        &server,
        loopback(),
        post(
            &call(3, "resources/read", json!({ "uri": "bbb://test/status" })),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(read.error_code(), i64::from(INTERNAL_ERROR));
    assert!(read.json()["error"].get("data").is_none());
}

#[tokio::test]
async fn a_subscription_delivers_the_change_the_host_published() {
    let host = TestHost::answering();
    let config = McpServerConfig {
        transport_timeout: Duration::from_millis(50),
        keep_alive_interval: Duration::from_millis(100),
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, host.clone());
    let fanout = server.spawn_change_fanout();
    let session = initialize(&server).await;
    let subscribed = exchange(
        &server,
        loopback(),
        post(
            &call(
                2,
                "resources/subscribe",
                json!({ "uri": "bbb://test/status" }),
            ),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(subscribed.result(), json!({}));

    let (mut client, service) = tokio::io::duplex(64 * 1024);
    let stream_server = Arc::clone(&server);
    let stream = tokio::spawn(stream_server.handle_connection(service, loopback()));
    let mut request = String::from("GET /mcp HTTP/1.1\r\nhost: h\r\naccept: text/event-stream\r\n");
    request.push_str(&format!("{SESSION_HEADER}: {session}\r\n\r\n"));
    client.write_all(request.as_bytes()).await.expect("write");

    let mut received = String::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = timeout(Duration::from_secs(5), client.read(&mut buffer))
            .await
            .expect("the stream answers")
            .expect("read");
        received.push_str(&String::from_utf8_lossy(&buffer[..read]));
        assert_ne!(read, 0, "the stream stays open while waiting for events");
        if received.contains(": keep-alive\n\n") {
            break;
        }
    }
    host.0
        .changes
        .send(ResourceChange {
            uri: "bbb://test/status",
        })
        .expect("publish");
    loop {
        let read = timeout(Duration::from_secs(5), client.read(&mut buffer))
            .await
            .expect("the notification arrives")
            .expect("read");
        assert_ne!(
            read, 0,
            "the stream delivers the notification after keep-alive"
        );
        received.push_str(&String::from_utf8_lossy(&buffer[..read]));
        if received.contains("notifications/resources/updated") {
            break;
        }
    }
    assert!(received.contains("bbb://test/status"), "{received}");
    assert!(received.contains("event: message"), "{received}");

    let second = exchange(&server, loopback(), {
        let mut request =
            String::from("GET /mcp HTTP/1.1\r\nhost: h\r\naccept: text/event-stream\r\n");
        request.push_str("connection: close\r\n");
        request.push_str(&format!("{SESSION_HEADER}: {session}\r\n\r\n"));
        request
    })
    .await;
    assert_eq!(second.status, 409);

    server.sessions().remove(&session);
    drop(client);
    let _ = timeout(Duration::from_secs(5), stream).await;
    fanout.abort();
}

#[tokio::test]
async fn a_credential_is_required_off_the_loopback_and_names_the_principal() {
    let config = McpServerConfig {
        auth: AuthConfig {
            loopback_scope: None,
            tokens: vec![TokenGrant {
                principal: "release-agent".to_string(),
                scope: Scope::Read,
                secret: "s3cret-token".to_string(),
            }],
            ..AuthConfig::default()
        },
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());

    let anonymous = exchange(
        &server,
        remote(),
        post(&call(1, "initialize", json!({})), None, &[]),
    )
    .await;
    assert_eq!(anonymous.status, 401);
    assert_eq!(anonymous.header("www-authenticate"), Some("Bearer"));

    let wrong = exchange(
        &server,
        remote(),
        post(
            &call(1, "initialize", json!({})),
            None,
            &[("authorization", "Bearer wrong-token")],
        ),
    )
    .await;
    assert_eq!(wrong.status, 401);

    let accepted = exchange(
        &server,
        remote(),
        post(
            &call(1, "initialize", json!({})),
            None,
            &[("authorization", "Bearer s3cret-token")],
        ),
    )
    .await;
    assert_eq!(accepted.status, 200);
    assert_eq!(accepted.result()["_meta"]["bbb/principal"], "release-agent");
    assert_eq!(accepted.result()["_meta"]["bbb/scope"], "read");
    let session = accepted
        .header(SESSION_HEADER)
        .expect("session")
        .to_string();

    let stolen = exchange(
        &server,
        remote(),
        post(&call(2, "ping", json!({})), Some(&session), &[]),
    )
    .await;
    assert_eq!(stolen.status, 401);
}

#[tokio::test]
async fn a_browser_origin_is_refused_unless_it_was_declared() {
    let server = server(TestHost::answering());
    let refused = exchange(
        &server,
        loopback(),
        post(
            &call(1, "initialize", json!({})),
            None,
            &[("origin", "https://evil.example")],
        ),
    )
    .await;
    assert_eq!(refused.status, 403);
    assert_eq!(refused.header("access-control-allow-origin"), None);

    let config = McpServerConfig {
        auth: AuthConfig {
            allowed_origins: vec!["https://console.example".to_string()],
            ..AuthConfig::default()
        },
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());

    let preflight_refused = exchange(
        &server,
        loopback(),
        "OPTIONS /mcp HTTP/1.1\r\nhost: h\r\norigin: https://evil.example\r\nconnection: close\r\n\r\n"
            .to_string(),
    )
    .await;
    assert_eq!(preflight_refused.status, 403);
    assert_eq!(
        preflight_refused.header("access-control-allow-origin"),
        None
    );

    let preflight = exchange(
        &server,
        loopback(),
        "OPTIONS /mcp HTTP/1.1\r\nhost: h\r\norigin: https://console.example\r\nconnection: close\r\n\r\n"
            .to_string(),
    )
    .await;
    assert_eq!(preflight.status, 204);
    assert_eq!(
        preflight.header("access-control-allow-origin"),
        Some("https://console.example")
    );
    assert_eq!(preflight.header("vary"), Some("Origin"));
    assert_eq!(
        preflight.header("access-control-allow-methods"),
        Some("GET, POST, DELETE, OPTIONS")
    );
    let allow_headers = preflight
        .header("access-control-allow-headers")
        .expect("preflight allow-headers");
    assert!(allow_headers.contains("Authorization"));
    assert!(allow_headers.contains("Content-Type"));
    assert!(allow_headers.contains("Mcp-Session-Id"));
    assert!(allow_headers.contains("Mcp-Protocol-Version"));
    assert_eq!(
        preflight.header("access-control-expose-headers"),
        Some("Mcp-Session-Id")
    );

    let accepted = exchange(
        &server,
        loopback(),
        post(
            &call(1, "initialize", json!({})),
            None,
            &[("origin", "https://console.example")],
        ),
    )
    .await;
    assert_eq!(accepted.status, 200);
    assert_eq!(
        accepted.header("access-control-allow-origin"),
        Some("https://console.example")
    );
    assert_eq!(accepted.header("vary"), Some("Origin"));
    assert_eq!(
        accepted.header("access-control-expose-headers"),
        Some("Mcp-Session-Id")
    );
    let session_id = accepted
        .header(crate::SESSION_HEADER)
        .expect("session header")
        .to_string();

    let initialized = exchange(
        &server,
        loopback(),
        post(
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            Some(&session_id),
            &[("origin", "https://console.example")],
        ),
    )
    .await;
    assert_eq!(initialized.status, 202);

    let (mut client, server_side) = tokio::io::duplex(4096);
    let serving = tokio::spawn(Arc::clone(&server).handle_connection(server_side, loopback()));
    let sse_req = format!(
        "GET /mcp HTTP/1.1\r\nhost: h\r\naccept: text/event-stream\r\norigin: https://console.example\r\n{}: {session_id}\r\nconnection: close\r\n\r\n",
        crate::SESSION_HEADER
    );
    client.write_all(sse_req.as_bytes()).await.expect("write");
    let mut reader = tokio::io::BufReader::new(client);
    let head = crate::http::read_response_head(&mut reader, &crate::HttpLimits::default())
        .await
        .expect("sse head");
    assert_eq!(head.status, 200);
    assert_eq!(
        head.header("access-control-allow-origin"),
        Some("https://console.example")
    );
    assert_eq!(head.header("vary"), Some("Origin"));
    assert_eq!(
        head.header("access-control-expose-headers"),
        Some("Mcp-Session-Id")
    );
    drop(reader);
    let _ = serving.await;

    let non_browser = exchange(
        &server,
        loopback(),
        "OPTIONS /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n".to_string(),
    )
    .await;
    assert_eq!(non_browser.status, 204);
    assert_eq!(non_browser.header("access-control-allow-origin"), None);
}

#[tokio::test]
async fn the_endpoint_refuses_what_it_does_not_serve() {
    let server = server(TestHost::answering());
    let elsewhere = exchange(
        &server,
        loopback(),
        "GET /admin HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n".to_string(),
    )
    .await;
    assert_eq!(elsewhere.status, 404);

    let put = exchange(
        &server,
        loopback(),
        "PUT /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n".to_string(),
    )
    .await;
    assert_eq!(put.status, 405);
    assert_eq!(put.header("allow"), Some("GET, POST, DELETE, OPTIONS"));

    let options = exchange(
        &server,
        loopback(),
        "OPTIONS /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n".to_string(),
    )
    .await;
    assert_eq!(options.status, 204);

    let batch = exchange(&server, loopback(), {
        let body = "[{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}]";
        format!(
            "POST /mcp HTTP/1.1\r\nhost: h\r\ncontent-type: application/json\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        )
    })
    .await;
    assert_eq!(batch.status, 400);
    assert_eq!(batch.error_code(), i64::from(crate::INVALID_REQUEST));

    let wrong_media = exchange(&server, loopback(), {
        let body = "{}";
        format!(
            "POST /mcp HTTP/1.1\r\nhost: h\r\ncontent-type: text/plain\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        )
    })
    .await;
    assert_eq!(wrong_media.status, 415);

    let notification = exchange(
        &server,
        loopback(),
        post(
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            None,
            &[],
        ),
    )
    .await;
    assert_eq!(notification.status, 202);

    let metadata = exchange(
        &server,
        loopback(),
        "GET /.well-known/oauth-protected-resource HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n".to_string(),
    )
    .await;
    assert_eq!(metadata.status, 404);
}

#[tokio::test]
async fn uninitialized_sessions_refuse_requests_and_event_streams_until_initialized() {
    let config = McpServerConfig {
        auth: AuthConfig {
            loopback_scope: Some(Scope::Manage),
            tokens: vec![TokenGrant {
                principal: "other-agent".to_string(),
                scope: Scope::Manage,
                secret: "other-secret".to_string(),
            }],
            ..AuthConfig::default()
        },
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());
    let session = initialize_unconfirmed(&server).await;

    let post_before = exchange(
        &server,
        loopback(),
        post(&call(2, "ping", json!({})), Some(&session), &[]),
    )
    .await;
    assert_eq!(post_before.status, 400);
    assert_eq!(post_before.error_code(), i64::from(crate::INVALID_REQUEST));

    let stream_before = exchange(&server, loopback(), {
        let mut request = String::from(
            "GET /mcp HTTP/1.1\r\nhost: h\r\naccept: text/event-stream\r\nconnection: close\r\n",
        );
        request.push_str(&format!("{SESSION_HEADER}: {session}\r\n\r\n"));
        request
    })
    .await;
    assert_eq!(stream_before.status, 400);

    let wrong_owner = exchange(
        &server,
        remote(),
        post(
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            Some(&session),
            &[("authorization", "Bearer other-secret")],
        ),
    )
    .await;
    assert_eq!(wrong_owner.status, 403);

    let wrong_version = exchange(
        &server,
        loopback(),
        post(
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            Some(&session),
            &[("mcp-protocol-version", "2024-11-05")],
        ),
    )
    .await;
    assert_eq!(wrong_version.status, 400);

    // An uninitialized session can still be explicitly terminated by its owner.
    let unconfirmed_to_delete = initialize_unconfirmed(&server).await;
    let deleted = exchange(
        &server,
        loopback(),
        format!(
            "DELETE /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n{SESSION_HEADER}: {unconfirmed_to_delete}\r\n\r\n"
        ),
    )
    .await;
    assert_eq!(deleted.status, 204);

    let confirmed = exchange(
        &server,
        loopback(),
        post(
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(confirmed.status, 202);

    let post_after = exchange(
        &server,
        loopback(),
        post(&call(3, "ping", json!({})), Some(&session), &[]),
    )
    .await;
    assert_eq!(post_after.status, 200);
    assert_eq!(post_after.result(), json!({}));
}

#[tokio::test]
async fn idle_ttl_and_lru_eviction_prevent_session_exhaustion_while_protecting_active_sessions() {
    // 1. Default 32-session cap evicts oldest abandoned idle sessions instead of
    // permanently locking out with HTTP 503.
    let server_32 = server(TestHost::answering());
    let mut created = Vec::new();
    for _ in 0..40 {
        created.push(initialize(&server_32).await);
    }
    assert_eq!(server_32.sessions().len(), 32);

    // Touch `created[8]` (currently the oldest surviving slot) so `created[9]` becomes oldest.
    let touched = exchange(
        &server_32,
        loopback(),
        post(&call(2, "ping", json!({})), Some(&created[8]), &[]),
    )
    .await;
    assert_eq!(touched.status, 200);

    let _forty_first = initialize(&server_32).await;
    assert_eq!(
        exchange(
            &server_32,
            loopback(),
            post(&call(3, "ping", json!({})), Some(&created[8]), &[]),
        )
        .await
        .status,
        200,
        "recently active session survives LRU eviction"
    );
    assert_eq!(
        exchange(
            &server_32,
            loopback(),
            post(&call(3, "ping", json!({})), Some(&created[9]), &[]),
        )
        .await
        .status,
        404,
        "oldest idle session was evicted"
    );

    // 2. Expired idle sessions reap on `initialize` and answer 404 on lookup.
    let ttl_config = McpServerConfig {
        sessions: crate::SessionLimits {
            max_sessions: 2,
            idle_ttl: Duration::from_millis(30),
            ..crate::SessionLimits::default()
        },
        ..McpServerConfig::default()
    };
    let ttl_server = McpServer::new(ttl_config, TestHost::answering());
    let expiring = initialize(&ttl_server).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        exchange(
            &ttl_server,
            loopback(),
            post(&call(2, "ping", json!({})), Some(&expiring), &[]),
        )
        .await
        .status,
        404
    );

    // 3. Active SSE streams and in-flight requests are protected from eviction;
    // abruptly disconnecting an SSE client immediately releases `stream_open`.
    let host = TestHost::new(Mode::Withhold);
    let active_config = McpServerConfig {
        sessions: crate::SessionLimits {
            max_sessions: 2,
            idle_ttl: Duration::from_millis(30),
            ..crate::SessionLimits::default()
        },
        request_timeout: Duration::from_secs(5),
        keep_alive_interval: Duration::from_secs(30),
        ..McpServerConfig::default()
    };
    let active_server = McpServer::new(active_config, host.clone());
    let sse_session = initialize(&active_server).await;
    let inflight_session = initialize(&active_server).await;

    let (mut sse_client, sse_service) = tokio::io::duplex(16 * 1024);
    let sse_task =
        tokio::spawn(Arc::clone(&active_server).handle_connection(sse_service, loopback()));
    sse_client
        .write_all(
            format!(
                "GET /mcp HTTP/1.1\r\nhost: h\r\naccept: text/event-stream\r\n{SESSION_HEADER}: {sse_session}\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .expect("write SSE GET");
    let head = read_head(&mut sse_client).await;
    assert!(head.starts_with(b"HTTP/1.1 200 OK"));

    let (mut inflight_client, inflight_service) = tokio::io::duplex(16 * 1024);
    let inflight_task =
        tokio::spawn(Arc::clone(&active_server).handle_connection(inflight_service, loopback()));
    inflight_client
        .write_all(
            post(
                &call(2, "tools/call", json!({ "name": "status" })),
                Some(&inflight_session),
                &[],
            )
            .as_bytes(),
        )
        .await
        .expect("write inflight call");
    while host.calls().is_empty() {
        tokio::task::yield_now().await;
    }

    // Wait past `idle_ttl`: neither active session may expire or be evicted!
    tokio::time::sleep(Duration::from_millis(50)).await;
    let saturated = exchange(
        &active_server,
        loopback(),
        post(
            &call(
                1,
                "initialize",
                json!({ "protocolVersion": PROTOCOL_VERSION, "capabilities": {} }),
            ),
            None,
            &[],
        ),
    )
    .await;
    assert_eq!(saturated.status, 503);
    assert!(saturated
        .body
        .contains("this server holds no more sessions"));

    // Simulate abrupt `Ctrl-C` on the SSE watcher without `DELETE`: `wait_for_peer_close`
    // closes `stream_open` immediately without waiting for the 30s keep-alive timer.
    drop(sse_client);
    timeout(Duration::from_secs(2), sse_task)
        .await
        .expect("SSE connection exits promptly on peer EOF")
        .expect("SSE task joins");

    // Now `sse_session` is idle, so a new `initialize` evicts it cleanly instead of 503.
    let replacement = initialize(&active_server).await;
    assert!(!replacement.is_empty());

    host.answer_withheld();
    drop(inflight_client);
    let _ = timeout(Duration::from_secs(2), inflight_task).await;
}

#[tokio::test]
async fn active_sse_streams_do_not_starve_short_lived_rpc_calls() {
    let config = McpServerConfig {
        max_connections: 2,
        sessions: crate::SessionLimits {
            max_sessions: 4,
            ..crate::SessionLimits::default()
        },
        keep_alive_interval: Duration::from_secs(30),
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let (stop, stopped) = oneshot::channel();
    let serving = tokio::spawn(Arc::clone(&server).serve(listener, async {
        let _ = stopped.await;
    }));

    let mut sessions = Vec::new();
    let mut sse_clients = Vec::new();
    for _ in 0..4 {
        let mut stream = TcpStream::connect(address).await.expect("connect");
        let init_req = post(
            &call(
                1,
                "initialize",
                json!({ "protocolVersion": PROTOCOL_VERSION, "capabilities": {} }),
            ),
            None,
            &[],
        );
        stream.write_all(init_req.as_bytes()).await.expect("init");
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await.expect("init reply");
        let init_resp = parse_response(&raw);
        assert_eq!(init_resp.status, 200);
        let session = init_resp
            .header(SESSION_HEADER)
            .expect("session id")
            .to_string();

        let mut ready_stream = TcpStream::connect(address).await.expect("connect ready");
        let ready_req = post(
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            Some(&session),
            &[],
        );
        ready_stream
            .write_all(ready_req.as_bytes())
            .await
            .expect("ready");
        let mut raw = Vec::new();
        ready_stream
            .read_to_end(&mut raw)
            .await
            .expect("ready reply");
        assert_eq!(parse_response(&raw).status, 202);

        let mut sse = TcpStream::connect(address).await.expect("connect sse");
        let get_req = format!(
            "GET /mcp HTTP/1.1\r\nhost: h\r\naccept: text/event-stream\r\n{SESSION_HEADER}: {session}\r\n\r\n"
        );
        sse.write_all(get_req.as_bytes()).await.expect("get sse");
        let head = read_head(&mut sse).await;
        assert!(head.starts_with(b"HTTP/1.1 200 OK"));
        sessions.push(session);
        sse_clients.push(sse);
    }

    // All 4 (`max_sessions`) SSE streams are open while `max_connections` is 2:
    // short-lived POST and DELETE requests must still succeed immediately!
    let mut rpc = TcpStream::connect(address).await.expect("connect rpc");
    let ping_req = post(&call(2, "ping", json!({})), Some(&sessions[0]), &[]);
    rpc.write_all(ping_req.as_bytes()).await.expect("ping");
    let mut raw = Vec::new();
    timeout(Duration::from_secs(2), rpc.read_to_end(&mut raw))
        .await
        .expect("RPC completes while SSE streams are active")
        .expect("read rpc");
    assert_eq!(parse_response(&raw).result(), json!({}));

    let mut delete_stream = TcpStream::connect(address).await.expect("connect delete");
    let delete_req = format!(
        "DELETE /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n{SESSION_HEADER}: {}\r\n\r\n",
        sessions[0]
    );
    delete_stream
        .write_all(delete_req.as_bytes())
        .await
        .expect("delete");
    let mut raw = Vec::new();
    timeout(Duration::from_secs(2), delete_stream.read_to_end(&mut raw))
        .await
        .expect("DELETE completes while SSE streams are active")
        .expect("read delete");
    assert_eq!(parse_response(&raw).status, 204);

    drop(sse_clients);
    let _ = stop.send(());
    let _ = timeout(Duration::from_secs(2), serving).await;
}

#[tokio::test]
async fn unauthenticated_or_bad_origin_expect_100_continue_does_not_continue_or_drain_body() {
    let config = McpServerConfig {
        auth: AuthConfig {
            loopback_scope: None,
            tokens: vec![TokenGrant {
                principal: "operator".to_string(),
                scope: Scope::Manage,
                secret: "s3cret".to_string(),
            }],
            allowed_origins: vec!["https://allowed.example".to_string()],
            ..AuthConfig::default()
        },
        transport_timeout: Duration::from_secs(5),
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());

    for (extra_header, expected_status) in [
        ("", 401),
        (
            "origin: https://evil.example\r\nauthorization: Bearer s3cret\r\n",
            403,
        ),
    ] {
        let (mut client, service) = tokio::io::duplex(4096);
        let handler = tokio::spawn(Arc::clone(&server).handle_connection(service, loopback()));
        let req = format!(
            "POST /mcp HTTP/1.1\r\nhost: h\r\ncontent-type: application/json\r\nexpect: 100-continue\r\ncontent-length: 65536\r\n{extra_header}\r\n"
        );
        // Deliberately send NO body bytes and keep `client` open: the server must reject
        // immediately without emitting `100 Continue` or waiting to drain 64 KiB.
        client.write_all(req.as_bytes()).await.expect("write head");

        let mut received = Vec::new();
        timeout(Duration::from_secs(1), client.read_to_end(&mut received))
            .await
            .expect("server rejects immediately without waiting for body")
            .expect("read refusal");
        let text = String::from_utf8_lossy(&received);
        assert!(
            !text.contains("100 Continue"),
            "server must not emit 100 Continue before auth/origin: {text}"
        );
        let parsed = parse_response(&received);
        assert_eq!(parsed.status, expected_status);
        timeout(Duration::from_secs(1), handler)
            .await
            .expect("handler finishes")
            .expect("join");
    }
}

#[tokio::test]
async fn per_ip_auth_failure_rate_limit_returns_429() {
    let config = McpServerConfig {
        auth: AuthConfig {
            loopback_scope: None,
            tokens: vec![TokenGrant {
                principal: "ops".to_string(),
                secret: "valid-secret".to_string(),
                scope: Scope::Command,
            }],
            allowed_origins: Vec::new(),
            ..AuthConfig::default()
        },
        transport_timeout: Duration::from_secs(5),
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());
    let peer = loopback();
    let bad_request = b"POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer wrong\r\nContent-Length: 2\r\n\r\n{}";

    for _ in 0..30 {
        let (mut client, server_stream) = tokio::io::duplex(4096);
        let srv = Arc::clone(&server);
        let handle = tokio::spawn(async move { srv.handle_connection(server_stream, peer).await });
        client.write_all(bad_request).await.expect("write");
        let mut reply = Vec::new();
        client.read_to_end(&mut reply).await.expect("read");
        handle.await.expect("join");
        let text = String::from_utf8_lossy(&reply);
        assert!(text.starts_with("HTTP/1.1 401"), "{text}");
    }

    // The 31st request from the same IP is rate-limited before authentication runs.
    let (mut client, server_stream) = tokio::io::duplex(4096);
    let srv = Arc::clone(&server);
    let handle = tokio::spawn(async move { srv.handle_connection(server_stream, peer).await });
    client.write_all(bad_request).await.expect("write");
    let mut reply = Vec::new();
    client.read_to_end(&mut reply).await.expect("read");
    handle.await.expect("join");
    let text = String::from_utf8_lossy(&reply);
    assert!(text.starts_with("HTTP/1.1 429"), "{text}");
    assert!(text.to_ascii_lowercase().contains("retry-after:"), "{text}");
}

#[tokio::test]
async fn zero_max_operation_records_completes_without_infinite_eviction_loop() {
    let config = McpServerConfig {
        max_operation_records: 0,
        ..McpServerConfig::default()
    };
    let server = McpServer::new(config, TestHost::answering());
    let session = initialize(&server).await;
    let response = exchange(
        &server,
        loopback(),
        post(
            &call(2, "tools/call", json!({ "name": "status" })),
            Some(&session),
            &[],
        ),
    )
    .await;
    assert_eq!(response.status, 200);
    assert_eq!(response.result()["isError"], false);
}
