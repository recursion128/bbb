use std::{path::Path, time::Duration};

use bbb_protocol::{
    block_state_types::vanilla_block_state_identity,
    game_rules::GameRuleValue,
    game_types::GameType,
    packets::{
        encode_play_accept_teleportation, encode_play_player_loaded,
        serverbound::{encode_play_chunk_batch_received, encode_play_perform_respawn},
        ClientIntent, PlayClientbound,
    },
    NbtSummaryValue,
};
use bbb_server_datapack::{
    registry_data_loader::DatapackLoadLimits,
    startup::{DatapackStartup, DatapackStartupOptions},
};
use bbb_server_persistence::{
    Java26LoadedChunk, Java26WorldCandidate, Java26WorldStorage, RegionCompression,
};
use bbb_server_types::save::{WorldSaveLimits, WorldSavePaths};
use clap::Parser;

use super::{
    configure_client,
    management::{exec_command, management_session, wait_until_loaded},
    options_with, temp_test_dir, RunningServer,
};
use crate::{ServerOptions, ServerRuntime};

fn custom_flat_world_options(root: &Path) -> ServerOptions {
    ServerOptions::try_parse_from([
        "bbb-server",
        // The fixtures below count chunks at the world's own radius and
        // drive an empty server that must keep ticking.
        "--view-distance",
        "2",
        "--simulation-distance",
        "2",
        "--pause-when-empty-seconds",
        "0",
        "--bind",
        "127.0.0.1:0",
        "--mcp-bind",
        "127.0.0.1:0",
        "--mcp-loopback-scope",
        "command",
        "--world",
        root.to_str().expect("the fixture path is UTF-8"),
        "--level-type",
        "flat",
        // A village the random seed puts on the origin would lift the spawn.
        "--generate-structures",
        "false",
    ])
    .unwrap()
}

fn write_seeded_pack_save(
    root: &Path,
    options: &ServerOptions,
    pack_id: &str,
    mark_uninitialized: bool,
) {
    let mut save = crate::runtime::created_world(root, options).unwrap();
    let NbtSummaryValue::Compound(data) = &mut save.level.entries_mut()[0].value else {
        panic!("created level data is a compound")
    };
    if mark_uninitialized {
        data.iter_mut()
            .find(|entry| entry.name == "initialized")
            .unwrap()
            .value = NbtSummaryValue::Byte(0);
    }
    let packs = data
        .iter_mut()
        .find(|entry| entry.name == "DataPacks")
        .unwrap();
    let NbtSummaryValue::Compound(packs) = &mut packs.value else {
        unreachable!()
    };
    let enabled = packs
        .iter_mut()
        .find(|entry| entry.name == "Enabled")
        .unwrap();
    let NbtSummaryValue::List(enabled) = &mut enabled.value else {
        unreachable!()
    };
    enabled.push(NbtSummaryValue::string(pack_id));

    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    storage
        .create_world(&save, RegionCompression::Zlib, 1)
        .unwrap();
    storage.close().unwrap();
}

#[rustfmt::skip]
fn write_custom_flat_pack(root: &Path) {
    let pack = root.join("datapacks/moon");
    std::fs::create_dir_all(pack.join("data/example/dimension")).unwrap();
    std::fs::create_dir_all(pack.join("data/example/dimension_type")).unwrap();
    let snapshot = bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot::embedded_shared().unwrap();
    let mut dimension_type: serde_json::Value =
        serde_json::from_slice(snapshot.read_path("dimension_type/overworld.json").unwrap()).unwrap();
    dimension_type["min_y"] = serde_json::json!(-16);
    dimension_type["height"] = serde_json::json!(64);
    dimension_type["logical_height"] = serde_json::json!(64);
    std::fs::write(pack.join("data/example/dimension_type/moon_type.json"), serde_json::to_vec(&dimension_type).unwrap()).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"moon","min_format":101,"max_format":101}}"#).unwrap();
    std::fs::write(pack.join("data/example/dimension/moon.json"), r#"{"type":"example:moon_type","generator":{"type":"minecraft:flat","settings":{"biome":"minecraft:plains","features":false,"lakes":false,"layers":[{"block":"minecraft:stone","height":1}],"structure_overrides":[]}}}"#).unwrap();
}

#[rustfmt::skip]
fn write_debug_overworld_pack(root: &Path) {
    let pack = root.join("datapacks/debug_overworld");
    std::fs::create_dir_all(pack.join("data/minecraft/dimension")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"debug overworld","min_format":101,"max_format":101}}"#).unwrap();
    std::fs::write(pack.join("data/minecraft/dimension/overworld.json"), r#"{"type":"minecraft:overworld","generator":{"type":"minecraft:debug"}}"#).unwrap();
}

/// A pack that changes the built-in Overworld type and adds `example:deep`, a
/// noise level over Overworld settings in a 0..256 dimension type.
#[rustfmt::skip]
fn write_deep_noise_pack(root: &Path) {
    let pack = root.join("datapacks/deep");
    for dir in ["data/example/dimension", "data/example/dimension_type", "data/minecraft/dimension_type"] {
        std::fs::create_dir_all(pack.join(dir)).unwrap();
    }
    let snapshot = bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot::embedded_shared().unwrap();
    let overworld: serde_json::Value =
        serde_json::from_slice(snapshot.read_path("dimension_type/overworld.json").unwrap()).unwrap();
    let mut modified = overworld.clone();
    modified["ambient_light"] = serde_json::json!(0.25);
    modified["height"] = serde_json::json!(320);
    modified["logical_height"] = serde_json::json!(320);
    std::fs::write(pack.join("data/minecraft/dimension_type/overworld.json"), serde_json::to_vec(&modified).unwrap()).unwrap();
    let mut short = overworld;
    short["min_y"] = serde_json::json!(0);
    short["height"] = serde_json::json!(256);
    short["logical_height"] = serde_json::json!(256);
    std::fs::write(pack.join("data/example/dimension_type/short.json"), serde_json::to_vec(&short).unwrap()).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"deep","min_format":101,"max_format":101}}"#).unwrap();
    std::fs::write(pack.join("data/example/dimension/deep.json"), r#"{"type":"example:short","generator":{"type":"minecraft:noise","biome_source":{"type":"minecraft:multi_noise","preset":"minecraft:overworld"},"settings":"minecraft:overworld"}}"#).unwrap();
}

fn load_runtime_candidate(root: &Path) -> (Java26WorldStorage, Java26WorldCandidate) {
    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    let bootstrap = storage.load_bootstrap().unwrap();
    let startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: Some(root.join("datapacks")),
        saved_data_packs: bootstrap.level().data_packs().cloned(),
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap();
    let candidate = bbb_server_core::world_creation::load_candidate_from_bootstrap(
        &storage,
        bootstrap,
        &startup.registries,
    )
    .unwrap();
    (storage, candidate)
}

fn saved_block_state(
    chunk: &Java26LoadedChunk,
    local_x: usize,
    y: i32,
    local_z: usize,
) -> &bbb_server_persistence::Java26BlockState {
    let section_y = i8::try_from(y.div_euclid(16)).unwrap();
    let local_y = usize::try_from(y.rem_euclid(16)).unwrap();
    chunk
        .data
        .sections()
        .iter()
        .find(|section| section.y == section_y)
        .and_then(|section| section.block_states.as_ref())
        .and_then(|states| states.value_at(local_x, local_y, local_z))
        .expect("the debug state is saved")
}

fn overworld_chunk(candidate: &Java26WorldCandidate, chunk: (i32, i32)) -> Java26LoadedChunk {
    saved_chunk(candidate, "minecraft:overworld", chunk)
        .expect("the Overworld debug chunk is saved")
}

fn saved_chunk(
    candidate: &Java26WorldCandidate,
    dimension: &str,
    (x, z): (i32, i32),
) -> Option<Java26LoadedChunk> {
    let (key, _) = candidate
        .dimensions
        .iter()
        .find(|(key, _)| key.resource_id() == dimension)?;
    let reader = candidate.column_reader.as_ref()?;
    reader.read_column(key, x, z).unwrap().chunk
}

/// A saved chunk's raw document less its light layers, light-only sections and
/// `isLightOn`.
fn chunk_without_light(root: &Path, (x, z): (i32, i32)) -> Vec<bbb_protocol::NbtSummaryEntry> {
    let region_dir = WorldSavePaths::new(root).unwrap().region_dir();
    let store = bbb_server_persistence::RegionFileStore::new(
        region_dir.join(format!("r.{}.{}.mca", x >> 5, z >> 5)),
        region_dir,
        WorldSaveLimits::default(),
    )
    .unwrap();
    let raw = store
        .read_chunk(x, z)
        .unwrap()
        .expect("the Overworld debug chunk is saved");
    store.close_cached();
    let mut entries = raw.entries().to_vec();
    entries.retain(|entry| entry.name != "isLightOn");
    for entry in &mut entries {
        if entry.name != "sections" {
            continue;
        }
        let NbtSummaryValue::List(sections) = &mut entry.value else {
            continue;
        };
        sections.retain_mut(|section| {
            let NbtSummaryValue::Compound(fields) = section else {
                return true;
            };
            fields.retain(|field| field.name != "BlockLight" && field.name != "SkyLight");
            fields.iter().any(|field| field.name != "Y")
        });
    }
    entries
}

fn assert_debug_candidate(candidate: &Java26WorldCandidate, game_type: GameType) -> i64 {
    assert!(candidate.level.initialized());
    assert_eq!(candidate.level.spawn().dimension(), "minecraft:overworld");
    assert_eq!(candidate.level.spawn().position(), [0, 80, 0]);
    assert_eq!(candidate.level.spawn().yaw(), 0.0);
    assert_eq!(candidate.level.spawn().pitch(), 0.0);
    assert_eq!(candidate.level.game_type(), game_type.id());
    assert_eq!(candidate.level.difficulty_settings().difficulty(), "easy");
    assert!(candidate.level.difficulty_settings().locked());
    assert_eq!(
        candidate.game_rules.get("minecraft:advance_weather"),
        Some(&GameRuleValue::Boolean(false))
    );

    let overworld_clock = candidate
        .world_clocks
        .entries()
        .iter()
        .find(|entry| entry.resource_id == "minecraft:overworld")
        .expect("the live Overworld clock is saved")
        .state;
    assert!(overworld_clock.total_ticks >= 6_000);
    assert!(overworld_clock.partial_tick.is_finite());
    assert!((0.0..1.0).contains(&overworld_clock.partial_tick));
    assert!(overworld_clock.rate.is_finite() && overworld_clock.rate > 0.0);
    assert!(!overworld_clock.paused);

    let chunk = &overworld_chunk(candidate, (1, 0));
    let source = bbb_server_datapack::debug_generator::DebugLevelSource::new(-64, 384).unwrap();
    for local_x in 0..16 {
        for local_z in 0..16 {
            let world_x = 16 + i32::try_from(local_x).unwrap();
            let world_z = i32::try_from(local_z).unwrap();
            for y in [60, 70] {
                let actual = saved_block_state(chunk, local_x, y, local_z);
                let expected =
                    vanilla_block_state_identity(source.block_state_at(world_x, y, world_z))
                        .expect("the debug source returns a Vanilla registry state");
                assert_eq!(actual.name, expected.name);
                assert_eq!(actual.properties, expected.properties);
            }
        }
    }
    overworld_clock.total_ticks
}

#[tokio::test]
async fn custom_flat_world_saves_and_reopens_through_the_runtime() {
    let root = temp_test_dir("custom-level-runtime");
    write_custom_flat_pack(&root);
    let options = custom_flat_world_options(&root);
    write_seeded_pack_save(&root, &options, "file/moon", false);

    let server = RunningServer::start(options).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    let forced = exec_command(
        management,
        &session,
        2,
        "execute in example:moon run forceload add 0 0",
    )
    .await;
    assert_eq!(
        forced["structuredContent"]["command"]["result"], 1,
        "{forced}"
    );
    wait_until_loaded(management, &session, 100, "execute in example:moon", (0, 0)).await;
    let floor = exec_command(
        management,
        &session,
        3,
        "execute in example:moon if block 0 -16 0 minecraft:stone",
    )
    .await;
    assert_eq!(
        floor["structuredContent"]["command"]["result"], 1,
        "{floor}"
    );
    // `--level-type flat` creates Vanilla's classic superflat overworld.
    for (id, check) in [
        "run forceload add 0 0",
        "if block 0 -64 0 minecraft:bedrock",
        "if block 0 -63 0 minecraft:dirt",
        "if block 0 -62 0 minecraft:dirt",
        "if block 0 -61 0 minecraft:grass_block",
        "if block 0 -60 0 minecraft:air",
        "if biome 0 -61 0 minecraft:plains",
    ]
    .into_iter()
    .enumerate()
    {
        let result = exec_command(
            management,
            &session,
            10 + i64::try_from(id).unwrap(),
            format!("execute {check}"),
        )
        .await;
        assert_eq!(
            result["structuredContent"]["command"]["result"], 1,
            "{check}: {result}"
        );
    }
    let changed = exec_command(
        management,
        &session,
        4,
        "execute in example:moon run setblock 0 20 0 minecraft:diamond_block",
    )
    .await;
    assert_eq!(
        changed["structuredContent"]["command"]["result"], 1,
        "{changed}"
    );
    server.stop_within(Duration::from_secs(10)).await;

    let (storage, candidate) = load_runtime_candidate(&root);
    // `FlatLevelSource.getSpawnHeight` is the top of the four layers, and the
    // spiral's first column is the spawn chunk's corner.
    assert_eq!(candidate.level.spawn().position(), [0, -60, 0]);
    storage.close().unwrap();
    let custom = root.join("dimensions/example/moon");
    assert!(custom.join("region/r.0.0.mca").is_file());
    assert!(custom.join("data/minecraft/world_border.dat").is_file());
    assert!(custom.join("data/minecraft/chunk_tickets.dat").is_file());

    let server = RunningServer::start(custom_flat_world_options(&root)).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    let restored = exec_command(
        management,
        &session,
        2,
        "execute in example:moon if block 0 20 0 minecraft:diamond_block",
    )
    .await;
    assert_eq!(
        restored["structuredContent"]["command"]["result"], 1,
        "{restored}"
    );
    server.stop_within(Duration::from_secs(10)).await;
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn custom_debug_world_saves_and_reopens_through_the_runtime() {
    let root = temp_test_dir("debug-runtime");
    write_debug_overworld_pack(&root);
    let options = custom_flat_world_options(&root);
    write_seeded_pack_save(&root, &options, "file/debug_overworld", true);

    let (storage, candidate) = load_runtime_candidate(&root);
    assert!(!candidate.level.initialized());
    assert_eq!(candidate.level.game_type(), GameType::Survival.id());
    storage.close().unwrap();

    let assert_edits_rejected = |management, session: String| async move {
        for (id, command) in [
            (3, "setblock 17 70 1 minecraft:diamond_block"),
            (4, "fill 16 60 0 31 60 15 minecraft:diamond_block"),
        ] {
            let reply = exec_command(management, &session, id, command).await;
            assert!(
                reply["structuredContent"]["command"]["result"].is_null(),
                "{reply}"
            );
        }
    };

    let server = RunningServer::start(options).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    let forced = exec_command(management, &session, 2, "forceload add 16 0").await;
    assert_eq!(
        forced["structuredContent"]["command"]["result"], 1,
        "{forced}"
    );
    wait_until_loaded(management, &session, 100, "execute", (16, 0)).await;
    assert_edits_rejected(management, session).await;
    server.stop_within(Duration::from_secs(10)).await;

    assert!(root.join("data/minecraft/world_clocks.dat").is_file());
    let (storage, candidate) = load_runtime_candidate(&root);
    let first_clock = assert_debug_candidate(&candidate, GameType::Spectator);
    let first_chunk = chunk_without_light(&root, (1, 0));
    storage.close().unwrap();

    let server = RunningServer::start(custom_flat_world_options(&root)).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    let queried = exec_command(management, &session, 2, "forceload query 16 0").await;
    assert_eq!(
        queried["structuredContent"]["command"]["result"], 1,
        "{queried}"
    );
    assert_edits_rejected(management, session).await;
    server.stop_within(Duration::from_secs(10)).await;

    let (storage, candidate) = load_runtime_candidate(&root);
    // `DedicatedServer.initServer` reapplies the startup game mode on every run
    // (`DedicatedServer.java:227-238`); Debug's Spectator default is first-init only.
    let second_clock = assert_debug_candidate(&candidate, GameType::Survival);
    assert!(second_clock >= first_clock);
    assert_eq!(chunk_without_light(&root, (1, 0)), first_chunk);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// Boots the runtime on `root` and stops it at once, which runs the final save.
async fn boot_and_stop(root: &Path, level_seed: &str) {
    let options = ServerOptions::try_parse_from([
        "bbb-server",
        // The fixtures below count chunks at the world's own radius and
        // drive an empty server that must keep ticking.
        "--view-distance",
        "2",
        "--simulation-distance",
        "2",
        "--pause-when-empty-seconds",
        "0",
        "--bind",
        "127.0.0.1:0",
        "--no-mcp",
        "--world",
        root.to_str().expect("the fixture path is UTF-8"),
        "--level-seed",
        level_seed,
    ])
    .unwrap();
    let runtime = ServerRuntime::bind(options).await.unwrap();
    runtime.run_until(async {}).await.unwrap();
}

/// The saved block column at the level spawn, top to bottom.
fn spawn_column(candidate: &Java26WorldCandidate) -> Vec<String> {
    let [x, _, z] = candidate.level.spawn().position();
    let chunk = &overworld_chunk(candidate, (x.div_euclid(16), z.div_euclid(16)));
    let (local_x, local_z) = (
        usize::try_from(x.rem_euclid(16)).unwrap(),
        usize::try_from(z.rem_euclid(16)).unwrap(),
    );
    (-64..320)
        .rev()
        .map(|y| saved_block_state(chunk, local_x, y, local_z).name.clone())
        .collect()
}

#[tokio::test]
async fn an_empty_world_becomes_a_seeded_normal_save_whose_spawn_is_on_land() {
    let root = temp_test_dir("normal-world");
    boot_and_stop(&root, "bbb").await;
    let (storage, candidate) = load_runtime_candidate(&root);
    // `WorldOptions.parseSeed` hashes text: "bbb".hashCode().
    assert_eq!(candidate.world_gen_settings.seed, 97_314);
    assert!(candidate.world_gen_settings.generate_structures);
    let overworld = candidate
        .world_gen_settings
        .dimensions
        .iter()
        .find(|(key, _)| key.resource_id() == "minecraft:overworld")
        .map(|(_, stem)| stem)
        .unwrap();
    assert_eq!(overworld.generator, "minecraft:noise");
    assert!(candidate.level.initialized());
    let spawn = candidate.level.spawn().position();
    let column = spawn_column(&candidate);
    let below = usize::try_from(319 - (spawn[1] - 1)).unwrap();
    for name in &column[..below] {
        assert!(
            !matches!(name.as_str(), "minecraft:water" | "minecraft:lava"),
            "spawn {spawn:?} is under {name}"
        );
    }
    assert!(
        !matches!(
            column[below].as_str(),
            "minecraft:air" | "minecraft:water" | "minecraft:lava"
        ),
        "spawn {spawn:?} stands on {}",
        column[below]
    );
    storage.close().unwrap();

    boot_and_stop(&root, "").await;
    let (storage, restarted) = load_runtime_candidate(&root);
    assert_eq!(restarted.world_gen_settings.seed, 97_314);
    assert_eq!(restarted.level.spawn().position(), spawn);
    assert_eq!(spawn_column(&restarted), column);
    storage.close().unwrap();

    let twin = temp_test_dir("normal-world-twin");
    boot_and_stop(&twin, "bbb").await;
    let (storage, twin_candidate) = load_runtime_candidate(&twin);
    assert_eq!(twin_candidate.level.spawn().position(), spawn);
    assert_eq!(spawn_column(&twin_candidate), column);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(twin).unwrap();
}

#[tokio::test]
#[ignore = "opt-in cold respawn latency measurement"]
async fn measure_cold_respawn_to_terrain() {
    const BLOCK_XZ: i32 = 8000;
    const CENTER: i32 = BLOCK_XZ / 16;
    let root = temp_test_dir("cold-respawn");
    let server = RunningServer::start(options_with(&[
        "--world",
        root.to_str().unwrap(),
        "--level-seed",
        "bbb-cold-respawn",
        "--mcp-loopback-scope",
        "command",
    ]))
    .await;
    let (mut client, _) =
        configure_client(server.address, "respawn-probe", ClientIntent::Login).await;
    let budget = Duration::from_secs(60);
    loop {
        if let PlayClientbound::PlayerPosition(position) = client.read_play_raw_within(budget).await
        {
            client
                .send(encode_play_accept_teleportation(position.id))
                .await;
            client.send(encode_play_player_loaded()).await;
            break;
        }
    }

    let management = server.management_addr();
    let session = management_session(management).await;
    let landing = format!("{BLOCK_XZ} 300 {BLOCK_XZ}");
    for (id, command) in [
        (2, format!("spawnpoint respawn-probe {landing}")),
        (3, format!("execute unless loaded {landing}")),
        (4, "kill respawn-probe".into()),
    ] {
        let answer = exec_command(management, &session, id, command).await;
        assert_eq!(
            answer["structuredContent"]["command"]["result"], 1,
            "{answer}"
        );
    }
    loop {
        match client.read_play_raw_within(budget).await {
            PlayClientbound::SetHealth(health) if health.health <= 0.0 => break,
            PlayClientbound::ChunkBatchFinished { .. } => {
                client.send(encode_play_chunk_batch_received(25.0)).await;
            }
            _ => {}
        }
    }

    let started = std::time::Instant::now();
    client.send(encode_play_perform_respawn()).await;
    let (respawn, terrain) = tokio::time::timeout(budget, async {
        let mut respawn = None;
        loop {
            match client.read_play_raw_within(budget).await {
                PlayClientbound::Respawn(_) => respawn = Some(started.elapsed()),
                PlayClientbound::PlayerPosition(position) if respawn.is_some() => {
                    let coordinate = f64::from(BLOCK_XZ) + 0.5;
                    assert_eq!(
                        (position.position.x, position.position.z),
                        (coordinate, coordinate)
                    );
                    client
                        .send(encode_play_accept_teleportation(position.id))
                        .await;
                }
                PlayClientbound::ChunkBatchFinished { .. } => {
                    client.send(encode_play_chunk_batch_received(25.0)).await;
                }
                PlayClientbound::LevelChunkWithLight(chunk)
                    if (chunk.x, chunk.z) == (CENTER, CENTER) =>
                {
                    break (
                        respawn.expect("the respawn packet precedes terrain"),
                        started.elapsed(),
                    );
                }
                _ => {}
            }
        }
    })
    .await
    .expect("the cold respawn sent its destination terrain");
    println!(
        "cold_respawn respawn_packet_micros={} destination_terrain_micros={}",
        respawn.as_micros(),
        terrain.as_micros()
    );

    drop(client);
    server.stop_within(budget).await;
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn datapack_dimension_types_and_custom_noise_levels_generate_and_save() {
    let root = temp_test_dir("deep-noise");
    write_deep_noise_pack(&root);
    let options = ServerOptions::try_parse_from([
        "bbb-server",
        // The fixtures below count chunks at the world's own radius and
        // drive an empty server that must keep ticking.
        "--view-distance",
        "2",
        "--simulation-distance",
        "2",
        "--pause-when-empty-seconds",
        "0",
        "--bind",
        "127.0.0.1:0",
        "--mcp-bind",
        "127.0.0.1:0",
        "--mcp-loopback-scope",
        "command",
        "--world",
        root.to_str().expect("the fixture path is UTF-8"),
        "--level-seed",
        "7",
        "--mcp-request-timeout-seconds",
        "300",
    ])
    .unwrap();
    write_seeded_pack_save(&root, &options, "file/deep", false);

    let server = RunningServer::start(options).await;
    let management = server.management_addr();
    let session = management_session(management).await;
    let forced = exec_command(
        management,
        &session,
        2,
        "execute in example:deep run forceload add 0 0",
    )
    .await;
    assert_eq!(
        forced["structuredContent"]["command"]["result"], 1,
        "{forced}"
    );
    wait_until_loaded(management, &session, 100, "execute in example:deep", (0, 0)).await;
    server.stop_within(Duration::from_secs(60)).await;

    assert!(root
        .join("dimensions/example/deep/region/r.0.0.mca")
        .is_file());
    let (storage, candidate) = load_runtime_candidate(&root);
    let deep = saved_chunk(&candidate, "example:deep", (0, 0))
        .expect("the forced example:deep chunk is saved");
    let solid = deep
        .data
        .sections()
        .iter()
        .filter(|section| (0..16).contains(&section.y))
        .filter_map(|section| section.block_states.as_ref())
        .flat_map(|states| {
            (0..16).flat_map(move |y| {
                (0..16).flat_map(move |x| (0..16).filter_map(move |z| states.value_at(x, y, z)))
            })
        })
        .filter(|state| state.name != "minecraft:air")
        .count();
    assert!(
        solid > 16 * 16,
        "example:deep generated {solid} solid blocks"
    );
    assert!(deep
        .data
        .sections()
        .iter()
        .filter(|section| section.block_states.is_some())
        .all(|section| (0..16).contains(&section.y)));
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

async fn connect_play_client(address: std::net::SocketAddr, username: &str) -> super::TestClient {
    let (mut client, _) = configure_client(address, username, ClientIntent::Login).await;
    let budget = Duration::from_secs(10);
    loop {
        if let PlayClientbound::PlayerPosition(position) = client.read_play_raw_within(budget).await
        {
            client
                .send(encode_play_accept_teleportation(position.id))
                .await;
            client.send(encode_play_player_loaded()).await;
            return client;
        }
    }
}

#[tokio::test]
async fn tick_panic_flushes_modified_world_state_and_tears_down_cleanly() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    let root = temp_test_dir("tick-panic-save");
    let options = custom_flat_world_options(&root);
    let mut runtime = ServerRuntime::bind(options).await.unwrap();
    let address = runtime.local_addr().unwrap();
    let management = runtime.management_addr().unwrap().unwrap();
    let hook = Arc::new(crate::runtime::TestPanicHook::default());
    runtime.set_test_panic_hook(Arc::clone(&hook));
    let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(runtime.run_until(async {
        let _ = shutdown_rx.await;
    }));

    let mut client = connect_play_client(address, "panic-observer").await;
    let session = management_session(management).await;
    let forced = exec_command(management, &session, 2, "forceload add 0 0").await;
    assert_eq!(
        forced["structuredContent"]["command"]["result"], 1,
        "{forced}"
    );
    wait_until_loaded(management, &session, 10, "execute", (0, 0)).await;
    let placed = exec_command(
        management,
        &session,
        3,
        "setblock 0 20 0 minecraft:diamond_block",
    )
    .await;
    assert_eq!(
        placed["structuredContent"]["command"]["result"], 1,
        "{placed}"
    );

    hook.panic_on_tick.store(true, Ordering::Release);
    let outcome = tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("runtime exits after world tick panic")
        .unwrap();
    assert!(
        matches!(outcome, Err(crate::ServerRuntimeError::WorldThread(_))),
        "expected WorldThread error for non-zero exit, got {outcome:?}"
    );

    // Active connections close cleanly and the listener stops accepting.
    client.wait_closed().await;
    drop(client);
    assert!(
        tokio::net::TcpStream::connect(address).await.is_err(),
        "listener must be closed after tick panic"
    );

    let (storage, candidate) = load_runtime_candidate(&root);
    let chunk = overworld_chunk(&candidate, (0, 0));
    assert_eq!(
        saved_block_state(&chunk, 0, 20, 0).name,
        "minecraft:diamond_block"
    );
    storage.close().unwrap();

    // Verify a secondary panic inside emergency save cannot abort cleanup.
    let secondary_hook = Arc::new(crate::runtime::TestPanicHook {
        panic_on_tick: AtomicBool::new(false),
        panic_on_emergency_save: AtomicBool::new(true),
    });
    let mut runtime = ServerRuntime::bind(custom_flat_world_options(&root))
        .await
        .unwrap();
    let address = runtime.local_addr().unwrap();
    runtime.set_test_panic_hook(Arc::clone(&secondary_hook));
    let (_shutdown_tx2, shutdown_rx2) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(runtime.run_until(async {
        let _ = shutdown_rx2.await;
    }));
    let mut client = connect_play_client(address, "secondary-panic").await;
    secondary_hook.panic_on_tick.store(true, Ordering::Release);
    let outcome = tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("runtime still tears down on secondary emergency save panic")
        .unwrap();
    assert!(matches!(
        outcome,
        Err(crate::ServerRuntimeError::WorldThread(_))
    ));
    client.wait_closed().await;
    drop(client);
    assert!(tokio::net::TcpStream::connect(address).await.is_err());

    std::fs::remove_dir_all(root).unwrap();
}
