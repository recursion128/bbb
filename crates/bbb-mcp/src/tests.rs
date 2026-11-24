use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, BufReader};

use crate::{
    http::{read_request, write_response, EventStream, HttpLimits, HttpMethod, HttpResponse},
    jsonrpc::{
        encode_error, encode_notification, encode_result, parse_message, JsonRpcError,
        JsonRpcMessage, RequestId, INVALID_PARAMS, INVALID_REQUEST, PARSE_ERROR,
    },
};

async fn parse_http(bytes: &[u8]) -> Result<Option<crate::http::HttpRequest>, crate::HttpError> {
    let mut reader = BufReader::new(bytes);
    let mut writer = Vec::new();
    read_request(&mut reader, &mut writer, &HttpLimits::default()).await
}

#[tokio::test]
async fn a_post_with_a_body_is_read_whole() {
    let request = parse_http(
        b"POST /mcp?session=1 HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: application/json, text/event-stream\r\nContent-Length: 2\r\n\r\n{}",
    )
    .await
    .expect("read")
    .expect("request");
    assert_eq!(request.method, HttpMethod::Post);
    assert_eq!(request.path, "/mcp");
    assert_eq!(request.query.as_deref(), Some("session=1"));
    assert_eq!(request.body, b"{}");
    assert!(request.keep_alive);
    assert_eq!(request.headers.get("HOST"), Some("127.0.0.1"));
    assert!(request.headers.accepts("text/event-stream"));
    assert!(request.headers.accepts("application/json"));
    assert!(!request.headers.accepts("text/html"));
}

#[tokio::test]
async fn an_absent_accept_admits_everything_and_a_wildcard_matches_a_subtype() {
    let request = parse_http(b"GET /mcp HTTP/1.1\r\nHost: h\r\n\r\n")
        .await
        .expect("read")
        .expect("request");
    assert!(request.headers.accepts("text/event-stream"));
    let request = parse_http(b"GET /mcp HTTP/1.1\r\nAccept: text/*;q=0.9\r\n\r\n")
        .await
        .expect("read")
        .expect("request");
    assert!(request.headers.accepts("text/event-stream"));
    assert!(!request.headers.accepts("application/json"));
}

#[tokio::test]
async fn a_clean_close_between_requests_is_not_a_failure() {
    assert!(parse_http(b"").await.expect("read").is_none());
}

#[tokio::test]
async fn http_1_0_closes_unless_keep_alive_is_asked_for() {
    let request = parse_http(b"GET /mcp HTTP/1.0\r\n\r\n")
        .await
        .expect("read")
        .expect("request");
    assert!(!request.keep_alive);
    let request = parse_http(b"GET /mcp HTTP/1.0\r\nConnection: keep-alive\r\n\r\n")
        .await
        .expect("read")
        .expect("request");
    assert!(request.keep_alive);
    let request = parse_http(b"GET /mcp HTTP/1.1\r\nConnection: close\r\n\r\n")
        .await
        .expect("read")
        .expect("request");
    assert!(!request.keep_alive);
}

#[tokio::test]
async fn every_refusal_carries_its_own_status() {
    let cases: [(&[u8], u16); 8] = [
        (b"GET /mcp\r\n\r\n", 400),
        (b"GET /mcp HTTP/2.0\r\n\r\n", 505),
        (
            b"POST /mcp HTTP/1.1\r\nTransfer-Encoding: gzip\r\n\r\n",
            411,
        ),
        (
            b"POST /mcp HTTP/1.1\r\nContent-Length: 4\r\nContent-Length: 4\r\n\r\nabcd",
            400,
        ),
        (b"POST /mcp HTTP/1.1\r\nContent-Length: nine\r\n\r\n", 400),
        (b"POST /mcp HTTP/1.1\r\nContent-Length: 8\r\n\r\nab", 400),
        (b"GET /mcp HTTP/1.1\r\n\tfolded: yes\r\n\r\n", 400),
        (b"GET /mcp HTTP/1.1\r\nno-colon\r\n\r\n", 400),
    ];
    for (bytes, status) in cases {
        let error = parse_http(bytes)
            .await
            .err()
            .unwrap_or_else(|| panic!("{} must be refused", String::from_utf8_lossy(bytes)));
        assert_eq!(error.status(), status, "{}", String::from_utf8_lossy(bytes));
    }
}

#[tokio::test]
async fn the_transport_bounds_are_enforced_before_the_body_is_buffered() {
    let limits = HttpLimits {
        max_request_line_bytes: 64,
        max_header_bytes: 128,
        max_headers: 2,
        max_body_bytes: 8,
    };
    async fn refuse(bytes: Vec<u8>, limits: &HttpLimits) -> u16 {
        let mut reader = BufReader::new(bytes.as_slice());
        let mut writer = Vec::new();
        read_request(&mut reader, &mut writer, limits)
            .await
            .expect_err("refusal")
            .status()
    }
    let long_target = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(128));
    assert_eq!(refuse(long_target.into_bytes(), &limits).await, 414);
    let long_header = format!("GET /mcp HTTP/1.1\r\nx: {}\r\n\r\n", "a".repeat(256));
    assert_eq!(refuse(long_header.into_bytes(), &limits).await, 431);
    let many_headers = b"GET /mcp HTTP/1.1\r\na: 1\r\nb: 2\r\nc: 3\r\n\r\n".to_vec();
    assert_eq!(refuse(many_headers, &limits).await, 431);
    let large_body = b"POST /mcp HTTP/1.1\r\nContent-Length: 9\r\n\r\n123456789".to_vec();
    assert_eq!(refuse(large_body, &limits).await, 413);
}

#[tokio::test]
async fn a_hundred_continue_precedes_the_body_it_unblocks() {
    let mut reader = BufReader::new(
        b"POST /mcp HTTP/1.1\r\nExpect: 100-continue\r\nContent-Length: 2\r\n\r\n{}".as_slice(),
    );
    let mut writer = Vec::new();
    let request = read_request(&mut reader, &mut writer, &HttpLimits::default())
        .await
        .expect("read")
        .expect("request");
    assert_eq!(request.body, b"{}");
    assert_eq!(writer, b"HTTP/1.1 100 Continue\r\n\r\n");
}

#[tokio::test]
async fn a_response_always_declares_its_length_and_its_connection() {
    let mut sink = Vec::new();
    write_response(
        &mut sink,
        &HttpResponse::json(200, b"{\"ok\":true}".to_vec()).with_header("mcp-session-id", "s-1"),
        true,
        true,
    )
    .await
    .expect("write");
    let written = String::from_utf8(sink).expect("utf-8");
    assert!(written.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(written.contains("content-type: application/json\r\n"));
    assert!(written.contains("mcp-session-id: s-1\r\n"));
    assert!(written.contains("content-length: 11\r\n"));
    assert!(written.contains("connection: keep-alive\r\n"));
    assert!(written.ends_with("\r\n\r\n{\"ok\":true}"));

    let mut sink = Vec::new();
    write_response(&mut sink, &HttpResponse::new(204), false, false)
        .await
        .expect("write");
    let written = String::from_utf8(sink).expect("utf-8");
    assert!(written.starts_with("HTTP/1.1 204 No Content\r\n"));
    assert!(!written.contains("content-length:"));
    assert!(written.contains("connection: close\r\n"));
}

#[tokio::test]
async fn an_event_stream_frames_one_data_line_per_payload_line() {
    let (client, server) = tokio::io::duplex(1024);
    let mut stream =
        EventStream::start(server, &[("mcp-session-id".to_string(), "s-1".to_string())])
            .await
            .expect("start");
    stream.send(Some(7), "{\"a\":1}").await.expect("send");
    stream.send(None, "line one\nline two").await.expect("send");
    stream.send(None, "cr\rline").await.expect("send");
    stream.keep_alive().await.expect("keep alive");
    drop(stream);

    let mut received = Vec::new();
    let mut client = client;
    client.read_to_end(&mut received).await.expect("read");
    let received = String::from_utf8(received).expect("utf-8");
    assert!(received.starts_with("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n"));
    assert!(received.contains("mcp-session-id: s-1\r\n"));
    assert!(received.contains("id: 7\nevent: message\ndata: {\"a\":1}\n\n"));
    assert!(received.contains("event: message\ndata: line one\ndata: line two\n\n"));
    assert!(received.contains("event: message\ndata: cr\ndata: line\n\n"));
    assert!(received.ends_with(": keep-alive\n\n"));
}

#[tokio::test]
async fn response_header_serialization_rejects_crlf_and_malformed_names() {
    let mut sink = Vec::new();
    let crlf_value = HttpResponse::new(200).with_header("x-test", "ok\r\nX-Injected: bad");
    let err = write_response(&mut sink, &crlf_value, false, true)
        .await
        .expect_err("CRLF in header value must be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(sink.is_empty());

    let lf_name = HttpResponse::new(200).with_header("x-bad\nname", "value");
    let err = write_response(&mut sink, &lf_name, false, true)
        .await
        .expect_err("newline in header name must be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(sink.is_empty());

    let (_client, server) = tokio::io::duplex(256);
    let err = EventStream::start(
        server,
        &[("x-origin".to_string(), "https://ok\r\nevil: 1".to_string())],
    )
    .await
    .err()
    .expect("CRLF in SSE header value must be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn a_batch_is_refused_because_the_pinned_revision_removed_it() {
    let error = parse_message(b"[{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}]")
        .expect_err("batches are refused");
    assert_eq!(error.code, INVALID_REQUEST);
}

#[test]
fn the_envelope_decides_between_a_request_a_notification_and_an_answer() {
    let request = parse_message(b"{\"jsonrpc\":\"2.0\",\"id\":\"a\",\"method\":\"tools/list\"}")
        .expect("request");
    let JsonRpcMessage::Request(request) = request else {
        panic!("a request with an id is a request");
    };
    assert_eq!(request.id, RequestId::Text("a".to_string()));
    assert_eq!(request.method, "tools/list");
    assert_eq!(request.params, json!({}));

    let notification =
        parse_message(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}")
            .expect("notification");
    assert!(!notification.expects_answer());
    assert!(matches!(notification, JsonRpcMessage::Notification(_)));

    let answer = parse_message(b"{\"jsonrpc\":\"2.0\",\"id\":3,\"result\":{}}").expect("answer");
    let JsonRpcMessage::Answer(answer) = answer else {
        panic!("a result is an answer");
    };
    assert_eq!(answer.id, RequestId::Number(3));
    assert_eq!(answer.outcome, Ok(json!({})));
}

#[test]
fn a_malformed_envelope_is_refused_with_the_reserved_code() {
    let cases: [(&[u8], i32); 6] = [
        (b"not json", PARSE_ERROR),
        (b"\"text\"", INVALID_REQUEST),
        (
            b"{\"jsonrpc\":\"1.0\",\"id\":1,\"method\":\"ping\"}",
            INVALID_REQUEST,
        ),
        (b"{\"id\":1,\"method\":\"ping\"}", INVALID_REQUEST),
        (
            b"{\"jsonrpc\":\"2.0\",\"id\":null,\"method\":\"ping\"}",
            INVALID_REQUEST,
        ),
        (
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\",\"params\":[1]}",
            INVALID_PARAMS,
        ),
    ];
    for (bytes, code) in cases {
        let error = parse_message(bytes)
            .err()
            .unwrap_or_else(|| panic!("{} must be refused", String::from_utf8_lossy(bytes)));
        assert_eq!(error.code, code, "{}", String::from_utf8_lossy(bytes));
    }
}

#[test]
fn answers_and_notifications_encode_in_the_shape_the_specification_fixes() {
    let id = RequestId::Number(1);
    assert_eq!(
        encode_result(&id, json!({"ok": true})),
        json!({"jsonrpc": "2.0", "id": 1, "result": {"ok": true}})
    );
    let error = JsonRpcError::invalid_params("bad").with_data(json!({"field": "name"}));
    assert_eq!(
        encode_error(Some(&id), &error),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {"code": INVALID_PARAMS, "message": "bad", "data": {"field": "name"}},
        })
    );
    assert_eq!(
        encode_error(None, &JsonRpcError::parse_error("nope"))["id"],
        Value::Null
    );
    assert_eq!(
        encode_notification("notifications/resources/updated", json!({"uri": "bbb://x"})),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/resources/updated",
            "params": {"uri": "bbb://x"},
        })
    );
}

mod remote;
mod transport;

#[tokio::test]
async fn shared_http_client_and_gateway_helpers_frame_responses_and_sse_streams() {
    use std::time::Duration;
    use tokio::io::AsyncWriteExt;

    // 1. `read_request_head` preserves leftover upgrade frames and decodes query params.
    let raw_req = b"GET /control-ws?token=a%20b&mode=live HTTP/1.1\r\nHost: 127.0.0.1\r\nUpgrade: websocket\r\n\r\n\x81\x02hi";
    let (mut client_side, mut server_side) = tokio::io::duplex(4096);
    client_side.write_all(raw_req).await.expect("write req");
    drop(client_side);
    let (parsed_req, leftover) = crate::read_request_head(&mut server_side, &HttpLimits::default())
        .await
        .expect("read head")
        .expect("some request");
    assert_eq!(parsed_req.path, "/control-ws");
    assert_eq!(parsed_req.query_param("token").as_deref(), Some("a b"));
    assert_eq!(parsed_req.query_param("mode").as_deref(), Some("live"));
    assert_eq!(parsed_req.header("upgrade"), Some("websocket"));
    assert_eq!(leftover, b"\x81\x02hi");

    // 2. `read_response` skips `100 Continue`, frames by `Content-Length` without waiting for
    // TCP EOF on keep-alive streams, and preserves non-UTF-8 binary body bytes.
    let binary_body: [u8; 6] = [0x89, b'P', b'N', b'G', 0xff, 0xfe];
    let (mut peer_writer, peer_reader) = tokio::io::duplex(4096);
    peer_writer
        .write_all(
            b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 6\r\n\r\n",
        )
        .await
        .expect("write response head");
    peer_writer
        .write_all(&binary_body)
        .await
        .expect("write binary body");
    // Leave `peer_writer` open to prove `read_response` does NOT wait for EOF!
    let mut buf_reader = BufReader::new(peer_reader);
    let resp = crate::read_response_timeout(
        &mut buf_reader,
        &HttpLimits::default(),
        Duration::from_secs(1),
    )
    .await
    .expect("keep-alive response returns without EOF");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.header("content-type"), Some("image/png"));
    assert_eq!(resp.body, binary_body);
    drop(peer_writer);

    // 3. `SseStream::open` enforces `head_timeout` on stalled peers, surfaces non-200 HTTP
    // errors, ignores `: keep-alive` comments, and reassembles multi-line `data:` payloads.
    let (_stalled_writer, stalled_reader) = tokio::io::duplex(1024);
    let timed_out = crate::SseStream::open(
        stalled_reader,
        &HttpLimits::default(),
        Duration::from_millis(25),
    )
    .await;
    assert!(matches!(
        timed_out,
        Err(crate::SseOpenError::Transport(crate::HttpError::TimedOut))
    ));

    let (mut err_writer, err_reader) = tokio::io::duplex(1024);
    err_writer
        .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 13\r\n\r\nuninitialized")
        .await
        .expect("write error");
    drop(err_writer);
    match crate::SseStream::open(err_reader, &HttpLimits::default(), Duration::from_secs(1)).await {
        Err(crate::SseOpenError::Http(http)) => {
            assert_eq!(http.status, 400);
            assert_eq!(http.body_text(), "uninitialized");
        }
        other => panic!("expected HTTP error, got {}", other.err().unwrap()),
    }

    let (sse_writer, sse_reader) = tokio::io::duplex(4096);
    let server_task = tokio::spawn(async move {
        let mut stream = EventStream::start(
            sse_writer,
            &[("mcp-session-id".to_string(), "s-42".to_string())],
        )
        .await
        .expect("start sse");
        stream.keep_alive().await.expect("keep alive");
        stream
            .send(Some(9), "{\"line\":1}\n{\"line\":2}")
            .await
            .expect("send event");
        stream.finish().await.expect("finish");
    });
    let mut sse =
        crate::SseStream::open(sse_reader, &HttpLimits::default(), Duration::from_secs(1))
            .await
            .expect("open sse");
    assert_eq!(sse.head().header("mcp-session-id"), Some("s-42"));
    let event = sse
        .next_event()
        .await
        .expect("read event")
        .expect("one event");
    assert_eq!(event.id.as_deref(), Some("9"));
    assert_eq!(event.event.as_deref(), Some("message"));
    assert_eq!(event.data, "{\"line\":1}\n{\"line\":2}");
    assert_eq!(sse.next_event().await.expect("clean eof"), None);
    let _ = server_task.await;

    // 4. `SseStream` decodes `Transfer-Encoding: chunked` incrementally across arbitrary chunk
    // boundaries and rejects mixed `Transfer-Encoding` + `Content-Length`.
    let (mut chunked_writer, chunked_reader) = tokio::io::duplex(1024);
    let chunked_body = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: text/event-stream\r\n",
        "Transfer-Encoding: chunked\r\n\r\n",
        "8\r\nid: 1\nda\r\n",
        "b\r\nta: hello\n\n\r\n",
        "0\r\n\r\n"
    );
    chunked_writer
        .write_all(chunked_body.as_bytes())
        .await
        .expect("write chunked sse");
    drop(chunked_writer);
    let mut chunked_sse = crate::SseStream::open(
        chunked_reader,
        &HttpLimits::default(),
        Duration::from_secs(1),
    )
    .await
    .expect("open chunked sse");
    let chunked_event = chunked_sse
        .next_event()
        .await
        .expect("read chunked event")
        .expect("chunked event");
    assert_eq!(chunked_event.id.as_deref(), Some("1"));
    assert_eq!(chunked_event.data, "hello");
    assert_eq!(chunked_sse.next_event().await.expect("chunked eof"), None);

    let (mut smuggled_writer, smuggled_reader) = tokio::io::duplex(1024);
    smuggled_writer
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nContent-Length: 5\r\n\r\n",
        )
        .await
        .expect("write smuggled sse");
    drop(smuggled_writer);
    assert!(matches!(
        crate::SseStream::open(
            smuggled_reader,
            &HttpLimits::default(),
            Duration::from_secs(1)
        )
        .await,
        Err(crate::SseOpenError::Transport(
            crate::HttpError::MalformedHeader
        ))
    ));
}

#[tokio::test]
async fn session_events_wake_before_wait_is_never_lost() {
    let events = crate::SessionEvents::default();
    let notified = events.notified();
    tokio::pin!(notified);
    notified.as_mut().enable();
    assert!(!events.is_closed());
    events.wake();
    assert!(events.is_closed());
    tokio::time::timeout(std::time::Duration::from_millis(50), notified)
        .await
        .expect("registered waiter wakes immediately after concurrent wake()");
}

#[test]
fn control_tunnel_wire_envelopes_and_operation_prefixes_round_trip() {
    assert_eq!(crate::OPERATION_URI_PREFIX, "bbb://server/operation/");
    assert_eq!(
        crate::CLIENT_OPERATION_URI_PREFIX,
        "bbb://client/operation/"
    );

    let req_default = crate::ControlRequest::from_json_str(r#"{"method":"client.status"}"#)
        .expect("default params");
    assert_eq!(req_default.method, "client.status");
    assert_eq!(req_default.params, Value::Null);

    let host_msg = crate::ControlTunnelHostMessage::Request {
        id: 7,
        confirmed: true,
        request: crate::ControlRequest::new("renderer.screenshot", json!({ "path": "shot.png" })),
    };
    let encoded_host = host_msg.to_json_string();
    assert_eq!(
        serde_json::from_str::<Value>(&encoded_host).expect("json"),
        json!({
            "type": "request",
            "id": 7,
            "confirmed": true,
            "request": {
                "method": "renderer.screenshot",
                "params": { "path": "shot.png" },
            },
        })
    );
    assert_eq!(
        crate::ControlTunnelHostMessage::from_json_str(&encoded_host).expect("decode host"),
        host_msg
    );

    let hello = crate::ControlTunnelClientMessage::Hello {
        version: "0.1.0".to_string(),
        generation: 12,
    };
    assert_eq!(
        crate::ControlTunnelClientMessage::from_json_slice(&hello.to_json_vec())
            .expect("decode hello"),
        hello
    );

    let plain_reply = crate::ControlTunnelClientMessage::Response {
        id: 7,
        response: crate::ControlResponse::ok(json!({ "ready": true })),
        image: None,
    };
    let plain_val = plain_reply.to_value();
    assert!(
        plain_val.get("image").is_none(),
        "None image is omitted on wire"
    );
    assert!(
        plain_val["response"].get("error").is_none(),
        "None error is omitted on wire"
    );
    assert_eq!(
        crate::ControlTunnelClientMessage::from_value(plain_val).expect("decode plain"),
        plain_reply
    );

    let screenshot = crate::ControlScreenshot {
        width: 320,
        height: 180,
        mime_type: "image/png",
        bytes: vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
    };
    let shot_reply = crate::ControlTunnelClientMessage::Response {
        id: 8,
        response: crate::ControlResponse::ok(json!({ "completed": true })),
        image: Some(screenshot.to_tunnel()),
    };
    let decoded_shot =
        crate::ControlTunnelClientMessage::from_json_str(&shot_reply.to_json_string())
            .expect("decode shot reply");
    assert_eq!(decoded_shot, shot_reply);
    if let crate::ControlTunnelClientMessage::Response {
        image: Some(wire_img),
        ..
    } = decoded_shot
    {
        assert_eq!(wire_img.decode().expect("decode png"), screenshot);
    } else {
        panic!("expected screenshot response");
    }
}

#[tokio::test]
async fn chunked_request_body_decodes_within_limits_and_rejects_oversize_or_smuggling() {
    let chunked = b"POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nTransfer-Encoding: chunked\r\n\r\n\
        2;ext=1\r\n{\"\r\n\
        4\r\na\":1\r\n\
        1\r\n}\r\n\
        0\r\nX-Trailer: ok\r\n\r\n";
    let request = parse_http(chunked)
        .await
        .expect("read chunked")
        .expect("request");
    assert_eq!(request.body, b"{\"a\":1}");

    // Combined Transfer-Encoding and Content-Length must be rejected (400).
    let smuggled =
        b"POST /mcp HTTP/1.1\r\nTransfer-Encoding: chunked\r\nContent-Length: 2\r\n\r\n0\r\n\r\n";
    let err = parse_http(smuggled).await.expect_err("smuggling refused");
    assert_eq!(err.status(), 400);

    // Chunked body exceeding max_body_bytes is rejected with 413.
    let limits = HttpLimits {
        max_body_bytes: 4,
        ..HttpLimits::default()
    };
    let mut reader = BufReader::new(
        b"POST /mcp HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n0\r\n\r\n"
            .as_slice(),
    );
    let mut writer = Vec::new();
    let err = read_request(&mut reader, &mut writer, &limits)
        .await
        .expect_err("oversize chunked refused");
    assert_eq!(err.status(), 413);
}

#[test]
fn secret_matches_handles_matching_different_and_mismatched_lengths() {
    assert!(crate::auth::secret_matches(
        "super-secret-token",
        "super-secret-token"
    ));
    assert!(!crate::auth::secret_matches(
        "super-secret-token",
        "super-secret-tokeN"
    ));
    assert!(!crate::auth::secret_matches("super-secret-token", "short"));
    assert!(!crate::auth::secret_matches(
        "super-secret-token",
        "super-secret-token-extra"
    ));
    assert!(crate::auth::secret_matches("", ""));
    assert!(!crate::auth::secret_matches("", "a"));
}

#[tokio::test]
async fn content_length_rejects_signed_digits_and_loopback_rejects_malformed_auth() {
    let err = parse_http(b"POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: +2\r\n\r\n{}")
        .await
        .expect_err("plus-signed Content-Length refused");
    assert_eq!(err.status(), 400);

    let req = parse_http(
        b"POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Basic dXNlcjpwYXNz\r\nContent-Length: 2\r\n\r\n{}",
    )
    .await
    .expect("read")
    .expect("request");
    let config = crate::AuthConfig::<crate::Scope>::default();
    let peer: std::net::SocketAddr = "127.0.0.1:43210".parse().unwrap();
    assert_eq!(
        crate::auth::authenticate(&config, &req.headers, peer, 0),
        Err(crate::auth::AuthFailure::InvalidCredential)
    );
}

#[test]
fn client_control_catalog_schemas_enforce_additional_properties_and_bounds() {
    for tool in crate::control::catalog() {
        let schema = tool.descriptor().input_schema;
        assert_eq!(schema["type"], "object", "tool {}", tool.name);
        assert_eq!(
            schema["additionalProperties"], false,
            "tool {} must reject unknown arguments",
            tool.name
        );
    }

    let screenshot = crate::control::tool_by_name("renderer.screenshot")
        .unwrap()
        .descriptor()
        .input_schema;
    assert_eq!(screenshot["properties"]["path"]["minLength"], 1);

    let look = crate::control::tool_by_name("input.look")
        .unwrap()
        .descriptor()
        .input_schema;
    assert_eq!(look["properties"]["pitch"]["minimum"], -90);
    assert_eq!(look["properties"]["pitch"]["maximum"], 90);

    let look_at = crate::control::tool_by_name("input.look_at")
        .unwrap()
        .descriptor()
        .input_schema;
    assert_eq!(look_at["oneOf"].as_array().map(Vec::len), Some(2));

    let step = crate::control::tool_by_name("sim.step")
        .unwrap()
        .descriptor()
        .input_schema;
    assert_eq!(step["properties"]["ticks"]["minimum"], 1);
    assert_eq!(step["properties"]["tick_nanos"]["minimum"], 1);
}
