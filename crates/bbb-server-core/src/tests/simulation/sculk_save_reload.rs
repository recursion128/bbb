//! The three sculk listener families through a real save and a reload: the
//! records `LevelChunk.getBlockEntityNbtForSaving`
//! produces, the `BlockEntity.loadStatic` ->
//! `LevelChunk.addAndRegisterBlockEntity` promotion
//! (`ChunkStore` restore), and what a delivered game event still
//! drives after the reload.

use super::*;

use crate::block_entity::game_event::block_pos_center;
use crate::block_entity::vibration::{VibrationData, VibrationInfo, VibrationSelector};
use crate::entity::{EntityData, GeneralLivingData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;

use super::helpers::{
    catalyst_state, place, sculk_catalyst_cursors as catalyst_cursors, sensor_state, shrieker_state,
};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
/// One listener per section, so the reload has to rebuild three of them: the
/// sensor in section 4, the shrieker in section 5, the catalyst in section 6.
const SENSOR: BlockPos = BlockPos { x: 0, y: 72, z: 0 };
/// Eight blocks above the sensor, which is exactly its listener radius.
const SHRIEKER: BlockPos = BlockPos { x: 0, y: 80, z: 0 };
const CATALYST: BlockPos = BlockPos { x: 12, y: 96, z: 0 };
/// A fourth listener the scenario breaks before the save, in the shrieker's
/// own section and four blocks below the catalyst, so a later `ENTITY_DIE` at
/// its own position would reach both.
const REMOVED: BlockPos = BlockPos { x: 12, y: 92, z: 0 };
/// The block change the sensor hears, four blocks below it and far enough from
/// the other two listeners that only the sensor takes it.
const SOURCE: BlockPos = BlockPos { x: 0, y: 68, z: 0 };

/// `SculkShriekerBlockEntity.warningLevel` (`:54`) off the live object.
fn warning_level(world: &ServerWorld, pos: BlockPos) -> i32 {
    world
        .get_block_entity(DIM, pos)
        .and_then(|entity| entity.sculk_shrieker())
        .expect("the shrieker's block entity")
        .warning_level()
}

/// `SculkShriekerBlockEntity.getVibrationData` (`:64-67`).
fn shrieker_vibration(world: &ServerWorld, pos: BlockPos) -> VibrationData {
    world
        .get_block_entity(DIM, pos)
        .and_then(|entity| entity.sculk_shrieker())
        .expect("the shrieker's block entity")
        .vibration_data()
        .clone()
}

/// `SculkSensorBlockEntity.getLastVibrationFrequency` (`:63-65`).
fn sensor_frequency(world: &ServerWorld, pos: BlockPos) -> i32 {
    world
        .get_block_entity(DIM, pos)
        .and_then(|entity| entity.sculk_sensor())
        .expect("the sensor's block entity")
        .last_vibration_frequency()
}

/// `SculkSensorBlockEntity.getVibrationData` (`:54-57`).
fn sensor_vibration(world: &ServerWorld, pos: BlockPos) -> VibrationData {
    world
        .get_block_entity(DIM, pos)
        .and_then(|entity| entity.sculk_sensor())
        .expect("the sensor's block entity")
        .vibration_data()
        .clone()
}

/// The listener's own `VibrationSystem.Data`, written the way a delivered
/// event leaves it mid-travel.
fn mid_travel(event: &'static str, distance: f32, pos: Vec3d, ticks: i32) -> VibrationData {
    let mut data = VibrationData::default();
    data.set_current_vibration(Some(VibrationInfo {
        game_event: event,
        distance,
        pos,
        source: Some(Uuid::from_u128(1)),
        projectile_owner: None,
    }));
    data.set_travel_time_in_ticks(ticks);
    data
}

fn set_vibration_data(world: &mut ServerWorld, pos: BlockPos, data: VibrationData) {
    let entity = world
        .level_block_entity_for_test(DIM, pos)
        .expect("the listener's block entity");
    if let Some(shrieker) = entity.sculk_shrieker_mut() {
        *shrieker.vibration_data_mut_for_test() = data;
        return;
    }
    let sensor = entity.sculk_sensor_mut().expect("a vibration listener");
    *sensor.vibration_data_mut_for_test() = data;
}

/// The three fields `VibrationSystem.Data.CODEC` (`:123-135`) carries: the
/// current vibration, the travel clock and the selector. The reload flag is
/// the codec's constructor's own, so it is not part of a round trip.
fn codec_fields(data: &VibrationData) -> (Option<VibrationInfo>, i32, VibrationSelector) {
    (
        data.current_vibration().copied(),
        data.travel_time_in_ticks(),
        data.selection_strategy().clone(),
    )
}

/// The record the chunk document holds for one position.
fn saved_record(parsed: &Java26ChunkData, pos: BlockPos) -> Option<&[NbtSummaryEntry]> {
    parsed.block_entities().iter().find_map(|record| {
        ["x", "y", "z"]
            .into_iter()
            .zip([pos.x, pos.y, pos.z])
            .all(|(name, value)| {
                record
                    .iter()
                    .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Int(value))
            })
            .then_some(record.as_slice())
    })
}

fn field<'a>(record: &'a [NbtSummaryEntry], name: &str) -> &'a NbtSummaryValue {
    record
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
        .unwrap_or_else(|| panic!("the record carries {name}: {record:?}"))
}

fn compound(value: &NbtSummaryValue) -> &[NbtSummaryEntry] {
    let NbtSummaryValue::Compound(entries) = value else {
        panic!("a saved compound field is a compound: {value:?}");
    };
    entries
}

/// A zombie standing at the death position, whose vanilla experience reward is
/// five (`Mob.getBaseExperienceReward`).
fn spawn_zombie(world: &mut ServerWorld, pos: BlockPos) -> i32 {
    let data = EntityData::GeneralLiving(
        GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).expect("a zombie is living"),
    );
    world
        .spawn_stored_entity_for_test(
            DIM,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            block_pos_center(pos),
            data,
        )
        .0
}

/// The whole loop: the three listener families hold non-default state produced
/// by real deliveries, the chunk is written entire, a fresh world is restored
/// from those bytes, every record and registration comes back, and a game
/// event delivered after the reload still drives each listener.
#[test]
fn the_three_sculk_listeners_survive_a_save_and_a_reload() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 4.5,
            y: 72.0,
            z: 4.5,
        },
    );
    // `WardenSpawnTracker.tryWarn` searches for non-creative players
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    for (pos, state) in [
        (SHRIEKER, shrieker_state(false, true)),
        (SENSOR, sensor_state("inactive", 0, false)),
        (CATALYST, catalyst_state(false)),
        (REMOVED, catalyst_state(false)),
    ] {
        place(&mut world, pos, state);
    }
    world.tick(receiver.drain());
    let air = world.fixture().air_state_id();
    assert!(world.set_block(DIM, REMOVED, air, SET_BLOCK_UPDATE_ALL));
    world.tick(receiver.drain());

    // A real block change reaches the sensor, whose tendrils event reaches the
    // shrieker: the state this save has to carry is produced, not written.
    let player = world.player_entity_id_for_test(ConnectionId(1));
    world.record_game_event_for_test(
        DIM,
        VanillaGameEvent::BLOCK_CHANGE,
        SOURCE,
        Some(player),
        None,
    );
    for _ in 0..24 {
        world.tick(receiver.drain());
        if world.block_state(DIM, SHRIEKER) == Some(shrieker_state(true, true)) {
            break;
        }
    }
    assert_eq!(
        world.block_state(DIM, SHRIEKER),
        Some(shrieker_state(true, true)),
        "the sensor's tendrils travelled to the shrieker"
    );
    assert_eq!(warning_level(&world, SHRIEKER), 1);
    assert_eq!(
        world.block_state(DIM, SENSOR),
        Some(sensor_state("active", 8, false))
    );
    assert_eq!(sensor_frequency(&world, SENSOR), 11);

    // A real `ENTITY_DIE` charges the catalyst's spreader and blooms it.
    let zombie = spawn_zombie(&mut world, REMOVED);
    world.record_game_event_for_test(
        DIM,
        VanillaGameEvent::ENTITY_DIE,
        REMOVED,
        Some(zombie),
        None,
    );
    world.tick(receiver.drain());
    assert_eq!(world.block_state(DIM, CATALYST), Some(catalyst_state(true)));
    let cursor = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "pos".into(),
            value: NbtSummaryValue::IntArray(vec![REMOVED.x, REMOVED.y + 1, REMOVED.z]),
        },
        NbtSummaryEntry {
            name: "charge".into(),
            value: NbtSummaryValue::Int(5),
        },
        NbtSummaryEntry {
            name: "decay_delay".into(),
            value: NbtSummaryValue::Int(0),
        },
        NbtSummaryEntry {
            name: "update_delay".into(),
            value: NbtSummaryValue::Int(1),
        },
    ]);
    assert_eq!(catalyst_cursors(&world, CATALYST), vec![cursor.clone()]);

    // The vibration record both families carry across the save, mid-travel.
    let shrieker_data = mid_travel(
        "minecraft:sculk_sensor_tendrils_clicking",
        8.0,
        block_pos_center(SENSOR),
        6,
    );
    let sensor_data = mid_travel("minecraft:block_change", 4.0, block_pos_center(SOURCE), 6);
    set_vibration_data(&mut world, SHRIEKER, shrieker_data.clone());
    set_vibration_data(&mut world, SENSOR, sensor_data.clone());

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    // The fixture column was generated rather than loaded, so the writer
    // produces a whole document for it.
    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the fixture column is written entire");
    let parsed = Java26ChunkData::parse(created.document.clone(), restore_chunk_context())
        .expect("the runtime's own document is a valid Java 26.1 chunk");

    // What the writer actually put in the document.
    let shrieker_record = saved_record(&parsed, SHRIEKER).expect("the shrieker's record");
    assert_eq!(
        field(shrieker_record, "id"),
        &NbtSummaryValue::string("minecraft:sculk_shrieker")
    );
    assert_eq!(
        field(shrieker_record, "warning_level"),
        &NbtSummaryValue::Int(1)
    );
    let listener = compound(field(shrieker_record, "listener"));
    let event = compound(field(listener, "event"));
    assert_eq!(
        field(event, "game_event"),
        &NbtSummaryValue::string("minecraft:sculk_sensor_tendrils_clicking")
    );
    assert_eq!(field(event, "distance"), &nbt_float(8.0));
    assert_eq!(
        field(event, "pos"),
        &NbtSummaryValue::List(vec![nbt_double(0.5), nbt_double(72.5), nbt_double(0.5),])
    );
    assert_eq!(
        field(event, "source"),
        &NbtSummaryValue::IntArray(vec![0, 0, 0, 1]),
        "`UUIDUtil.uuidToIntArray` is the shape the codec reads back"
    );
    assert_eq!(
        field(listener, "event_delay"),
        &NbtSummaryValue::Int(6),
        "the travel clock is the record's own `event_delay`"
    );
    assert_eq!(
        field(compound(field(listener, "selector")), "tick"),
        &NbtSummaryValue::Long(-1),
        "no candidate is waiting"
    );

    let sensor_record = saved_record(&parsed, SENSOR).expect("the sensor's record");
    assert_eq!(
        field(sensor_record, "id"),
        &NbtSummaryValue::string("minecraft:sculk_sensor")
    );
    assert_eq!(
        field(sensor_record, "last_vibration_frequency"),
        &NbtSummaryValue::Int(11)
    );
    assert_eq!(
        field(compound(field(sensor_record, "listener")), "event_delay"),
        &NbtSummaryValue::Int(6)
    );

    let catalyst_record = saved_record(&parsed, CATALYST).expect("the catalyst's record");
    assert_eq!(
        field(catalyst_record, "id"),
        &NbtSummaryValue::string("minecraft:sculk_catalyst")
    );
    assert_eq!(
        field(catalyst_record, "cursors"),
        &NbtSummaryValue::List(vec![cursor.clone()]),
        "`SculkSpreader.save` writes the cursor list as it stands"
    );
    assert!(
        saved_record(&parsed, REMOVED).is_none(),
        "the listener broken before the save has no record"
    );

    // The reload: a fresh world over the document the writer produced.
    let reloaded = restore_candidate(world.game_time(), Some(parsed), Vec::new());
    let mut restored = spawn_pinned_restore(datapack_config(), &reloaded);

    assert_eq!(
        restored.block_state(DIM, SHRIEKER),
        Some(shrieker_state(true, true))
    );
    assert_eq!(warning_level(&restored, SHRIEKER), 1);
    assert_eq!(
        codec_fields(&shrieker_vibration(&restored, SHRIEKER)),
        codec_fields(&shrieker_data),
        "the shrieker's vibration record round-tripped"
    );
    assert_eq!(
        restored.block_state(DIM, SENSOR),
        Some(sensor_state("active", 8, false))
    );
    assert_eq!(sensor_frequency(&restored, SENSOR), 11);
    assert_eq!(
        codec_fields(&sensor_vibration(&restored, SENSOR)),
        codec_fields(&sensor_data),
        "the sensor's vibration record round-tripped"
    );
    assert_eq!(
        restored.block_state(DIM, CATALYST),
        Some(catalyst_state(true))
    );
    assert_eq!(catalyst_cursors(&restored, CATALYST), vec![cursor.clone()]);
    assert!(
        restored.get_block_entity(DIM, REMOVED).is_none(),
        "no object was promoted for the broken listener"
    );

    // The registrations: every section is rebuilt with exactly the listeners
    // that survived, and the one the broken listener shared with the shrieker
    // holds only the shrieker again.
    let listeners_in = |pos: BlockPos| {
        restored
            .game_event_listener_section_for_test(DIM, pos)
            .unwrap_or_else(|| panic!("the section holding {pos:?} registered"))
            .into_iter()
            .map(|entry| entry.pos)
            .collect::<Vec<_>>()
    };
    assert_eq!(listeners_in(SENSOR), vec![SENSOR]);
    assert_eq!(listeners_in(SHRIEKER), vec![SHRIEKER]);
    assert_eq!(listeners_in(CATALYST), vec![CATALYST]);

    // A session is needed again for `tryWarn`, and the in-flight records the
    // save carried arrive as the inert no-ops the state makes them.
    join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut restored,
        1,
        Vec3d {
            x: 4.5,
            y: 72.0,
            z: 4.5,
        },
    );
    restored.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let player = restored.player_entity_id_for_test(ConnectionId(1));
    // The records the save carried resume and complete: the shrieker is still
    // SHRIEKING and the sensor still ACTIVE, so neither arrival has an effect,
    // and both clocks run out on their own.
    for _ in 0..8 {
        restored.tick(receiver.drain());
        if shrieker_vibration(&restored, SHRIEKER)
            .current_vibration()
            .is_none()
            && sensor_vibration(&restored, SENSOR)
                .current_vibration()
                .is_none()
        {
            break;
        }
    }
    assert!(
        shrieker_vibration(&restored, SHRIEKER)
            .current_vibration()
            .is_none()
            && sensor_vibration(&restored, SENSOR)
                .current_vibration()
                .is_none(),
        "the mid-travel records completed after the reload"
    );
    // The shrieker's saved 90-tick reset is still pending, and the sensor's
    // deactivate tick with it; wait for both to answer the quiet state a fresh
    // event needs.
    for _ in 0..140 {
        restored.tick(receiver.drain());
        if restored.block_state(DIM, SHRIEKER) == Some(shrieker_state(false, true))
            && restored.block_state(DIM, SENSOR) == Some(sensor_state("inactive", 0, false))
        {
            break;
        }
    }
    assert_eq!(
        restored.block_state(DIM, SHRIEKER),
        Some(shrieker_state(false, true)),
        "the shrieker's scheduled reset survived the reload"
    );
    assert_eq!(
        restored.block_state(DIM, SENSOR),
        Some(sensor_state("inactive", 0, false)),
        "the sensor's scheduled deactivate survived the reload"
    );

    // A game event delivered after the reload drives all three families again.
    restored.record_game_event_for_test(
        DIM,
        VanillaGameEvent::BLOCK_CHANGE,
        SOURCE,
        Some(player),
        None,
    );
    for _ in 0..24 {
        restored.tick(receiver.drain());
        if restored.block_state(DIM, SHRIEKER) == Some(shrieker_state(true, true)) {
            break;
        }
    }
    assert_eq!(
        restored.block_state(DIM, SENSOR),
        Some(sensor_state("active", 8, false)),
        "the reloaded sensor activated on a real block change"
    );
    assert_eq!(sensor_frequency(&restored, SENSOR), 11);
    assert_eq!(
        restored.block_state(DIM, SHRIEKER),
        Some(shrieker_state(true, true)),
        "the reloaded shrieker shrieked on the sensor's tendrils"
    );
    assert_eq!(warning_level(&restored, SHRIEKER), 1);
    let (raised, _) = restored.game_events(DIM);
    assert!(
        raised.iter().any(|event| {
            event.game_event.resource_id() == "minecraft:shriek"
                && event.pos == SHRIEKER
                && event.source_entity_id == Some(player)
        }),
        "the reloaded shrieker raised GameEvent.SHRIEK: {raised:?}"
    );

    let zombie = spawn_zombie(&mut restored, REMOVED);
    restored.record_game_event_for_test(
        DIM,
        VanillaGameEvent::ENTITY_DIE,
        REMOVED,
        Some(zombie),
        None,
    );
    restored.tick(receiver.drain());
    assert_eq!(
        restored.block_state(DIM, CATALYST),
        Some(catalyst_state(true)),
        "the reloaded catalyst bloomed"
    );
    // The cursor the save carried kept travelling on the reloaded level, which
    // now ticks its spreader, so the die writes a fresh cursor beside whatever
    // is left of it; the travel itself is pinned in `sculk_spread.rs`.
    assert!(
        catalyst_cursors(&restored, CATALYST).contains(&cursor),
        "the reloaded catalyst charged a cursor"
    );
}
