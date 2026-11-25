use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bbb_client_assets::{KnownPacksManager, PackRoots};
use bbb_mcp::control::{
    ControlRequest, ControlResponse, ControlTunnelClientMessage, ControlTunnelHostMessage,
    ControlTunnelScreenshot, RemoteControlEndpoint,
};
use bbb_protocol::MC_VERSION;
use bbb_web_host::{
    ws_codec::{WsMessage, WsStream},
    WebHostConfig, WebHostServer,
};
use sha1::{Digest, Sha1};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};

fn create_synthetic_vanilla_fixture(root: &Path) -> (String, Vec<u8>) {
    let sources_dir = root.join("sources").join(MC_VERSION);
    let downloads_dir = root.join("downloads").join(MC_VERSION);
    let objects_dir = downloads_dir.join("assets").join("objects");
    let indexes_dir = downloads_dir.join("assets").join("indexes");

    fs::create_dir_all(sources_dir.join("assets/minecraft/blockstates")).unwrap();
    fs::create_dir_all(sources_dir.join("assets/minecraft/font/include")).unwrap();
    fs::create_dir_all(sources_dir.join("data/minecraft")).unwrap();
    fs::create_dir_all(&indexes_dir).unwrap();

    fs::write(
        sources_dir.join("pack.mcmeta"),
        serde_json::to_vec(&serde_json::json!({
            "pack": {
                "description": "Stub jar pack",
                "min_format": [bbb_protocol::MC_RESOURCE_PACK_FORMAT.major, bbb_protocol::MC_RESOURCE_PACK_FORMAT.minor],
                "max_format": [bbb_protocol::MC_RESOURCE_PACK_FORMAT.major, bbb_protocol::MC_RESOURCE_PACK_FORMAT.minor]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        sources_dir.join("version.json"),
        serde_json::to_vec(&serde_json::json!({
            "id": MC_VERSION,
            "name": MC_VERSION,
            "protocol_version": bbb_protocol::PROTOCOL_VERSION,
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        sources_dir.join("assets/minecraft/blockstates/stone.json"),
        br#"{"variants":{"":{"model":"minecraft:block/stone"}}}"#,
    )
    .unwrap();
    fs::write(
        sources_dir.join("assets/minecraft/font/include/unifont.json"),
        br#"{"providers":[]}"#,
    )
    .unwrap();

    let write_obj = |bytes: &[u8]| -> (String, usize) {
        let mut h = Sha1::new();
        h.update(bytes);
        let hex = format!("{:x}", h.finalize());
        let dir = objects_dir.join(&hex[..2]);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(&hex), bytes).unwrap();
        (hex, bytes.len())
    };

    let (mcmeta_hash, mcmeta_size) = write_obj(
        br#"{"language":{"en_us":{"name":"English","region":"US","bidirectional":false},"zh_cn":{"name":"Chinese","region":"CN","bidirectional":false}}}"#,
    );
    let (lang_hash, lang_size) = write_obj(br#"{"block.minecraft.stone":"Stone"}"#);
    let (unifont_hash, unifont_size) =
        write_obj(br#"{"providers":[{"type":"unihex","hex_file":"minecraft:font/unifont.zip"}]}"#);
    let (sounds_hash, sounds_size) = write_obj(br#"{}"#);
    let ogg_bytes = b"OggS-fake-vorbis-payload".to_vec();
    let (ogg_hash, ogg_size) = write_obj(&ogg_bytes);

    let index_bytes = serde_json::to_vec(&serde_json::json!({
        "objects": {
            "pack.mcmeta": { "hash": mcmeta_hash, "size": mcmeta_size },
            "minecraft/lang/en_us.json": { "hash": lang_hash, "size": lang_size },
            "minecraft/font/include/unifont.json": { "hash": unifont_hash, "size": unifont_size },
            "minecraft/sounds.json": { "hash": sounds_hash, "size": sounds_size },
            "minecraft/sounds/ambient/cave/cave1.ogg": { "hash": ogg_hash, "size": ogg_size }
        }
    }))
    .unwrap();
    fs::write(indexes_dir.join("30.json"), &index_bytes).unwrap();
    fs::write(
        downloads_dir.join(format!("{MC_VERSION}.json")),
        serde_json::to_vec(&serde_json::json!({
            "assetIndex": { "id": "30", "sha1": "deadbeef" }
        }))
        .unwrap(),
    )
    .unwrap();

    (ogg_hash, ogg_bytes)
}

async fn raw_http_get(
    addr: SocketAddr,
    target: &str,
    extra_headers: &[(&str, &str)],
) -> (u16, String, Vec<u8>) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut req = format!("GET {target} HTTP/1.1\r\n");
    let has_host = extra_headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("host"));
    if !has_host {
        req.push_str(&format!("Host: {addr}\r\n"));
    }
    let has_connection = extra_headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("connection"));
    if !has_connection {
        req.push_str("Connection: close\r\n");
    }
    for (k, v) in extra_headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(raw[..split].to_vec()).unwrap();
    let body = raw[split + 4..].to_vec();
    let first_line = head.lines().next().unwrap();
    let status: u16 = first_line
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    (status, head, body)
}

async fn read_one_http_response(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
) -> (u16, String, Vec<u8>) {
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos;
        }
        let n = stream.read(&mut chunk).await.unwrap();
        assert!(n > 0, "unexpected EOF reading HTTP response headers");
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8(buf[..header_end].to_vec()).unwrap();
    let status: u16 = head
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let content_len: usize = if matches!(status, 204 | 304) {
        0
    } else {
        head.lines()
            .find_map(|line| {
                let (k, v) = line.split_once(':')?;
                k.trim()
                    .eq_ignore_ascii_case("content-length")
                    .then(|| v.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0)
    };
    let body_start = header_end + 4;
    while buf.len() < body_start + content_len {
        let n = stream.read(&mut chunk).await.unwrap();
        assert!(n > 0, "unexpected EOF reading HTTP response body");
        buf.extend_from_slice(&chunk[..n]);
    }
    let body = buf[body_start..body_start + content_len].to_vec();
    buf.drain(..body_start + content_len);
    (status, head, body)
}

async fn client_ws_connect(
    addr: SocketAddr,
    target: &str,
) -> Result<WsStream<TcpStream>, (u16, String)> {
    client_ws_connect_with_headers(addr, target, &[]).await
}

async fn client_ws_connect_with_headers(
    addr: SocketAddr,
    target: &str,
    extra_headers: &[(&str, &str)],
) -> Result<WsStream<TcpStream>, (u16, String)> {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut req = format!("GET {target} HTTP/1.1\r\n");
    let has_host = extra_headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("host"));
    if !has_host {
        req.push_str(&format!("Host: {addr}\r\n"));
    }
    req.push_str(
        "Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Sec-WebSocket-Version: 13\r\n",
    );
    for (k, v) in extra_headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    let header_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos;
        }
        let n = stream.read(&mut chunk).await.unwrap();
        assert!(n > 0, "unexpected EOF during WS handshake");
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8(buf[..header_end].to_vec()).unwrap();
    let status: u16 = head
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    if status != 101 {
        return Err((status, head));
    }
    let leftover = buf[header_end + 4..].to_vec();
    Ok(WsStream::new(stream, leftover))
}

async fn send_client_ws_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) {
    let mut frame = Vec::new();
    frame.push(0x80 | opcode);
    let mask = [0x12u8, 0x34, 0x56, 0x78];
    if payload.len() <= 125 {
        frame.push(0x80 | (payload.len() as u8));
    } else {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    }
    frame.extend_from_slice(&mask);
    for (i, b) in payload.iter().enumerate() {
        frame.push(b ^ mask[i & 3]);
    }
    stream.write_all(&frame).await.unwrap();
    stream.flush().await.unwrap();
}

#[tokio::test]
async fn gateway_serves_isolated_assets_etag_and_rejects_traversal() {
    let tmp = unique_temp_dir("bbb-web-host-test");
    let fixture_root = tmp.join("vanilla");
    let wasm_dir = tmp.join("wasm");
    let cache_dir = tmp.join("cache");
    fs::create_dir_all(wasm_dir.join("pkg/snippets/worker")).unwrap();
    fs::write(
        wasm_dir.join("pkg/bbb_client.js"),
        b"export default function(){}",
    )
    .unwrap();
    fs::write(
        wasm_dir.join("pkg/snippets/worker/workerHelpers.js"),
        b"// worker helper",
    )
    .unwrap();
    let (ogg_hash, ogg_bytes) = create_synthetic_vanilla_fixture(&fixture_root);

    let server = WebHostServer::bind(WebHostConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        default_server: "127.0.0.1:25565".to_string(),
        allow_servers: Vec::new(),
        vanilla_root: fixture_root,
        wasm_dir,
        cache_dir,
        default_username: "Tester".to_string(),
        default_locale: Some("ZH_cn ".to_string()),
        default_uuid: None,
        default_render_distance: Some(8),
        server_resource_pack_policy: None,
        allow_pack_loopback: true,
        allow_pack_private: false,
        control_enabled: false,
    })
    .await
    .unwrap();
    let addr = server.local_addr();
    let etag = server.assets().core_bundle_etag().to_string();
    let bundle_ptr = server.assets().core_bundle_bytes().as_ptr();
    let (mut probe_client, mut probe_server) = tokio::io::duplex(256);
    probe_client
        .write_all(b"GET /assets/core-26.1.zip HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();
    let (probe_req, _) = bbb_web_host::http::read_http_request(&mut probe_server)
        .await
        .unwrap()
        .unwrap();
    let probe_resp = server.assets().serve(&probe_req);
    assert_eq!(
        probe_resp.body.as_ptr(),
        bundle_ptr,
        "core-26.1.zip response body must share PreparedWebAssets backing allocation zero-copy"
    );
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        server
            .serve(async {
                let _ = stop_rx.await;
            })
            .await
            .unwrap();
    });

    let (status, head, body) = raw_http_get(addr, "/", &[]).await;
    assert_eq!(status, 200);
    let lower_head = head.to_ascii_lowercase();
    assert!(lower_head.contains("cross-origin-opener-policy: same-origin"));
    assert!(lower_head.contains("cross-origin-embedder-policy: require-corp"));
    assert!(lower_head.contains("cross-origin-resource-policy: same-origin"));
    assert!(String::from_utf8_lossy(&body).contains("bbb-canvas"));

    let (status, pkg_head, worker_body) =
        raw_http_get(addr, "/pkg/snippets/worker/workerHelpers.js", &[]).await;
    assert_eq!(status, 200);
    assert_eq!(worker_body, b"// worker helper");
    let pkg_etag = pkg_head
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case("etag")
                .then(|| v.trim().to_string())
        })
        .expect("expected ETag on /pkg/*");

    let mut ka_stream = TcpStream::connect(addr).await.unwrap();
    let mut ka_buf = Vec::new();
    let pipelined = format!(
        "GET /pkg/snippets/worker/workerHelpers.js HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Accept-Encoding: gzip, br\r\n\r\n\
         GET /pkg/snippets/worker/workerHelpers.js HTTP/1.1\r\n\
         Host: {addr}\r\n\
         If-None-Match: {pkg_etag}\r\n\
         Connection: close\r\n\r\n"
    );
    ka_stream.write_all(pipelined.as_bytes()).await.unwrap();
    let (gz_status, gz_head, gz_body) = read_one_http_response(&mut ka_stream, &mut ka_buf).await;
    assert_eq!(gz_status, 200);
    assert!(gz_head
        .to_ascii_lowercase()
        .contains("connection: keep-alive"));
    assert!(gz_head
        .to_ascii_lowercase()
        .contains("content-encoding: gzip"));
    let gz_etag = gz_head
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case("etag")
                .then(|| v.trim().to_string())
        })
        .expect("expected ETag on gzipped /pkg/*");
    assert_ne!(
        gz_etag, pkg_etag,
        "gzip and identity representations must use distinct strong ETags"
    );
    assert_eq!(&gz_body[..2], &[0x1f, 0x8b]);
    let (nm_status, nm_head, nm_body) = read_one_http_response(&mut ka_stream, &mut ka_buf).await;
    assert_eq!(nm_status, 304);
    assert!(nm_head.to_ascii_lowercase().contains("connection: close"));
    assert!(nm_body.is_empty());

    let (mismatch_status, _, mismatch_body) = raw_http_get(
        addr,
        "/pkg/snippets/worker/workerHelpers.js",
        &[("Accept-Encoding", "gzip"), ("If-None-Match", &pkg_etag)],
    )
    .await;
    assert_eq!(mismatch_status, 200);
    assert_eq!(&mismatch_body[..2], &[0x1f, 0x8b]);

    let (gz_nm_status, _, gz_nm_body) = raw_http_get(
        addr,
        "/pkg/snippets/worker/workerHelpers.js",
        &[("Accept-Encoding", "gzip"), ("If-None-Match", &gz_etag)],
    )
    .await;
    assert_eq!(gz_nm_status, 304);
    assert!(gz_nm_body.is_empty());

    let (status, _, config_body) = raw_http_get(addr, "/config.json", &[]).await;
    assert_eq!(status, 200);
    let config_json: serde_json::Value = serde_json::from_slice(&config_body).unwrap();
    assert!(config_json.get("server_resource_pack_policy").is_none());
    assert_eq!(config_json["lang"], "zh_cn");

    let (status, _, zip_body) = raw_http_get(addr, "/assets/core-26.1.zip", &[]).await;
    assert_eq!(status, 200);
    let (_, _, index_body) = raw_http_get(addr, "/assets/asset-index.json", &[]).await;
    let roots =
        PackRoots::from_web_bundle(Arc::from(zip_body), Some("30"), Some(&index_body)).unwrap();
    assert!(roots.vanilla_asset_index_observation().is_some());
    let stack = roots.resource_stack();
    let unifont_loc =
        bbb_client_assets::ResourceLocation::parse("minecraft:font/include/unifont.json").unwrap();
    let unifont_res = stack
        .get_resource(&unifont_loc)
        .expect("unifont.json in core bundle");
    let unifont_text = stack
        .read_text_resource(&unifont_res, "unifont.json")
        .unwrap();
    assert!(
        unifont_text.contains("unihex"),
        "expected asset-index unifont.json to override jar stub, got: {unifont_text}"
    );
    let metadata = roots.load_pack_metadata_catalog();
    assert!(
        metadata.languages.contains_key("zh_cn"),
        "expected merged pack.mcmeta language table"
    );

    let (status_304, _, empty_body) =
        raw_http_get(addr, "/assets/core-26.1.zip", &[("If-None-Match", &etag)]).await;
    assert_eq!(status_304, 304);
    assert!(empty_body.is_empty());

    let ogg_path = format!("/assets/objects/{}/{}", &ogg_hash[..2], ogg_hash);
    let (ogg_status, _, fetched_ogg) = raw_http_get(addr, &ogg_path, &[]).await;
    assert_eq!(ogg_status, 200);
    assert_eq!(fetched_ogg, ogg_bytes);

    let (bad1, _, _) = raw_http_get(addr, "/assets/objects/../26.1.json", &[]).await;
    assert_eq!(bad1, 400);
    let (bad2, _, _) = raw_http_get(addr, "/pkg/../Cargo.toml", &[]).await;
    assert_eq!(bad2, 400);

    let _ = stop_tx.send(());
    join.await.unwrap();
    let _ = fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn websocket_relay_transfers_binary_frames_and_enforces_allowlist() {
    let upstream_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = upstream_listener.local_addr().unwrap();
    let upstream_task = tokio::spawn(async move {
        let (mut sock, _) = upstream_listener.accept().await.unwrap();
        let mut buf = vec![0u8; 512];
        let n = sock.read(&mut buf).await.unwrap();
        let mut reply = b"mc-ack:".to_vec();
        reply.extend_from_slice(&buf[..n]);
        sock.write_all(&reply).await.unwrap();
        sock.shutdown().await.unwrap();
    });

    let tmp = unique_temp_dir("bbb-web-ws-test");
    let fixture_root = tmp.join("vanilla");
    create_synthetic_vanilla_fixture(&fixture_root);

    let server = WebHostServer::bind(WebHostConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        default_server: upstream_addr.to_string(),
        allow_servers: Vec::new(),
        vanilla_root: fixture_root,
        wasm_dir: tmp.join("wasm"),
        cache_dir: tmp.join("cache"),
        default_username: "Tester".to_string(),
        default_locale: None,
        default_uuid: None,
        default_render_distance: None,
        server_resource_pack_policy: Some("accept".to_string()),
        allow_pack_loopback: false,
        allow_pack_private: false,
        control_enabled: false,
    })
    .await
    .unwrap();
    let addr = server.local_addr();
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        server
            .serve(async {
                let _ = stop_rx.await;
            })
            .await
            .unwrap();
    });

    let forbidden = client_ws_connect(addr, "/ws?server=127.0.0.1%3A9").await;
    assert!(matches!(forbidden, Err((403, _))));
    let forbidden_localhost_other_port = client_ws_connect(addr, "/ws?server=localhost%3A9").await;
    assert!(matches!(forbidden_localhost_other_port, Err((403, _))));

    let cross_origin_ws =
        client_ws_connect_with_headers(addr, "/ws", &[("Origin", "https://evil.example")]).await;
    assert!(
        matches!(cross_origin_ws, Err((403, _))),
        "expected 403 for cross-origin /ws, got {:?}",
        cross_origin_ws.err()
    );
    let null_origin_ws = client_ws_connect_with_headers(addr, "/ws", &[("Origin", "null")]).await;
    assert!(
        matches!(null_origin_ws, Err((403, _))),
        "expected 403 for null Origin /ws, got {:?}",
        null_origin_ws.err()
    );

    // Non-upgrade GET /ws must be rejected with 400 before dialing upstream.
    let (bad_upgrade_status, _, _) = raw_http_get(addr, "/ws", &[]).await;
    assert_eq!(bad_upgrade_status, 400);

    // HEAD on proxy endpoints must return 405 without buffering upstream GET bodies.
    let mut head_stream = TcpStream::connect(addr).await.unwrap();
    head_stream
        .write_all(
            format!(
                "HEAD /proxy/pack?url=http%3A%2F%2F127.0.0.1%2Fpack.zip HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    let mut head_raw = Vec::new();
    head_stream.read_to_end(&mut head_raw).await.unwrap();
    let head_resp = String::from_utf8(head_raw).unwrap();
    assert!(
        head_resp.starts_with("HTTP/1.1 405"),
        "expected 405 for HEAD /proxy/pack, got: {head_resp}"
    );

    let localhost_alias = format!("localhost:{}", upstream_addr.port());
    let encoded_allowed: String =
        url::form_urlencoded::byte_serialize(localhost_alias.as_bytes()).collect();
    let mut raw = TcpStream::connect(addr).await.unwrap();
    let upgrade_req = format!(
        "GET /ws?server={encoded_allowed} HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Sec-WebSocket-Version: 13\r\n\r\n"
    );
    raw.write_all(upgrade_req.as_bytes()).await.unwrap();
    let mut head_buf = [0u8; 512];
    let n = raw.read(&mut head_buf).await.unwrap();
    let head_str = std::str::from_utf8(&head_buf[..n]).unwrap();
    assert!(head_str.starts_with("HTTP/1.1 101"));

    let payload: Vec<u8> = (0u8..200).collect();
    send_client_ws_frame(&mut raw, 0x2, &payload).await;

    let mut ws = WsStream::new(raw, Vec::new());
    let reply = ws.recv().await.unwrap().unwrap();
    let mut expected = b"mc-ack:".to_vec();
    expected.extend_from_slice(&payload);
    assert_eq!(reply, WsMessage::Binary(expected));

    upstream_task.await.unwrap();
    let _ = stop_tx.send(());
    join.await.unwrap();
    let _ = fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn reverse_control_tunnel_roundtrips_and_writes_host_screenshot() {
    let tmp = unique_temp_dir("bbb-web-ctrl-test");
    let fixture_root = tmp.join("vanilla");
    create_synthetic_vanilla_fixture(&fixture_root);

    let server = WebHostServer::bind(WebHostConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        default_server: "127.0.0.1:25565".to_string(),
        allow_servers: Vec::new(),
        vanilla_root: fixture_root,
        wasm_dir: tmp.join("wasm"),
        cache_dir: tmp.join("cache"),
        default_username: "Tester".to_string(),
        default_locale: None,
        default_uuid: None,
        default_render_distance: None,
        server_resource_pack_policy: Some("accept".to_string()),
        allow_pack_loopback: false,
        allow_pack_private: false,
        control_enabled: true,
    })
    .await
    .unwrap();
    let addr = server.local_addr();
    let bridge = server.control_bridge();
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        server
            .serve(async {
                let _ = stop_rx.await;
            })
            .await
            .unwrap();
    });

    let mut raw = TcpStream::connect(addr).await.unwrap();
    let upgrade_req = format!(
        "GET /control-ws HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Sec-WebSocket-Version: 13\r\n\r\n"
    );
    raw.write_all(upgrade_req.as_bytes()).await.unwrap();
    let mut head_buf = [0u8; 512];
    let n = raw.read(&mut head_buf).await.unwrap();
    assert!(std::str::from_utf8(&head_buf[..n])
        .unwrap()
        .starts_with("HTTP/1.1 101"));

    let hello = ControlTunnelClientMessage::Hello {
        version: "test".to_string(),
        generation: 42,
    }
    .to_json_vec();
    send_client_ws_frame(&mut raw, 0x1, &hello).await;

    for _ in 0..50 {
        if bridge.generation() == 42 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(bridge.generation(), 42);

    let png_bytes = b"\x89PNG\r\n\x1a\nfake-png-payload".to_vec();
    let png_b64 = BASE64.encode(&png_bytes);
    let browser_task = tokio::spawn(async move {
        let mut ws_reader = WsStream::new(raw, Vec::new());
        let Some(WsMessage::Text(req_json)) = ws_reader.recv().await.unwrap() else {
            panic!("expected text ControlTunnelHostMessage");
        };
        let ControlTunnelHostMessage::Request {
            id,
            confirmed,
            request,
        } = ControlTunnelHostMessage::from_json_str(&req_json).unwrap();
        assert!(confirmed);
        assert_eq!(request.method, "renderer.screenshot");
        let resp = ControlTunnelClientMessage::Response {
            id,
            response: ControlResponse {
                ok: true,
                result: Some(serde_json::json!({ "width": 64, "height": 32 })),
                error: None,
            },
            image: Some(ControlTunnelScreenshot {
                width: 64,
                height: 32,
                mime_type: "image/png".to_string(),
                data_base64: png_b64,
            }),
        }
        .to_json_vec();
        let (mut _r, mut w) = ws_reader.into_parts();
        w.send(&WsMessage::Binary(resp)).await.unwrap();
    });

    let out_png = tmp.join("shot.png");
    let answer = bridge
        .call(
            ControlRequest {
                method: "renderer.screenshot".to_string(),
                params: serde_json::json!({ "path": out_png.display().to_string() }),
            },
            true,
        )
        .await;
    browser_task.await.unwrap();
    assert!(answer.response.ok, "{:?}", answer.response);
    let captured = answer.image.expect("expected decoded screenshot");
    assert_eq!(captured.width, 64);
    assert_eq!(captured.height, 32);
    assert_eq!(captured.bytes, png_bytes);
    assert_eq!(fs::read(&out_png).unwrap(), png_bytes);

    let _ = stop_tx.send(());
    join.await.unwrap();
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn real_vanilla_bundle_loads_known_packs_when_prepared() {
    let default_vanilla = std::env::var_os("BBB_VANILLA_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("bbb").join("vanilla"))
        })
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".cache").join("bbb").join("vanilla"))
        });
    let Some(vanilla_root) =
        default_vanilla.filter(|p| p.join("sources/26.1/version.json").is_file())
    else {
        return;
    };
    let tmp = unique_temp_dir("bbb-web-real-vanilla");
    let prepared = bbb_web_host::assets_http::PreparedWebAssets::build_or_load(
        bbb_web_host::assets_http::AssetBuildOptions {
            vanilla_root: vanilla_root.clone(),
            wasm_dir: tmp.join("wasm"),
            cache_dir: tmp.join("cache"),
            default_server: "127.0.0.1:25565".to_string(),
            default_username: "Tester".to_string(),
            default_locale: None,
            default_uuid: None,
            default_render_distance: None,
            server_resource_pack_policy: Some("accept".to_string()),
            control_enabled: false,
        },
    )
    .unwrap();
    let index_path = vanilla_root
        .join("downloads")
        .join(MC_VERSION)
        .join("assets/indexes")
        .join(format!("{}.json", prepared.asset_index_id()));
    let index_bytes = fs::read(index_path).unwrap();
    let roots = PackRoots::from_web_bundle(
        Arc::from(prepared.core_bundle_bytes().to_vec()),
        Some(prepared.asset_index_id()),
        Some(&index_bytes),
    )
    .unwrap();
    let known = KnownPacksManager::load(&roots).unwrap();
    assert!(known.available_packs().len() >= 1);
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn builds_core_bundle_directly_from_launcher_jar_root_without_sources() {
    use std::io::Write;
    let tmp = unique_temp_dir("bbb-web-launcher-jar");
    let mc_root = tmp.join(".minecraft");
    let version_dir = mc_root.join("versions").join(MC_VERSION);
    let objects_dir = mc_root.join("assets/objects");
    let indexes_dir = mc_root.join("assets/indexes");
    fs::create_dir_all(&version_dir).unwrap();
    fs::create_dir_all(&indexes_dir).unwrap();

    let jar_path = version_dir.join("client.jar");
    {
        let file = fs::File::create(&jar_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("version.json", opts).unwrap();
        zip.write_all(
            &serde_json::to_vec(&serde_json::json!({
                "id": MC_VERSION,
                "name": MC_VERSION,
                "protocol_version": bbb_protocol::PROTOCOL_VERSION,
            }))
            .unwrap(),
        )
        .unwrap();
        zip.start_file("assets/minecraft/blockstates/stone.json", opts)
            .unwrap();
        zip.write_all(br#"{"variants":{"":{"model":"minecraft:block/stone"}}}"#)
            .unwrap();
        zip.start_file("data/minecraft/tags/block/mineable/pickaxe.json", opts)
            .unwrap();
        zip.write_all(br#"{"values":["minecraft:stone"]}"#).unwrap();
        zip.start_file("data/minecraft/dimension_type/overworld.json", opts)
            .unwrap();
        zip.write_all(br#"{"ultrawarm":false}"#).unwrap();
        zip.start_file("data/minecraft/recipe/stone_bricks.json", opts)
            .unwrap();
        zip.write_all(br#"{"type":"minecraft:crafting_shaped"}"#)
            .unwrap();
        zip.start_file("assets/minecraft/resourcepacks/programmer_art.zip", opts)
            .unwrap();
        zip.write_all(b"PK-stub").unwrap();
        zip.start_file("assets/minecraft/resourcepacks/high_contrast.zip", opts)
            .unwrap();
        zip.write_all(b"PK-stub").unwrap();
        zip.start_file("net/minecraft/client/Main.class", opts)
            .unwrap();
        zip.write_all(b"\xca\xfe\xba\xbe").unwrap();
        zip.finish().unwrap();
    }

    let lang_bytes = br#"{"block.minecraft.stone":"Stone"}"#;
    let mut h = Sha1::new();
    h.update(lang_bytes);
    let lang_hash = format!("{:x}", h.finalize());
    let lang_dir = objects_dir.join(&lang_hash[..2]);
    fs::create_dir_all(&lang_dir).unwrap();
    fs::write(lang_dir.join(&lang_hash), lang_bytes).unwrap();

    let index_bytes = serde_json::to_vec(&serde_json::json!({
        "objects": {
            "minecraft/lang/en_us.json": { "hash": lang_hash, "size": lang_bytes.len() }
        }
    }))
    .unwrap();
    fs::write(indexes_dir.join("30.json"), &index_bytes).unwrap();
    assert!(!bbb_web_host::assets_http::has_vanilla_bundle_input(&mc_root));
    fs::write(
        version_dir.join(format!("{MC_VERSION}.json")),
        serde_json::to_vec(&serde_json::json!({
            "assetIndex": { "id": "30", "sha1": "cafebabe" }
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(bbb_web_host::assets_http::has_vanilla_bundle_input(&mc_root));

    let prepared = bbb_web_host::assets_http::PreparedWebAssets::build_or_load(
        bbb_web_host::assets_http::AssetBuildOptions {
            vanilla_root: mc_root,
            wasm_dir: tmp.join("wasm"),
            cache_dir: tmp.join("cache"),
            default_server: "127.0.0.1:25565".to_string(),
            default_username: "Tester".to_string(),
            default_locale: None,
            default_uuid: None,
            default_render_distance: None,
            server_resource_pack_policy: Some("decline".to_string()),
            control_enabled: false,
        },
    )
    .unwrap();

    let roots = PackRoots::from_web_bundle(
        Arc::from(prepared.core_bundle_bytes().to_vec()),
        Some(prepared.asset_index_id()),
        Some(&index_bytes),
    )
    .unwrap();
    let stack = roots.resource_stack();
    let stone_loc =
        bbb_client_assets::ResourceLocation::parse("minecraft:blockstates/stone.json").unwrap();
    assert!(stack.get_resource(&stone_loc).is_some());
    let block_tags = roots.load_tag_catalog("block").unwrap();
    assert!(block_tags.contains("minecraft:mineable/pickaxe", "minecraft:stone"));
    let dim_loc =
        bbb_client_assets::ResourceLocation::parse("minecraft:dimension_type/overworld.json")
            .unwrap();
    assert!(stack.get_data_resource(&dim_loc).is_some());
    let recipe_loc =
        bbb_client_assets::ResourceLocation::parse("minecraft:recipe/stone_bricks.json").unwrap();
    assert!(
        stack.get_data_resource(&recipe_loc).is_none(),
        "server-only recipes must be omitted from core-26.1.zip"
    );
    let prog_art_loc =
        bbb_client_assets::ResourceLocation::parse("minecraft:resourcepacks/programmer_art.zip")
            .unwrap();
    assert!(
        stack.get_resource(&prog_art_loc).is_none(),
        "programmer_art built-in pack must be omitted from core-26.1.zip"
    );
    let lang_loc = bbb_client_assets::ResourceLocation::parse("minecraft:lang/en_us.json").unwrap();
    assert!(stack.get_resource(&lang_loc).is_some());
    let _ = fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn control_ws_rejected_when_disabled_and_validates_origin_when_enabled() {
    let tmp = unique_temp_dir("bbb-web-ctrl-gate-test");
    let fixture_root = tmp.join("vanilla");
    create_synthetic_vanilla_fixture(&fixture_root);

    let disabled_server = WebHostServer::bind(WebHostConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        default_server: "127.0.0.1:25565".to_string(),
        allow_servers: Vec::new(),
        vanilla_root: fixture_root.clone(),
        wasm_dir: tmp.join("wasm"),
        cache_dir: tmp.join("cache-disabled"),
        default_username: "Tester".to_string(),
        default_locale: None,
        default_uuid: None,
        default_render_distance: None,
        server_resource_pack_policy: None,
        allow_pack_loopback: false,
        allow_pack_private: false,
        control_enabled: false,
    })
    .await
    .unwrap();
    let disabled_addr = disabled_server.local_addr();
    let (stop_disabled_tx, stop_disabled_rx) = oneshot::channel::<()>();
    let disabled_join = tokio::spawn(async move {
        disabled_server
            .serve(async {
                let _ = stop_disabled_rx.await;
            })
            .await
            .unwrap();
    });

    let disabled_res = client_ws_connect(disabled_addr, "/control-ws").await;
    assert!(
        matches!(disabled_res, Err((404, _))),
        "expected 404 when control_enabled == false, got {:?}",
        disabled_res.err()
    );

    let _ = stop_disabled_tx.send(());
    disabled_join.await.unwrap();

    let enabled_server = WebHostServer::bind(WebHostConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        default_server: "127.0.0.1:25565".to_string(),
        allow_servers: Vec::new(),
        vanilla_root: fixture_root,
        wasm_dir: tmp.join("wasm"),
        cache_dir: tmp.join("cache-enabled"),
        default_username: "Tester".to_string(),
        default_locale: None,
        default_uuid: None,
        default_render_distance: None,
        server_resource_pack_policy: None,
        allow_pack_loopback: false,
        allow_pack_private: false,
        control_enabled: true,
    })
    .await
    .unwrap();
    let enabled_addr = enabled_server.local_addr();
    let (stop_enabled_tx, stop_enabled_rx) = oneshot::channel::<()>();
    let enabled_join = tokio::spawn(async move {
        enabled_server
            .serve(async {
                let _ = stop_enabled_rx.await;
            })
            .await
            .unwrap();
    });

    let cross_origin = client_ws_connect_with_headers(
        enabled_addr,
        "/control-ws",
        &[("Origin", "https://evil.example")],
    )
    .await;
    assert!(
        matches!(cross_origin, Err((403, _))),
        "expected 403 for cross-origin /control-ws, got {:?}",
        cross_origin.err()
    );

    let null_origin =
        client_ws_connect_with_headers(enabled_addr, "/control-ws", &[("Origin", "null")]).await;
    assert!(
        matches!(null_origin, Err((403, _))),
        "expected 403 for null Origin /control-ws, got {:?}",
        null_origin.err()
    );

    let other_port = if enabled_addr.port() == u16::MAX {
        u16::MAX - 1
    } else {
        enabled_addr.port() + 1
    };
    let cross_port_loopback_origin = format!("http://localhost:{other_port}");
    let cross_port_loopback = client_ws_connect_with_headers(
        enabled_addr,
        "/control-ws",
        &[("Origin", cross_port_loopback_origin.as_str())],
    )
    .await;
    assert!(
        matches!(cross_port_loopback, Err((403, _))),
        "expected 403 for cross-port loopback /control-ws, got {:?}",
        cross_port_loopback.err()
    );

    let remote_host = format!("remote.example:{}", enabled_addr.port());
    let matching_port_loopback_origin = format!("http://localhost:{}", enabled_addr.port());
    let remote_host_loopback_origin = client_ws_connect_with_headers(
        enabled_addr,
        "/control-ws",
        &[
            ("Host", remote_host.as_str()),
            ("Origin", matching_port_loopback_origin.as_str()),
        ],
    )
    .await;
    assert!(
        matches!(remote_host_loopback_origin, Err((403, _))),
        "expected 403 for remote Host + loopback Origin /control-ws, got {:?}",
        remote_host_loopback_origin.err()
    );

    let same_origin_header = format!("http://{enabled_addr}");
    let mut same_origin = client_ws_connect_with_headers(
        enabled_addr,
        "/control-ws",
        &[("Origin", same_origin_header.as_str())],
    )
    .await
    .expect("expected 101 for same-origin /control-ws");

    let loopback_origin = client_ws_connect_with_headers(
        enabled_addr,
        "/control-ws",
        &[("Origin", matching_port_loopback_origin.as_str())],
    )
    .await
    .expect("expected 101 for matching-port loopback /control-ws");

    // Connecting a newer control tab must close the superseded session promptly.
    let superseded = tokio::time::timeout(std::time::Duration::from_secs(1), same_origin.recv())
        .await
        .expect("superseded /control-ws session must terminate when replaced");
    assert!(
        matches!(superseded, Ok(None) | Err(_)),
        "expected closed stream on superseded /control-ws, got {superseded:?}"
    );
    drop(same_origin);
    drop(loopback_origin);

    let _ = stop_enabled_tx.send(());
    enabled_join.await.unwrap();
    let _ = fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn proxy_endpoints_enforce_origin_and_stream_loopback_pack() {
    let upstream_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = upstream_listener.local_addr().unwrap();
    let upstream_task = tokio::spawn(async move {
        for idx in 0..2 {
            let (mut sock, _) = upstream_listener.accept().await.unwrap();
            let mut scratch = [0u8; 1024];
            let _ = sock.read(&mut scratch).await;
            if idx == 0 {
                sock.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: 8\r\n\r\nPK-fixed",
                )
                .await
                .unwrap();
            } else {
                sock.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nTransfer-Encoding: chunked\r\n\r\n8\r\nPK-chunk\r\n0\r\n\r\n",
                )
                .await
                .unwrap();
            }
        }
    });

    let tmp = unique_temp_dir("bbb-web-proxy-origin-test");
    let fixture_root = tmp.join("vanilla");
    create_synthetic_vanilla_fixture(&fixture_root);

    let server = WebHostServer::bind(WebHostConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        default_server: "127.0.0.1:25565".to_string(),
        allow_servers: Vec::new(),
        vanilla_root: fixture_root,
        wasm_dir: tmp.join("wasm"),
        cache_dir: tmp.join("cache"),
        default_username: "Tester".to_string(),
        default_locale: None,
        default_uuid: None,
        default_render_distance: None,
        server_resource_pack_policy: None,
        allow_pack_loopback: true,
        allow_pack_private: false,
        control_enabled: false,
    })
    .await
    .unwrap();
    let addr = server.local_addr();
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        server
            .serve(async {
                let _ = stop_rx.await;
            })
            .await
            .unwrap();
    });

    let pack_url = format!("http://{upstream_addr}/pack.zip");
    let encoded_pack: String = url::form_urlencoded::byte_serialize(pack_url.as_bytes()).collect();
    let pack_target = format!("/proxy/pack?url={encoded_pack}");
    let skin_target =
        "/proxy/skin?url=https%3A%2F%2Fapi.mojang.com%2Fusers%2Fprofiles%2Fminecraft%2FNotch";

    // Cross-origin and cross-site requests to /proxy/skin and /proxy/pack must fail with 403.
    let other_port = if addr.port() == u16::MAX {
        u16::MAX - 1
    } else {
        addr.port() + 1
    };
    let wrong_port_origin = format!("http://localhost:{other_port}");
    for target in [pack_target.as_str(), skin_target] {
        let (st_evil, _, _) =
            raw_http_get(addr, target, &[("Origin", "https://evil.example")]).await;
        assert_eq!(st_evil, 403, "expected 403 for evil Origin on {target}");

        let (st_port, _, _) =
            raw_http_get(addr, target, &[("Origin", wrong_port_origin.as_str())]).await;
        assert_eq!(
            st_port, 403,
            "expected 403 for cross-port Origin on {target}"
        );

        let (st_cross_site, _, _) =
            raw_http_get(addr, target, &[("Sec-Fetch-Site", "cross-site")]).await;
        assert_eq!(
            st_cross_site, 403,
            "expected 403 for Sec-Fetch-Site: cross-site on {target}"
        );

        let (st_same_site, _, _) =
            raw_http_get(addr, target, &[("Sec-Fetch-Site", "same-site")]).await;
        assert_eq!(
            st_same_site, 403,
            "expected 403 for Sec-Fetch-Site: same-site on {target}"
        );
    }

    // Browser same-origin GET (absent Origin + Sec-Fetch-Site: same-origin) succeeds and streams.
    let (st_fixed, head_fixed, body_fixed) =
        raw_http_get(addr, &pack_target, &[("Sec-Fetch-Site", "same-origin")]).await;
    assert_eq!(st_fixed, 200);
    assert!(head_fixed
        .to_ascii_lowercase()
        .contains("content-length: 8"));
    assert_eq!(body_fixed, b"PK-fixed");

    let matching_origin = format!("http://localhost:{}", addr.port());
    let (st_chunked, head_chunked, body_chunked) = raw_http_get(
        addr,
        &pack_target,
        &[
            ("Origin", matching_origin.as_str()),
            ("Sec-Fetch-Site", "same-origin"),
        ],
    )
    .await;
    assert_eq!(st_chunked, 200);
    assert!(head_chunked
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked"));
    assert_eq!(body_chunked, b"8\r\nPK-chunk\r\n0\r\n\r\n");

    upstream_task.await.unwrap();
    let _ = stop_tx.send(());
    join.await.unwrap();
    let _ = fs::remove_dir_all(&tmp);
}
