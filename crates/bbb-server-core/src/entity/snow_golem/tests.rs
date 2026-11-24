use super::*;
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::{EntityData, EntityIdCounter};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::packets::Vec3d;
use std::sync::Arc;

fn set(native: &mut NativeLevel, pos: BlockPos, block: &str) {
    let block = state(block);
    native.world.set_block(DIMENSION, pos, block, 2);
    assert_eq!(native.with_level(|level| level.block_state(pos)), block);
}

fn body(position: Vec3d, no_ai: bool) -> (EntityStore, Box<Entity>) {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, position)
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!();
    };
    data.set_no_ai(no_ai);
    (store, entity)
}

fn run_tick(native: &mut NativeLevel, store: &mut EntityStore, entity: &mut Entity) {
    assert!(
        native.with_level(|level| { crate::entity::general_living::tick(entity, store, level) })
    );
}

fn griefing(native: &mut NativeLevel, value: bool) {
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        1,
        &format!("gamerule minecraft:mob_griefing {value}"),
    );
}

fn biome_overrides() -> Arc<bbb_server_datapack::startup::DatapackStartup> {
    let mut startup = crate::registry_sync::builtin_datapack_startup_shared();
    let biomes = Arc::make_mut(&mut startup)
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/biome")
        .unwrap();
    for (id, melts) in [("minecraft:plains", true), ("minecraft:desert", false)] {
        let biome = biomes.elements.iter_mut().find(|e| e.id == id).unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&biome.source_json).unwrap();
        json["attributes"]["minecraft:gameplay/snow_golem_melts"] = melts.into();
        biome.source_json = serde_json::to_vec(&json).unwrap().into();
        biome.source = Some(
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(json)).unwrap())
                .unwrap(),
        );
    }
    startup
}

#[test]
fn snow_golem_melting_reads_actual_biome_overrides_with_no_ai_and_griefing_disabled() {
    let startup = biome_overrides();
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    for no_ai in [false, true] {
        for enabled in [false, true] {
            for (biome, melts) in [("minecraft:plains", true), ("minecraft:desert", false)] {
                let mut native = NativeLevel::with_datapacks(Arc::clone(&startup));
                assert!(native
                    .world
                    .set_noise_biome_for_test(DIMENSION, (0, 16, 0), biome));
                set(&mut native, BlockPos { y: 63, ..pos }, "minecraft:stone");
                griefing(&mut native, enabled);
                let (mut store, mut entity) = body(position, no_ai);
                let before = entity.living().unwrap().health();
                run_tick(&mut native, &mut store, &mut entity);
                assert_eq!(
                    before - entity.living().unwrap().health(),
                    if melts { 1.0 } else { 0.0 }
                );
                assert_eq!(
                    native.with_level(|level| level.block_state(pos)),
                    state(if enabled {
                        "minecraft:snow"
                    } else {
                        "minecraft:air"
                    })
                );
            }
        }
    }
}

fn corner_level() -> NativeLevel {
    let mut native = NativeLevel::new();
    for chunk in [(-1, -1), (-1, 0), (0, -1)] {
        native
            .world
            .load_entity_chunk_for_test(DIMENSION, chunk)
            .unwrap();
    }
    for x in [-1, 0] {
        for z in [-1, 0] {
            set(&mut native, BlockPos { x, y: 63, z }, "minecraft:stone");
        }
    }
    native
}

#[test]
fn snow_golem_places_four_corner_layers_across_negative_coordinates_with_event_context() {
    let mut native = corner_level();
    let (mut store, mut entity) = body(
        Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        true,
    );
    let before = entity.living().unwrap().health();
    run_tick(&mut native, &mut store, &mut entity);
    assert_eq!(entity.living().unwrap().health(), before);
    let snow = state("minecraft:snow");
    let events = native.world.positioned_game_events_for_test(DIMENSION);
    for x in [-1, 0] {
        for z in [-1, 0] {
            let pos = BlockPos { x, y: 64, z };
            assert_eq!(native.with_level(|level| level.block_state(pos)), snow);
            assert!(events.iter().any(|event| {
                event.recorded.game_event == VanillaGameEvent::BLOCK_PLACE
                    && event.recorded.pos == pos
                    && event.recorded.source_entity_id == Some(entity.id())
                    && event.recorded.affected_state == Some(snow)
            }));
        }
    }
}

#[test]
fn snow_golem_requires_snow_support_and_keeps_occupied_blocks() {
    let mut native = corner_level();
    let unsupported = BlockPos {
        x: -1,
        y: 64,
        z: -1,
    };
    let occupied = BlockPos { x: 0, y: 64, z: -1 };
    set(
        &mut native,
        BlockPos {
            y: 63,
            ..unsupported
        },
        "minecraft:ice",
    );
    set(
        &mut native,
        BlockPos { y: 63, ..occupied },
        "minecraft:dirt",
    );
    set(&mut native, occupied, "minecraft:short_grass");
    let (mut store, mut entity) = body(
        Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        true,
    );
    run_tick(&mut native, &mut store, &mut entity);
    assert_eq!(
        native.with_level(|level| level.block_state(unsupported)),
        state("minecraft:air")
    );
    assert_eq!(
        native.with_level(|level| level.block_state(occupied)),
        state("minecraft:short_grass")
    );
    for x in [-1, 0] {
        assert_eq!(
            native.with_level(|level| level.block_state(BlockPos { x, y: 64, z: 0 })),
            state("minecraft:snow")
        );
    }
}
