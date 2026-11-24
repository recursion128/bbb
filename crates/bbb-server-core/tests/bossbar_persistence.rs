use std::collections::BTreeMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use bbb_protocol::difficulties::Difficulty;
use bbb_server_core::world_creation::created_save;
use bbb_server_persistence::{
    Java26CustomBossEventPacked, Java26CustomBossEventsPacked, Java26LevelKey,
    Java26LevelSaveState, Java26SaveRequest, Java26WorldCreation, Java26WorldStorage,
    RegionCompression, SaveHolderResolver,
};
use bbb_server_types::fixture::FlatFixture;
use bbb_server_types::save::{Java26BuiltinDimension, WorldSaveLimits, WorldSavePaths};

fn root() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "bbb-custom-boss-events-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn custom_boss_events_round_trip_through_the_whole_world_transaction() {
    let root = root();
    fs::create_dir(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let fixture = FlatFixture::new().unwrap();
    let created = created_save(
        &Java26WorldCreation {
            level_name: "bossbar-test".to_string(),
            game_type: 1,
            difficulty: "easy".to_string(),
            spawn_position: [0, 65, 0],
            last_played: 1,
        },
        &fixture,
        &[],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 1)
        .unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert!(baseline.custom_boss_events.0.is_empty());

    let alice = Uuid::from_u128(1);
    let custom_boss_events = Java26CustomBossEventsPacked(BTreeMap::from([(
        "minecraft:test".to_string(),
        Java26CustomBossEventPacked {
            name: bbb_protocol::component_literal_nbt("Saved boss"),
            visible: true,
            value: 25,
            max: 40,
            color: "purple".to_string(),
            overlay: "notched_12".to_string(),
            darken_screen: true,
            play_boss_music: true,
            create_world_fog: true,
            players: vec![alice, alice],
        },
    )]));
    let spawn = baseline.level.spawn();
    let request = Java26SaveRequest {
        data_packs: None,
        poi_edits: Vec::new(),
        level_state: Java26LevelSaveState {
            game_time: baseline.level.game_time(),
            last_played: 2,
            game_type: baseline.level.game_type(),
            initialized: baseline.level.initialized(),
            difficulty: Difficulty::Easy,
            difficulty_locked: baseline.level.difficulty_settings().locked(),
            spawn: bbb_server_persistence::Java26LevelSpawn::new(
                spawn.dimension().to_string(),
                spawn.position(),
                spawn.yaw(),
                spawn.pitch(),
            ),
        },
        region_timestamp: 2,
        region_compression: RegionCompression::Zlib,
        player_patches: Vec::new(),
        departed_player_stats: BTreeMap::new(),
        player_advancements: BTreeMap::new(),
        chunk_writes: Vec::new(),
        chunk_write_epochs: BTreeMap::new(),
        entity_writes: Vec::new(),
        game_rules: baseline.game_rules.clone(),
        world_clocks: baseline.world_clocks.clone(),
        map_writes: BTreeMap::new(),
        map_last_id: None,
        raids: baseline.raids.clone(),
        ender_dragon_fights: BTreeMap::new(),
        world_borders: Java26BuiltinDimension::ALL
            .into_iter()
            .map(|dimension| {
                let key = Java26LevelKey::from(dimension);
                (key.clone(), baseline.world_borders[&key])
            })
            .collect(),
        chunk_tickets: BTreeMap::new(),
        random_sequences: baseline.random_sequences.clone(),
        scheduled_events: baseline.scheduled_events.clone(),
        stopwatches: baseline.stopwatches.clone(),
        weather: baseline.weather,
        wandering_trader: baseline.wandering_trader,
        scoreboard: baseline.scoreboard.clone(),
        custom_boss_events,
        command_storage_writes: BTreeMap::new(),
        item_stack_resolver: SaveHolderResolver::unavailable(),
    };
    storage.save_snapshot(&baseline, &request).unwrap();
    assert!(paths
        .saved_data_dir()
        .join("minecraft")
        .join("custom_boss_events.dat")
        .is_file());
    storage.close().unwrap();

    let reopened = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = reopened.load_candidate().unwrap();
    let event = &reloaded.custom_boss_events.0["minecraft:test"];
    assert_eq!(event.value, 25);
    assert_eq!(event.max, 40);
    assert_eq!(event.color, "purple");
    assert_eq!(event.overlay, "notched_12");
    assert_eq!(event.players, [alice]);
    reopened.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
