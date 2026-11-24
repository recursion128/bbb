use super::*;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::save::Java26BuiltinDimension;
use serde_json::{json, Value};
use std::sync::Arc;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const POS: BlockPos = BlockPos { x: 2, y: 70, z: 2 };
const BURNOUT: &str = "minecraft:gameplay/increased_fire_burnout";
const PORTAL: &str = "minecraft:gameplay/nether_portal_spawns_piglin";
const SUN: &str = "minecraft:visual/sun_angle";

fn edit(startup: &mut DatapackStartup, registry: &str, id: &str, change: impl FnOnce(&mut Value)) {
    let element = startup
        .registries
        .registries
        .iter_mut()
        .find(|entry| entry.registry_id == registry)
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .unwrap();
    let mut value: Value = serde_json::from_slice(&element.source_json).unwrap();
    change(&mut value);
    element.source_json = serde_json::to_vec(&value).unwrap().into();
    element.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(value)).unwrap())
            .unwrap(),
    );
    element.codec_value = if registry == "minecraft:timeline" {
        let value = bbb_registry_codec::decode_exact_synchronized_registry_element(
            registry,
            &bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap(),
        )
        .unwrap();
        Some(
            bbb_registry_codec::registry_codec::CanonicalRegistryCodecValue {
                network_nbt: value.projected_network_nbt().unwrap(),
                references: vec![],
            },
        )
    } else {
        None
    };
}

fn world() -> ServerWorld {
    let mut config = crate::tests::datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    edit(
        startup,
        "minecraft:dimension_type",
        "minecraft:overworld",
        |value| {
            value["attributes"][BURNOUT] = json!(true);
            value["attributes"][PORTAL] = json!(false);
            value["attributes"][SUN] = json!(0.0);
            value["timelines"] = json!(["minecraft:early_game"]);
        },
    );
    edit(
        startup,
        "minecraft:worldgen/biome",
        "minecraft:desert",
        |value| {
            value["attributes"][BURNOUT] = json!({"modifier":"override","argument":false});
            value["attributes"][PORTAL] = json!({"modifier":"override","argument":true});
            value["attributes"][SUN] = json!(180.0);
        },
    );
    edit(
        startup,
        "minecraft:timeline",
        "minecraft:early_game",
        |value| {
            *value = json!({"clock":"minecraft:overworld", "period_ticks":20, "tracks": {
                PORTAL: {"modifier":"and", "keyframes":[
                    {"ticks":0,"value":true}, {"ticks":10,"value":false}
                ]}
            }});
        },
    );
    ServerWorld::new(config).unwrap()
}

fn state(name: &str) -> i32 {
    crate::block_behavior::default_block_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
}

fn biome(world: &mut ServerWorld, pos: BlockPos, name: &str) {
    let biome = world
        .registries
        .registry_element_index("minecraft:worldgen/biome", name)
        .unwrap();
    let chunks = &mut world.level_mut_by_id(&DIMENSION.into()).chunks;
    chunks
        .try_ensure_served_full_column(pos.x >> 4, pos.z >> 4)
        .unwrap();
    chunks
        .column_mut(pos.x >> 4, pos.z >> 4)
        .unwrap()
        .set_noise_biome((pos.x >> 2) & 3, pos.y >> 2, (pos.z >> 2) & 3, biome)
        .unwrap();
}

fn clock(world: &mut ServerWorld, ticks: i64) {
    let clock = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    let mut state = world.world_clock_states_for_test()[clock];
    state.total_ticks = ticks;
    world.set_world_clock_state_for_test(clock, state);
}

#[test]
fn daylight_detector_uses_its_biome_sun_angle() {
    let mut world = world();
    let other = BlockPos { x: 10, ..POS };
    for (pos, name, expected) in [
        (POS, "minecraft:plains", 15),
        (other, "minecraft:desert", 0),
    ] {
        biome(&mut world, pos, name);
        let detector = state("minecraft:daylight_detector");
        world.set_block(DIMENSION, pos, detector, 2 | 16);
        world.with_level(DIMENSION, |level| {
            crate::block_behavior::daylight_detector::update_signal_strength(level, pos, detector);
        });
        assert_eq!(
            crate::block_behavior::daylight_detector::signal(
                world.block_state(DIMENSION, pos).unwrap()
            ),
            expected
        );
    }
}

#[test]
fn fire_burnout_uses_dimension_and_biome_modifier() {
    use crate::block_behavior::fire;
    let mut world = world();
    let neighbour = BlockPos {
        x: POS.x + 1,
        ..POS
    };
    let hay = state("minecraft:hay_block");
    let fire = state("minecraft:fire");
    let infiniburn = world.level_infiniburn_blocks_for_test(DIMENSION);
    for (name, burned) in [("minecraft:plains", true), ("minecraft:desert", false)] {
        biome(&mut world, POS, name);
        world.set_block(
            DIMENSION,
            BlockPos {
                y: POS.y - 1,
                ..POS
            },
            state("minecraft:netherrack"),
            2 | 16,
        );
        world.set_raw_block_state_for_test(DIMENSION, POS, fire);
        world.set_block(DIMENSION, neighbour, hay, 2 | 16);
        world.with_level(DIMENSION, |level| {
            level.level_random().reseed(6);
            let facts = fire::FireFacts {
                infiniburn_blocks: &infiniburn,
                players: &[],
                fire_spread_radius: -1,
                difficulty_id: 3,
                spawning_monsters: true,
                raining: false,
            };
            assert_eq!(fire::tick_with_facts(level, &facts, POS, fire), Some(()));
        });
        assert_eq!(
            world.block_state(DIMENSION, neighbour).unwrap() != hay,
            burned,
            "{name}"
        );
    }
}

#[test]
fn portal_spawn_uses_biome_and_timeline_boolean_modifiers() {
    let mut world = world();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        crate::connection::ConnectionId(1),
        Vec3d {
            x: 2.5,
            y: 70.0,
            z: 6.5,
        },
    );
    world.set_block(
        DIMENSION,
        BlockPos {
            y: POS.y - 1,
            ..POS
        },
        state("minecraft:obsidian"),
        2 | 16,
    );
    let portal = state("minecraft:nether_portal");
    world.set_raw_block_state_for_test(DIMENSION, POS, portal);
    for (name, ticks, count) in [
        ("minecraft:plains", 0, 0),
        ("minecraft:desert", 0, 1),
        ("minecraft:desert", 10, 1),
    ] {
        biome(&mut world, POS, name);
        clock(&mut world, ticks);
        world.with_level(DIMENSION, |level| level.level_random().reseed(329));
        world.run_live_random_tick_sample_for_test(DIMENSION, POS, portal);
        let found = world
            .stored_entity_ids_for_test(DIMENSION)
            .into_iter()
            .filter(|id| {
                world
                    .stored_entity_for_test(DIMENSION, *id)
                    .unwrap()
                    .entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
            })
            .count();
        assert_eq!(found, count, "{name} at {ticks}");
    }
}
