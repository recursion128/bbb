//! Shared fixtures, block-state builders, entity helpers, and packet readers
//! for `tests::simulation`.

use super::*;

use crate::entity::{EntityData, GeneralLivingData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::tests::entities::console_command;
use bbb_facts::scoreboard_team_options::{TeamCollisionRule, TeamVisibility};
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID;
use bbb_protocol::packets::{
    BlockPos, DebugBrainDump, DebugSubscriptionValue, EntityDataValueKind, MovePlayer,
    PlayerRotation, SoundEventHolder,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_resource_id;
use bbb_server_types::sculk_cursor::{cursor_nbt, SculkCursorFacts};
use uuid::Uuid;

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

pub(in crate::tests::simulation) const CREAKING_HEART: BlockPos = BlockPos { x: 0, y: 65, z: 4 };
pub(in crate::tests::simulation) const CREAKING_PROTECTOR_POS: Vec3d = Vec3d {
    x: 3.5,
    y: 65.0,
    z: 4.5,
};
pub(in crate::tests::simulation) const CREAKING_WATCHER_POS: Vec3d = Vec3d {
    x: 7.5,
    y: 65.0,
    z: 4.5,
};
pub(in crate::tests::simulation) const CREAKING_LOOKING_WEST: f32 = 90.0;

pub(in crate::tests::simulation) fn at(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

pub(in crate::tests::simulation) fn default_block_state(resource_id: &str) -> i32 {
    let block_type = bbb_protocol::block_types::vanilla_block_type_for_resource_id(resource_id)
        .expect("built-in block type");
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block_type.id)
        .expect("built-in default block state")
}

pub(in crate::tests::simulation) fn state_with(
    resource_id: &str,
    properties: &[(&str, &str)],
) -> i32 {
    let props = properties
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(resource_id, &props)
        .expect("built-in Java 26.1 block state")
}

pub(in crate::tests::simulation) fn shrieker_state(shrieking: bool, can_summon: bool) -> i32 {
    state_with(
        "minecraft:sculk_shrieker",
        &[
            ("can_summon", if can_summon { "true" } else { "false" }),
            ("shrieking", if shrieking { "true" } else { "false" }),
            ("waterlogged", "false"),
        ],
    )
}

pub(in crate::tests::simulation) fn sensor_state(
    phase: &str,
    power: i32,
    waterlogged: bool,
) -> i32 {
    let power = power.to_string();
    state_with(
        "minecraft:sculk_sensor",
        &[
            ("sculk_sensor_phase", phase),
            ("power", power.as_str()),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

pub(in crate::tests::simulation) fn catalyst_state(bloom: bool) -> i32 {
    state_with(
        "minecraft:sculk_catalyst",
        &[("bloom", if bloom { "true" } else { "false" })],
    )
}

pub(in crate::tests::simulation) fn place(world: &mut ServerWorld, pos: BlockPos, state: i32) {
    assert!(
        world.set_block(OVERWORLD, pos, state, SET_BLOCK_UPDATE_ALL),
        "the write at {pos:?} landed"
    );
}

pub(in crate::tests::simulation) fn operator_datapack_config() -> ServerWorldConfig {
    ServerWorldConfig {
        datapacks: datapack_config().datapacks,
        ..operator_config(&[("alice", 1)])
    }
}

pub(in crate::tests::simulation) fn joined_world(
    config: ServerWorldConfig,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    (sender, receiver, world)
}

pub(in crate::tests::simulation) fn joined_stand_world(
    config: ServerWorldConfig,
    pos: Vec3d,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = joined_world(config);
    stand_at(&sender, &receiver, &mut world, 1, pos);
    (sender, receiver, world)
}

pub(in crate::tests::simulation) fn restored_world(
    config: ServerWorldConfig,
    candidate: &Java26WorldCandidate,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_restore(config, candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    (sender, receiver, world)
}

pub(in crate::tests::simulation) fn frozen_observer_world(
    commands: &[&str],
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    config.keep_alive_interval_ticks = 100_000;
    let mut world = spawn_pinned_world(config);
    world.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    for line in commands {
        console_command_ok(&sender, &receiver, &mut world, line);
    }
    (sender, receiver, world)
}

pub(in crate::tests::simulation) fn admin_command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    operation_id: u64,
    command: &str,
) -> TickResult {
    enqueue(
        sender,
        WorldCommand::Admin {
            envelope: admin_envelope(
                operation_id,
                &format!("{operation_id}:{command}"),
                AdminScope::Command,
            ),
            request: AdminRequest::ExecuteCommand {
                command: command.to_string(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain())
}

pub(in crate::tests::simulation) fn console_command_ok(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    line: &str,
) {
    let operation = world.publication().tick;
    let tick = console_command(sender, receiver, world, operation, line);
    let outcome = admin_outcome(&tick);
    assert!(!outcome.replayed, "{line}");
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{line}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{line}: {report:?}");
}

pub(in crate::tests::simulation) fn restore_saved_entities(
    candidate: &mut Java26WorldCandidate,
    writes: &[bbb_server_persistence::Java26EntityChunkWrite],
) {
    for write in writes {
        let data =
            Java26EntityChunkData::parse(write.document.clone(), write.chunk_x, write.chunk_z)
                .expect("the saved entity chunk parses");
        candidate
            .dimensions
            .get_mut(&write.dimension)
            .expect("the written dimension is in the candidate")
            .entity_chunks
            .insert(
                (write.chunk_x, write.chunk_z),
                Java26LoadedEntityChunk {
                    timestamp: 100,
                    data,
                },
            );
    }
}

pub(in crate::tests::simulation) fn spawn_living_at(
    world: &mut ServerWorld,
    entity_type_id: i32,
    position: Vec3d,
) -> (i32, Uuid) {
    let data = EntityData::GeneralLiving(
        GeneralLivingData::new(entity_type_id).expect("the fixture type is living"),
    );
    world.spawn_stored_entity_for_test(OVERWORLD, entity_type_id, position, data)
}

pub(in crate::tests::simulation) fn set_no_ai(world: &mut ServerWorld, mob_id: i32, no_ai: bool) {
    let NbtSummaryValue::Compound(mut record) = world
        .stored_entity_command_data_for_test(OVERWORLD, mob_id)
        .expect("the mob is stored")
    else {
        unreachable!("an entity record is a compound");
    };
    record.retain(|entry| entry.name != "NoAI");
    record.push(nbt("NoAI", NbtSummaryValue::Byte(i8::from(no_ai))));
    world.load_stored_entity_record_for_test(OVERWORLD, mob_id, &record);
}

pub(in crate::tests::simulation) fn stage_one_blow_from_death(world: &mut ServerWorld, mob: i32) {
    let entity = world
        .stored_entity_mut_for_test(OVERWORLD, mob)
        .expect("the mob is stored");
    crate::entity::living::set_health(entity, 0.1);
}

pub(in crate::tests::simulation) fn entity_command_record(
    world: &ServerWorld,
    uuid: Uuid,
) -> Vec<NbtSummaryEntry> {
    let snapshot = world.command_entity_data_snapshot_for_test();
    let NbtSummaryValue::Compound(entries) = &snapshot
        .iter()
        .find(|entry| entry.uuid == uuid)
        .expect("entity command record exists")
        .data
    else {
        panic!("entity command record is a compound");
    };
    entries.clone()
}

pub(in crate::tests::simulation) fn nbt_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

pub(in crate::tests::simulation) fn stored_damage_source(
    uuid: Uuid,
    entity_id: i32,
    resource_id: &str,
    is_mob: bool,
    position: Vec3d,
) -> crate::commands::CommandDamageSourceEntity {
    crate::commands::CommandDamageSourceEntity::Stored {
        entity_id,
        uuid: Some(uuid),
        position: Some(position),
        entity_resource_id: resource_id.to_string(),
        is_mob,
        is_living_non_player: true,
        display_name: NbtSummaryValue::string(resource_id),
    }
}

pub(in crate::tests::simulation) fn brain_dumps(
    tick: &TickResult,
) -> std::collections::BTreeMap<i32, DebugBrainDump> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::DebugEntityValue(update),
                ..
            } => match &update.update.value {
                Some(DebugSubscriptionValue::Brain(brain)) => {
                    Some((update.entity_id, brain.clone()))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

pub(in crate::tests::simulation) fn level_sounds(tick: &TickResult) -> Vec<String> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Sound(sound),
                ..
            } => Some(sound),
            _ => None,
        })
        .map(|sound| {
            let SoundEventHolder::Reference { registry_id } = sound.sound else {
                panic!("sound is a registry reference")
            };
            vanilla_sound_event_resource_id(registry_id)
                .expect("a built-in sound event")
                .to_string()
        })
        .collect()
}

pub(in crate::tests::simulation) fn catalyst_session_world(
    pos: BlockPos,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = joined_stand_world(
        datapack_config(),
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    place(&mut world, pos, catalyst_state(false));
    world.tick(receiver.drain());
    (sender, receiver, world)
}

pub(in crate::tests::simulation) fn charge_spreader(
    world: &mut ServerWorld,
    pos: BlockPos,
    facts: &[SculkCursorFacts],
) {
    world
        .level_block_entity_for_test(OVERWORLD, pos)
        .expect("the catalyst's block entity")
        .sculk_catalyst_mut()
        .expect("a sculk catalyst payload")
        .set_cursors(facts.iter().map(cursor_nbt).collect());
}

pub(in crate::tests::simulation) fn sculk_cursor(
    pos: BlockPos,
    charge: i32,
    decay_delay: i32,
    update_delay: i32,
) -> SculkCursorFacts {
    SculkCursorFacts {
        pos,
        charge,
        decay_delay,
        update_delay,
        facings: None,
    }
}

pub(in crate::tests::simulation) fn sculk_catalyst_cursors(
    world: &ServerWorld,
    pos: BlockPos,
) -> Vec<NbtSummaryValue> {
    match world
        .get_block_entity(OVERWORLD, pos)
        .expect("the catalyst's block entity")
        .payload()
    {
        BlockEntityPayload::SculkCatalyst(catalyst) => catalyst.cursors().to_vec(),
        other => panic!("expected a sculk catalyst payload, got {other:?}"),
    }
}

pub(in crate::tests::simulation) fn creaking_fixture_with_config(
    config: ServerWorldConfig,
    teams: Vec<(Vec<String>, TeamCollisionRule)>,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.scoreboard = bbb_server_persistence::Java26ScoreboardPacked {
        teams: teams
            .into_iter()
            .enumerate()
            .map(|(index, (players, collision_rule))| {
                bbb_server_persistence::Java26ScoreboardTeam {
                    name: format!("team{index}").into(),
                    display_name: None,
                    color: None,
                    allow_friendly_fire: true,
                    see_friendly_invisibles: true,
                    member_name_prefix: None,
                    member_name_suffix: None,
                    name_tag_visibility: TeamVisibility::Always,
                    death_message_visibility: TeamVisibility::Always,
                    collision_rule,
                    players: players.into_iter().map(Into::into).collect(),
                }
            })
            .collect(),
        ..Default::default()
    };
    let (sender, receiver, mut world) = restored_world(config, &candidate);
    world.set_game_time_for_test(13_000);
    world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(13_000),
        &mut Vec::new(),
    );
    (sender, receiver, world)
}

pub(in crate::tests::simulation) fn creaking_fixture_with_teams(
    teams: Vec<Vec<String>>,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let mut config = operator_config(&[("alice", 1)]);
    config.game_type = GameType::Survival;
    creaking_fixture_with_config(
        config,
        teams
            .into_iter()
            .map(|players| (players, TeamCollisionRule::Always))
            .collect(),
    )
}

pub(in crate::tests::simulation) fn creaking_lay_floor(
    world: &mut ServerWorld,
    xs: std::ops::RangeInclusive<i32>,
    max_air_y: i32,
) {
    let stone = block_state_id("minecraft:stone");
    let air = block_state_id("minecraft:air");
    for x in xs {
        for z in 3..=6 {
            let _ = world.set_block(OVERWORLD, at(x, 64, z), stone, SET_BLOCK_UPDATE_ALL);
            for y in 65..=max_air_y {
                let _ = world.set_block(OVERWORLD, at(x, y, z), air, SET_BLOCK_UPDATE_ALL);
            }
        }
    }
}

pub(in crate::tests::simulation) fn creaking_lay_corridor(world: &mut ServerWorld) {
    creaking_lay_floor(world, -2..=10, 66);
    let stone = block_state_id("minecraft:stone");
    let air = block_state_id("minecraft:air");
    let key = Java26LevelKey::from_builtin(OVERWORLD);
    for x in [
        CREAKING_PROTECTOR_POS.x as i32,
        CREAKING_WATCHER_POS.x as i32,
    ] {
        for (y, state) in [(64, stone), (65, air), (66, air)] {
            assert_eq!(
                world.block_state_by_key(&world.require_level_id(&key), BlockPos { x, y, z: 4 }),
                Some(state),
                "the corridor cell ({x}, {y}, 4)"
            );
        }
    }
}

pub(in crate::tests::simulation) fn creaking_place_heart(
    world: &mut ServerWorld,
    heart_pos: BlockPos,
    heart_state_name: &str,
) {
    let log = state_with("minecraft:pale_oak_log", &[("axis", "y")]);
    for y in [heart_pos.y - 1, heart_pos.y + 1] {
        place(world, BlockPos { y, ..heart_pos }, log);
    }
    let heart = state_with(
        "minecraft:creaking_heart",
        &[
            ("axis", "y"),
            ("creaking_heart_state", heart_state_name),
            ("natural", "false"),
        ],
    );
    place(world, heart_pos, heart);
}

pub(in crate::tests::simulation) fn creaking_spawn_protector(
    world: &mut ServerWorld,
    position: Vec3d,
    uuid: Uuid,
    home: BlockPos,
) -> i32 {
    let entity_id = world
        .restore_stored_entity_for_test(
            OVERWORLD,
            uuid,
            VANILLA_ENTITY_TYPE_CREAKING_ID,
            position,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_CREAKING_ID)
                    .expect("creaking is living"),
            ),
        )
        .expect("the protector is restored with its own uuid");
    assert!(world
        .stored_entity_mut_for_test(OVERWORLD, entity_id)
        .expect("the protector is live")
        .set_creaking_transient(home));
    world
        .level_block_entity_for_test(OVERWORLD, home)
        .and_then(BlockEntity::creaking_heart_mut)
        .expect("the heart's block entity")
        .bind_protector(uuid);
    entity_id
}

pub(in crate::tests::simulation) fn creaking_move_player(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    position: Option<Vec3d>,
    y_rot: f32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position,
            rotation: Some(PlayerRotation { y_rot, x_rot: 0.0 }),
            on_ground: true,
            horizontal_collision: false,
        }),
    )
}

pub(in crate::tests::simulation) fn creaking_protector(
    world: &ServerWorld,
    entity_id: i32,
) -> &crate::entity::Entity {
    world
        .stored_entity_for_test(OVERWORLD, entity_id)
        .expect("the protector is live")
}

pub(in crate::tests::simulation) fn creaking_synched_rows(
    tick: &TickResult,
    entity_id: i32,
) -> std::collections::BTreeMap<u8, EntityDataValueKind> {
    let mut rows = std::collections::BTreeMap::new();
    for event in &tick.events {
        if let WorldEvent::Send {
            packet: ClientboundPlayPacket::SetEntityData(data),
            ..
        } = event
        {
            if data.id == entity_id {
                for value in &data.values {
                    rows.insert(value.data_id, value.value.clone());
                }
            }
        }
    }
    rows
}

pub(in crate::tests::simulation) fn creaking_memory_line<'a>(
    dump: &'a DebugBrainDump,
    module: &str,
) -> Option<&'a str> {
    dump.memories
        .iter()
        .find(|line| line.starts_with(&format!("{module}:")))
        .map(String::as_str)
}

pub(in crate::tests::simulation) fn uuid_scanning_players_immediately() -> Uuid {
    Uuid::from_u128(0x7e57)
}
