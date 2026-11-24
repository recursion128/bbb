use std::{net::SocketAddr, time::Duration};

use bbb_protocol::{
    codec::offline_player_uuid,
    game_types::GameType,
    packets::{
        decode_login_clientbound, encode_handshake, encode_login_hello, ClientIntent,
        LoginClientbound, PlayClientbound, PlayerAbilities,
    },
};
use clap::Parser;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use super::{
    join_play_client, options_with, probe_status, temp_test_dir, test_options, RunningServer,
    TestClient,
};
use crate::{ServerOptions, ServerRuntime};

/// One MCP request/response over the management endpoint, on its own
/// connection: the transport is `connection: close` here so the test never has
/// to guess where an answer ends.
pub(super) async fn management_call(
    address: SocketAddr,
    session: Option<&str>,
    body: serde_json::Value,
) -> (u16, Vec<(String, String)>, serde_json::Value) {
    let body = body.to_string();
    let mut request = String::from("POST /mcp HTTP/1.1\r\nhost: 127.0.0.1\r\n");
    request.push_str("accept: application/json, text/event-stream\r\n");
    request.push_str("content-type: application/json\r\nconnection: close\r\n");
    if let Some(session) = session {
        request.push_str(&format!("mcp-session-id: {session}\r\n"));
    }
    request.push_str(&format!("content-length: {}\r\n\r\n{body}", body.len()));

    let mut stream = TcpStream::connect(address).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut received = Vec::new();
    // A noise world finds its spawn before the endpoint serves the backlog.
    tokio::time::timeout(Duration::from_secs(60), stream.read_to_end(&mut received))
        .await
        .expect("the management endpoint answers")
        .unwrap();
    let received = String::from_utf8(received).unwrap();
    let (head, payload) = received.split_once("\r\n\r\n").expect("a response head");
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .expect("a status code");
    let headers = lines
        .filter_map(|line| line.split_once(": "))
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    let document = serde_json::from_str(payload).unwrap_or(serde_json::Value::Null);
    (status, headers, document)
}

pub(super) async fn management_session(address: SocketAddr) -> String {
    let (status, headers, document) = management_call(
        address,
        None,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {} },
        }),
    )
    .await;
    assert_eq!(status, 200, "{document}");
    assert_eq!(document["result"]["protocolVersion"], "2025-06-18");
    let session = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("mcp-session-id"))
        .map(|(_, value)| value.clone())
        .expect("a session id");
    let (notify_status, _, notify_doc) = management_call(
        address,
        Some(&session),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        }),
    )
    .await;
    assert_eq!(notify_status, 202, "{notify_doc}");
    session
}

pub(super) async fn management_tool(
    address: SocketAddr,
    session: &str,
    id: i64,
    name: &str,
    arguments: serde_json::Value,
) -> serde_json::Value {
    let (status, _, document) = management_call(
        address,
        Some(session),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments },
        }),
    )
    .await;
    assert_eq!(status, 200, "{document}");
    assert!(document.get("error").is_none(), "{document}");
    document["result"].clone()
}

pub(super) async fn exec_command(
    address: SocketAddr,
    session: &str,
    id: i64,
    command: impl Into<String>,
) -> serde_json::Value {
    management_tool(
        address,
        session,
        id,
        "command.execute",
        serde_json::json!({ "command": command.into() }),
    )
    .await
}

#[tokio::test]
async fn mcp_world_inspection_reads_live_block_properties_and_surface() {
    use serde_json::json;

    let server = RunningServer::start(options_with(&["--mcp-loopback-scope", "command"])).await;
    let client = join_play_client(server.address, "inspect-build").await;
    let x = client.spawn.0.floor() as i32;
    let z = client.spawn.2.floor() as i32;
    let address = server.management_addr();
    let session = management_session(address).await;
    let info = management_tool(address, &session, 2, "world.info", json!({})).await;
    let world = &info["structuredContent"];
    assert_eq!(world["spawn"]["dimension"], "minecraft:overworld");
    assert!(world["dimensions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dimension| dimension["id"] == "minecraft:overworld"));

    let placed = exec_command(
        address,
        &session,
        3,
        format!("setblock {x} 90 {z} minecraft:oak_log[axis=x]"),
    )
    .await;
    assert_eq!(
        placed["structuredContent"]["command"]["result"], 1,
        "{placed}"
    );

    let blocks = management_tool(
        address,
        &session,
        4,
        "world.blocks",
        json!({
            "dimension": "minecraft:overworld",
            "positions": [{"x":x,"y":90,"z":z},{"x":4096,"y":90,"z":4096}]
        }),
    )
    .await;
    let blocks = &blocks["structuredContent"]["blocks"];
    assert_eq!(blocks[0]["state"]["name"], "minecraft:oak_log");
    assert_eq!(blocks[0]["state"]["properties"]["axis"], "x");
    assert_eq!(blocks[1]["state"], json!(null));

    for (id, (x, z), loaded, top_y) in [(5, (x, z), true, Some(90)), (6, (4096, 4096), false, None)]
    {
        let surface = management_tool(
            address,
            &session,
            id,
            "world.surface",
            json!({
                "dimension":"minecraft:overworld",
                "minX":x,"minZ":z,"maxX":x,"maxZ":z
            }),
        )
        .await;
        assert_eq!(
            surface["structuredContent"]["columns"],
            json!([{"x":x,"z":z,"loaded":loaded,"topY":top_y}]),
            "{surface}"
        );
    }
    let oversized = management_tool(
        address,
        &session,
        7,
        "world.surface",
        json!({"dimension":"minecraft:overworld","minX":0,"minZ":0,"maxX":16,"maxZ":15}),
    )
    .await;
    assert_eq!(oversized["isError"], true, "{oversized}");
    server.stop().await;
}

pub(super) async fn read_setting(
    management: SocketAddr,
    session: &str,
    id: i64,
    key: &str,
) -> serde_json::Value {
    let settings = management_tool(
        management,
        session,
        id,
        "serversettings",
        serde_json::json!({}),
    )
    .await;
    settings["structuredContent"]["settings"]
        .as_array()
        .expect("settings")
        .iter()
        .find(|entry| entry["key"] == key)
        .expect("listed key")["value"]
        .clone()
}

/// Polls `if loaded` until the chunk holding block `x, z` is: a forceload
/// ticket queues its column's generation rather than running it inside the
/// command. Request ids count up from `id`, since the endpoint replays a
/// repeated one.
pub(super) async fn wait_until_loaded(
    address: SocketAddr,
    session: &str,
    mut id: i64,
    execute: &str,
    (x, z): (i32, i32),
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let answer = exec_command(
            address,
            session,
            id,
            format!("{execute} if loaded {x} 0 {z}"),
        )
        .await;
        if answer["structuredContent"]["command"]["result"] == 1 {
            return;
        }
        assert!(tokio::time::Instant::now() < deadline, "{answer}");
        id += 1;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn management_settings_reach_the_tick_and_the_status_the_next_client_reads() {
    let server = RunningServer::start(test_options()).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    // The readout answers from the world, so it already carries the motd the
    // command line asked for.
    assert_eq!(
        read_setting(management, &session, 2, "motd").await,
        "runtime fixture"
    );

    let written = management_tool(
        management,
        &session,
        3,
        "serversettings.set",
        serde_json::json!({ "key": "motd", "value": "managed fixture" }),
    )
    .await;
    assert_eq!(written["isError"], false, "{written}");
    assert_eq!(
        written["structuredContent"]["setting"]["value"],
        "managed fixture"
    );

    // The next status probe reads the value the tick applied, not the one the
    // process started with.
    let (_, status) = probe_status(server.address).await;
    assert_eq!(status["description"]["text"], "managed fixture");
    server.stop().await;
}

/// `--force-gamemode` reaches the world the runtime builds.
///
/// A fresh world cannot tell the two settings apart at the join: with no saved
/// `playerGameType` both branches of
/// `ServerPlayer.calculateGameModeForNewPlayer` (ServerPlayer.java:2029-2036)
/// answer with the level default. What separates them is
/// `/defaultgamemode`, whose `enforceGameTypeForPlayers(getForcedGameType())`
/// (DefaultGameModeCommands.java:23) moves the sessions already placed only
/// while `getForcedGameType` has an answer (DedicatedServer.java:792-794).
/// The unforced half of the pair is
/// `bbb_server_core`'s `defaultgamemode_moves_the_next_joins_mode_and_the_level_it_is_saved_in`.
#[tokio::test]
async fn the_force_game_mode_flag_reaches_the_world() {
    // `/defaultgamemode` is a command, and the command surface is the one
    // scope a loopback caller is not granted by default.
    let server = RunningServer::start(options_with(&[
        "--force-gamemode",
        "--mcp-loopback-scope",
        "command",
    ]))
    .await;
    let management = server.management_addr();
    let mut client = join_play_client(server.address, "forced").await;
    let session = management_session(management).await;
    let ran = exec_command(management, &session, 2, "defaultgamemode survival").await;
    assert_eq!(ran["isError"], false, "{ran}");

    // `ServerPlayer.setGameMode` (ServerPlayer.java:1734-1756) reaches the
    // session it moved through `onUpdateAbilities`. The join burst's own tail
    // is still arriving, so this reads past it; a runtime that dropped the flag
    // sends no second abilities packet at all and the wait runs out.
    let moved = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let PlayClientbound::PlayerAbilities(update) = client.read_play().await {
                return update;
            }
        }
    })
    .await
    .expect("the forced game mode reaches the session already playing");
    assert_eq!(moved, PlayerAbilities::for_game_type(GameType::Survival));
    server.stop().await;
}

#[tokio::test]
async fn a_management_stop_ends_the_process_the_way_a_signal_does() {
    let server = RunningServer::start(test_options()).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    let stopping = management_tool(
        management,
        &session,
        2,
        "server.stop",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(stopping["isError"], false, "{stopping}");
    assert_eq!(stopping["structuredContent"]["stopping"], true);

    // No signal was sent: the tick that applied the request is what ends the
    // run, and it still returns a report.
    let report = server.wait_stopped().await;
    assert_eq!(report.rejected_connections, 0);
}

#[tokio::test]
async fn stored_user_lists_are_written_where_the_command_line_said() {
    let directory = temp_test_dir("admin-lists");
    let admin_arg = directory.to_str().unwrap();
    let server = RunningServer::start(options_with(&["--admin-dir", admin_arg])).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    let added = management_tool(
        management,
        &session,
        2,
        "allowlist.add",
        serde_json::json!({ "add": [{ "name": "bbb-client" }] }),
    )
    .await;
    assert_eq!(added["isError"], false, "{added}");
    assert_eq!(
        added["structuredContent"]["allowlist"][0]["name"],
        "bbb-client"
    );

    // The world published a revision, and the runtime is the only thing that
    // writes it down.
    let document = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(bytes) = std::fs::read(directory.join("whitelist.json")) {
                if !bytes.is_empty() {
                    return bytes;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the allowlist reaches disk");
    let document: serde_json::Value = serde_json::from_slice(&document).unwrap();
    assert_eq!(document[0]["name"], "bbb-client");
    assert!(document[0]["uuid"].is_string());
    server.stop().await;

    // A restart reads them back, which is the only reason to write them at all.
    let server = RunningServer::start(options_with(&["--admin-dir", admin_arg])).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    let restored =
        management_tool(management, &session, 2, "allowlist", serde_json::json!({})).await;
    assert_eq!(
        restored["structuredContent"]["allowlist"][0]["name"],
        "bbb-client"
    );
    server.stop().await;
    std::fs::remove_dir_all(&directory).ok();
}

/// `/whitelist on|off` and `/setidletimeout` save `server.properties`
/// (`DedicatedServer.java:324-335`, `:611-613`); a restart reads the saved
/// values unless a flag names them explicitly.
#[tokio::test]
async fn allowlist_and_idle_settings_survive_a_restart_below_explicit_flags() {
    let directory = temp_test_dir("admin-settings");
    let file = directory.join("server-settings.json");
    // One boot: run the commands, wait for the file to say `expected`, stop.
    // Answers the `use_allowlist` the boot started with.
    let boot = |flags: &'static [&'static str],
                commands: &'static [&'static str],
                expected: serde_json::Value| {
        let directory = directory.clone();
        let file = file.clone();
        async move {
            let mut args = vec![
                "bbb-server",
                "--bind",
                "127.0.0.1:0",
                "--mcp-bind",
                "127.0.0.1:0",
                "--mcp-loopback-scope",
                "command",
                "--admin-dir",
                directory.to_str().unwrap(),
            ];
            args.extend_from_slice(flags);
            let server = RunningServer::start(ServerOptions::try_parse_from(args).unwrap()).await;
            let management = server.management_addr();
            let session = management_session(management).await;
            let use_allowlist = read_setting(management, &session, 2, "use_allowlist").await;
            for (id, command) in (3..).zip(commands) {
                let ran = exec_command(management, &session, id, *command).await;
                assert_eq!(ran["isError"], false, "{ran}");
            }
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let saved = std::fs::read(&file)
                        .ok()
                        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
                    // The file carries every saved property; these are the
                    // ones this test moves.
                    if saved.as_ref().is_some_and(|saved| {
                        expected
                            .as_object()
                            .expect("an object")
                            .iter()
                            .all(|(key, value)| saved.get(key) == Some(value))
                    }) {
                        return;
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap_or_else(|_| panic!("{} never became {expected}", file.display()));
            server.stop().await;
            use_allowlist
        }
    };
    let saved = |white_list: bool, minutes: i32| {
        serde_json::json!({
            "white-list": white_list,
            "enforce-whitelist": true,
            "player-idle-timeout": minutes,
        })
    };

    let started = boot(
        &["--enforce-whitelist", "true"],
        &["whitelist on", "setidletimeout 7"],
        saved(true, 7),
    )
    .await;
    assert_eq!(started, false);
    // No flags: the file's values, including the idle timeout the next save
    // writes back unchanged.
    let started = boot(&[], &["whitelist off"], saved(false, 7)).await;
    assert_eq!(started, true);
    // Explicit flags outrank the file.
    let started = boot(
        &["--white-list", "true", "--player-idle-timeout", "2"],
        &["whitelist off"],
        saved(false, 2),
    )
    .await;
    assert_eq!(started, true);
    std::fs::remove_dir_all(&directory).ok();
}

#[tokio::test]
async fn the_management_endpoint_can_be_switched_off_entirely() {
    let options =
        ServerOptions::try_parse_from(["bbb-server", "--bind", "127.0.0.1:0", "--no-mcp"]).unwrap();
    let runtime = ServerRuntime::bind(options).await.unwrap();
    assert_eq!(runtime.management_addr().unwrap(), None);
}

/// `DedicatedServerSettings.update` (`DedicatedServerSettings.java:23-26`)
/// writes a management change back, so the next start reads it; a flag given
/// at that start still wins over the file.
#[tokio::test]
async fn management_setting_changes_survive_a_restart_below_explicit_flags() {
    let directory = temp_test_dir("saved-settings");
    let start = |extra: &[&str]| {
        let mut arguments = vec![
            "bbb-server",
            "--bind",
            "127.0.0.1:0",
            "--mcp-bind",
            "127.0.0.1:0",
            "--admin-dir",
            directory.to_str().unwrap(),
        ];
        arguments.extend_from_slice(extra);
        ServerOptions::try_parse_from(arguments).unwrap()
    };

    let server = RunningServer::start(start(&[])).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    assert_eq!(
        read_setting(management, &session, 9, "view_distance").await,
        10
    );
    for (id, key, value) in [
        (2, "motd", serde_json::json!("saved motd")),
        (3, "view_distance", serde_json::json!(7)),
        (4, "max_players", serde_json::json!(5)),
        (5, "difficulty", serde_json::json!("hard")),
    ] {
        let written = management_tool(
            management,
            &session,
            id,
            "serversettings.set",
            serde_json::json!({ "key": key, "value": value }),
        )
        .await;
        assert_eq!(written["isError"], false, "{written}");
    }
    server.stop().await;
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("server-settings.json")).unwrap())
            .unwrap();
    assert_eq!(saved["motd"], "saved motd");
    assert_eq!(saved["view-distance"], 7);

    let server = RunningServer::start(start(&["--motd", "flag motd"])).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    for (id, key, expected) in [
        (9, "motd", serde_json::json!("flag motd")),
        (10, "view_distance", serde_json::json!(7)),
        (11, "max_players", serde_json::json!(5)),
        (12, "difficulty", serde_json::json!("hard")),
    ] {
        assert_eq!(read_setting(management, &session, id, key).await, expected);
    }
    server.stop().await;
    std::fs::remove_dir_all(directory).unwrap();
}

/// `ServerLoginPacketListenerImpl.java:140-145` asks `canPlayerLogin` in the
/// login phase, so a full server or an unlisted profile is refused with a
/// login disconnect before configuration starts.
#[tokio::test]
async fn a_full_server_refuses_the_next_login_in_the_login_phase() {
    let mut options = test_options();
    options.max_players = Some(1);
    let server = RunningServer::start(options).await;
    let first = join_play_client(server.address, "first").await;
    let refusal = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let mut client = TestClient::connect(server.address).await;
            client
                .send(
                    encode_handshake("localhost", server.address.port(), ClientIntent::Login)
                        .unwrap(),
                )
                .await;
            client
                .send(encode_login_hello("second", offline_player_uuid("second")).unwrap())
                .await;
            let (packet_id, payload) = client.read().await;
            if let packet @ LoginClientbound::Disconnect { .. } =
                decode_login_clientbound(packet_id, &payload).unwrap()
            {
                return packet;
            }
            // The count reached the login check a tick after the join.
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("the second login is refused");
    assert!(
        matches!(&refusal, LoginClientbound::Disconnect { raw_reason, .. }
            if raw_reason.contains("multiplayer.disconnect.server_full")),
        "{refusal:?}"
    );
    drop(first);
    server.stop().await;
}
