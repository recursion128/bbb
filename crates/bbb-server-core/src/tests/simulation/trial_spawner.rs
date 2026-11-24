//! `TrialSpawnerBlock.getTicker` over a real
//! level: `TrialSpawner.tickServer`.

use super::helpers::{nbt_field as saved_field, restore_saved_entities};
use super::nbt as entry;
use super::*;

use crate::block_entity::trial_spawner::{TrialSpawnerState, TrialSpawnerStateData};
use crate::fluid::with_property;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::{SoundEventHolder, SoundSource};
use bbb_server_datapack::loot::{parse_loot_table, LootElement, LootElementValue};
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use serde_json::json;
use std::sync::{Arc, OnceLock};

mod team_spawn;
mod weather_rewards;

/// The fixture's spawner cell, which is the position every helper here pins.
/// `BlockPos.asLong` of `(4, 64, 6)` is `1099511652416`, whose `% 20` is 16, so
/// `TrialSpawnerStateData.tryDetectPlayers`
/// detects when `game_time % 20 == 4` and every 20 ticks from there.
fn at() -> BlockPos {
    BlockPos { x: 4, y: 64, z: 6 }
}

fn saved_compound(value: &NbtSummaryValue) -> &[NbtSummaryEntry] {
    let NbtSummaryValue::Compound(entries) = value else {
        panic!("a saved compound field is a compound: {value:?}");
    };
    entries
}

/// `TrialSpawnerConfig.spawnPotentialsDefinition`
/// with one weighted entry naming a zombie at a fixed `Pos`, so the spawn
/// position is the test's rather than the level random's.
fn zombie_potential(position: [f64; 3]) -> NbtSummaryValue {
    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
        entry("weight", NbtSummaryValue::Int(1)),
        entry(
            "data",
            NbtSummaryValue::Compound(vec![entry(
                "entity",
                NbtSummaryValue::Compound(vec![
                    entry("id", NbtSummaryValue::string("minecraft:zombie")),
                    entry(
                        "Pos",
                        NbtSummaryValue::List(position.iter().copied().map(nbt_double).collect()),
                    ),
                ]),
            )]),
        ),
    ])])
}

/// A saved `minecraft:trial_spawner` record whose `normal_config` names that
/// one potential.
fn pinned_record(position: [f64; 3]) -> Vec<NbtSummaryEntry> {
    pinned_record_at(at(), position)
}

/// [`pinned_record`] at an arbitrary cell, so a level can hold two spawners
/// with different detection phases.
fn pinned_record_at(pos: BlockPos, position: [f64; 3]) -> Vec<NbtSummaryEntry> {
    vec![
        entry("id", NbtSummaryValue::string("minecraft:trial_spawner")),
        entry("x", NbtSummaryValue::Int(pos.x)),
        entry("y", NbtSummaryValue::Int(pos.y)),
        entry("z", NbtSummaryValue::Int(pos.z)),
        entry("components", NbtSummaryValue::Compound(Vec::new())),
        entry(
            "normal_config",
            NbtSummaryValue::Compound(vec![entry("spawn_potentials", zombie_potential(position))]),
        ),
    ]
}

/// One saved chunk whose fixture cell holds a trial spawner in `state` with
/// `record` as its block entity.
fn trial_spawner_chunk(state: &str, record: Vec<NbtSummaryEntry>) -> Java26ChunkData {
    trial_spawner_chunk_with(state, "false", record)
}

/// The same cell with `ominous=true`, which is how a test reaches the ominous
/// config: the bad-omen trigger lives in `tryDetectPlayers`'s mob-effect arm.
fn ominous_trial_spawner_chunk(record: Vec<NbtSummaryEntry>) -> Java26ChunkData {
    trial_spawner_chunk_with("inactive", "true", record)
}

fn trial_spawner_chunk_with(
    state: &str,
    ominous: &str,
    record: Vec<NbtSummaryEntry>,
) -> Java26ChunkData {
    Java26ChunkData::parse(
        trial_spawner_chunk_raw(state, ominous, record),
        restore_chunk_context(),
    )
    .expect("the trial spawner chunk parses")
}

fn trial_spawner_chunk_raw(
    state: &str,
    ominous: &str,
    record: Vec<NbtSummaryEntry>,
) -> bbb_server_types::save::NamedCompound {
    let cell = (
        [4_usize, 0_usize, 6_usize],
        "minecraft:trial_spawner",
        &[("ominous", ominous), ("trial_spawner_state", state)][..],
    );
    let mut raw = restore_fixture_chunk_raw(vec![restore_section_with_cells(
        4,
        "minecraft:air",
        &[cell],
    )]);
    for entry in raw.entries_mut() {
        if entry.name == "block_entities" {
            entry.value = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(record.clone())]);
        }
    }
    raw
}

/// The fixture game type is creative, which `PlayerDetector.NO_CREATIVE_PLAYERS`
/// refuses, so every scenario that wants a detection joins as survival.
fn survival_config() -> ServerWorldConfig {
    ServerWorldConfig {
        game_type: GameType::Survival,
        ..datapack_config()
    }
}

/// A session two blocks from the spawner, inside its `required_player_range`,
/// with the fixture clock reset so every phase below is measured.
fn trial_spawner_world(
    config: ServerWorldConfig,
    record: Vec<NbtSummaryEntry>,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let candidate = restore_candidate(
        0,
        Some(trial_spawner_chunk("inactive", record.clone())),
        Vec::new(),
    );
    open_trial_spawner_world(config, &candidate)
}

fn open_trial_spawner_world(
    config: ServerWorldConfig,
    candidate: &Java26WorldCandidate,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = helpers::restored_world(config, candidate);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 4.5,
        },
    );
    // The join's own ticks run block entities; the measurement starts from the
    // state the fixture names and from the clock the fixture carries.
    set_trial_spawner_state(&mut world, TrialSpawnerState::Inactive);
    world.set_game_time_for_test(0);
    (sender, receiver, world)
}

/// `TrialSpawnerBlockEntity.setState`'s property write (`:72-75`), which a
/// test uses to name the state a scenario starts from.
fn set_trial_spawner_state(world: &mut ServerWorld, state: TrialSpawnerState) {
    let block_state = world
        .block_state(Java26BuiltinDimension::Overworld, at())
        .expect("the spawner's column is loaded");
    let Some(next) = with_property(block_state, "trial_spawner_state", state.property_value())
    else {
        panic!("minecraft:trial_spawner has that state");
    };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at(),
        next,
        SET_BLOCK_UPDATE_ALL,
    );
}

fn trial_spawner_state(world: &mut ServerWorld) -> TrialSpawnerState {
    trial_spawner_state_at(world, at())
}

fn trial_spawner_state_at(world: &mut ServerWorld, pos: BlockPos) -> TrialSpawnerState {
    let block_state = world
        .block_state(Java26BuiltinDimension::Overworld, pos)
        .expect("the spawner's column is loaded");
    TrialSpawnerState::from_block_state(block_state)
}

fn trial_spawner_data(world: &mut ServerWorld) -> TrialSpawnerStateData {
    trial_spawner_data_at(world, at())
}

fn trial_spawner_data_at(world: &mut ServerWorld, pos: BlockPos) -> TrialSpawnerStateData {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .and_then(|entity| entity.trial_spawner())
        .expect("the trial spawner block entity is live")
        .state_data()
        .clone()
}

/// The one stored zombie, as `(entity id, uuid, position)`.
fn stored_zombie(world: &mut ServerWorld) -> (i32, Uuid, Vec3d) {
    let zombie = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_ZOMBIE_ID)
        })
        .expect("the trial spawner's zombie was published");
    let entity = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, zombie)
        .expect("the zombie is stored");
    (zombie, entity.uuid(), entity.position())
}

/// The 3011/3012 pairs one tick published, in order.
fn trial_spawner_events(tick: &TickResult) -> Vec<(i32, BlockPos, i32)> {
    level_events_sent(tick)
        .into_iter()
        .filter(|(_, event)| matches!(event.event_type, 3011 | 3012))
        .map(|(_, event)| (event.event_type, event.pos, event.data))
        .collect()
}

/// The positions one tick sent a `ClientboundBlockEntityDataPacket` for.
fn block_entity_data_positions(tick: &TickResult) -> Vec<BlockPos> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::BlockEntityData(update),
                ..
            } => Some(update.pos),
            _ => None,
        })
        .collect()
}

#[test]
fn filling_the_spawn_data_publishes_the_block_entity_update_and_a_quiet_tick_does_not() {
    let (_, receiver, mut world) =
        trial_spawner_world(survival_config(), pinned_record([4.5, 65.0, 6.5]));

    // The first tick fills `spawn_data` from the potentials, which is the
    // `markUpdated` `TrialSpawnerStateData.getOrCreateNextSpawnData`
    // (`:225-235`) asks for; the queue is read at the head of the next tick.
    world.tick(receiver.drain());
    let published = world.tick(receiver.drain());
    assert_eq!(block_entity_data_positions(&published), vec![at()]);

    // At game_time 3 the detection phase is off and the spawn data is already
    // filled, so nothing marks the block changed.
    let quiet = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 3);
    assert!(
        block_entity_data_positions(&quiet).is_empty(),
        "{:?}",
        quiet.events
    );
}

#[test]
fn a_spawned_mob_republishes_the_block_entity_even_when_the_reroll_is_identical() {
    let (_, receiver, mut world) =
        trial_spawner_world(survival_config(), pinned_record([4.5, 65.0, 6.5]));
    for _ in 0..44 {
        world.tick(receiver.drain());
    }
    assert_eq!(trial_spawner_data(&mut world).total_mobs_spawned(), 1);
    assert_eq!(trial_spawner_data(&mut world).next_mob_spawns_at(), 84);

    // `TrialSpawnerState` marks the block updated on the single
    // potential's identical redraw; the update reaches clients next tick.
    let published = world.tick(receiver.drain());
    assert_eq!(block_entity_data_positions(&published), vec![at()]);
}

#[test]
fn an_untracked_mob_leaves_current_mobs_and_pushes_the_next_spawn() {
    let (_, receiver, mut world) =
        trial_spawner_world(survival_config(), pinned_record([4.5, 65.0, 6.5]));
    for _ in 0..44 {
        world.tick(receiver.drain());
    }
    let (zombie, uuid, _) = stored_zombie(&mut world);
    assert_eq!(trial_spawner_data(&mut world).next_mob_spawns_at(), 84);

    // `TrialSpawner.shouldMobBeUntracked` (`:280-286`) drops a mob whose
    // block position is past `47 * 47` from the spawner.
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, zombie)
        .expect("the zombie is live")
        .set_pos(Vec3d {
            x: 4.5,
            y: 65.0,
            z: 60.5,
        });

    world.tick(receiver.drain());

    let data = trial_spawner_data(&mut world);
    assert!(data.current_mobs().is_empty(), "{:?}", data.current_mobs());
    assert!(!data.current_mobs().contains(&uuid));
    assert_eq!(data.total_mobs_spawned(), 1);
    assert_eq!(data.next_mob_spawns_at(), 45 + 40);
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Active);
}

#[test]
fn a_discarded_mob_is_untracked_too() {
    let (_, receiver, mut world) =
        trial_spawner_world(survival_config(), pinned_record([4.5, 65.0, 6.5]));
    for _ in 0..44 {
        world.tick(receiver.drain());
    }
    let (zombie, uuid, _) = stored_zombie(&mut world);
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, zombie)
        .expect("the zombie is live")
        .discard();

    world.tick(receiver.drain());

    let data = trial_spawner_data(&mut world);
    assert!(data.current_mobs().is_empty());
    assert!(!data.current_mobs().contains(&uuid));
    assert_eq!(data.next_mob_spawns_at(), 45 + 40);
}

#[test]
fn a_player_without_sight_of_the_spawner_is_never_detected() {
    let (_, receiver, mut world) =
        trial_spawner_world(survival_config(), pinned_record([4.5, 65.0, 6.5]));
    // `PlayerDetector.inLineOfSight` clips from
    // the spawner's centre to the player's eye, so a full block on that ray
    // hides the player.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 4, y: 65, z: 5 },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));

    for _ in 0..40 {
        world.tick(receiver.drain());
    }

    assert_eq!(world.game_time(), 40);
    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::WaitingForPlayers
    );
    assert!(trial_spawner_data(&mut world)
        .registered_players()
        .is_empty());
}

#[test]
fn a_creative_or_spectator_player_is_never_detected() {
    for game_type in [GameType::Creative, GameType::Spectator] {
        let (_, receiver, mut world) = trial_spawner_world(
            ServerWorldConfig {
                game_type,
                ..datapack_config()
            },
            pinned_record([4.5, 65.0, 6.5]),
        );

        for _ in 0..40 {
            world.tick(receiver.drain());
        }

        assert_eq!(world.game_time(), 40, "{game_type:?}");
        assert_eq!(
            trial_spawner_state(&mut world),
            TrialSpawnerState::WaitingForPlayers,
            "{game_type:?}"
        );
        assert!(
            trial_spawner_data(&mut world)
                .registered_players()
                .is_empty(),
            "{game_type:?}"
        );
    }
}

#[test]
fn a_disabled_rule_resets_the_statistics_and_drops_active_back_to_waiting() {
    let (sender, receiver, mut world) = trial_spawner_world(
        ServerWorldConfig {
            admin_lists: operator_config(&[("alice", 1)]).admin_lists,
            ..survival_config()
        },
        pinned_record([4.5, 65.0, 6.5]),
    );
    for _ in 0..4 {
        world.tick(receiver.drain());
    }
    let active = trial_spawner_data(&mut world);
    assert_eq!(active.registered_players().len(), 1);
    assert_eq!(active.next_mob_spawns_at(), 44);

    // `TrialSpawner.canSpawnInLevel` (`:150-158`) reads the rule first.
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule spawner_blocks_work false",
    );
    // `:88-93`: ACTIVE answers a disabled level by resetting the statistics
    // and falling back to WAITING_FOR_PLAYERS.
    world.tick(receiver.drain());

    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::WaitingForPlayers
    );
    let reset = trial_spawner_data(&mut world);
    assert!(reset.registered_players().is_empty());
    assert_eq!(reset.total_mobs_spawned(), 0);
    assert_eq!(reset.next_mob_spawns_at(), 0);
    assert_eq!(reset.cooldown_ends_at(), 0);

    // `:78-83`: WAITING_FOR_PLAYERS resets the same statistics and stays.
    set_trial_spawner_state(&mut world, TrialSpawnerState::WaitingForPlayers);
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at())
        .and_then(BlockEntity::trial_spawner_mut)
        .expect("the trial spawner block entity is live")
        .state_data_mut()
        .registered_players_mut()
        .insert(Uuid::from_u128(1));
    world.tick(receiver.drain());

    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::WaitingForPlayers
    );
    assert!(trial_spawner_data(&mut world)
        .registered_players()
        .is_empty());
}

#[test]
fn a_peaceful_level_drops_an_active_spawner_to_waiting_for_players() {
    let (_, receiver, mut world) = trial_spawner_world(
        ServerWorldConfig {
            difficulty: Difficulty::Peaceful,
            ..survival_config()
        },
        pinned_record([4.5, 65.0, 6.5]),
    );
    set_trial_spawner_state(&mut world, TrialSpawnerState::Active);

    world.tick(receiver.drain());

    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::WaitingForPlayers
    );
    let data = trial_spawner_data(&mut world);
    assert_eq!(data.total_mobs_spawned(), 0);
    assert_eq!(data.next_mob_spawns_at(), 0);
}

#[test]
fn a_runtime_difficulty_change_to_peaceful_drops_an_active_spawner() {
    let (sender, receiver, mut world) = trial_spawner_world(
        ServerWorldConfig {
            admin_lists: operator_config(&[("alice", 1)]).admin_lists,
            ..survival_config()
        },
        pinned_record([4.5, 65.0, 6.5]),
    );
    set_trial_spawner_state(&mut world, TrialSpawnerState::Active);

    // `TrialSpawner.canSpawnInLevel` reads `level.getDifficulty()`, which
    // `MinecraftServer.setDifficulty` moves at runtime.
    chat_command(&sender, &receiver, &mut world, 1, "difficulty peaceful");

    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::WaitingForPlayers
    );
    let data = trial_spawner_data(&mut world);
    assert_eq!(data.total_mobs_spawned(), 0);
    assert_eq!(data.next_mob_spawns_at(), 0);
}

#[test]
fn a_refused_spawn_leaves_the_state_data_alone() {
    let (_, receiver, mut world) =
        trial_spawner_world(survival_config(), pinned_record([4.5, 65.0, 6.5]));
    for _ in 0..44 {
        world.tick(receiver.drain());
    }
    let before = trial_spawner_data(&mut world);
    assert_eq!(before.total_mobs_spawned(), 1);

    // The next spawn is due at 84; block its cell so `TrialSpawner.spawnMob`'s
    // `noCollision` gate (`:185-188`) refuses it.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 4, y: 65, z: 6 },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 84);

    let ticked = world.tick(receiver.drain());
    let flushed = world.tick(receiver.drain());
    let after = trial_spawner_data(&mut world);
    assert_eq!(after, before, "a refused spawn touches no bookkeeping");
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Active);
    assert_eq!(
        world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter(|id| {
                world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                    .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_ZOMBIE_ID)
            })
            .count(),
        1,
        "the refused spawn adds no mob"
    );
    let mut events = trial_spawner_events(&ticked);
    events.extend(trial_spawner_events(&flushed));
    assert!(events.is_empty(), "{events:?}");
    let events = world.game_events(Java26BuiltinDimension::Overworld).0;
    assert_eq!(
        events
            .iter()
            .filter(|event| {
                event.game_event == crate::level_event::VanillaGameEvent::ENTITY_PLACE
            })
            .count(),
        1
    );
}

#[test]
fn the_state_and_the_packed_data_survive_a_save_and_a_restart() {
    let candidate = restore_candidate(
        0,
        Some(trial_spawner_chunk(
            "inactive",
            pinned_record([4.5, 65.0, 6.5]),
        )),
        Vec::new(),
    );
    let (_, receiver, mut world) = open_trial_spawner_world(survival_config(), &candidate);
    for _ in 0..44 {
        world.tick(receiver.drain());
    }
    let (_, uuid, _) = stored_zombie(&mut world);
    let saved = trial_spawner_data(&mut world);

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let write = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the spawner's column is written");
    let parsed = Java26ChunkData::parse(&write.document, restore_chunk_context())
        .expect("the written document is a valid Java 26.1 chunk");
    let reloaded = restore_candidate(world.game_time(), Some(parsed), Vec::new());
    let mut restored = spawn_pinned_restore(datapack_config(), &reloaded);

    assert_eq!(
        trial_spawner_state(&mut restored),
        TrialSpawnerState::Active,
        "the state property is in the saved chunk"
    );
    let reloaded = trial_spawner_data(&mut restored);
    assert_eq!(
        reloaded.current_mobs().iter().copied().collect::<Vec<_>>(),
        vec![uuid]
    );
    assert_eq!(reloaded.total_mobs_spawned(), saved.total_mobs_spawned());
    assert_eq!(reloaded.next_mob_spawns_at(), saved.next_mob_spawns_at());
    assert_eq!(
        reloaded.next_spawn_data(),
        saved.next_spawn_data(),
        "the drawn spawn data is packed"
    );
}

#[test]
fn every_built_in_trial_spawner_config_decodes_with_its_mobs() {
    let config = datapack_config();
    let registry = config
        .datapacks
        .as_ref()
        .unwrap()
        .registries
        .registry("minecraft:trial_spawner")
        .unwrap();
    assert!(!registry.elements.is_empty());
    for element in &registry.elements {
        let value = element
            .source
            .as_ref()
            .unwrap()
            .source_order_nbt_projection()
            .unwrap();
        let config = crate::block_entity::trial_spawner::TrialSpawnerConfig::from_nbt(&value)
            .unwrap_or_else(|| panic!("{} decodes", element.id));
        assert!(!config.spawn_potentials().is_empty(), "{}", element.id);
    }
}

#[test]
fn a_registry_config_reference_spawns_its_mob_and_saves_unchanged() {
    const CONFIG: &str = "minecraft:trial_chamber/melee/zombie/normal";
    let mut record = pinned_record([0.0; 3]);
    record.retain(|entry| entry.name != "normal_config");
    record.push(entry("normal_config", NbtSummaryValue::string(CONFIG)));
    let candidate = restore_candidate(
        0,
        Some(trial_spawner_chunk("inactive", record.clone())),
        Vec::new(),
    );
    let mut config = survival_config();
    let element = std::sync::Arc::make_mut(config.datapacks.as_mut().unwrap())
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:trial_spawner")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == CONFIG)
        .unwrap();
    let mut source = bbb_registry_codec::parse_registry_json(&element.source_json).unwrap();
    let bbb_registry_codec::RegistryJsonValue::Object(fields) = &mut source else {
        panic!("config object");
    };
    fields.push((
        bbb_protocol::JavaString::from(vec![0xd800]),
        bbb_registry_codec::RegistryJsonValue::String(vec![0xdc00].into()),
    ));
    let source = serde_json::to_vec(&source).unwrap();
    let root = std::env::temp_dir().join(format!("bbb-trial-source-{}", uuid::Uuid::new_v4()));
    let pack = root.join("raw");
    let path = pack.join(format!(
        "data/minecraft/trial_spawner/{}.json",
        CONFIG.trim_start_matches("minecraft:")
    ));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), br#"{"pack":{"description":"raw config","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(path, &source).unwrap();
    let startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: Some(root.clone()),
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    let element = startup
        .registries
        .element("minecraft:trial_spawner", CONFIG)
        .unwrap();
    assert_eq!(element.source_json.as_ref(), source);
    assert!(element.source.is_some());
    assert!(element.codec_value.is_none());
    config.datapacks = Some(Arc::new(startup));
    let (_, receiver, mut world) = open_trial_spawner_world(config, &candidate);
    for _ in 0..60 {
        world.tick(receiver.drain());
    }
    assert!(trial_spawner_data(&mut world).total_mobs_spawned() >= 1);
    stored_zombie(&mut world);

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let write = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the spawner's column is written");
    let parsed = Java26ChunkData::parse(&write.document, restore_chunk_context())
        .expect("the written document is a valid Java 26.1 chunk");
    assert_eq!(
        saved_field(&parsed.block_entities()[0], "normal_config"),
        Some(&NbtSummaryValue::string(CONFIG))
    );
}

/// `TrialSpawnerConfig.totalMobs`/`simultaneousMobs` narrowed to one mob and
/// one eject table per config, with the per-player additions zeroed so the
/// pinned spawn finishes the target even with a second session.
fn reward_config_nbt(tables: &[&str]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("spawn_potentials", zombie_potential([4.5, 65.0, 6.5])),
        entry("total_mobs", NbtSummaryValue::Float(1.0_f32.to_bits())),
        entry(
            "total_mobs_added_per_player",
            NbtSummaryValue::Float(0.0_f32.to_bits()),
        ),
        entry(
            "simultaneous_mobs",
            NbtSummaryValue::Float(1.0_f32.to_bits()),
        ),
        entry(
            "simultaneous_mobs_added_per_player",
            NbtSummaryValue::Float(0.0_f32.to_bits()),
        ),
        entry(
            "loot_tables_to_eject",
            NbtSummaryValue::List(
                tables
                    .iter()
                    .map(|table| {
                        NbtSummaryValue::Compound(vec![
                            entry("data", NbtSummaryValue::string(*table)),
                            entry("weight", NbtSummaryValue::Int(1)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

/// A saved record whose normal and ominous configs each name one eject table,
/// with a cooldown short enough to measure the reward gates.
fn reward_record(
    target_cooldown_length: i32,
    normal_tables: &[&str],
    ominous_tables: &[&str],
) -> Vec<NbtSummaryEntry> {
    vec![
        entry("id", NbtSummaryValue::string("minecraft:trial_spawner")),
        entry("x", NbtSummaryValue::Int(at().x)),
        entry("y", NbtSummaryValue::Int(at().y)),
        entry("z", NbtSummaryValue::Int(at().z)),
        entry("components", NbtSummaryValue::Compound(Vec::new())),
        entry("normal_config", reward_config_nbt(normal_tables)),
        entry("ominous_config", reward_config_nbt(ominous_tables)),
        entry(
            "target_cooldown_length",
            NbtSummaryValue::Int(target_cooldown_length),
        ),
    ]
}

/// The built-in datapack plus four loot tables, so an ejection's drops are
/// known items. The injected tables declare no `random_sequence`, which keeps
/// their roll on the level random the triangle draws also use.
fn reward_datapack_config() -> ServerWorldConfig {
    static STARTUP: OnceLock<Arc<DatapackStartup>> = OnceLock::new();
    let startup = Arc::clone(STARTUP.get_or_init(|| {
        let mut startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        })
        .expect("the built-in datapack loads");
        let registry = startup
            .loot
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == "minecraft:loot_table")
            .expect("the loot registry is present");
        for (id, table) in [
            (
                "eject",
                json!({
                    "type": "minecraft:chest",
                    "pools": [
                        {
                            "rolls": 1,
                            "entries": [{"type": "minecraft:item", "name": "minecraft:apple"}]
                        },
                        {
                            "rolls": 1,
                            "entries": [{"type": "minecraft:item", "name": "minecraft:diamond"}]
                        },
                    ]
                }),
            ),
            (
                "empty",
                json!({
                    "type": "minecraft:chest",
                    "pools": [{
                        "rolls": 1,
                        "entries": [{
                            "type": "minecraft:item",
                            "name": "minecraft:apple",
                            "conditions": [{
                                "condition": "minecraft:random_chance",
                                "chance": 0.0,
                            }],
                        }]
                    }]
                }),
            ),
            (
                "normal_eject",
                json!({
                    "type": "minecraft:chest",
                    "pools": [{
                        "rolls": 1,
                        "entries": [{"type": "minecraft:item", "name": "minecraft:apple"}]
                    }]
                }),
            ),
            (
                "ominous_eject",
                json!({
                    "type": "minecraft:chest",
                    "pools": [{
                        "rolls": 1,
                        "entries": [{"type": "minecraft:item", "name": "minecraft:diamond"}]
                    }]
                }),
            ),
        ] {
            registry.elements.push(LootElement {
                id: format!("test:{id}"),
                source_pack: "test".into(),
                value: LootElementValue::Table(
                    parse_loot_table(&table, None).expect("the test loot table parses"),
                ),
            });
        }
        registry
            .elements
            .sort_by(|left, right| left.id.cmp(&right.id));
        Arc::new(startup)
    }));
    ServerWorldConfig {
        datapacks: Some(startup),
        game_type: GameType::Survival,
        ..ServerWorldConfig::default()
    }
}

/// The reward fixture with two sessions two blocks apart, both inside the
/// spawner's range and sight, so one ejection consumes exactly one of them.
fn reward_world(
    ominous: bool,
    target_cooldown_length: i32,
    normal_tables: &[&str],
    ominous_tables: &[&str],
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    reward_world_with_config(
        reward_datapack_config(),
        ominous,
        target_cooldown_length,
        normal_tables,
        ominous_tables,
    )
}

fn reward_world_with_config(
    config: ServerWorldConfig,
    ominous: bool,
    target_cooldown_length: i32,
    normal_tables: &[&str],
    ominous_tables: &[&str],
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let record = reward_record(target_cooldown_length, normal_tables, ominous_tables);
    let chunk = if ominous {
        ominous_trial_spawner_chunk(record)
    } else {
        trial_spawner_chunk("inactive", record)
    };
    let candidate = restore_candidate(0, Some(chunk), Vec::new());
    let mut world = spawn_pinned_restore(config, &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 4.5,
        },
    );
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        2,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 4.5,
        },
    );
    set_trial_spawner_state(&mut world, TrialSpawnerState::Inactive);
    world.set_game_time_for_test(0);
    (sender, receiver, world)
}

fn trial_world_predicate_config(
    mut config: ServerWorldConfig,
    table_id: &str,
    items: &[&str],
    functions: &[serde_json::Value],
) -> ServerWorldConfig {
    let startup = Arc::make_mut(config.datapacks.as_mut().expect("the datapack is loaded"));
    let table = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .and_then(|registry| {
            registry
                .elements
                .iter_mut()
                .find(|table| table.id == table_id)
        })
        .expect("the fixture table exists");
    table.value = LootElementValue::Table(parse_loot_table(&json!({
        "type": "minecraft:empty",
        "pools": items.iter().map(|item| json!({
            "rolls": 1,
            "conditions": [
                {"condition":"minecraft:value_check","value":{"type":"minecraft:score",
                    "target":{"type":"minecraft:fixed","name":"#enabled"},"score":"trial"},"range":1},
                {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107}
            ],
            "entries": [{"type":"minecraft:item","name":item,"functions":functions}]
        })).collect::<Vec<_>>()
    }), None).expect("the EMPTY world-predicate table parses"));
    config
}

fn set_trial_world_predicate_state(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    score: i32,
    clock: i32,
) {
    for (index, command) in [
        "scoreboard objectives add trial dummy".to_owned(),
        format!("scoreboard players set #enabled trial {score}"),
        "time of minecraft:the_end pause".to_owned(),
        format!("time of minecraft:the_end set {clock}"),
    ]
    .into_iter()
    .enumerate()
    {
        world.set_game_time_for_test(0);
        let result = crate::tests::entities::console_command(
            sender,
            receiver,
            world,
            100 + index as u64,
            &command,
        );
        assert!(
            crate::tests::entities::console_command_answer(&result)
                .0
                .is_some(),
            "{command}"
        );
    }
    set_trial_spawner_state(world, TrialSpawnerState::Inactive);
    world.set_game_time_for_test(0);
}

/// Runs the real path from INACTIVE to WAITING_FOR_REWARD_EJECTION and returns
/// `cooldownEndsAt - targetCooldownLength`, the instant every reward gate is
/// measured from.
fn drive_to_reward_ejection(receiver: &WorldCommandReceiver, world: &mut ServerWorld) -> i64 {
    for _ in 0..4 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 4, "the first detection arms the spawner");
    assert_eq!(trial_spawner_state(world), TrialSpawnerState::Active);

    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 44);
    let (zombie, _, _) = stored_zombie(world);
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, zombie)
        .expect("the spawned zombie is live")
        .discard();
    world.tick(receiver.drain());
    assert_eq!(
        trial_spawner_state(world),
        TrialSpawnerState::WaitingForRewardEjection
    );

    let data = trial_spawner_data(world);
    let target = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at())
        .and_then(|entity| entity.trial_spawner())
        .expect("the trial spawner block entity is live")
        .config()
        .target_cooldown_length();
    data.cooldown_ends_at() - i64::from(target)
}

/// Ticks `count` times. The tick that produces an effect is taken separately,
/// so its events and the drain tick after it stay readable.
fn tick_many(receiver: &WorldCommandReceiver, world: &mut ServerWorld, count: usize) {
    for _ in 0..count {
        world.tick(receiver.drain());
    }
}

/// The live item entities of the spawner's level as
/// `(item resource id, count, position, delta movement)`.
fn ejected_items(world: &ServerWorld) -> Vec<(String, i32, Vec3d, Vec3d)> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            let stack = item.item();
            Some((
                stack.item_resource_id()?.to_string(),
                stack.count(),
                entity.position(),
                entity.delta_movement(),
            ))
        })
        .collect()
}

/// `DefaultDispenseItemBehavior.spawnItem`'s three triangles
/// the y mean is `0.2` and the two
/// horizontal means are zero, each within `0.0172275 * 2`.
fn assert_eject_velocity(delta: Vec3d) {
    let spread = 0.0172275 * 2.0;
    assert!(
        (0.2 - spread..=0.2 + spread).contains(&delta.y),
        "{delta:?}"
    );
    assert!(delta.x.abs() <= spread, "{delta:?}");
    assert!(delta.z.abs() <= spread, "{delta:?}");
}

/// The distinct 3014 events of one tick; every session in range is sent the
/// same one.
fn eject_level_events(tick: &TickResult) -> Vec<(BlockPos, i32)> {
    let mut events = Vec::new();
    for (_, event) in level_events_sent(tick) {
        let payload = (event.pos, event.data);
        if event.event_type == 3014 && !events.contains(&payload) {
            events.push(payload);
        }
    }
    events
}

/// The distinct `ClientboundSoundPacket`s of two consecutive ticks, which is
/// where a sound queued by the block-entity phase is drained. The same
/// broadcast reaches every session, so its seed deduplicates the fan-out.
fn block_sounds(first: &TickResult, second: &TickResult) -> Vec<bbb_protocol::packets::SoundEvent> {
    let mut seeds = std::collections::BTreeSet::new();
    let mut sounds = Vec::new();
    for sound in sounds_sent(first).into_iter().chain(sounds_sent(second)) {
        if seeds.insert(sound.seed) {
            sounds.push(sound);
        }
    }
    sounds
}

fn shutter_registry_id(resource_id: &str) -> i32 {
    bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id)
        .expect("the shutter sound is a built-in sound event")
}

/// The whole reward half on a real level: the shutter opens 40 ticks after the
/// cooldown starts, one table is rolled per detected player every 30 ticks,
/// the drops land on the dispense position with the triangle velocity, 3014
/// rides only a non-empty roll, and the last player closes the shutter.
#[test]
fn a_trial_spawner_ejects_one_table_per_detected_player_between_its_shutters() {
    let config = trial_world_predicate_config(
        reward_datapack_config(),
        "test:eject",
        &["minecraft:apple", "minecraft:raw_iron"],
        &[
            json!({"function":"minecraft:set_count","count":{
                "type":"minecraft:storage","storage":"test:trial","path":"count"
            }}),
            json!({"function":"minecraft:furnace_smelt"}),
        ],
    );
    let (sender, receiver, mut world) =
        reward_world_with_config(config, false, 200, &["test:eject"], &["test:eject"]);
    let stored = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut world,
        200,
        "data modify storage test:trial count set value 1",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&stored).0,
        Some(1)
    );
    set_trial_world_predicate_state(&sender, &receiver, &mut world, 1, 107);
    let started = drive_to_reward_ejection(&receiver, &mut world);
    assert_eq!(started, 45);
    let data = trial_spawner_data(&mut world);
    assert_eq!(data.cooldown_ends_at(), 245);
    assert_eq!(data.registered_players().len(), 2);

    // `TrialSpawnerState`: nothing opens before the 40th tick.
    tick_many(&receiver, &mut world, 39);
    assert_eq!(world.game_time(), 84);
    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::WaitingForRewardEjection
    );

    let opening = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 85);
    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::EjectingReward
    );
    let opening_flush = world.tick(receiver.drain());
    let sounds = block_sounds(&opening, &opening_flush);
    assert_eq!(sounds.len(), 1, "{sounds:?}");
    assert_eq!(
        sounds[0].sound,
        SoundEventHolder::Reference {
            registry_id: shutter_registry_id("minecraft:block.trial_spawner.open_shutter")
        }
    );
    assert_eq!(sounds[0].source, SoundSource::Blocks);
    assert_eq!(
        sounds[0].position,
        Vec3d {
            x: 4.5,
            y: 64.5,
            z: 6.5
        }
    );
    assert!(ejected_items(&world).is_empty(), "the shutter opens first");

    // The next multiple of 30 after the shutter is 105.
    tick_many(&receiver, &mut world, 18);
    assert_eq!(world.game_time(), 104);
    assert!(ejected_items(&world).is_empty());

    let first = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 105);
    // Read before the next entity tick moves the drops.
    let drops = ejected_items(&world);
    assert_eq!(
        drops
            .iter()
            .map(|(id, count, ..)| (id.as_str(), *count))
            .collect::<Vec<_>>(),
        vec![("minecraft:apple", 1), ("minecraft:iron_ingot", 1)]
    );
    for (_, _, position, delta) in &drops {
        assert_eq!(
            *position,
            Vec3d {
                x: 4.5,
                y: 65.075,
                z: 6.5
            }
        );
        assert_eject_velocity(*delta);
    }
    let first_flush = world.tick(receiver.drain());
    // `ServerLevel.levelEvent` broadcasts inside the body that raised it
    // so the roll's own tick carries the 3014.
    assert_eq!(
        eject_level_events(&first),
        vec![(at(), 0)],
        "one 3014 for the whole roll, in the tick that rolled it"
    );
    assert_eq!(
        eject_level_events(&first_flush),
        Vec::new(),
        "the following tick rolled nothing, so it carries nothing"
    );
    let data = trial_spawner_data(&mut world);
    assert_eq!(data.registered_players().len(), 1);
    assert_eq!(data.ejecting_loot_table(), Some("test:eject"));
    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::EjectingReward
    );

    let stored = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut world,
        201,
        "data modify storage test:trial count set value 3",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&stored).0,
        Some(1)
    );
    tick_many(&receiver, &mut world, 27);
    assert_eq!(world.game_time(), 134);
    let second = world.tick(receiver.drain());
    let second_flush = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 136);
    assert_eq!(ejected_items(&world).len(), 4);
    let mut counts = ejected_items(&world)
        .into_iter()
        .map(|(id, count, ..)| (id, count))
        .collect::<Vec<_>>();
    counts.sort();
    assert_eq!(
        counts,
        vec![
            ("minecraft:apple".to_owned(), 1),
            ("minecraft:apple".to_owned(), 3),
            ("minecraft:iron_ingot".to_owned(), 1),
            ("minecraft:iron_ingot".to_owned(), 3),
        ]
    );
    assert!(trial_spawner_data(&mut world)
        .registered_players()
        .is_empty());
    assert_eq!(
        eject_level_events(&second).len() + eject_level_events(&second_flush).len(),
        1
    );

    tick_many(&receiver, &mut world, 28);
    assert_eq!(world.game_time(), 164);
    let closing = world.tick(receiver.drain());
    let closing_flush = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 166);
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Cooldown);
    assert_eq!(trial_spawner_data(&mut world).ejecting_loot_table(), None);
    let sounds = block_sounds(&closing, &closing_flush);
    assert_eq!(sounds.len(), 1, "{sounds:?}");
    assert_eq!(
        sounds[0].sound,
        SoundEventHolder::Reference {
            registry_id: shutter_registry_id("minecraft:block.trial_spawner.close_shutter")
        }
    );
    assert_eq!(
        eject_level_events(&closing).len() + eject_level_events(&closing_flush).len(),
        0,
        "the close arm rolls nothing"
    );

    tick_many(&receiver, &mut world, 78);
    assert_eq!(world.game_time(), 244);
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Cooldown);
    world.tick(receiver.drain());
    assert_eq!(world.game_time(), 245);
    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::WaitingForPlayers
    );
    let reset = trial_spawner_data(&mut world);
    assert!(reset.registered_players().is_empty());
    assert_eq!(reset.total_mobs_spawned(), 0);
    assert_eq!(reset.cooldown_ends_at(), 0);
    assert_eq!(reset.next_mob_spawns_at(), 0);
    assert_eq!(reset.ejecting_loot_table(), None);
}

/// `LootContext.Builder.create`: every built-in
/// eject table declares a `random_sequence`, so its roll draws from the
/// server's sequence generator rather than from the level random.
#[test]
fn a_built_in_eject_table_rolls_through_its_random_sequence() {
    let key = "minecraft:spawners/trial_chamber/key";
    let (_, receiver, mut world) = reward_world(false, 200, &[key], &[key]);
    assert_eq!(drive_to_reward_ejection(&receiver, &mut world), 45);
    tick_many(&receiver, &mut world, 39);
    assert_eq!(world.game_time(), 84);
    world.tick(receiver.drain());
    assert_eq!(world.game_time(), 85);

    tick_many(&receiver, &mut world, 19);
    assert_eq!(world.game_time(), 104);
    let ejected = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 105);
    let drops = ejected_items(&world);
    assert_eq!(drops.len(), 1, "{drops:?}");
    assert_eq!(drops[0].0, "minecraft:trial_key");
    assert_eq!(drops[0].1, 1);
    assert_eq!(
        drops[0].2,
        Vec3d {
            x: 4.5,
            y: 65.075,
            z: 6.5
        }
    );
    assert_eject_velocity(drops[0].3);
    assert_eq!(
        eject_level_events(&ejected),
        vec![(at(), 0)],
        "the roll's own tick carries its 3014"
    );
    let flushed = world.tick(receiver.drain());
    assert_eq!(
        eject_level_events(&flushed),
        Vec::new(),
        "the tick after the roll carries nothing"
    );
}

/// `TrialSpawnerState`: a roll that produces nothing still
/// consumes the detected player the step took.
#[test]
fn an_empty_roll_consumes_the_player_without_a_level_event() {
    for (score, clock) in [(0, 107), (1, 108)] {
        let config = trial_world_predicate_config(
            reward_datapack_config(),
            "test:empty",
            &["minecraft:apple"],
            &[],
        );
        let (sender, receiver, mut world) =
            reward_world_with_config(config, false, 200, &["test:empty"], &["test:empty"]);
        set_trial_world_predicate_state(&sender, &receiver, &mut world, score, clock);
        assert_eq!(drive_to_reward_ejection(&receiver, &mut world), 45);
        tick_many(&receiver, &mut world, 39);
        assert_eq!(world.game_time(), 84);
        world.tick(receiver.drain());
        assert_eq!(world.game_time(), 85);

        tick_many(&receiver, &mut world, 19);
        assert_eq!(world.game_time(), 104);
        let ejected = world.tick(receiver.drain());
        let flushed = world.tick(receiver.drain());

        assert!(ejected_items(&world).is_empty(), "the table drops nothing");
        assert_eq!(
            eject_level_events(&ejected).len() + eject_level_events(&flushed).len(),
            0
        );
        let data = trial_spawner_data(&mut world);
        assert_eq!(data.registered_players().len(), 1);
        assert_eq!(data.ejecting_loot_table(), Some("test:empty"));
        assert_eq!(
            trial_spawner_state(&mut world),
            TrialSpawnerState::EjectingReward
        );
    }
}

/// `TrialSpawnerState` with `TrialSpawner.activeConfig`
/// an ominous block rolls the ominous config's
/// table, and its cooldown end clears the block's OMINOUS property.
#[test]
fn an_ominous_spawner_ejects_the_ominous_config_table_and_clears_ominous() {
    let (_, receiver, mut world) =
        reward_world(true, 130, &["test:normal_eject"], &["test:ominous_eject"]);
    assert_eq!(drive_to_reward_ejection(&receiver, &mut world), 45);
    assert_eq!(trial_spawner_data(&mut world).cooldown_ends_at(), 175);

    tick_many(&receiver, &mut world, 39);
    assert_eq!(world.game_time(), 84);
    world.tick(receiver.drain());
    assert_eq!(world.game_time(), 85);
    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::EjectingReward
    );

    tick_many(&receiver, &mut world, 19);
    assert_eq!(world.game_time(), 104);
    world.tick(receiver.drain());
    assert_eq!(world.game_time(), 105);
    let drops = ejected_items(&world);
    assert_eq!(
        drops.iter().map(|(id, ..)| id.as_str()).collect::<Vec<_>>(),
        vec!["minecraft:diamond"]
    );
    assert_eject_velocity(drops[0].3);
    world.tick(receiver.drain());

    // Both sessions are consumed one per ejection, so the next 30-tick gate
    // closes the shutter.
    tick_many(&receiver, &mut world, 28);
    assert_eq!(world.game_time(), 134);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    assert_eq!(world.game_time(), 136);
    assert!(trial_spawner_data(&mut world)
        .registered_players()
        .is_empty());

    tick_many(&receiver, &mut world, 28);
    assert_eq!(world.game_time(), 164);
    let closing = world.tick(receiver.drain());
    let closing_flush = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 166);
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Cooldown);
    let sounds = block_sounds(&closing, &closing_flush);
    assert_eq!(sounds.len(), 1, "{sounds:?}");
    assert_eq!(
        sounds[0].sound,
        SoundEventHolder::Reference {
            registry_id: shutter_registry_id("minecraft:block.trial_spawner.close_shutter")
        }
    );

    // `TrialSpawnerState` over `TrialSpawner.removeOminous`.
    tick_many(&receiver, &mut world, 9);
    assert_eq!(world.game_time(), 175);
    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::WaitingForPlayers
    );
    assert!(!trial_spawner_ominous(&mut world));
    let reset = trial_spawner_data(&mut world);
    assert!(reset.registered_players().is_empty());
    assert_eq!(reset.total_mobs_spawned(), 0);
    assert_eq!(reset.cooldown_ends_at(), 0);
}

/// `BlockState.getOptionalValue(OMINOUS)`.
fn trial_spawner_ominous(world: &mut ServerWorld) -> bool {
    let block_state = world
        .block_state(Java26BuiltinDimension::Overworld, at())
        .expect("the spawner's column is loaded");
    bbb_protocol::block_state_types::vanilla_block_state_identity(block_state)
        .and_then(|identity| identity.properties.get("ominous").cloned())
        .as_deref()
        == Some("true")
}

/// The two reward states keep both halves of their persistence across a save:
/// the block-state property and the packed `ejecting_loot_table`.
#[test]
fn the_reward_state_and_its_ejecting_table_survive_a_save_and_a_restart() {
    for (state, table) in [
        (TrialSpawnerState::WaitingForRewardEjection, None),
        (TrialSpawnerState::EjectingReward, Some("test:eject")),
    ] {
        let candidate = restore_candidate(
            0,
            Some(trial_spawner_chunk(
                "inactive",
                reward_record(200, &["test:eject"], &["test:eject"]),
            )),
            Vec::new(),
        );
        let (_, _receiver, mut world) =
            open_trial_spawner_world(reward_datapack_config(), &candidate);
        set_trial_spawner_state(&mut world, state);
        {
            let spawner = world
                .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at())
                .and_then(BlockEntity::trial_spawner_mut)
                .expect("the trial spawner block entity is live");
            let data = spawner.state_data_mut();
            data.set_cooldown_ends_at(261);
            data.set_ejecting_loot_table(table.map(str::to_owned));
            data.registered_players_mut().insert(Uuid::from_u128(7));
        }
        let saved = trial_spawner_data(&mut world);

        let request = world
            .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
            .expect("the save is produced");
        let write = request
            .chunk_writes
            .iter()
            .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
            .expect("the spawner's column is written");
        let parsed = Java26ChunkData::parse(&write.document, restore_chunk_context())
            .expect("the written document is a valid Java 26.1 chunk");
        let reloaded = restore_candidate(world.game_time(), Some(parsed), Vec::new());
        let mut restored = spawn_pinned_restore(reward_datapack_config(), &reloaded);

        assert_eq!(trial_spawner_state(&mut restored), state, "{state:?}");
        let reloaded = trial_spawner_data(&mut restored);
        assert_eq!(reloaded.cooldown_ends_at(), saved.cooldown_ends_at());
        assert_eq!(reloaded.ejecting_loot_table(), table, "{state:?}");
        assert_eq!(
            reloaded.detected_players().len(),
            saved.detected_players().len()
        );
    }
}

/// The fixture's survival session with operator rights, which `/effect` needs
/// (`EffectCommands` above `Commands.LEVEL_ALL`).
fn operator_survival_config() -> ServerWorldConfig {
    ServerWorldConfig {
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..survival_config()
    }
}

/// The same with the equipment loot table, which an equipped mob needs.
fn operator_equipment_config() -> ServerWorldConfig {
    ServerWorldConfig {
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..equipment_datapack_config()
    }
}

/// One `SpawnData` naming `resource_id` at a fixed `Pos`, optionally equipped
/// from the test's own equipment table with a preserved mainhand drop chance
fn activation_potential(
    position: [f64; 3],
    resource_id: &str,
    equipment_table: Option<&str>,
) -> NbtSummaryValue {
    let entity = vec![
        entry("id", NbtSummaryValue::string(resource_id)),
        entry(
            "Pos",
            NbtSummaryValue::List(position.iter().copied().map(nbt_double).collect()),
        ),
    ];
    let mut data = vec![entry("entity", NbtSummaryValue::Compound(entity))];
    if let Some(table) = equipment_table {
        data.push(entry(
            "equipment",
            NbtSummaryValue::Compound(vec![
                entry("loot_table", NbtSummaryValue::string(table)),
                entry(
                    "slot_drop_chances",
                    NbtSummaryValue::Compound(vec![entry(
                        "mainhand",
                        NbtSummaryValue::Float(2.0_f32.to_bits()),
                    )]),
                ),
            ]),
        ));
    }
    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
        entry("weight", NbtSummaryValue::Int(1)),
        entry("data", NbtSummaryValue::Compound(data)),
    ])])
}

/// The activation fixture. The normal half spawns a zombie on the default
/// 40-tick countdown; the ominous half spawns a skeleton at the same cell on 7
/// ticks with a target of one mob, so which half a tick ran is visible in the
/// spawn as well as in the counters. The cell is the one the fixture can spawn
/// into: it is the only one with a solid block under it (the spawner itself).
fn activation_record(equipped: bool) -> Vec<NbtSummaryEntry> {
    activation_record_with_equipment(equipped.then_some("test:equipment"))
}

/// [`activation_record`] against an arbitrary equipment loot table, so a test
/// can pick the one whose entries carry a component patch.
fn activation_record_with_equipment(table: Option<&str>) -> Vec<NbtSummaryEntry> {
    vec![
        entry("id", NbtSummaryValue::string("minecraft:trial_spawner")),
        entry("x", NbtSummaryValue::Int(at().x)),
        entry("y", NbtSummaryValue::Int(at().y)),
        entry("z", NbtSummaryValue::Int(at().z)),
        entry("components", NbtSummaryValue::Compound(Vec::new())),
        entry(
            "normal_config",
            NbtSummaryValue::Compound(vec![entry(
                "spawn_potentials",
                activation_potential([4.5, 65.0, 6.5], "minecraft:zombie", table),
            )]),
        ),
        entry(
            "ominous_config",
            NbtSummaryValue::Compound(vec![
                entry(
                    "spawn_potentials",
                    activation_potential([4.5, 65.0, 6.5], "minecraft:skeleton", table),
                ),
                entry("ticks_between_spawn", NbtSummaryValue::Int(7)),
                entry("total_mobs", NbtSummaryValue::Float(1.0_f32.to_bits())),
                entry(
                    "total_mobs_added_per_player",
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                ),
                entry(
                    "simultaneous_mobs",
                    NbtSummaryValue::Float(1.0_f32.to_bits()),
                ),
                entry(
                    "simultaneous_mobs_added_per_player",
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                ),
            ]),
        ),
        entry("target_cooldown_length", NbtSummaryValue::Int(200)),
    ]
}

/// One equipment loot table: a single `minecraft:diamond_sword` entry, with
/// `functions` when the test needs a component patch on the stack.
fn equipment_table(id: &str, functions: Option<serde_json::Value>) -> LootElement {
    let mut item = json!({
        "type": "minecraft:item",
        "name": "minecraft:diamond_sword",
    });
    if let Some(functions) = functions {
        item["functions"] = functions;
    }
    LootElement {
        id: id.into(),
        source_pack: "test".into(),
        value: LootElementValue::Table(
            parse_loot_table(
                &json!({
                    "type": "minecraft:chest",
                    "pools": [{ "rolls": 1, "entries": [item] }]
                }),
                None,
            )
            .expect("the test equipment table parses"),
        ),
    }
}

/// The built-in datapack plus the two equipment loot tables, so an equipped
/// mob's drop is a known item. Neither declares a `random_sequence`, which
/// keeps each roll on the sequence the equipment path derives from the table
/// id.
fn equipment_datapack_config() -> ServerWorldConfig {
    static STARTUP: OnceLock<Arc<DatapackStartup>> = OnceLock::new();
    let startup = Arc::clone(STARTUP.get_or_init(|| {
        let mut startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        })
        .expect("the built-in datapack loads");
        let registry = startup
            .loot
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == "minecraft:loot_table")
            .expect("the loot registry is present");
        registry
            .elements
            .push(equipment_table("test:equipment", None));
        registry.elements.push(equipment_table(
            "test:equipment_components",
            Some(json!([{
                "function": "minecraft:set_components",
                "components": {"minecraft:custom_data": {"bbb": 1}}
            }])),
        ));
        registry
            .elements
            .sort_by(|left, right| left.id.cmp(&right.id));
        Arc::new(startup)
    }));
    ServerWorldConfig {
        datapacks: Some(startup),
        game_type: GameType::Survival,
        ..ServerWorldConfig::default()
    }
}

/// A world whose spawner already carries `ominous=true`.
fn ominous_spawner_world(
    config: ServerWorldConfig,
    record: Vec<NbtSummaryEntry>,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let candidate = restore_candidate(0, Some(ominous_trial_spawner_chunk(record)), Vec::new());
    open_trial_spawner_world(config, &candidate)
}

/// The spawner's own level events in one tick, in order, with the per-session
/// fan-out deduplicated: 3011/3012 spawn and removal flames, 3013/3019 the
/// detection broadcast and 3020 the ominous activation pair.
fn trial_spawner_broadcasts(tick: &TickResult) -> Vec<(i32, BlockPos, i32)> {
    let mut events = Vec::new();
    for (_, event) in level_events_sent(tick) {
        let payload = (event.event_type, event.pos, event.data);
        if (3011..=3020).contains(&event.event_type) && !events.contains(&payload) {
            events.push(payload);
        }
    }
    events
}

/// The pickup delay of every live item entity in the spawner's level.
fn item_pickup_delays(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            Some(item.pickup_delay())
        })
        .collect()
}

fn set_no_ai(world: &mut ServerWorld, mob_id: i32) {
    helpers::set_no_ai(world, mob_id, true);
}

/// The one stored mob of `entity_type_id`, as `(entity id, uuid, position)`.
fn stored_mob(world: &mut ServerWorld, entity_type_id: i32) -> (i32, Uuid, Vec3d) {
    let id = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| entity.entity_type_id() == entity_type_id)
        })
        .expect("the trial spawner's mob was published");
    let entity = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
        .expect("the mob is stored");
    (id, entity.uuid(), entity.position())
}

/// The player's eye block, which `TrialSpawnerStateData` floors out of
/// `getEyePosition`.
fn eye_block() -> BlockPos {
    BlockPos { x: 4, y: 65, z: 4 }
}

/// `TrialSpawnerStateData` with `TrialSpawner.applyOminous`
/// a player holding BAD_OMEN turns the spawner
/// ominous, loses the bad omen and gains the trial omen its amplifier scales,
/// and both 3020 events ride the level event queue in vanilla's order.
#[test]
fn a_bad_omen_turns_the_spawner_ominous_and_becomes_a_trial_omen() {
    let (sender, receiver, mut world) =
        trial_spawner_world(operator_survival_config(), activation_record(false));
    // INACTIVE leaves on the first tick; `/effect` costs a tick of its own, so
    // the detection phase is measured from the clock.
    world.tick(receiver.drain());
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "effect give alice bad_omen 100 2",
    );
    assert_eq!(world.game_time(), 2);
    world.tick(receiver.drain());
    let became_ominous = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 4);

    assert!(trial_spawner_ominous(&mut world));
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Active);
    let trial_omen = world
        .player_active_effect_for_test(ConnectionId(1), "minecraft:trial_omen")
        .expect("the bad omen became a trial omen");
    // `:204-205`: `18000 * (amplifier + 1)` ticks, added at amplifier zero.
    // The session's own `tickEffects` runs after the
    // block-entity phase, so one of them is already spent.
    assert_eq!(trial_omen.duration(), 18_000 * 3 - 1);
    assert_eq!(trial_omen.amplifier(), 0);
    assert!(
        world
            .player_active_effect_for_test(ConnectionId(1), "minecraft:bad_omen")
            .is_none(),
        "the bad omen is removed"
    );

    // `ServerLevel.levelEvent` broadcasts inside the body that raised it
    // so the activation pair rides the tick that
    // applied the omen, and `becameOminous` suppresses the detection event of
    // that same tick.
    assert_eq!(
        trial_spawner_broadcasts(&became_ominous),
        vec![(3020, eye_block(), 0), (3020, at(), 1)]
    );
    let quiet = world.tick(receiver.drain());
    assert_eq!(
        trial_spawner_broadcasts(&quiet),
        Vec::new(),
        "the omen was applied on the previous tick, so this one raises nothing"
    );
}

/// The trial omen arm of `TrialSpawnerStateData.findPlayerWithOminousEffect`
/// (`:166-168`): the effect is not transformed, and the spawner still turns.
#[test]
fn a_trial_omen_turns_the_spawner_ominous_without_transforming_it() {
    let (sender, receiver, mut world) =
        trial_spawner_world(operator_survival_config(), activation_record(false));
    world.tick(receiver.drain());
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "effect give alice trial_omen 100 3",
    );
    world.tick(receiver.drain());
    let became_ominous = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 4);

    assert!(trial_spawner_ominous(&mut world));
    let trial_omen = world
        .player_active_effect_for_test(ConnectionId(1), "minecraft:trial_omen")
        .expect("the session keeps its trial omen");
    // `:130-132` transforms only a bad omen, so the instance is untouched
    // beyond its own countdown.
    assert_eq!(trial_omen.amplifier(), 3);
    assert!(
        (1900..=2000).contains(&trial_omen.duration()),
        "{}",
        trial_omen.duration()
    );
    // `ServerLevel.levelEvent` broadcasts inside the body that raised it
    // the tick that read the trial omen carries
    // the pair, and the one after it raises nothing.
    assert_eq!(
        trial_spawner_broadcasts(&became_ominous),
        vec![(3020, eye_block(), 0), (3020, at(), 1)]
    );
    let quiet = world.tick(receiver.drain());
    assert_eq!(trial_spawner_broadcasts(&quiet), Vec::new());
}

/// `TrialSpawnerStateData.resetAfterBecomingOminous` (`:179-199`) over a real
/// level: one 3012 per tracked mob at its block position, the preserved
/// equipment dropped as an item entity, the mob discarded, and the counters
/// reset off the ominous config.
#[test]
fn an_activation_retires_the_tracked_mobs_and_drops_their_preserved_equipment() {
    let (sender, receiver, mut world) =
        trial_spawner_world(operator_equipment_config(), activation_record(true));
    // INACTIVE leaves at 1, the detection at 4 arms it, the pinned mob spawns
    // at 44 with its equipment.
    world.tick(receiver.drain());
    for _ in 0..43 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 44);
    let (zombie, uuid, position) = stored_zombie(&mut world);
    assert_eq!(
        position,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 6.5
        }
    );
    let data = trial_spawner_data(&mut world);
    assert_eq!(
        data.current_mobs().iter().copied().collect::<Vec<_>>(),
        vec![uuid]
    );
    assert_eq!(data.total_mobs_spawned(), 1);

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "effect give alice bad_omen 100 0",
    );
    for _ in 0..18 {
        world.tick(receiver.drain());
    }
    let retired = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 64);

    assert!(trial_spawner_ominous(&mut world));
    let reset = trial_spawner_data(&mut world);
    assert!(
        reset.current_mobs().is_empty(),
        "{:?}",
        reset.current_mobs()
    );
    assert_eq!(reset.total_mobs_spawned(), 0);
    // The ominous config's `ticks_between_spawn`, then the constant
    // `ticksBetweenItemSpawners`.
    assert_eq!(reset.next_mob_spawns_at(), 71);
    assert_eq!(reset.cooldown_ends_at(), 224);
    assert_eq!(
        reset.next_spawn_data(),
        None,
        "the ominous half can reroll it"
    );
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, zombie)
        .is_none());
    let items = ejected_items(&world);
    assert_eq!(
        items
            .iter()
            .map(|(id, count, ..)| (id.as_str(), *count))
            .collect::<Vec<_>>(),
        vec![("minecraft:diamond_sword", 1)]
    );
    // `Entity.spawnAtLocation` at the mob's own
    // position, with the default pickup delay of.
    assert_eq!(items[0].2, position);
    assert_eq!(item_pickup_delays(&world), vec![10]);

    // `ServerLevel.levelEvent` broadcasts inside the body that raised it
    // so the activation pair and the 3012 per
    // retired mob ride the tick that became ominous.
    assert_eq!(
        trial_spawner_broadcasts(&retired),
        vec![
            (3020, eye_block(), 0),
            (3020, at(), 1),
            (3012, BlockPos { x: 4, y: 65, z: 6 }, 0),
        ]
    );
    let quiet = world.tick(receiver.drain());
    assert_eq!(
        trial_spawner_broadcasts(&quiet),
        Vec::new(),
        "the tracked mobs were already retired on the previous tick"
    );
}

/// `TrialSpawnerStateData.tryDetectPlayers`'s detection broadcast
/// (`:147-153`): 3013 for a normal spawner, 3019 for an ominous one, and
/// nothing when the set did not grow or the phase is off.
#[test]
fn a_detection_broadcasts_3013_or_3019_only_when_it_added_a_player() {
    let (_, receiver, mut world) = trial_spawner_world(survival_config(), activation_record(false));
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    let armed = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 4);
    assert_eq!(
        trial_spawner_broadcasts(&armed),
        vec![(3013, at(), 1)],
        "the tick that added the player carries its own 3013"
    );
    let quiet = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 5);
    assert_eq!(
        trial_spawner_broadcasts(&quiet),
        Vec::new(),
        "the phase added no player on this tick"
    );

    // 24 is the next phase tick and adds nothing; 25 is not a phase tick.
    for _ in 0..19 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 24);
    let again = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 25);
    assert!(trial_spawner_broadcasts(&again).is_empty());

    let (_, receiver, mut world) =
        ominous_spawner_world(survival_config(), activation_record(false));
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    let armed = world.tick(receiver.drain());
    assert_eq!(
        trial_spawner_broadcasts(&armed),
        vec![(3019, at(), 1)],
        "the ominous spawner's detection broadcasts 3019 instead"
    );
    let quiet = world.tick(receiver.drain());
    assert_eq!(trial_spawner_broadcasts(&quiet), Vec::new());
}

/// The mid-tick switch: the activation happens inside `tickAndGetNext`, so the
/// same tick's state write must keep the property it wrote, the reset must use
/// the ominous config, and the ticks behind it must spawn the ominous half's
/// mob.
#[test]
fn an_activation_inside_the_tick_keeps_ominous_and_the_ominous_config() {
    let (sender, receiver, mut world) =
        trial_spawner_world(operator_survival_config(), activation_record(false));
    world.tick(receiver.drain());
    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::WaitingForPlayers
    );
    // A player the spawner has already detected, so the 40-tick first-player
    // buffer of `:148` cannot hide the ominous countdown.
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at())
        .and_then(BlockEntity::trial_spawner_mut)
        .expect("the trial spawner block entity is live")
        .state_data_mut()
        .registered_players_mut()
        .insert(Uuid::from_u128(1));
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "effect give alice bad_omen 100 0",
    );
    for _ in 0..2 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 4);

    // Both properties of the one write the tick makes, and the reset counters
    // off the ominous config.
    assert!(trial_spawner_ominous(&mut world));
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Active);
    let reset = trial_spawner_data(&mut world);
    assert_eq!(reset.next_mob_spawns_at(), 11);
    assert_eq!(reset.cooldown_ends_at(), 164);
    assert!(reset.current_mobs().is_empty());
    assert_eq!(reset.total_mobs_spawned(), 0);

    // `TrialSpawnerStateData` cleared the pending spawn data, so
    // the ACTIVE arm refills it from the ominous half's potentials.
    for _ in 0..7 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 11);
    let (_, uuid, _) = stored_mob(&mut world, VANILLA_ENTITY_TYPE_SKELETON_ID);
    let spawned = trial_spawner_data(&mut world);
    assert_eq!(spawned.total_mobs_spawned(), 1);
    assert_eq!(
        spawned.current_mobs().iter().copied().collect::<Vec<_>>(),
        vec![uuid]
    );
    // `TrialSpawnerState` uses the ominous config's own countdown,
    // and its target of one mob, so no second spawn follows.
    assert_eq!(spawned.next_mob_spawns_at(), 18);
    for _ in 0..14 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 25);
    assert_eq!(trial_spawner_data(&mut world).total_mobs_spawned(), 1);
    assert_eq!(
        world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter(|id| world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_SKELETON_ID))
            .count(),
        1
    );
}

/// `SpawnData.equipment` over the entity-region save:
/// an equipment table entry with no components writes a patch-less stack, so
/// both the mob and its mainhand survive the save and the restart.
#[test]
fn an_equipped_mob_and_its_stack_survive_a_save_and_a_restart() {
    let mut config = operator_equipment_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let table = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .and_then(|registry| {
            registry
                .elements
                .iter_mut()
                .find(|table| table.id == "test:equipment")
        })
        .expect("the fixture equipment table exists");
    table.value = LootElementValue::Table(
        parse_loot_table(
            &json!({
                "type":"minecraft:equipment",
                "pools":[{
                    "rolls":1,
                    "conditions":[
                        {"condition":"minecraft:value_check","value":{"type":"minecraft:score",
                            "target":{"type":"minecraft:fixed","name":"#enabled"},"score":"trial"},"range":1},
                        {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107}
                    ],
                    "entries":[{"type":"minecraft:item","name":"minecraft:diamond_sword"}]
                }]
            }),
            None,
        )
        .expect("the EQUIPMENT world-predicate table parses"),
    );
    let (sender, receiver, mut world) =
        trial_spawner_world(config.clone(), activation_record(true));
    set_trial_world_predicate_state(&sender, &receiver, &mut world, 1, 107);
    for _ in 0..44 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 44);
    let (_, uuid, position) = stored_zombie(&mut world);

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let write = request
        .entity_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .expect("the entity chunk is written");
    let data = Java26EntityChunkData::parse(write.document.clone(), 0, 0)
        .expect("the saved entity chunk parses");
    let record = data
        .entities()
        .find(|record| record.id() == "minecraft:zombie")
        .expect("the equipped mob is saved");
    let equipment = saved_compound(
        saved_field(record.raw(), "equipment").expect("the mob saves its equipment"),
    );
    let mainhand =
        saved_compound(saved_field(equipment, "mainhand").expect("the mainhand is saved"));
    assert_eq!(
        saved_field(mainhand, "id"),
        Some(&NbtSummaryValue::string("minecraft:diamond_sword"))
    );
    assert!(
        saved_field(mainhand, "components").is_none(),
        "an empty patch writes no components entry"
    );

    let mut candidate = restore_candidate(0, None, Vec::new());
    restore_saved_entities(&mut candidate, &request.entity_writes);
    let restored = spawn_pinned_restore(operator_equipment_config(), &candidate);
    let entity = restored
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find_map(|id| {
            let entity = restored.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            (entity.uuid() == uuid).then_some(entity)
        })
        .expect("the mob survives the restart");
    assert_eq!(entity.position(), position);
    let mainhand = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
        .expect("the mainhand survives the restart");
    assert_eq!(mainhand.item_resource_id(), Some("minecraft:diamond_sword"));
    assert_eq!(mainhand.count(), 1);
    assert!(crate::tests::saved_item_components(&restored, mainhand).is_empty());

    for (score, clock) in [(0, 107), (1, 108)] {
        let (sender, receiver, mut world) =
            trial_spawner_world(config.clone(), activation_record(true));
        set_trial_world_predicate_state(&sender, &receiver, &mut world, score, clock);
        for _ in 0..44 {
            world.tick(receiver.drain());
        }
        assert_eq!(world.game_time(), 44);
        let (id, _, _) = stored_zombie(&mut world);
        let living = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .and_then(|entity| entity.living())
            .expect("the unequipped zombie is live");
        assert!(
            living.item_by_slot(EquipmentSlot::MainHand).is_none(),
            "score {score}, clock {clock}"
        );
    }
}

/// The activation's whole write survives a save: the block-state property, the
/// cleared mob set and the ominous counters, plus the preserved equipment the
/// retirement dropped as an item entity.
#[test]
fn an_ominous_activation_survives_a_save_and_a_restart() {
    let candidate = restore_candidate(
        0,
        Some(trial_spawner_chunk("inactive", activation_record(true))),
        Vec::new(),
    );
    let (sender, receiver, mut world) =
        open_trial_spawner_world(operator_equipment_config(), &candidate);
    world.tick(receiver.drain());
    for _ in 0..43 {
        world.tick(receiver.drain());
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "effect give alice bad_omen 100 0",
    );
    for _ in 0..19 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 64);
    assert!(trial_spawner_ominous(&mut world));
    let saved = trial_spawner_data(&mut world);

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let write = request
        .entity_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .expect("the entity chunk is written");
    let entities = Java26EntityChunkData::parse(write.document.clone(), 0, 0)
        .expect("the saved entity chunk parses");
    let dropped = entities
        .entities()
        .find(|record| record.id() == "minecraft:item")
        .expect("the preserved equipment is saved as an item entity");
    let item =
        saved_compound(saved_field(dropped.raw(), "Item").expect("the drop saves its stack"));
    assert_eq!(
        saved_field(item, "id"),
        Some(&NbtSummaryValue::string("minecraft:diamond_sword"))
    );
    assert!(
        saved_field(item, "components").is_none(),
        "an empty patch writes no components entry"
    );
    let write = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the spawner's column is written");
    let parsed = Java26ChunkData::parse(&write.document, restore_chunk_context())
        .expect("the written document is a valid Java 26.1 chunk");
    let mut reloaded = restore_candidate(world.game_time(), Some(parsed), Vec::new());
    restore_saved_entities(&mut reloaded, &request.entity_writes);
    let mut restored = spawn_pinned_restore(equipment_datapack_config(), &reloaded);

    assert!(trial_spawner_ominous(&mut restored));
    assert_eq!(
        trial_spawner_state(&mut restored),
        TrialSpawnerState::Active
    );
    let reloaded = trial_spawner_data(&mut restored);
    assert!(reloaded.current_mobs().is_empty());
    assert_eq!(reloaded.total_mobs_spawned(), 0);
    assert_eq!(reloaded.next_mob_spawns_at(), saved.next_mob_spawns_at());
    assert_eq!(reloaded.cooldown_ends_at(), saved.cooldown_ends_at());
    assert_eq!(reloaded.next_spawn_data(), None);
    assert_eq!(
        stored_item_stacks(&restored)
            .iter()
            .map(|(resource_id, count, ..)| (resource_id.as_str(), *count))
            .collect::<Vec<_>>(),
        vec![("minecraft:diamond_sword", 1)]
    );
}

/// A non-empty component patch survives the same round trip verbatim: the
/// fixture's second equipment table sets `minecraft:custom_data`, and the
/// saved mob keeps it byte for byte.
#[test]
fn an_equipment_component_patch_survives_a_save_and_a_restart() {
    let record = activation_record_with_equipment(Some("test:equipment_components"));
    let (_, receiver, mut world) = trial_spawner_world(operator_equipment_config(), record);
    for _ in 0..44 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 44);
    let (_, uuid, _) = stored_zombie(&mut world);
    let patch = NbtSummaryValue::Compound(vec![entry(
        "minecraft:custom_data",
        NbtSummaryValue::Compound(vec![entry("bbb", NbtSummaryValue::Byte(1))]),
    )]);

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let write = request
        .entity_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .expect("the entity chunk is written");
    let data = Java26EntityChunkData::parse(write.document.clone(), 0, 0)
        .expect("the saved entity chunk parses");
    let record = data
        .entities()
        .find(|record| record.id() == "minecraft:zombie")
        .expect("the equipped mob is saved");
    let equipment = saved_compound(
        saved_field(record.raw(), "equipment").expect("the mob saves its equipment"),
    );
    let mainhand =
        saved_compound(saved_field(equipment, "mainhand").expect("the mainhand is saved"));
    assert_eq!(
        saved_field(mainhand, "components"),
        Some(&patch),
        "a non-empty patch is written verbatim"
    );

    let mut candidate = restore_candidate(0, None, Vec::new());
    restore_saved_entities(&mut candidate, &request.entity_writes);
    let restored = spawn_pinned_restore(operator_equipment_config(), &candidate);
    let entity = restored
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find_map(|id| {
            let entity = restored.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            (entity.uuid() == uuid).then_some(entity)
        })
        .expect("the mob survives the restart");
    let mainhand = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
        .expect("the mainhand survives the restart");
    assert_eq!(mainhand.item_resource_id(), Some("minecraft:diamond_sword"));
    assert_eq!(
        crate::tests::saved_item_components(&restored, mainhand),
        vec![entry(
            "minecraft:custom_data",
            NbtSummaryValue::Compound(vec![entry("bbb", NbtSummaryValue::Byte(1))])
        )]
    );
}

/// `TrialSpawnerConfig.itemsToDropWhenOminous`
/// with the pinned zombie, the total narrowed to one mob so the ACTIVE arm
/// spawns nothing further, and the table the fixture injects.
fn ominous_item_config_nbt() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("spawn_potentials", zombie_potential([4.5, 65.0, 6.5])),
        entry(
            "items_to_drop_when_ominous",
            NbtSummaryValue::string("test:ominous"),
        ),
        entry("total_mobs", NbtSummaryValue::Float(1.0_f32.to_bits())),
        entry(
            "total_mobs_added_per_player",
            NbtSummaryValue::Float(0.0_f32.to_bits()),
        ),
        entry(
            "simultaneous_mobs",
            NbtSummaryValue::Float(1.0_f32.to_bits()),
        ),
        entry(
            "simultaneous_mobs_added_per_player",
            NbtSummaryValue::Float(0.0_f32.to_bits()),
        ),
    ])
}

fn ominous_item_record() -> Vec<NbtSummaryEntry> {
    vec![
        entry("id", NbtSummaryValue::string("minecraft:trial_spawner")),
        entry("x", NbtSummaryValue::Int(at().x)),
        entry("y", NbtSummaryValue::Int(at().y)),
        entry("z", NbtSummaryValue::Int(at().z)),
        entry("components", NbtSummaryValue::Compound(Vec::new())),
        entry("normal_config", ominous_item_config_nbt()),
        entry("ominous_config", ominous_item_config_nbt()),
    ]
}

/// The built-in datapack plus one dispensing table whose single drop is a
/// known non-projectile item.
fn dispensing_datapack_config() -> ServerWorldConfig {
    static STARTUP: OnceLock<Arc<DatapackStartup>> = OnceLock::new();
    let startup = Arc::clone(STARTUP.get_or_init(|| {
        let mut startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        })
        .expect("the built-in datapack loads");
        let registry = startup
            .loot
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == "minecraft:loot_table")
            .expect("the loot registry is present");
        registry.elements.push(LootElement {
            id: "test:ominous".into(),
            source_pack: "test".into(),
            value: LootElementValue::Table(
                parse_loot_table(
                    &json!({
                        "type": "minecraft:chest",
                        "pools": [{
                            "rolls": 1,
                            "entries": [{"type": "minecraft:item", "name": "minecraft:apple"}]
                        }]
                    }),
                    None,
                )
                .expect("the test dispensing table parses"),
            ),
        });
        registry
            .elements
            .sort_by(|left, right| left.id.cmp(&right.id));
        Arc::new(startup)
    }));
    ServerWorldConfig {
        datapacks: Some(startup),
        game_type: GameType::Survival,
        ..ServerWorldConfig::default()
    }
}

/// The dispensing fixture with `cooldownEndsAt` pinned so the first ACTIVE
/// attempt lands on a tick both candidates exist.
fn dispensing_world(
    ominous: bool,
    cooldown_ends_at: i64,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    dispensing_world_with_config(dispensing_datapack_config(), ominous, cooldown_ends_at)
}

fn dispensing_world_with_config(
    config: ServerWorldConfig,
    ominous: bool,
    cooldown_ends_at: i64,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = if ominous {
        ominous_spawner_world(config, ominous_item_record())
    } else {
        trial_spawner_world(config, ominous_item_record())
    };
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at())
        .and_then(BlockEntity::trial_spawner_mut)
        .expect("the trial spawner block entity is live")
        .state_data_mut()
        .set_cooldown_ends_at(cooldown_ends_at);
    (sender, receiver, world)
}

/// The live `minecraft:ominous_item_spawner` entities as
/// `(entity id, position, spawn_item_after_ticks, tick count)`.
fn ominous_item_spawners(world: &ServerWorld) -> Vec<(i32, Vec3d, i64, i64)> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            let crate::entity::EntityData::OminousItemSpawner(data) = entity.data() else {
                return None;
            };
            Some((
                id,
                entity.position(),
                data.spawn_item_after_ticks(),
                i64::from(entity.tick_count()),
            ))
        })
        .collect()
}

/// Every live item entity as `(resource id, count, position, pickup delay)`.
fn stored_item_stacks(world: &ServerWorld) -> Vec<(String, i32, Vec3d, i32)> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            let stack = item.item();
            Some((
                stack.item_resource_id()?.to_string(),
                stack.count(),
                entity.position(),
                item.pickup_delay(),
            ))
        })
        .collect()
}

/// Runs the fixture from the clock `open_trial_spawner_world` left at 0 to the
/// first ominous item spawn: ACTIVE at 4, the pinned zombie at 44, and the
/// attempt on the pinned cooldown at 100. Returns the spawn tick's position,
/// delay and result, because the sound it queued drains on the next tick.
fn drive_to_first_ominous_item_spawn(
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> (Vec3d, i64, TickResult) {
    world.tick(receiver.drain());
    tick_many(receiver, world, 3);
    assert_eq!(world.game_time(), 4);
    assert_eq!(trial_spawner_state(world), TrialSpawnerState::Active);
    tick_many(receiver, world, 40);
    assert_eq!(world.game_time(), 44);
    stored_mob(world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    tick_many(receiver, world, 55);
    assert_eq!(world.game_time(), 99);
    let spawned = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 100);
    let spawners = ominous_item_spawners(world);
    assert_eq!(spawners.len(), 1, "{spawners:?}");
    let (_, position, delay, tick_count) = spawners[0];
    assert_eq!(tick_count, 0, "the entity starts its own countdown");
    (position, delay, spawned)
}

/// `TrialSpawnerState.spawnOminousOminousItemSpawner`
/// on a real level: one `minecraft:ominous_item_spawner` above the tracked mob
/// or the detected player, the 1.0-volume block sound with the two-draw pitch,
/// and `cooldownEndsAt = gameTime + 160`.
#[test]
fn an_ominous_active_spawner_spawns_one_ominous_item_spawner_and_sets_the_cooldown() {
    let mending = bbb_facts::enchantment_types::VanillaEnchantmentIdentity::Mending;
    let config = trial_world_predicate_config(
        dispensing_datapack_config(),
        "test:ominous",
        &["minecraft:book"],
        &[
            json!({"function":"minecraft:enchant_randomly","options":[mending.resource_id()]}),
            json!({"function":"minecraft:copy_custom_data",
                "source":{"type":"minecraft:storage","source":"test:trial"},
                "ops":[{"source":"marker","target":"storage_marker","op":"replace"}]
            }),
        ],
    );
    let (sender, receiver, mut world) = dispensing_world_with_config(config, true, 100);
    let stored = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut world,
        200,
        "data modify storage test:trial marker set value 7",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&stored).0,
        Some(1)
    );
    set_trial_world_predicate_state(&sender, &receiver, &mut world, 1, 107);
    let (position, _, spawned) = drive_to_first_ominous_item_spawn(&receiver, &mut world);
    let spawner_id = ominous_item_spawners(&world)[0].0;
    let entity = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, spawner_id)
        .expect("the ominous item spawner is live");
    let crate::entity::EntityData::OminousItemSpawner(item) = entity.data() else {
        panic!("the live entity is the item spawner")
    };
    assert_eq!(
        item.item().item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_ENCHANTED_BOOK_ID)
    );
    assert_eq!(item.item().count(), 1);
    assert_eq!(
        crate::tests::saved_item_components(&world, item.item()),
        vec![
            entry(
                "minecraft:custom_data",
                NbtSummaryValue::Compound(vec![entry("storage_marker", NbtSummaryValue::Int(7))])
            ),
            entry(
                "minecraft:stored_enchantments",
                NbtSummaryValue::Compound(vec![entry(
                    mending.resource_id(),
                    NbtSummaryValue::Int(mending.max_level())
                )])
            )
        ]
    );

    // `:176-196`: the position is above one of the two eligible targets, and
    // which one is the level random's `nextBoolean` (`:213`).
    let (mob_id, _, mob_position) = stored_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let mob_height = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, mob_id)
        .expect("the zombie is stored")
        .bb_height();
    // `:198-205`: the spawn is the centre of the cell the clip ended in, one
    // block down, so a mob that walked off its cell centre still lands on one.
    let expected_mob: Vec<Vec3d> = (0..4)
        .map(|offset| Vec3d {
            x: mob_position.x.floor() + 0.5,
            y: (mob_position.y + f64::from(mob_height + 2.0 + offset as f32)).floor() - 0.5,
            z: mob_position.z.floor() + 0.5,
        })
        .collect();
    let player_position = Vec3d {
        x: 4.5,
        y: 64.0,
        z: 4.5,
    };
    let expected_player: Vec<Vec3d> = (0..4)
        .map(|offset| Vec3d {
            y: (player_position.y + f64::from(1.8_f32 + 2.0 + offset as f32)).floor() - 0.5,
            ..player_position
        })
        .collect();
    assert!(
        expected_mob.contains(&position) || expected_player.contains(&position),
        "{position:?}"
    );

    // The sound is queued by the block-entity phase and drains on the tick
    // after it; the strolling zombie's steps are not block sounds.
    let flush = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 101);
    let sounds: Vec<_> = block_sounds(&spawned, &flush)
        .into_iter()
        .filter(|sound| sound.source == SoundSource::Blocks)
        .collect();
    assert_eq!(sounds.len(), 1, "{sounds:?}");
    assert_eq!(
        sounds[0].sound,
        SoundEventHolder::Reference {
            registry_id: shutter_registry_id("minecraft:block.trial_spawner.spawn_item_begin")
        }
    );
    assert_eq!(sounds[0].source, SoundSource::Blocks);
    assert_eq!(sounds[0].volume, 1.0);
    // `:169`: `(nextFloat - nextFloat) * 0.2F + 1.0F`.
    assert!((0.8..1.2).contains(&sounds[0].pitch), "{}", sounds[0].pitch);
    assert_eq!(
        sounds[0].position,
        Vec3d {
            x: position.x.floor() + 0.5,
            y: position.y.floor() + 0.5,
            z: position.z.floor() + 0.5,
        }
    );

    // `:170`: `ticksBetweenItemSpawners` is the constant 160.
    assert_eq!(trial_spawner_data(&mut world).cooldown_ends_at(), 260);
}

/// `TrialSpawnerState`: only the ominous flag reaches the dispensing
/// roll, so a normal ACTIVE spawner never spawns the entity or moves its
/// cooldown.
#[test]
fn a_normal_active_spawner_never_spawns_an_ominous_item_spawner() {
    let (_, receiver, mut world) = dispensing_world(false, 100);
    world.tick(receiver.drain());
    tick_many(&receiver, &mut world, 3);
    assert_eq!(world.game_time(), 4);
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Active);
    tick_many(&receiver, &mut world, 97);
    assert_eq!(world.game_time(), 101);

    assert!(ominous_item_spawners(&world).is_empty());
    assert_eq!(trial_spawner_data(&mut world).cooldown_ends_at(), 100);
}

/// `TrialSpawnerState.timeToSpawnItemSpawner` (`:223-225`): a second attempt
/// before `cooldownEndsAt` spawns nothing, and the one after it does.
#[test]
fn the_ominous_item_cooldown_gates_the_next_spawn() {
    let (_, receiver, mut world) = dispensing_world(true, 100);
    let (_, delay, _) = drive_to_first_ominous_item_spawn(&receiver, &mut world);
    assert_eq!(trial_spawner_data(&mut world).cooldown_ends_at(), 260);
    // The zombie would otherwise kill the player the next spawn aims at.
    let (zombie, _, _) = stored_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    set_no_ai(&mut world, zombie);

    // The first spawner releases and kills itself at most 120 ticks in, so the
    // gate is read off the cooldown and the empty store.
    let remaining = (259 - world.game_time()) as usize;
    tick_many(&receiver, &mut world, remaining);
    assert_eq!(world.game_time(), 259);
    assert!(delay <= 120);
    assert!(ominous_item_spawners(&world).is_empty());
    assert_eq!(trial_spawner_data(&mut world).cooldown_ends_at(), 260);

    world.tick(receiver.drain());
    assert_eq!(world.game_time(), 260);
    assert_eq!(ominous_item_spawners(&world).len(), 1);
    assert_eq!(trial_spawner_data(&mut world).cooldown_ends_at(), 420);
}

/// `OminousItemSpawner.tickServer`/`spawnItem` (`OminousItemSpawner`)
/// on the entity the trial spawner published: the about-to-spawn
/// sound 36 ticks early, then the item entity, 3021, `ENTITY_PLACE` and the
/// entity's own removal.
#[test]
fn the_ominous_item_spawner_sounds_early_releases_its_item_and_dies() {
    let (_, receiver, mut world) = dispensing_world(true, 100);
    let (spawner_position, delay, _) = drive_to_first_ominous_item_spawn(&receiver, &mut world);

    // `:55`: the sound fires exactly on `tickCount == spawnItemAfterTicks - 36`,
    // and the entity's tick count is its age because it spawned at t=100.
    let sound_tick = 100 + delay - 36;
    let before_sound = (sound_tick - 1 - world.game_time()) as usize;
    tick_many(&receiver, &mut world, before_sound);
    assert_eq!(world.game_time(), sound_tick - 1);
    let sounding = world.tick(receiver.drain());
    assert_eq!(world.game_time(), sound_tick);
    let sounding_flush = world.tick(receiver.drain());
    let sounds = block_sounds(&sounding, &sounding_flush);
    assert_eq!(sounds.len(), 1, "{sounds:?}");
    assert_eq!(
        sounds[0].sound,
        SoundEventHolder::Reference {
            registry_id: shutter_registry_id("minecraft:block.trial_spawner.about_to_spawn_item")
        }
    );
    assert_eq!(sounds[0].source, SoundSource::Neutral);
    assert_eq!(
        sounds[0].position,
        Vec3d {
            x: spawner_position.x.floor() + 0.5,
            y: spawner_position.y.floor() + 0.5,
            z: spawner_position.z.floor() + 0.5,
        }
    );
    let [in_flight] = ominous_item_spawners(&world)[..] else {
        panic!("the trial spawner created one ominous item spawner")
    };
    let spawner_entity_id = in_flight.0;

    // `:59-62`: the release and `kill` land on `tickCount >= spawnItemAfterTicks`.
    let release_tick = 100 + delay;
    let before_release = (release_tick - 1 - world.game_time()) as usize;
    tick_many(&receiver, &mut world, before_release);
    assert_eq!(world.game_time(), release_tick - 1);
    let released = world.tick(receiver.drain());
    assert_eq!(world.game_time(), release_tick);

    assert!(ominous_item_spawners(&world).is_empty());
    let items = stored_item_stacks(&world);
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].0, "minecraft:apple");
    assert_eq!(items[0].1, 1);
    assert_eq!(
        items[0].2, spawner_position,
        "the item lands where it spawned"
    );
    // `ItemEntity(Level, x, y, z, ItemStack)` never
    // calls `setDefaultPickUpDelay`, so the constructor's zero stands.
    assert_eq!(items[0].3, 0);
    let block_position = BlockPos {
        x: spawner_position.x.floor() as i32,
        y: spawner_position.y.floor() as i32,
        z: spawner_position.z.floor() as i32,
    };
    let events = level_events_sent(&released)
        .into_iter()
        .filter(|(_, event)| event.event_type == 3021)
        .map(|(_, event)| (event.pos, event.data))
        .collect::<Vec<_>>();
    assert_eq!(events, vec![(block_position, 1)]);
    let item_entity_id = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_ITEM_ID)
        })
        .expect("the released item is stored");
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    // `tickServer` runs `spawnItem` and then
    // `this.kill(level)`, so the release tick ends with the spawner's own
    // `ENTITY_DIE` after the item's `ENTITY_PLACE`.
    let [placed, died] = game_events[game_events.len() - 2..] else {
        panic!("the release records two game events: {game_events:?}")
    };
    assert_eq!(placed.game_event, VanillaGameEvent::ENTITY_PLACE);
    assert_eq!(placed.pos, block_position);
    assert_eq!(placed.source_entity_id, Some(item_entity_id));
    assert_eq!(died.game_event, VanillaGameEvent::ENTITY_DIE);
    assert_eq!(died.pos, block_position);
    assert_eq!(
        died.source_entity_id,
        Some(spawner_entity_id),
        "the spawner is its own `GameEvent.Context.sourceEntity`"
    );
}

/// `OminousItemSpawner.addAdditionalSaveData`/`readAdditionalSaveData`
/// the item and the delay survive a save,
/// and the countdown restarts because `Entity.saveWithoutId`
/// writes no `tickCount`.
#[test]
fn an_in_flight_ominous_item_spawner_survives_a_save_and_a_restart() {
    let (_, receiver, mut world) = dispensing_world(true, 100);
    let (_, delay, _) = drive_to_first_ominous_item_spawn(&receiver, &mut world);
    tick_many(&receiver, &mut world, 10);
    assert_eq!(world.game_time(), 110);
    assert_eq!(ominous_item_spawners(&world)[0].3, 10);
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let write = request
        .entity_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .expect("the entity chunk is written");
    let data = Java26EntityChunkData::parse(write.document.clone(), 0, 0)
        .expect("the entity chunk parses");
    let record = data
        .entities()
        .find(|record| record.id() == "minecraft:ominous_item_spawner")
        .expect("the in-flight spawner is saved");
    let saved = record
        .raw()
        .iter()
        .map(|entry| (entry.name.to_jdk_string(), &entry.value))
        .collect::<Vec<_>>();
    assert!(saved
        .iter()
        .any(|(name, value)| *name == "spawn_item_after_ticks"
            && **value == NbtSummaryValue::Long(delay)));
    assert!(saved.iter().any(|(name, _)| *name == "item"));

    let mut candidate = restore_candidate(0, None, Vec::new());
    restore_saved_entities(&mut candidate, &request.entity_writes);
    let mut restored = ServerWorld::restore(dispensing_datapack_config(), &candidate)
        .expect("the saved world restores");
    let spawners = ominous_item_spawners(&restored);
    assert_eq!(spawners.len(), 1, "{spawners:?}");
    let (_, _, restored_delay, restored_ticks) = spawners[0];
    assert_eq!(restored_delay, delay, "the delay is the saved one");
    assert_eq!(restored_ticks, 0, "vanilla saves no tickCount");

    // The remaining countdown is the whole saved delay, and the release lands
    // exactly that many ticks after the reload.
    for _ in 0..restored_delay {
        restored.tick(Vec::new());
    }
    assert!(ominous_item_spawners(&restored).is_empty());
    assert_eq!(
        stored_item_stacks(&restored)
            .iter()
            .map(|(resource_id, ..)| resource_id.as_str())
            .collect::<Vec<_>>(),
        vec!["minecraft:apple"]
    );
}

mod player_filters;
