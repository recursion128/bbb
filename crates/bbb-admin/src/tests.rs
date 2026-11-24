//! The client is exercised against the real endpoint, not against a mock of
//! it: a management client whose correctness rests on a hand-written server
//! stub proves only that the stub and the client agree.

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use bbb_mcp::{
    control::OPERATION_URI_PREFIX, ResourceChange, PROTOCOL_VERSION, RESOURCE_NOT_FOUND,
};
use bbb_server_admin::{
    catalog, resources, AdminCall, AdminGateway, AuthConfig, GatewayError, McpServer,
    McpServerConfig, PendingOperation, TokenGrant,
};
use bbb_server_core::admin::{AdminError, AdminOutcome, AdminResult, AdminScope, PlayerRef};
use clap::Parser;
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{broadcast, oneshot},
    task::JoinHandle,
    time::timeout,
};

use crate::{
    cli::{build_arguments, read_token},
    client::{read_answer, tls_server_name, AdminClient, ClientConfiguration, ClientError},
    endpoint_url, run, Cli, CliError, Outcome,
};

/// A gateway that answers every submitted call from the request itself, so a
/// test states what the world would decide rather than what the transport
/// should return.
struct TestGateway {
    next_operation: AtomicU64,
    calls: Mutex<Vec<AdminCall>>,
    changes: broadcast::Sender<ResourceChange>,
}

impl TestGateway {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            next_operation: AtomicU64::new(1),
            calls: Mutex::new(Vec::new()),
            changes: broadcast::channel(16).0,
        })
    }

    fn calls(&self) -> Vec<AdminCall> {
        self.calls.lock().expect("calls").clone()
    }

    fn outcome(operation_id: u64, call: &AdminCall) -> AdminOutcome {
        let required = call.request.required_scope();
        let result = if call.scope < required {
            Err(AdminError::ScopeDenied {
                required,
                granted: call.scope,
            })
        } else {
            Ok(AdminResult::Players(vec![PlayerRef::by_name("bbb-client")]))
        };
        AdminOutcome {
            operation_id,
            request_id: call.request_id.clone(),
            principal: call.principal.clone(),
            scope: call.scope,
            action: call.request.action(),
            tick: 42,
            replayed: false,
            result,
        }
    }
}

impl AdminGateway for TestGateway {
    fn submit(&self, call: AdminCall) -> Result<PendingOperation, GatewayError> {
        let operation_id = self.next_operation.fetch_add(1, Ordering::Relaxed);
        self.calls.lock().expect("calls").push(call.clone());
        let (sender, receiver) = oneshot::channel();
        let _ = sender.send(Self::outcome(operation_id, &call));
        Ok(PendingOperation {
            operation_id,
            answer: receiver,
        })
    }

    fn now(&self) -> i64 {
        1_755_000_000_000
    }

    fn subscribe(&self) -> broadcast::Receiver<ResourceChange> {
        self.changes.subscribe()
    }
}

struct Endpoint {
    address: SocketAddr,
    gateway: Arc<TestGateway>,
    shutdown: Option<oneshot::Sender<()>>,
    served: Option<JoinHandle<()>>,
}

impl Endpoint {
    async fn start(auth: AuthConfig) -> Self {
        let gateway = TestGateway::new();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let config = McpServerConfig {
            auth,
            ..McpServerConfig::default()
        };
        let server = McpServer::new(config, Arc::clone(&gateway) as Arc<dyn AdminGateway>);
        let (shutdown, wait) = oneshot::channel();
        let served = tokio::spawn(server.serve(listener, async {
            let _ = wait.await;
        }));
        Self {
            address,
            gateway,
            shutdown: Some(shutdown),
            served: Some(served),
        }
    }

    async fn loopback() -> Self {
        Self::start(AuthConfig::default()).await
    }

    fn url(&self) -> String {
        endpoint_url(self.address, "/mcp", false)
    }

    async fn stop(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(served) = self.served.take() {
            let _ = timeout(Duration::from_secs(5), served).await;
        }
    }
}

/// Runs the command line the way the binary does, and returns what it printed.
async fn invoke(endpoint: &Endpoint, arguments: &[&str]) -> Result<(Outcome, String), CliError> {
    let url = endpoint.url();
    let mut line = vec!["bbb-admin", "--endpoint", url.as_str()];
    line.extend_from_slice(arguments);
    let cli = Cli::parse_from(line);
    let mut out = Vec::new();
    let outcome = timeout(Duration::from_secs(10), run(cli, &mut out))
        .await
        .expect("the command answers")?;
    Ok((outcome, String::from_utf8(out).expect("utf-8")))
}

#[tokio::test]
async fn a_command_reaches_the_endpoint_and_prints_what_the_world_answered() {
    let endpoint = Endpoint::loopback().await;
    let (outcome, printed) = invoke(&endpoint, &["status"]).await.expect("status");
    assert_eq!(outcome, Outcome::Answered);
    let answer: Value = serde_json::from_str(&printed).expect("the answer is JSON");
    assert_eq!(answer["players"][0]["name"], "bbb-client");

    let calls = endpoint.gateway.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].principal, "loopback");
    assert_eq!(calls[0].scope, AdminScope::Manage);
    endpoint.stop().await;
}

#[tokio::test]
async fn the_listing_is_the_endpoint_s_own_catalog_and_not_a_second_copy_of_it() {
    let endpoint = Endpoint::loopback().await;
    let (_, printed) = invoke(&endpoint, &["tools"]).await.expect("tools");
    for tool in catalog() {
        assert!(
            printed.contains(tool.name),
            "{} is missing from the listing",
            tool.name
        );
    }
    // The scope and the read/write mark come from the endpoint's annotations,
    // so a tool that changed authority changes this output without anyone
    // editing the client.
    assert!(printed.contains("server.status  [read: read]"));
    assert!(printed.contains("command.execute  [command: write, destructive]"));

    let (_, printed) = invoke(&endpoint, &["resources"]).await.expect("resources");
    for resource in resources() {
        assert!(
            printed.contains(resource.uri),
            "{} is missing",
            resource.uri
        );
    }
    endpoint.stop().await;
}

#[tokio::test]
async fn a_tool_is_called_with_the_arguments_the_command_line_named() {
    let endpoint = Endpoint::loopback().await;
    let (outcome, _) = invoke(
        &endpoint,
        &["call", "allowlist.add", r#"add=[{"name":"alice"}]"#],
    )
    .await
    .expect("call");
    assert_eq!(outcome, Outcome::Answered);

    let calls = endpoint.gateway.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].request.action(), "allowlist.add");
    endpoint.stop().await;
}

#[tokio::test]
async fn a_refusal_the_world_decided_is_an_answer_and_not_a_transport_failure() {
    let endpoint = Endpoint::start(AuthConfig {
        loopback_scope: Some(AdminScope::Read),
        ..AuthConfig::default()
    })
    .await;
    let (outcome, printed) = invoke(&endpoint, &["call", "allowlist.clear"])
        .await
        .expect("the refusal still answers");
    assert_eq!(outcome, Outcome::Refused);
    assert_eq!(outcome.exit_code(), 1);
    // The refusal text went to stderr rather than to the answer stream, so a
    // caller piping the output never mistakes it for state.
    assert!(printed.is_empty(), "printed {printed:?}");
    endpoint.stop().await;
}

#[tokio::test]
async fn a_resource_read_prints_the_document_the_endpoint_produced() {
    let endpoint = Endpoint::loopback().await;
    let (outcome, printed) = invoke(&endpoint, &["read", "bbb://server/players"])
        .await
        .expect("read");
    assert_eq!(outcome, Outcome::Answered);
    let answer: Value = serde_json::from_str(&printed).expect("the document is JSON");
    assert_eq!(answer["players"][0]["name"], "bbb-client");

    let refused = invoke(&endpoint, &["read", "bbb://server/nothing"]).await;
    let Err(CliError::Client(ClientError::Rpc { code, .. })) = refused else {
        panic!("an undeclared resource is a protocol error");
    };
    assert_eq!(code, i64::from(RESOURCE_NOT_FOUND));
    endpoint.stop().await;
}

#[tokio::test]
async fn an_operation_record_is_readable_by_the_id_the_call_reported() {
    let endpoint = Endpoint::loopback().await;
    // A record belongs to a session, so the same client has to submit and then
    // read; that is what `--raw` on one call plus a second command cannot do.
    let mut client = AdminClient::new(ClientConfiguration {
        endpoint: endpoint.url(),
        ..ClientConfiguration::default()
    })
    .expect("client");
    client.connect().await.expect("handshake");
    let result = client
        .call_tool("server.status", serde_json::json!({}), Some("r-42"))
        .await
        .expect("call");
    let operation_id = result["_meta"]["bbb/operationId"].as_u64().expect("id");
    let record = client
        .read_resource(&format!("{OPERATION_URI_PREFIX}{operation_id}"))
        .await
        .expect("record");
    let document: Value =
        serde_json::from_str(record["contents"][0]["text"].as_str().expect("text")).expect("json");
    assert_eq!(document["state"], "applied");
    assert_eq!(document["requestId"], "r-42");
    client.disconnect().await.expect("disconnect");

    // The caller's own id is what the gateway saw, so a retry carrying it is
    // recognisable as the same operation.
    assert_eq!(endpoint.gateway.calls()[0].request_id, "r-42");
    endpoint.stop().await;
}

#[tokio::test]
async fn watching_prints_each_change_the_endpoint_announced() {
    let endpoint = Endpoint::loopback().await;
    let gateway = Arc::clone(&endpoint.gateway);
    let url = endpoint.url();
    let watching = tokio::spawn(async move {
        let cli = Cli::parse_from([
            "bbb-admin",
            "--endpoint",
            &url,
            "watch",
            "bbb://server/players",
            "--events",
            "1",
        ]);
        let mut out = Vec::new();
        let outcome = run(cli, &mut out).await.expect("watch");
        (outcome, String::from_utf8(out).expect("utf-8"))
    });

    // The subscription has to exist before the change is published, and there
    // is no way to observe that from outside, so the announcement repeats until
    // the watcher has it.
    let (outcome, printed) = loop {
        let _ = gateway.changes.send(ResourceChange {
            uri: "bbb://server/players",
        });
        if watching.is_finished() {
            break watching.await.expect("the watcher finishes");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    assert_eq!(outcome, Outcome::Answered);
    let event: Value = serde_json::from_str(printed.trim()).expect("the event is JSON");
    assert_eq!(event["method"], "notifications/resources/updated");
    assert_eq!(event["params"]["uri"], "bbb://server/players");
    endpoint.stop().await;
}

#[tokio::test]
async fn a_secret_travels_in_a_file_and_an_endpoint_that_needs_one_refuses_without_it() {
    let endpoint = Endpoint::start(AuthConfig {
        loopback_scope: None,
        tokens: vec![TokenGrant {
            principal: "operator".to_string(),
            scope: AdminScope::Manage,
            secret: "s3cret-token".to_string(),
        }],
        ..AuthConfig::default()
    })
    .await;

    let refused = invoke(&endpoint, &["status"]).await;
    let Err(CliError::Client(ClientError::Http { status, .. })) = refused else {
        panic!("an endpoint that requires a token refuses without one");
    };
    assert_eq!(status, 401);

    let directory = std::env::temp_dir().join("bbb-admin-token-test");
    std::fs::create_dir_all(&directory).expect("directory");
    let token_file = directory.join("token");
    std::fs::write(&token_file, "# the operator's secret\ns3cret-token\n").expect("write");
    let path = token_file.display().to_string();
    let (outcome, _) = invoke(&endpoint, &["--token-file", path.as_str(), "status"])
        .await
        .expect("status");
    assert_eq!(outcome, Outcome::Answered);
    assert_eq!(endpoint.gateway.calls()[0].principal, "operator");
    let _ = std::fs::remove_file(&token_file);
    endpoint.stop().await;
}

#[tokio::test]
async fn the_client_speaks_one_protocol_revision_and_says_so() {
    let endpoint = Endpoint::loopback().await;
    let (_, printed) = invoke(&endpoint, &["hello"]).await.expect("hello");
    let hello: Value = serde_json::from_str(&printed).expect("json");
    assert_eq!(hello["protocolVersion"], PROTOCOL_VERSION);
    assert_eq!(hello["_meta"]["bbb/scope"], "manage");
    endpoint.stop().await;
}

#[test]
fn an_endpoint_that_is_not_http_is_refused_before_a_socket_is_opened() {
    for endpoint in [
        "ws://127.0.0.1:25585/mcp",
        "not a url",
        "file:///etc/passwd",
    ] {
        let built = AdminClient::new(ClientConfiguration {
            endpoint: endpoint.to_string(),
            ..ClientConfiguration::default()
        });
        assert!(
            matches!(built, Err(ClientError::Endpoint(_, _))),
            "{endpoint} should not be a management endpoint"
        );
    }
}

#[test]
fn https_without_a_named_authority_is_refused_rather_than_trusting_nothing_quietly() {
    let built = AdminClient::new(ClientConfiguration {
        endpoint: "https://example.invalid/mcp".to_string(),
        ..ClientConfiguration::default()
    });
    assert!(matches!(built, Err(ClientError::Tls(_))));
}

#[test]
fn an_argument_is_json_when_it_parses_as_json_and_a_string_otherwise() {
    let arguments = build_arguments(
        Some(r#"{"flush":true}"#),
        &[
            "gamerule=minecraft:advance_time".to_string(),
            "value=false".to_string(),
            "add=[{\"name\":\"alice\"}]".to_string(),
        ],
    )
    .expect("arguments");
    assert_eq!(arguments["flush"], Value::Bool(true));
    assert_eq!(arguments["gamerule"], "minecraft:advance_time");
    assert_eq!(arguments["value"], Value::Bool(false));
    assert_eq!(arguments["add"][0]["name"], "alice");

    assert!(matches!(
        build_arguments(None, &["players".to_string()]),
        Err(CliError::Argument(_))
    ));
    assert!(matches!(
        build_arguments(Some("[1]"), &[]),
        Err(CliError::ArgumentsJson(_))
    ));
}

#[test]
fn a_token_file_with_nothing_in_it_is_an_error_and_not_an_anonymous_request() {
    let directory = std::env::temp_dir().join("bbb-admin-empty-token-test");
    std::fs::create_dir_all(&directory).expect("directory");
    let path = directory.join("token");
    std::fs::write(&path, "# only a comment\n\n").expect("write");
    assert!(matches!(
        read_token(Some(&path)),
        Err(CliError::EmptyToken(_))
    ));
    let _ = std::fs::remove_file(&path);
    assert_eq!(read_token(None).expect("no file"), None);
}

#[tokio::test]
async fn watch_honors_head_timeout_when_sse_response_headers_stall() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.expect("accept");
        let (read_half, mut write_half) = socket.into_split();
        let mut reader = tokio::io::BufReader::new(read_half);
        let mut continue_sink = Vec::new();
        let limits = bbb_mcp::HttpLimits::default();

        // 1. initialize
        let _ = bbb_mcp::read_request(&mut reader, &mut continue_sink, &limits).await;
        let init_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "serverInfo": { "name": "stub", "version": "0.1.0" },
            }
        })
        .to_string();
        let init_reply = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nmcp-session-id: s-stall\r\ncontent-length: {}\r\n\r\n{}",
            init_body.len(),
            init_body
        );
        write_half
            .write_all(init_reply.as_bytes())
            .await
            .expect("write init");

        // 2. notifications/initialized
        let _ = bbb_mcp::read_request(&mut reader, &mut continue_sink, &limits).await;
        write_half
            .write_all(b"HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n")
            .await
            .expect("write initialized");

        // 3. resources/subscribe
        let _ = bbb_mcp::read_request(&mut reader, &mut continue_sink, &limits).await;
        let sub_body = r#"{"jsonrpc":"2.0","id":2,"result":{}}"#;
        let sub_reply = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            sub_body.len(),
            sub_body
        );
        write_half
            .write_all(sub_reply.as_bytes())
            .await
            .expect("write subscribe");

        // 4. GET SSE stream: send incomplete header block without \r\n\r\n and hold open.
        let _ = bbb_mcp::read_request(&mut reader, &mut continue_sink, &limits).await;
        write_half
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n")
            .await
            .expect("write partial head");
        write_half.flush().await.expect("flush partial head");
        tokio::time::sleep(Duration::from_secs(5)).await;
    });

    let mut client = AdminClient::new(ClientConfiguration {
        endpoint: endpoint_url(address, "/mcp", false),
        timeout: Duration::from_millis(150),
        ..ClientConfiguration::default()
    })
    .expect("client");
    client.connect().await.expect("handshake");

    let started = std::time::Instant::now();
    let watched = client
        .watch(&["bbb://server/players".to_string()], |_| false)
        .await;
    let elapsed = started.elapsed();
    let Err(ClientError::Transport(message)) = watched else {
        panic!("stalled SSE header block must fail with transport timeout, got {watched:?}");
    };
    assert!(
        message.contains("deadline") || message.contains("timed out"),
        "unexpected error message: {message}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "SSE header timeout took too long: {elapsed:?}"
    );
    peer.abort();
}

#[tokio::test]
async fn responses_frame_by_content_length_without_waiting_for_eof_or_mangling_non_utf8() {
    // 1. Direct byte-level check: non-UTF-8 body + trailing bytes on a half-open stream.
    let (mut client_side, mut server_side) = tokio::io::duplex(1024);
    let raw_body: [u8; 6] = [0xff, 0x00, 0xfe, b'b', b'a', b'd'];
    let mut wire = format!(
        "HTTP/1.1 502 Bad Gateway\r\ncontent-type: application/octet-stream\r\nconnection: keep-alive\r\ncontent-length: {}\r\n\r\n",
        raw_body.len()
    )
    .into_bytes();
    wire.extend_from_slice(&raw_body);
    wire.extend_from_slice(&[0xaa, 0xbb, 0xcc]);
    server_side.write_all(&wire).await.expect("write wire");

    // `server_side` stays open (no EOF); `read_answer` must return immediately with exact bytes.
    let answer = read_answer(&mut client_side, Duration::from_millis(200))
        .await
        .expect("framed response");
    assert_eq!(answer.status, 502);
    assert_eq!(answer.body, raw_body);
    drop(server_side);

    // 2. End-to-end `AdminClient` check against a TCP peer that holds sockets open without EOF.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let peer = tokio::spawn(async move {
        let mut held_sockets = Vec::new();
        for step in 0..2 {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut scratch = [0u8; 2048];
            let _ = socket.read(&mut scratch).await;
            if step == 0 {
                let body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {},
                        "serverInfo": { "name": "keep-alive-peer", "version": "0.1.0" },
                    }
                })
                .to_string();
                let mut reply = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nconnection: close\r\nmcp-session-id: s-keep\r\ncontent-length: {}\r\n\r\n{}",
                    body.len(),
                    body
                )
                .into_bytes();
                reply.extend_from_slice(&[0xff, 0xfe]);
                socket.write_all(&reply).await.expect("write init");
            } else {
                socket
                    .write_all(b"HTTP/1.1 202 Accepted\r\nconnection: close\r\ncontent-length: 0\r\n\r\n\xff")
                    .await
                    .expect("write initialized");
            }
            socket.flush().await.expect("flush");
            held_sockets.push(socket);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
        drop(held_sockets);
    });

    let mut client = AdminClient::new(ClientConfiguration {
        endpoint: endpoint_url(address, "/mcp", false),
        timeout: Duration::from_millis(300),
        ..ClientConfiguration::default()
    })
    .expect("client");
    let hello = client
        .connect()
        .await
        .expect("connect without waiting for EOF");
    assert_eq!(hello["serverInfo"]["name"], "keep-alive-peer");
    peer.abort();
}

#[test]
fn bracketed_ipv6_hosts_strip_brackets_for_tls_server_name() {
    let parsed = tls_server_name("[::1]").expect("bracketed IPv6 parses as ServerName");
    assert_eq!(
        parsed,
        rustls::pki_types::ServerName::IpAddress(
            std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST).into()
        )
    );
    assert!(matches!(
        tls_server_name("localhost"),
        Ok(rustls::pki_types::ServerName::DnsName(_))
    ));

    let ca_pem = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/fixtures/mcp-tls/ca.crt"),
    )
    .expect("fixture ca");
    let authorities = bbb_mcp::pem_blocks(&ca_pem, "CERTIFICATE").expect("ca blocks");
    let client = AdminClient::new(ClientConfiguration {
        endpoint: "https://[::1]:25585/mcp".to_string(),
        certificate_authorities: authorities,
        ..ClientConfiguration::default()
    });
    assert!(client.is_ok(), "AdminClient accepts https://[::1]");
}

#[tokio::test]
async fn watch_times_out_silent_sse_streams_and_survives_keep_alives() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let peer = tokio::spawn(async move {
        let limits = bbb_mcp::HttpLimits::default();
        for mode in ["silent", "keepalive"] {
            let (socket, _) = listener.accept().await.expect("accept");
            tokio::spawn(async move {
                let (read_half, mut write_half) = socket.into_split();
                let mut reader = tokio::io::BufReader::new(read_half);
                let mut sink = Vec::new();
                let _ = bbb_mcp::read_request(&mut reader, &mut sink, &limits).await;
                let init_body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": { "protocolVersion": PROTOCOL_VERSION, "capabilities": {} }
                })
                .to_string();
                write_half
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nmcp-session-id: s-{mode}\r\ncontent-length: {}\r\n\r\n{}",
                            init_body.len(),
                            init_body
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();

                let _ = bbb_mcp::read_request(&mut reader, &mut sink, &limits).await;
                write_half
                    .write_all(b"HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n")
                    .await
                    .unwrap();

                let _ = bbb_mcp::read_request(&mut reader, &mut sink, &limits).await;
                let sub_body = r#"{"jsonrpc":"2.0","id":2,"result":{}}"#;
                write_half
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                            sub_body.len(),
                            sub_body
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();

                let _ = bbb_mcp::read_request(&mut reader, &mut sink, &limits).await;
                write_half
                    .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n")
                    .await
                    .unwrap();
                if mode == "silent" {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                } else {
                    for _ in 0..4 {
                        tokio::time::sleep(Duration::from_millis(40)).await;
                        write_half.write_all(b": keep-alive\n\n").await.unwrap();
                        write_half.flush().await.unwrap();
                    }
                    write_half
                        .write_all(b"event: message\ndata: {\"ok\":true}\n\n")
                        .await
                        .unwrap();
                    write_half.flush().await.unwrap();
                }
            });
        }
    });

    // 1. Silent dead SSE stream triggers read idle timeout promptly.
    let mut silent_client = AdminClient::new(ClientConfiguration {
        endpoint: endpoint_url(address, "/mcp", false),
        timeout: Duration::from_millis(120),
        ..ClientConfiguration::default()
    })
    .unwrap();
    silent_client.connect().await.unwrap();
    let err = silent_client
        .watch(&["bbb://server/players".to_string()], |_| true)
        .await
        .expect_err("silent SSE stream must time out");
    assert!(matches!(err, ClientError::Transport(_)));

    // 2. Keep-alives arriving inside `timeout` keep `watch` alive beyond `timeout`.
    let mut live_client = AdminClient::new(ClientConfiguration {
        endpoint: endpoint_url(address, "/mcp", false),
        timeout: Duration::from_millis(120),
        ..ClientConfiguration::default()
    })
    .unwrap();
    live_client.connect().await.unwrap();
    let mut seen = Vec::new();
    live_client
        .watch(&["bbb://server/players".to_string()], |val| {
            seen.push(val);
            false
        })
        .await
        .expect("keep-alives keep stream alive until event");
    assert_eq!(seen, vec![serde_json::json!({ "ok": true })]);
    peer.abort();
}

#[tokio::test]
async fn initialize_tool_call_and_disconnect_reuse_one_keep_alive_connection() {
    let gateway = TestGateway::new();
    let server = McpServer::new(
        McpServerConfig::default(),
        Arc::clone(&gateway) as Arc<dyn AdminGateway>,
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let accepted_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = Arc::clone(&accepted_count);
    let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
    let serve_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                accepted = listener.accept() => {
                    let Ok((stream, peer)) = accepted else { break };
                    counter.fetch_add(1, Ordering::Relaxed);
                    let server = Arc::clone(&server);
                    tokio::spawn(async move {
                        server.handle_connection(stream, peer).await;
                    });
                }
            }
        }
    });

    let mut client = AdminClient::new(ClientConfiguration {
        endpoint: endpoint_url(address, "/mcp", false),
        ..ClientConfiguration::default()
    })
    .expect("client");
    client.connect().await.expect("connect");
    let status = client
        .call_tool("server.status", serde_json::json!({}), None)
        .await
        .expect("status");
    assert_eq!(status["isError"], false);
    client.disconnect().await.expect("disconnect");

    assert_eq!(
        accepted_count.load(Ordering::Relaxed),
        1,
        "initialize -> notifications/initialized -> tools/call -> DELETE must reuse one TCP connection"
    );
    let _ = stop_tx.send(());
    let _ = timeout(Duration::from_secs(2), serve_task).await;
}
