//! The world simulating itself: chunk tickets, scheduled and fluid ticks, the
//! frozen clock, and the redstone mechanisms driven through a real level.

use super::*;

use std::collections::HashSet;

use crate::block_behavior::BlockMobSpawnRequest;
use crate::entity::{EntityLifecycleEvent, GroundPathType};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    VANILLA_ENTITY_TYPE_SNIFFER_ID, VANILLA_ENTITY_TYPE_TADPOLE_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::SoundEventHolder;

mod analog_comparator;
mod beacon;
mod beehive;
mod block_entity_removal;
mod block_sound;
mod brain_runtime;
mod conduit;
mod creaking_ai;
mod creaking_alliance;
mod creaking_fall;
mod creaking_freeze;
mod creaking_heart;
mod creaking_protector_sync;
mod creaking_visibility;
mod daylight_burn;
mod death_protection;
mod dripstone_clay;
mod drowned_navigation;
mod entity_die_game_event;
mod entity_game_event_listener;
mod entity_game_event_origins;
mod entity_killed_stat;
mod entity_stack_rows;
mod equipment_attributes;
mod equipment_drop_chance;
mod farmland_push;
mod fox_berries;
mod fox_food;
mod game_event;
mod helpers;
mod internal_inventory_death;
mod mob_death_loot;
mod mob_item_pickup;
mod piglin;
mod piston;
mod redstone_wire;
mod sculk_catalyst;
mod sculk_save_reload;
mod sculk_sensor;
mod sculk_shrieker;
mod sculk_spread;
mod spawner;
mod trial_spawner;
mod vault;
mod warden_spawn_tracker;

#[test]
fn ground_navigation_block_update_cache_hits_and_invalidates_on_live_write() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let dimension = Java26BuiltinDimension::Overworld;
    let pos = BlockPos { x: 8, y: 300, z: 8 };

    world.cache_ground_path_type_for_test(dimension, pos, GroundPathType::Cocoa);
    assert_eq!(
        world.ground_path_type_for_test(dimension, pos),
        Some(GroundPathType::Cocoa)
    );

    let stone = block_state_id("minecraft:stone");
    assert!(world.set_block(dimension, pos, stone, SET_BLOCK_UPDATE_CLIENTS));
    assert_eq!(world.cached_ground_path_type_for_test(dimension, pos), None);
    assert_eq!(
        world.ground_path_type_for_test(dimension, pos),
        Some(GroundPathType::Blocked)
    );
    assert_eq!(
        world.cached_ground_path_type_for_test(dimension, pos),
        Some(GroundPathType::Blocked)
    );
}

#[test]
fn placed_source_water_builds_a_bubble_column_on_its_twentieth_block_tick() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let bottom = BlockPos { x: 1, y: 64, z: 1 };
    let top = BlockPos { x: 1, y: 65, z: 1 };
    let source_water = helpers::state_with("minecraft:water", &[("level", "0")]);
    let bubble_column = helpers::state_with("minecraft:bubble_column", &[("drag", "false")]);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 1, y: 63, z: 1 },
        block_state_id("minecraft:soul_sand"),
        SET_BLOCK_UPDATE_ALL,
    ));
    for pos in [bottom, top] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            source_water,
            SET_BLOCK_UPDATE_ALL,
        ));
    }

    for elapsed in 1..20 {
        world.tick(Vec::new());
        assert_eq!(
            live_block(&mut world, bottom),
            Some(source_water),
            "water changed after {elapsed} tick(s)"
        );
    }
    world.tick(Vec::new());
    assert_eq!(live_block(&mut world, bottom), Some(bubble_column));
    assert_eq!(live_block(&mut world, top), Some(bubble_column));
}

#[test]
fn turtle_egg_mob_request_drains_into_the_authoritative_entity_store() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let home_pos = BlockPos { x: 3, y: 70, z: -2 };
    let position = Vec3d {
        x: 3.3,
        y: 70.0,
        z: -1.7,
    };

    let entity_id = world
        .apply_block_mob_spawn_request_for_test(
            Java26BuiltinDimension::Overworld,
            BlockMobSpawnRequest::HatchedTurtle { position, home_pos },
        )
        .expect("the turtle factory accepts its catalog type");
    let turtle = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("spawned turtle remains in the level store");
    assert_eq!(turtle.entity_type_id(), VANILLA_ENTITY_TYPE_TURTLE_ID);
    assert_eq!(turtle.position(), position);
}

#[test]
fn frogspawn_mob_request_drains_into_the_authoritative_entity_store() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let position = Vec3d {
        x: 4.2,
        y: 63.5,
        z: -6.3,
    };

    let entity_id = world
        .apply_block_mob_spawn_request_for_test(
            Java26BuiltinDimension::Overworld,
            BlockMobSpawnRequest::FrogspawnTadpole {
                position,
                y_rot: 271.0,
            },
        )
        .expect("the tadpole factory accepts its catalog type");
    let tadpole = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("spawned tadpole remains in the level store");
    assert_eq!(tadpole.entity_type_id(), VANILLA_ENTITY_TYPE_TADPOLE_ID);
    assert_eq!(tadpole.position(), position);
    assert_eq!((tadpole.y_rot(), tadpole.x_rot()), (271.0, 0.0));
}

#[test]
fn ageable_hatch_requests_drain_into_the_authoritative_entity_store() {
    for (request, entity_type_id, position, y_rot) in [
        (
            BlockMobSpawnRequest::HatchedSniffer {
                position: Vec3d {
                    x: 5.5,
                    y: 64.5,
                    z: 6.5,
                },
                y_rot: -120.0,
            },
            VANILLA_ENTITY_TYPE_SNIFFER_ID,
            Vec3d {
                x: 5.5,
                y: 64.5,
                z: 6.5,
            },
            -120.0,
        ),
        (
            BlockMobSpawnRequest::HydratedHappyGhast {
                position: Vec3d {
                    x: 7.5,
                    y: 64.0,
                    z: 8.5,
                },
                y_rot: 90.0,
            },
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            Vec3d {
                x: 7.5,
                y: 64.0,
                z: 8.5,
            },
            90.0,
        ),
    ] {
        let candidate = restore_candidate(10, None, Vec::new());
        let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
        let entity_id = world
            .apply_block_mob_spawn_request_for_test(Java26BuiltinDimension::Overworld, request)
            .expect("the generated catalog accepts the hatch baby");
        let entity = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
            .unwrap();
        assert_eq!(entity.entity_type_id(), entity_type_id);
        assert_eq!(entity.position(), position);
        assert_eq!((entity.y_rot(), entity.x_rot()), (y_rot, 0.0));
    }
}

#[test]
fn restored_sniffer_tick_publishes_exact_sound_add_and_block_order() {
    let pos = BlockPos { x: 1, y: 64, z: 1 };
    let egg = (
        [1_usize, 0_usize, 1_usize],
        "minecraft:sniffer_egg",
        &[("hatch", "2")][..],
    );
    let candidate = restore_candidate(
        10,
        Some(restore_chunk_with_ticks(
            vec![restore_section_with_cells(4, "minecraft:air", &[egg])],
            vec![restore_saved_tick(
                "minecraft:sniffer_egg",
                [1, 64, 1],
                3,
                0,
            )],
            Vec::new(),
        )),
        Vec::new(),
    );
    let (_sender, receiver, mut world) =
        helpers::restored_world(ServerWorldConfig::default(), &candidate);
    let hatched = world.tick(receiver.drain());

    let sound = hatched.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(sound),
            ..
        } => Some(sound),
        _ => None,
    });
    let sound = sound.expect("the hatch sound reaches the nearby player");
    assert_eq!(
        (sound.volume, sound.pitch >= 0.9 && sound.pitch < 1.1),
        (0.7, true)
    );
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:block.sniffer_egg.hatch"
            )
            .unwrap()
        }
    );
    assert_eq!(
        live_block(&mut world, pos),
        Some(block_state_id("minecraft:air"))
    );
}

#[test]
fn restored_dried_ghast_tick_publishes_add_then_entity_sound_before_block() {
    let pos = BlockPos { x: 1, y: 64, z: 1 };
    let dried = (
        [1_usize, 0_usize, 1_usize],
        "minecraft:dried_ghast",
        &[
            ("facing", "west"),
            ("hydration", "3"),
            ("waterlogged", "true"),
        ][..],
    );
    let candidate = restore_candidate(
        10,
        Some(restore_chunk_with_ticks(
            vec![restore_section_with_cells(4, "minecraft:air", &[dried])],
            vec![restore_saved_tick(
                "minecraft:dried_ghast",
                [1, 64, 1],
                3,
                0,
            )],
            Vec::new(),
        )),
        Vec::new(),
    );
    let (_sender, receiver, mut world) =
        helpers::restored_world(ServerWorldConfig::default(), &candidate);
    let hatched = world.tick(receiver.drain());

    let sound = hatched.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SoundEntity(sound),
            ..
        } => Some(sound),
        _ => None,
    });
    let sound = sound.expect("the Ghastling sound reaches the nearby player");
    assert_eq!(
        (sound.source, sound.volume, sound.pitch),
        (SoundSource::Blocks, 1.0, 1.0)
    );
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.ghastling.spawn"
            )
            .unwrap()
        }
    );
    assert_eq!(
        live_block(&mut world, pos),
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID
        )
    );
}

fn place_powered_tnt(world: &mut ServerWorld, pos: BlockPos) {
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: pos.x + 1,
            ..pos
        },
        block_state_id("minecraft:redstone_block"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_TNT_ID,
        )
        .expect("built-in TNT default state"),
        SET_BLOCK_UPDATE_ALL,
    ));
}

#[test]
fn tnt_redstone_priming_publishes_before_sound_event_and_removal() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let pos = BlockPos { x: 3, y: 70, z: 4 };
    let before = world
        .entity_tracking_counts(Java26BuiltinDimension::Overworld)
        .0;

    place_powered_tnt(&mut world, pos);

    assert_eq!(
        live_block(&mut world, pos),
        Some(block_state_id("minecraft:air"))
    );
    assert_eq!(
        world
            .entity_tracking_counts(Java26BuiltinDimension::Overworld)
            .0,
        before + 1
    );
    let sounds = world.drain_block_sounds_for_test(Java26BuiltinDimension::Overworld);
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].pos, pos);
    assert_eq!(sounds[0].resource_id, "minecraft:entity.tnt.primed");
    assert_eq!((sounds[0].volume, sounds[0].pitch), (1.0, 1.0));
    let (events, count) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(count, 1);
    assert_eq!(events[0].game_event.constant_name(), "PRIME_FUSE");
    assert_eq!(events[0].pos, pos);
    assert_eq!(events[0].source_entity_id, None);
}

#[test]
fn tnt_redstone_priming_disabled_rule_keeps_the_block_without_effects() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let pos = BlockPos { x: 5, y: 70, z: 6 };
    world.set_tnt_explodes_for_test(false);
    let before = world
        .entity_tracking_counts(Java26BuiltinDimension::Overworld)
        .0;

    place_powered_tnt(&mut world, pos);

    assert_eq!(
        live_block(&mut world, pos),
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_TNT_ID,
        )
    );
    assert_eq!(
        world
            .entity_tracking_counts(Java26BuiltinDimension::Overworld)
            .0,
        before
    );
    assert!(world
        .drain_block_sounds_for_test(Java26BuiltinDimension::Overworld)
        .is_empty());
    assert_eq!(world.game_events(Java26BuiltinDimension::Overworld).1, 0);
}

#[test]
fn a_restored_block_tick_turns_a_falling_block_into_an_entity_that_lands() {
    // Section Y=4 is world y 64..79, which the flat generator fills with air:
    // sand at y=66 with air under it, and the solid floor at y=63.
    let sand = ([1_usize, 2_usize, 2_usize], "minecraft:sand", &[][..]);
    let candidate = restore_candidate(
        10,
        Some(restore_chunk_with_ticks(
            vec![restore_section_with_cells(4, "minecraft:air", &[sand])],
            vec![restore_saved_tick("minecraft:sand", [1, 66, 2], 0, 0)],
            Vec::new(),
        )),
        Vec::new(),
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert_eq!(
        published_block(&world, BlockPos { x: 1, y: 66, z: 2 }),
        Some(block_state_id("minecraft:sand"))
    );

    // `FallingBlock.tick` replaces the block with a `FallingBlockEntity`, and
    // the entity's own tick lands it on the floor two blocks below.
    let (_sender, receiver) = mailbox();
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        published_block(&world, BlockPos { x: 1, y: 66, z: 2 }),
        None
    );
    assert_eq!(
        published_block(&world, BlockPos { x: 1, y: 64, z: 2 }),
        Some(block_state_id("minecraft:sand"))
    );

    // The cell the sand left is air again, which is what the generator puts
    // there — so it is out of the overlay, and only the record of what the
    // save's own document says keeps it in the write-back.
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .expect("the scenario chunk is saved");
    let parsed = Java26ChunkData::parse(&write.document, restore_chunk_context()).unwrap();
    let section = parsed
        .sections()
        .iter()
        .find(|section| section.y == 4)
        .expect("section y = 4 is saved");
    let states = section.block_states.as_ref().unwrap();
    assert_eq!(
        states.value_at(1, 2, 2).unwrap().name,
        "minecraft:air",
        "the vacated cell must be written back as air"
    );
    assert_eq!(states.value_at(1, 0, 2).unwrap().name, "minecraft:sand");
}

#[test]
fn a_brushable_block_over_air_falls_through_the_production_level() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let pos = BlockPos { x: 1, y: 70, z: 2 };
    // `BrushableBlock.DUSTED` has no default in the
    // name-only lookup, so the untouched value is spelled here.
    let state = helpers::state_with("minecraft:suspicious_sand", &[("dusted", "0")]);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let (_sender, receiver) = mailbox();
    // `BrushableBlock.TICK_DELAY` is two; one more
    // tick leaves the entity in flight rather than landed.
    for _ in 0..3 {
        world.tick(receiver.drain());
    }

    // The cell the block asked from is cleared where it asked, which is the
    // level write `FallingBlockEntity.fall` makes (`:99`).
    assert_eq!(published_block(&world, pos), None);
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .entity_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .expect("the entity chunk is written");
    let data =
        bbb_server_persistence::Java26EntityChunkData::parse(write.document.clone(), 0, 0).unwrap();
    let records = data.entities().collect::<Vec<_>>();
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(records[0].id(), "minecraft:falling_block");

    // `cancelDrop` discards the entity on landing without placing the block
    // so nothing reaches the floor.
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        published_block(&world, BlockPos { x: 1, y: 64, z: 2 }),
        None
    );
}

/// `FallingBlock.onPlace` arms
/// `getDelayAfterPlace()`, which is two for every registration but the dragon
/// egg.
///
/// The delay is only observable as the tick the block leaves its cell on, so
/// each block is walked one tick at a time up to its own.
#[test]
fn a_placed_falling_block_leaves_its_cell_on_the_tick_its_own_delay_names() {
    for (name, delay) in [("minecraft:sand", 2), ("minecraft:dragon_egg", 5)] {
        // Section Y=4 is world y 64..79, which the flat generator fills with
        // air, so the placed block has nothing under it and `isFree` holds.
        let candidate = restore_candidate(10, None, Vec::new());
        let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
        let pos = BlockPos { x: 1, y: 70, z: 2 };
        let state = block_state_id(name);
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            state,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));

        // The publication is republished on a tick, so the placed block is
        // only visible from the first one onwards.
        let (_sender, receiver) = mailbox();
        for tick in 1..delay {
            world.tick(receiver.drain());
            assert_eq!(
                published_block(&world, pos),
                Some(state),
                "{name} at {tick}"
            );
        }
        world.tick(receiver.drain());
        assert_ne!(published_block(&world, pos), Some(state), "{name}");
    }
}

use crate::block_behavior::redstone_lamp;

/// A world with a forced ticket, a dirt floor and an unlit lamp standing on it,
/// answering the lamp's cell and the cell its power goes into.
///
/// The floor is spelled rather than inherited from the generator because it is
/// load-bearing: dirt is a redstone conductor
/// (`BlockBehaviour`, the default `isCollisionShapeFullBlock`
/// predicate), so `SignalGetter.getSignal` on the cell below the lamp folds
/// `getDirectSignalTo` in and reads six further
/// neighbours -- one of which is the lamp itself. That is the whole chain the
/// server world oracle's scenario stands on, and a floor of air would not
/// exercise it.
fn redstone_lamp_world() -> (ServerWorld, BlockPos, BlockPos) {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let lamp = BlockPos { x: 1, y: 64, z: 1 };
    let power = BlockPos { x: 2, y: 64, z: 1 };
    for floor in [
        BlockPos { x: 1, y: 63, z: 1 },
        BlockPos { x: 2, y: 63, z: 1 },
    ] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            floor,
            block_state_id("minecraft:dirt"),
            SET_BLOCK_UPDATE_ALL,
        ));
    }
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        lamp,
        redstone_lamp_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));
    (world, lamp, power)
}

/// The mirror of the server world oracle's `redstone_lamp_release` scenario,
/// on this crate's own `ServerWorld`.
///
/// The oracle drives the same three writes through the official server's
/// `/setblock` and compares the region file two hundred ticks later; this drives
/// them through `Level.setBlock` and watches the four
/// ticks themselves. It is here because every other test of this mechanism runs
/// against `crate::block_behavior`'s in-memory fixture, and a fixture cannot
/// answer whether the real level's `getBlockState`, its neighbour updater and
/// its scheduled-tick drain agree with each other.
#[test]
fn a_released_redstone_lamp_goes_dark_on_the_fourth_tick_of_a_real_world() {
    let (mut world, lamp, power) = redstone_lamp_world();

    // `RedstoneLampBlock.neighborChanged`:
    // lighting up happens inside the write that powered it, with no tick in
    // between. `Level.setBlock`'s own `updateNeighborsAt` is what carries it.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        power,
        block_state_id("minecraft:redstone_block"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(true))
    );

    // Taking the power away arms a tick (`:42-43`) and changes nothing yet.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        power,
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(true))
    );

    for elapsed in 1..redstone_lamp::TICK_DELAY {
        world.tick(Vec::new());
        assert_eq!(
            live_block(&mut world, lamp),
            Some(redstone_lamp_state(true)),
            "still lit after {elapsed} tick(s)"
        );
    }
    // `RedstoneLampBlock.tick` (`:52-56`) on the fourth.
    world.tick(Vec::new());
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false))
    );

    // Nothing on the way there fell back on `crate::redstone`'s fail-closed
    // answer. This is the assertion that separates "the lamp went out" from
    // "the lamp went out for the right reason": a world that refuses the
    // question leaves the lamp lit and says so here.
}

/// The mirror of the oracle's `redstone_lamp_repowered` scenario.
///
/// The pending tick still runs; what stops it writing is the second
/// `hasNeighborSignal` inside `RedstoneLampBlock.tick` (`:53`).
#[test]
fn a_redstone_lamp_repowered_inside_its_delay_stays_lit_in_a_real_world() {
    let (mut world, lamp, power) = redstone_lamp_world();
    let redstone_block = block_state_id("minecraft:redstone_block");

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        power,
        redstone_block,
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        power,
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL,
    ));
    // Back inside the four-tick window. `neighborChanged` (`:41`) finds `LIT`
    // and the signal equal again, so it neither writes nor arms a second tick.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        power,
        redstone_block,
        SET_BLOCK_UPDATE_ALL,
    ));

    for _ in 0..redstone_lamp::TICK_DELAY * 2 {
        world.tick(Vec::new());
    }
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(true)),
        "the pending tick found the signal back and wrote nothing"
    );
}

fn redstone_torch_state(lit: bool) -> i32 {
    helpers::state_with(
        "minecraft:redstone_torch",
        &[("lit", if lit { "true" } else { "false" })],
    )
}

/// A world with a forced ticket and a lit redstone torch standing on dirt,
/// answering the torch's cell and the cell under it.
///
/// The support is the torch's whole input: `RedstoneTorchBlock.hasNeighborSignal`
/// asks `hasSignal(pos.below(), DOWN)` and
/// nothing else, so powering that one cell is what makes the inverter turn
/// over.
fn redstone_torch_world() -> (ServerWorld, BlockPos, BlockPos) {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let torch = BlockPos { x: 1, y: 64, z: 1 };
    // The flat fixture's own floor is already stone here, so the support is
    // written as dirt: the write has to change something for the torch's
    // input to be a cell this test owns.
    let support = BlockPos { x: 1, y: 63, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        support,
        block_state_id("minecraft:dirt"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        torch,
        redstone_torch_state(true),
        SET_BLOCK_UPDATE_ALL,
    ));
    (world, torch, support)
}

/// The torch is an inverter with a two-tick delay, on this crate's own
/// `ServerWorld` rather than on `crate::block_behavior`'s fixture.
///
/// The fixture cannot answer whether the real `Level.setBlock` fan-out reaches
/// the torch at all, whether `crate::redstone` reads the support through the
/// real `getBlockState`, or whether the real scheduled-tick drain runs the
/// body; this drives all three.
#[test]
fn a_redstone_torch_goes_out_two_ticks_after_its_support_is_powered_in_a_real_world() {
    let (mut world, torch, support) = redstone_torch_world();
    assert_eq!(
        live_block(&mut world, torch),
        Some(redstone_torch_state(true)),
        "a placed torch starts lit (`RedstoneTorchBlock.java:40`)"
    );

    // `Level.setBlock`'s own `updateNeighborsAt` carries the
    // update to the torch; `RedstoneTorchBlock.neighborChanged`
    // only arms a tick and writes nothing.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        support,
        block_state_id("minecraft:redstone_block"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        live_block(&mut world, torch),
        Some(redstone_torch_state(true)),
        "neighborChanged never writes"
    );

    for elapsed in 1..crate::block_behavior::redstone_torch::TOGGLE_DELAY {
        world.tick(Vec::new());
        assert_eq!(
            live_block(&mut world, torch),
            Some(redstone_torch_state(true)),
            "still lit after {elapsed} tick(s)"
        );
    }
    // `RedstoneTorchBlock.tick` (`:73-92`) re-reads the input and writes.
    world.tick(Vec::new());
    assert_eq!(
        live_block(&mut world, torch),
        Some(redstone_torch_state(false))
    );

    // And back the other way once the input goes.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        support,
        block_state_id("minecraft:dirt"),
        SET_BLOCK_UPDATE_ALL,
    ));
    for _ in 0..crate::block_behavior::redstone_torch::TOGGLE_DELAY {
        world.tick(Vec::new());
    }
    assert_eq!(
        live_block(&mut world, torch),
        Some(redstone_torch_state(true))
    );
}

/// `RedstoneTorchBlock.onPlace` and
/// `affectNeighborsAfterRemoval` both reach **two** cells out, which
/// is one further than any `Level.setBlock` fan-out goes.
///
/// The lamp here is the instrument. It sits diagonally from the torch, beside
/// the stone the torch strongly powers from below, so the only thing that can
/// ever tell it anything is `notifyNeighbors`: six
/// `updateNeighborsAt` calls centred on the torch's *neighbours*.
/// `Level.setBlock`'s own `updateNeighborsAt` is centred on
/// the torch and stops one cell short of it, and so does the shape fan-out.
///
/// So the lamp lighting up proves `onPlace` ran and the lamp going dark proves
/// `affectNeighborsAfterRemoval` ran; without either leg the lamp would stay as
/// it was forever.
#[test]
fn a_placed_and_a_removed_torch_both_reach_two_cells_out_in_a_real_world() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let torch = BlockPos { x: 1, y: 64, z: 1 };
    let support = BlockPos { x: 1, y: 63, z: 1 };
    // The conductor the torch strongly powers: `getDirectSignal` answers only
    // `DOWN`, and `getDirectSignalTo`
    // asks the cell below with `DOWN`.
    let conductor = BlockPos { x: 1, y: 65, z: 1 };
    let lamp = BlockPos { x: 2, y: 65, z: 1 };
    for (pos, state) in [
        (support, block_state_id("minecraft:dirt")),
        (conductor, block_state_id("minecraft:stone")),
        (lamp, redstone_lamp_state(false)),
    ] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            state,
            SET_BLOCK_UPDATE_ALL,
        ));
    }
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false)),
        "nothing powers the lamp yet"
    );

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        torch,
        redstone_torch_state(true),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(true)),
        "onPlace told the conductor's neighbourhood, which the lamp is in"
    );

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        torch,
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL,
    ));
    // Going dark is the lamp's own four-tick delay
    // not the torch's.
    for _ in 0..redstone_lamp::TICK_DELAY {
        world.tick(Vec::new());
    }
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false)),
        "affectNeighborsAfterRemoval told the same neighbourhood"
    );
}

/// `Level.updateNeighbourForOutputSignal` reaches two
/// cells out through a conductor and tells **only** `Blocks.COMPARATOR`.
///
/// The lamp is again the instrument, and it is deliberately left in the one
/// state that a stray notification would change: unlit while a redstone block
/// above it says it should be lit. It was put into that state with
/// `Block.UPDATE_CLIENTS` alone, which writes the power in without any
/// neighbour fan-out.
///
/// The cell it sits in is exactly the cell the output-signal walk hops to:
/// `updateNeighborsAt` stops one short of it, and the walk
/// itself relays through the conductor beside the changed block.
/// So a walk that dropped the
/// `state.is(Blocks.COMPARATOR)` test would light this lamp, and a
/// correct one leaves it alone.
#[test]
fn a_real_worlds_output_signal_walk_leaves_a_non_comparator_alone() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let source = BlockPos { x: 1, y: 64, z: 1 };
    let conductor = BlockPos { x: 2, y: 64, z: 1 };
    let lamp = BlockPos { x: 3, y: 64, z: 1 };
    let power = BlockPos { x: 3, y: 65, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        conductor,
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        lamp,
        redstone_lamp_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));
    // `Block.UPDATE_CLIENTS` alone: the power lands with no
    // `updateNeighborsAt`, so the lamp is left disagreeing with its own input.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        power,
        block_state_id("minecraft:redstone_block"),
        crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
    ));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false)),
        "the lamp is armed: unlit and powered"
    );

    // `minecraft:cake` answers `hasAnalogOutputSignal`
    // and has no block entity, so it is the smallest thing in this world that
    // opens the `Level` gate.
    let cake = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_CAKE_ID,
    )
    .expect("cake has a default state");
    assert!(crate::redstone::has_analog_output_signal(cake));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        source,
        cake,
        SET_BLOCK_UPDATE_ALL,
    ));
    for _ in 0..redstone_lamp::TICK_DELAY {
        world.tick(Vec::new());
    }
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false)),
        "the output-signal walk hopped to this cell and told it nothing"
    );
}

fn comparator_state(facing: &str) -> i32 {
    helpers::state_with(
        "minecraft:comparator",
        &[
            ("facing", facing),
            ("mode", "compare"),
            ("powered", "false"),
        ],
    )
}

fn item_frame_comparator_world(comparator: BlockPos, conductor: BlockPos) -> ServerWorld {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let _ = world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            y: comparator.y - 1,
            ..comparator
        },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    );
    let _ = world.set_block(
        Java26BuiltinDimension::Overworld,
        conductor,
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        comparator,
        comparator_state("east"),
        SET_BLOCK_UPDATE_ALL,
    ));
    world
}

fn stone_frame_stack() -> bbb_server_types::item_stack::RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 1,
        component_patch: Default::default(),
    })
}

fn spawn_comparator_frame(
    world: &mut ServerWorld,
    attached: BlockPos,
    direction: Direction,
    glow: bool,
    occupied: bool,
    rotations: usize,
) -> i32 {
    let entity_type_id = if glow {
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
    } else {
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_FRAME_ID
    };
    let data = crate::entity::ItemFrameData::new(attached, direction).with_saved_fields(
        occupied.then(stone_frame_stack),
        i32::try_from(rotations).expect("test rotations fit i32"),
        1.0,
        false,
        false,
    );
    let (entity_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        entity_type_id,
        Vec3d {
            x: f64::from(attached.x) + 0.5,
            y: f64::from(attached.y) + 0.5,
            z: f64::from(attached.z) + 0.5,
        },
        if glow {
            crate::entity::EntityData::GlowItemFrame(data)
        } else {
            crate::entity::EntityData::ItemFrame(data)
        },
    );
    crate::entity::hanging::apply_direction(
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, entity_id)
            .expect("the frame is live"),
    );
    entity_id
}

fn live_comparator_output(world: &mut ServerWorld, pos: BlockPos) -> i32 {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .and_then(|entity| entity.comparator())
        .expect("the comparator has its block entity")
        .output_signal()
}

fn run_comparator_delay(world: &mut ServerWorld) {
    world.tick(Vec::new());
    world.tick(Vec::new());
}

/// `ComparatorBlock.getItemFrame` uses an exact direction and unit AABB, and
/// accepts exactly one frame.
#[test]
fn live_item_frame_projection_distinguishes_zero_wrong_facing_and_multiple() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let conductor = BlockPos { x: 1, y: 64, z: 0 };
    let attached = BlockPos { x: 2, y: 64, z: 0 };
    for case in ["empty", "outside-cell", "wrong-facing", "multiple"] {
        let mut world = item_frame_comparator_world(comparator, conductor);
        match case {
            "empty" => {
                spawn_comparator_frame(&mut world, attached, Direction::East, false, false, 0);
            }
            "outside-cell" => {
                spawn_comparator_frame(
                    &mut world,
                    BlockPos {
                        x: attached.x + 1,
                        ..attached
                    },
                    Direction::East,
                    false,
                    true,
                    7,
                );
            }
            "wrong-facing" => {
                spawn_comparator_frame(&mut world, attached, Direction::North, false, true, 7);
            }
            "multiple" => {
                for glow in [false, true] {
                    spawn_comparator_frame(&mut world, attached, Direction::East, glow, true, 7);
                }
            }
            _ => unreachable!(),
        }
        world.item_frame_output_changed_for_test(Java26BuiltinDimension::Overworld, attached);
        run_comparator_delay(&mut world);
        assert_eq!(live_comparator_output(&mut world, comparator), 0, "{case}");
    }
}

/// The scheduled refresh re-queries the current store even when the frame is
/// two cells away across a section boundary.
#[test]
fn live_item_frame_output_crosses_a_conductor_and_section_boundary() {
    let comparator = BlockPos { x: 14, y: 64, z: 0 };
    let conductor = BlockPos { x: 15, y: 64, z: 0 };
    let attached = BlockPos { x: 16, y: 64, z: 0 };
    for (glow, rotations, expected) in [(false, 0, 1), (true, 7, 8)] {
        let mut world = item_frame_comparator_world(comparator, conductor);
        spawn_comparator_frame(&mut world, attached, Direction::East, glow, true, rotations);
        world.item_frame_output_changed_for_test(Java26BuiltinDimension::Overworld, attached);
        run_comparator_delay(&mut world);
        assert_eq!(
            live_comparator_output(&mut world, comparator),
            expected,
            "glow={glow}"
        );
    }
}

#[test]
fn queued_comparator_refresh_re_reads_rotated_cleared_and_removed_frames() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let conductor = BlockPos { x: 1, y: 64, z: 0 };
    let attached = BlockPos { x: 2, y: 64, z: 0 };

    let mut rotated = item_frame_comparator_world(comparator, conductor);
    let rotated_id =
        spawn_comparator_frame(&mut rotated, attached, Direction::East, false, true, 0);
    rotated.item_frame_output_changed_for_test(Java26BuiltinDimension::Overworld, attached);
    rotated
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, rotated_id)
        .unwrap()
        .discard();
    spawn_comparator_frame(&mut rotated, attached, Direction::East, false, true, 1);
    run_comparator_delay(&mut rotated);
    assert_eq!(live_comparator_output(&mut rotated, comparator), 2);

    let mut cleared = item_frame_comparator_world(comparator, conductor);
    let cleared_id =
        spawn_comparator_frame(&mut cleared, attached, Direction::East, false, true, 0);
    cleared.item_frame_output_changed_for_test(Java26BuiltinDimension::Overworld, attached);
    let outcome = cleared
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            cleared_id,
            "minecraft:generic",
            1.0,
            None,
        )
        .expect("the frame remains a live damage target");
    assert!(outcome.applied && outcome.succeeded);
    run_comparator_delay(&mut cleared);
    assert_eq!(live_comparator_output(&mut cleared, comparator), 0);

    let mut removed = item_frame_comparator_world(comparator, conductor);
    let removed_id =
        spawn_comparator_frame(&mut removed, attached, Direction::East, false, true, 0);
    removed.item_frame_output_changed_for_test(Java26BuiltinDimension::Overworld, attached);
    removed
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, removed_id)
        .unwrap()
        .discard();
    run_comparator_delay(&mut removed);
    assert_eq!(live_comparator_output(&mut removed, comparator), 0);
}

/// `TickRateManager` decides, once a tick,
/// whether the world is allowed to change; `/tick freeze` and `/tick step` are
/// the only things that set it.
#[test]
fn a_frozen_world_holds_its_game_time_and_its_scheduled_ticks_until_it_is_stepped() {
    let sand = ([1_usize, 2_usize, 2_usize], "minecraft:sand", &[][..]);
    let candidate = restore_candidate(
        10,
        Some(restore_chunk_with_ticks(
            vec![restore_section_with_cells(4, "minecraft:air", &[sand])],
            vec![restore_saved_tick("minecraft:sand", [1, 66, 2], 0, 0)],
            Vec::new(),
        )),
        Vec::new(),
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    world.tick_rate_mut().set_frozen(true);

    let (_sender, receiver) = mailbox();
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    // `ServerLevel.tickTime` is behind the `runs` gate
    // and so is `blockTicks.tick`, so neither the clock nor the
    // sand moved.
    assert_eq!(world.game_time(), 10);
    assert_eq!(
        published_block(&world, BlockPos { x: 1, y: 66, z: 2 }),
        Some(block_state_id("minecraft:sand"))
    );
    // `MinecraftServer.tickCount` is outside the
    // gate: a frozen server is still a running server, which is what keeps its
    // keep-alives and its time synchronisation going.
    assert_eq!(world.publication().tick, 40);

    assert!(world.tick_rate_mut().step_game_if_paused(40));
    for _ in 0..80 {
        world.tick(Vec::new());
    }
    // Exactly the forty ticks that were asked for ran, and the world froze
    // again on its own.
    assert_eq!(world.game_time(), 50);
    assert!(world.tick_rate().is_frozen());
    assert!(!world.tick_rate().runs_normally());
    assert_eq!(
        published_block(&world, BlockPos { x: 1, y: 66, z: 2 }),
        None
    );
    assert_eq!(
        published_block(&world, BlockPos { x: 1, y: 64, z: 2 }),
        Some(block_state_id("minecraft:sand"))
    );
}

#[test]
fn a_restored_fluid_tick_spreads_water_downwards() {
    let water = (
        [1_usize, 2_usize, 2_usize],
        "minecraft:water",
        &[("level", "0")][..],
    );
    let candidate = restore_candidate(
        10,
        Some(restore_chunk_with_ticks(
            vec![restore_section_with_cells(4, "minecraft:air", &[water])],
            Vec::new(),
            vec![restore_saved_tick("minecraft:water", [1, 66, 2], 0, 0)],
        )),
        Vec::new(),
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let (_sender, receiver) = mailbox();
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    // `FlowingFluid.spread` runs off the fluid scheduler and fills the two air
    // cells between the source and the floor with falling water.
    for y in [65, 64] {
        let state = published_block(&world, BlockPos { x: 1, y, z: 2 })
            .unwrap_or_else(|| panic!("expected water at y={y}"));
        let identity =
            bbb_protocol::block_state_types::vanilla_block_state_identity(state).unwrap();
        assert_eq!(identity.name, "minecraft:water", "at y={y}");
    }
}

/// One saved chunk at an arbitrary column, so a scenario can straddle the
/// boundary a ticket draws.
fn restore_chunk_at(
    chunk_x: i32,
    chunk_z: i32,
    sections: Vec<NbtSummaryValue>,
    block_ticks: Vec<NbtSummaryValue>,
) -> Java26ChunkData {
    let mut raw = restore_fixture_chunk_raw(sections);
    for entry in raw.entries_mut() {
        match entry.name.to_jdk_string().as_str() {
            "xPos" => entry.value = NbtSummaryValue::Int(chunk_x),
            "zPos" => entry.value = NbtSummaryValue::Int(chunk_z),
            "block_ticks" => entry.value = NbtSummaryValue::List(block_ticks.clone()),
            _ => {}
        }
    }
    Java26ChunkData::parse(
        raw,
        Java26ChunkContext {
            expected_x: chunk_x,
            expected_z: chunk_z,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap()
}

/// A save carrying several columns and one `TicketType.FORCED` ticket over
/// chunk (0, 0).
fn restore_candidate_with_chunks(
    game_time: i64,
    chunks: Vec<((i32, i32), Java26ChunkData)>,
) -> Java26WorldCandidate {
    let mut candidate = restore_candidate(game_time, None, Vec::new());
    let dimension = candidate
        .dimensions
        .get_mut(&level_key(Java26BuiltinDimension::Overworld))
        .expect("the fixture candidate carries the overworld");
    for (position, data) in chunks {
        dimension.chunks.insert(
            position,
            Java26LoadedChunk {
                timestamp: 100,
                data,
            },
        );
    }
    candidate
}

/// `/forceload add 0 0` files one ticket at `ChunkMap.FORCED_TICKET_LEVEL`
/// = `ENTITY_TICKING_LEVEL`, so the ticketed column is
/// the only entity-ticking one and each step away costs one level
/// (`ChunkTracker.computeLevelFromNeighbor`): its
/// eight neighbours land on
/// `BLOCK_TICKING_LEVEL` and everything past them is off the tracker.
#[test]
fn a_forced_ticket_simulates_its_own_column_and_block_ticks_the_ring_around_it() {
    let candidate = restore_candidate(10, None, Vec::new());
    let world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        vec![(0, 0)]
    );
    let ring = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |z| (x, z)))
        .collect::<Vec<_>>();
    assert_eq!(
        world.block_ticking_chunks(Java26BuiltinDimension::Overworld),
        ring
    );
}

/// `Entity` adds a radius-three portal ticket;
/// `TicketStorage` stores it at FULL level 33 minus that radius.
/// `ChunkLevel` defines FULL, block and entity levels, while
/// `ChunkLevel` maps FULL to 33: level 30 therefore reaches radii
/// three, two and one respectively.
#[test]
fn a_portal_ticket_simulates_until_its_three_hundred_ticks_run_out() {
    let mut candidate = restore_candidate_without_tickets(10, None, Vec::new());
    candidate.chunk_tickets.insert(
        level_key(Java26BuiltinDimension::Overworld),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 30,
            ticks_left: Some(300),
        }],
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let square = |radius: i32| {
        (-radius..=radius)
            .flat_map(|x| (-radius..=radius).map(move |z| (x, z)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(1),
        "the portal source entity-ticks its radius-one square"
    );
    assert_eq!(
        world.block_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(2),
        "the portal source block-ticks its radius-two square"
    );
    let chunks = world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    for (chunk_x, chunk_z) in square(3) {
        assert!(chunks.is_loaded(chunk_x, chunk_z));
        assert!(chunks.is_ticking_full(chunk_x, chunk_z));
    }
    assert!(
        !chunks.is_ticking_full(4, 0),
        "the FULL reach stops outside radius three"
    );

    let (_sender, receiver) = mailbox();
    // `decreaseTicksLeft` runs once per tick and `isTimedOut` is `< 0`, so 300
    // ticks take it to zero and it is the 301st that drops it.
    for _ in 0..300 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(1),
        "the last tick the ticket survives"
    );
    assert_eq!(
        world.block_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(2),
        "block ticking also survives through tick 300"
    );
    world.tick(receiver.drain());
    assert!(
        world
            .entity_ticking_chunks(Java26BuiltinDimension::Overworld)
            .is_empty(),
        "and the next one takes it below zero"
    );
    assert!(world
        .block_ticking_chunks(Java26BuiltinDimension::Overworld)
        .is_empty());
}

/// `MinecraftServer.prepareLevels` activates
/// the tickets the save carried and adds none of its own, so a save that never
/// forceloaded anything has no simulation source at all while nobody is
/// online: its pending fluid tick stays pending forever.
#[test]
fn a_save_with_no_ticket_and_nobody_online_simulates_nothing() {
    let water = (
        [1_usize, 2_usize, 2_usize],
        "minecraft:water",
        &[("level", "0")][..],
    );
    let candidate = restore_candidate_without_tickets(
        10,
        Some(restore_chunk_with_ticks(
            vec![restore_section_with_cells(4, "minecraft:air", &[water])],
            Vec::new(),
            vec![restore_saved_tick("minecraft:water", [1, 66, 2], 0, 0)],
        )),
        Vec::new(),
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert!(world
        .entity_ticking_chunks(Java26BuiltinDimension::Overworld)
        .is_empty());
    assert!(world
        .block_ticking_chunks(Java26BuiltinDimension::Overworld)
        .is_empty());
    let (_sender, receiver) = mailbox();
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        published_block(&world, BlockPos { x: 1, y: 65, z: 2 }),
        None
    );
    let water_source = helpers::state_with("minecraft:water", &[("level", "0")]);
    assert_eq!(
        published_block(&world, BlockPos { x: 1, y: 66, z: 2 }),
        Some(water_source)
    );
}

/// The two gates are one level apart, and that level is observable: a column
/// one step outside the forced one runs its scheduled block ticks --
/// `FallingBlock.tick` turns the sand into a
/// `FallingBlockEntity` and the block goes -- but the entity that made never
/// ticks, because `ServerLevel`'s entity loop tests
/// `inEntityTickingRange`, so the sand never lands.
/// Two steps out nothing runs at all and the sand is still sand.
#[test]
fn a_block_ticking_neighbour_runs_the_tick_that_makes_an_entity_but_never_ticks_it() {
    let sand = ([1_usize, 2_usize, 2_usize], "minecraft:sand", &[][..]);
    let candidate = restore_candidate_with_chunks(
        10,
        vec![
            (
                (1, 0),
                restore_chunk_at(
                    1,
                    0,
                    vec![restore_section_with_cells(4, "minecraft:air", &[sand])],
                    vec![restore_saved_tick("minecraft:sand", [17, 66, 2], 0, 0)],
                ),
            ),
            (
                (2, 0),
                restore_chunk_at(
                    2,
                    0,
                    vec![restore_section_with_cells(4, "minecraft:air", &[sand])],
                    vec![restore_saved_tick("minecraft:sand", [33, 66, 2], 0, 0)],
                ),
            ),
        ],
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert!(world
        .block_ticking_chunks(Java26BuiltinDimension::Overworld)
        .contains(&(1, 0)));
    assert!(!world
        .entity_ticking_chunks(Java26BuiltinDimension::Overworld)
        .contains(&(1, 0)));
    assert!(!world
        .block_ticking_chunks(Java26BuiltinDimension::Overworld)
        .contains(&(2, 0)));

    let (_sender, receiver) = mailbox();
    for _ in 0..200 {
        world.tick(receiver.drain());
    }
    // Block-ticking column: the block tick ran and took the sand away, and no
    // sand ever appears below it because the entity is frozen where it spawned.
    assert_eq!(
        published_block(&world, BlockPos { x: 17, y: 66, z: 2 }),
        None
    );
    for y in 64..=65 {
        assert_eq!(
            published_block(&world, BlockPos { x: 17, y, z: 2 }),
            None,
            "the entity must not have landed at y={y}"
        );
    }
    // Two columns out: the scheduled tick never ran at all.
    assert_eq!(
        published_block(&world, BlockPos { x: 33, y: 66, z: 2 }),
        Some(block_state_id("minecraft:sand"))
    );
}

fn write_dimension_gameplay_pack(root: &std::path::Path, members: &[&str]) {
    let pack = root.join("datapacks/runtime");
    std::fs::create_dir_all(pack.join("data/test/dimension_type")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/tags/block")).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"test","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    for (element, tag) in [("custom", "custom_fire"), ("missing", "missing_fire")] {
        let source = format!(
            r##"{{
              "has_skylight":false,
              "has_ceiling":false,
              "has_ender_dragon_fight":false,
              "coordinate_scale":1.0,
              "min_y":-64,
              "height":384,
              "logical_height":384,
              "infiniburn":"#test:{tag}",
              "ambient_light":0.0,
              "monster_spawn_light_level":7,
              "monster_spawn_block_light_limit":0
            }}"##
        );
        std::fs::write(
            pack.join(format!("data/test/dimension_type/{element}.json")),
            source,
        )
        .unwrap();
    }
    std::fs::write(
        pack.join("data/test/tags/block/custom_fire.json"),
        serde_json::to_vec(&serde_json::json!({ "values": members })).unwrap(),
    )
    .unwrap();
}

fn write_gameplay_tag_override(
    root: &std::path::Path,
    registry: &str,
    tag: &str,
    members: Option<&[&str]>,
) {
    let path = root
        .join("datapacks/runtime/data/minecraft/tags")
        .join(registry)
        .join(format!("{tag}.json"));
    let Some(members) = members else {
        let _ = std::fs::remove_file(path);
        return;
    };
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({
            "replace": true,
            "values": members,
        }))
        .unwrap(),
    )
    .unwrap();
}

fn turtle_egg_placement_event(world: &mut ServerWorld, pos: BlockPos, support: &str) -> bool {
    let dimension = Java26BuiltinDimension::Overworld;
    world.drain_level_events_for_test(dimension);
    assert!(world.set_block(
        dimension,
        BlockPos {
            y: pos.y - 1,
            ..pos
        },
        block_state_id(support),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        dimension,
        pos,
        helpers::default_block_state("minecraft:turtle_egg"),
        SET_BLOCK_UPDATE_ALL,
    ));
    let placed = world
        .drain_level_events_for_test(dimension)
        .into_iter()
        .any(|event| event.event_type == 2012 && event.pos == pos && event.data == 15);
    assert!(world.set_block(
        dimension,
        pos,
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL,
    ));
    placed
}

fn bubble_column_probe(world: &mut ServerWorld, x: i32) -> i32 {
    let dimension = Java26BuiltinDimension::Overworld;
    let foot = BlockPos { x, y: 63, z: 6 };
    let water_pos = BlockPos { x, y: 64, z: 6 };
    let water_top = BlockPos { x, y: 65, z: 6 };
    let source_water = helpers::state_with("minecraft:water", &[("level", "0")]);
    assert!(world.set_block(
        dimension,
        foot,
        block_state_id("minecraft:soul_sand"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(dimension, water_pos, source_water, SET_BLOCK_UPDATE_ALL,));
    assert!(world.set_block(dimension, water_top, source_water, SET_BLOCK_UPDATE_ALL,));
    for _ in 0..20 {
        world.tick(Vec::new());
    }
    live_block(world, water_pos).unwrap()
}

fn snowy_probe(world: &mut ServerWorld, x: i32, above: &str) -> bool {
    let dimension = Java26BuiltinDimension::Overworld;
    let pos = BlockPos { x, y: 64, z: 10 };
    assert!(world.set_block(
        dimension,
        pos,
        helpers::default_block_state("minecraft:grass_block"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        dimension,
        BlockPos {
            y: pos.y + 1,
            ..pos
        },
        helpers::default_block_state(above),
        SET_BLOCK_UPDATE_ALL,
    ));
    bbb_server_types::block_queries::bool_property(live_block(world, pos).unwrap(), "snowy")
        .unwrap()
}

fn dimension_gameplay_options(
    root: &std::path::Path,
) -> bbb_server_datapack::startup::DatapackStartupOptions {
    bbb_server_datapack::startup::DatapackStartupOptions {
        datapacks_dir: Some(root.join("datapacks")),
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    }
}

#[test]
fn live_dimension_infiniburn_tag_reprojects_after_reload_and_fails_closed() {
    let root = std::env::temp_dir().join(format!("bbb-live-dimension-fire-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write_dimension_gameplay_pack(&root, &["minecraft:stone"]);
    let options = dimension_gameplay_options(&root);
    let startup = bbb_server_datapack::startup::DatapackStartup::load(&options).unwrap();
    let effective_data_packs = startup.selection.data_packs().clone();
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(Arc::new(startup)),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    world.set_level_dimension_type_for_test(Java26BuiltinDimension::Overworld, "test:custom");
    let stone = bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:stone")
        .unwrap()
        .id;
    let netherrack =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:netherrack")
            .unwrap()
            .id;
    let source_water = helpers::state_with("minecraft:water", &[("level", "0")]);
    let bubble_column = helpers::state_with("minecraft:bubble_column", &[("drag", "false")]);

    assert!(crate::block_behavior::spreading_snowy::is_snowy_setting(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        helpers::default_block_state("minecraft:snow"),
    ));

    assert_eq!(
        world.level_infiniburn_blocks_for_test(Java26BuiltinDimension::Overworld),
        HashSet::from([stone])
    );
    assert_eq!(
        world.level_infiniburn_blocks_for_test(Java26BuiltinDimension::Overworld),
        HashSet::from([stone])
    );

    world.set_level_dimension_type_for_test(Java26BuiltinDimension::Overworld, "test:missing");
    assert!(world
        .level_infiniburn_blocks_for_test(Java26BuiltinDimension::Overworld)
        .is_empty());
    world.set_level_dimension_type_for_test(Java26BuiltinDimension::Overworld, "test:absent");
    assert!(world
        .level_infiniburn_blocks_for_test(Java26BuiltinDimension::Overworld)
        .is_empty());
    world.set_level_dimension_type_for_test(Java26BuiltinDimension::Overworld, "test:custom");

    assert!(turtle_egg_placement_event(
        &mut world,
        BlockPos { x: 2, y: 65, z: 2 },
        "minecraft:sand",
    ));
    assert!(!turtle_egg_placement_event(
        &mut world,
        BlockPos { x: 3, y: 65, z: 2 },
        "minecraft:stone",
    ));
    assert!(world
        .level_registry_tags_for_test(Java26BuiltinDimension::Overworld)
        .contains(
            "minecraft:fluid",
            "minecraft:bubble_column_can_occupy",
            "minecraft:water",
        ));
    assert!(world
        .level_registry_tags_for_test(Java26BuiltinDimension::Overworld)
        .contains(
            "minecraft:block",
            "minecraft:enables_bubble_column_push_up",
            "minecraft:soul_sand",
        ));
    assert_eq!(bubble_column_probe(&mut world, 2), bubble_column);
    assert!(snowy_probe(&mut world, 2, "minecraft:snow"));
    assert!(!snowy_probe(&mut world, 3, "minecraft:stone"));

    write_dimension_gameplay_pack(&root, &["minecraft:netherrack"]);
    write_gameplay_tag_override(&root, "block", "sand", Some(&["minecraft:stone"]));
    write_gameplay_tag_override(&root, "block", "snow", Some(&["minecraft:stone"]));
    write_gameplay_tag_override(
        &root,
        "fluid",
        "bubble_column_can_occupy",
        Some(&["minecraft:lava"]),
    );
    let reloaded = bbb_server_datapack::startup::DatapackStartup::reload_with_data_packs(
        &options,
        &effective_data_packs,
        world.datapacks().unwrap(),
    )
    .unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Built(Arc::new(reloaded)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        world.level_infiniburn_blocks_for_test(Java26BuiltinDimension::Overworld),
        HashSet::from([netherrack])
    );
    assert!(world.datapacks().is_some());
    assert!(!turtle_egg_placement_event(
        &mut world,
        BlockPos { x: 4, y: 65, z: 2 },
        "minecraft:sand",
    ));
    assert!(turtle_egg_placement_event(
        &mut world,
        BlockPos { x: 5, y: 65, z: 2 },
        "minecraft:stone",
    ));
    assert_eq!(bubble_column_probe(&mut world, 3), source_water);
    assert!(!snowy_probe(&mut world, 4, "minecraft:snow"));
    assert!(snowy_probe(&mut world, 5, "minecraft:stone"));

    write_dimension_gameplay_pack(&root, &["minecraft:stone"]);
    write_gameplay_tag_override(&root, "block", "sand", Some(&["minecraft:sand"]));
    write_gameplay_tag_override(&root, "block", "snow", Some(&["minecraft:snow"]));
    write_gameplay_tag_override(
        &root,
        "fluid",
        "bubble_column_can_occupy",
        Some(&["minecraft:water"]),
    );
    let mut invalid = bbb_server_datapack::startup::DatapackStartup::reload_with_data_packs(
        &options,
        &effective_data_packs,
        world.datapacks().unwrap(),
    )
    .unwrap();
    invalid
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:dimension_type")
        .unwrap()
        .elements
        .swap(0, 1);
    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Built(Arc::new(invalid)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        world.level_infiniburn_blocks_for_test(Java26BuiltinDimension::Overworld),
        HashSet::from([netherrack])
    );
    assert!(!turtle_egg_placement_event(
        &mut world,
        BlockPos { x: 6, y: 65, z: 2 },
        "minecraft:sand",
    ));
    assert!(turtle_egg_placement_event(
        &mut world,
        BlockPos { x: 7, y: 65, z: 2 },
        "minecraft:stone",
    ));
    assert_eq!(bubble_column_probe(&mut world, 4), source_water);
    assert!(!snowy_probe(&mut world, 6, "minecraft:snow"));
    assert!(snowy_probe(&mut world, 7, "minecraft:stone"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn level_slice_has_skylight_comes_from_each_builtin_dimension_type() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    for (dimension, expected) in [
        (Java26BuiltinDimension::Nether, 0),
        (Java26BuiltinDimension::End, 15),
    ] {
        assert_eq!(
            world.level_light_facts_for_test(dimension, pos),
            (0, 0, 0, false)
        );
        assert_eq!(
            world.chunk_store_for_test(dimension).has_skylight(),
            expected == 15
        );
        world.ensure_entity_transfer_full_chunk_for_test(dimension, (0, 0));
        let actual = world.level_light_facts_for_test(dimension, pos);
        assert_eq!(actual, (0, expected, expected, expected == 15));
        assert_eq!(
            world.level_light_facts_for_test(dimension, BlockPos { x: 16, ..pos }),
            (0, 0, 0, false),
        );
    }
}

#[test]
fn a_live_nether_portal_sample_spawns_a_zombified_piglin_over_its_floor() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "portal-witness", 1);
    for command in ["difficulty hard", "gamerule random_tick_speed 0"] {
        sender
            .try_send(WorldCommand::Admin {
                envelope: admin_envelope(1, command, AdminScope::Command),
                request: AdminRequest::ExecuteCommand {
                    command: command.to_owned(),
                },
            })
            .unwrap();
        assert!(admin_outcome(&world.tick(receiver.drain())).result.is_ok());
    }
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 8.5,
            y: 300.0,
            z: 8.5,
        },
        None,
    );
    let pos = BlockPos { x: 8, y: 300, z: 8 };
    let floor = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    let portal = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_NETHER_PORTAL_ID,
    )
    .expect("built-in nether portal default state");
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        floor,
        block_state_id("minecraft:obsidian"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        portal,
        SET_BLOCK_UPDATE_ALL,
    ));
    // A spawn is `nextInt(2000) < 3` at hard, so 16 384 samples miss one with
    // odds of about e^-24.
    let entity_id = (0..16_384).find_map(|_| {
        world.run_live_random_tick_sample_for_test(
            Java26BuiltinDimension::Overworld,
            pos,
            portal,
        );
        world.stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter().find(|id| world.stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID))
    }).expect("a live portal sample eventually spawns its piglin");
    let piglin = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the spawned piglin remains in the level store");
    assert_eq!(
        piglin.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
    );
    assert_eq!(
        piglin.position(),
        Vec3d {
            x: 8.5,
            y: 300.0,
            z: 8.5,
        }
    );
    assert_eq!(piglin.portal_cooldown(), 300);
    // `nextInt(20) == 0` picks the spear over the sword.
    let weapon = piglin
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
        .and_then(|stack| stack.item_resource_id().map(str::to_owned));
    assert!(
        matches!(
            weapon.as_deref(),
            Some("minecraft:golden_spear" | "minecraft:golden_sword")
        ),
        "{weapon:?}"
    );
}

#[test]
fn portal_structure_finalize_keeps_jockey_relationship_equipment_and_cooldowns() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(datapack_config(), &candidate).unwrap();
    let dimension = Java26BuiltinDimension::Overworld;
    assert!(world.set_block(
        dimension,
        BlockPos { x: 4, y: 64, z: 4 },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    // A baby (1 in 20) that rolls a new chicken mount (1 in 20 of the rest)
    // is about 1 spawn in 420; 20 000 spawns all miss with odds of e^-48.
    let chicken = (0..20_000)
        .find_map(|_| {
            world
                .apply_block_mob_spawn_request_for_test(
                    dimension,
                    BlockMobSpawnRequest::PortalZombifiedPiglin {
                        position: Vec3d {
                            x: 4.5,
                            y: 65.0,
                            z: 4.5,
                        },
                        y_rot: 37.0,
                    },
                )
                .expect("the staged portal spawn finalizes");
            world
                .stored_entity_ids_for_test(dimension)
                .into_iter()
                .find(|id| {
                    world
                        .stored_entity_for_test(dimension, *id)
                        .is_some_and(|entity| {
                            entity.entity_type_id() == VANILLA_ENTITY_TYPE_CHICKEN_ID
                        })
                })
        })
        .expect("a finalized portal mob gets a chicken mount");
    let [root] = world.stored_passengers_for_test(dimension, chicken)[..] else {
        panic!("the chicken carries the piglin");
    };
    assert_eq!(
        world
            .stored_entity_for_test(dimension, root)
            .unwrap()
            .portal_cooldown(),
        300
    );
    assert_eq!(
        world
            .stored_entity_for_test(dimension, chicken)
            .unwrap()
            .portal_cooldown(),
        300
    );

    assert!(world
        .stored_entity_for_test(dimension, root)
        .and_then(|entity| entity.living())
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
        .is_some());
    let events = world.drain_entity_lifecycle_events_for_test(dimension);
    assert!(events.iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::PassengersChanged {
            vehicle_id,
            passenger_ids,
        } if *vehicle_id == chicken && passenger_ids == &[root]
    )));
}

fn detector_rail_state(powered: bool) -> i32 {
    helpers::state_with(
        "minecraft:detector_rail",
        &[
            ("shape", "north_south"),
            ("powered", if powered { "true" } else { "false" }),
            ("waterlogged", "false"),
        ],
    )
}

/// `DetectorRailBlock.entityInside`: a
/// minecart standing in an unpowered rail's cell presses it on the tick that
/// walks the entity against the blocks it occupies
/// (`Entity.checkInsideBlocks`). Nothing else could:
/// an unpowered rail has no scheduled tick to recheck at, and it
/// went down over an empty cell so `onPlace` left it released.
#[test]
fn a_minecart_presses_the_unpowered_detector_rail_it_stands_in() {
    let rail = BlockPos { x: 0, y: 64, z: 0 };
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let _ = world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { y: 63, ..rail },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        rail,
        detector_rail_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        live_block(&mut world, rail),
        Some(detector_rail_state(false))
    );

    world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
        Vec3d {
            x: f64::from(rail.x) + 0.5,
            y: f64::from(rail.y),
            z: f64::from(rail.z) + 0.5,
        },
        crate::entity::EntityData::Minecart(crate::entity::MinecartData::new()),
    );
    assert_eq!(
        live_block(&mut world, rail),
        Some(detector_rail_state(false))
    );

    world.tick(Vec::new());
    assert_eq!(
        live_block(&mut world, rail),
        Some(detector_rail_state(true))
    );
}

/// `BasePressurePlateBlock.entityInside`
/// on the production level. It is the only entry point that can raise a
/// released plate -- `tick` rechecks a pressed one -- and the walk
/// that reaches it is `Entity.checkInsideBlocks`,
/// which carries the live entity because the level's occupancy snapshot is
/// built outside the tick that moved it.
#[test]
fn an_armor_stand_standing_on_a_pressure_plate_presses_it() {
    let plate = BlockPos { x: 0, y: 64, z: 0 };
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let _ = world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { y: 63, ..plate },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    );
    let released = pressure_plate_state(false);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        plate,
        released,
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(live_block(&mut world, plate), Some(released));

    let (entity_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: f64::from(plate.x) + 0.5,
            y: f64::from(plate.y),
            z: f64::from(plate.z) + 0.5,
        },
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        ),
    );
    assert_eq!(live_block(&mut world, plate), Some(released));

    world.tick(Vec::new());
    assert_eq!(
        live_block(&mut world, plate),
        Some(pressure_plate_state(true))
    );
    assert!(world
        .game_events(Java26BuiltinDimension::Overworld)
        .0
        .iter()
        .any(
            |event| event.game_event == crate::level_event::VanillaGameEvent::BLOCK_ACTIVATE
                && event.pos == plate
                && event.source_entity_id == Some(entity_id)
        ));
}

fn pressure_plate_state(powered: bool) -> i32 {
    helpers::state_with(
        "minecraft:oak_pressure_plate",
        &[("powered", if powered { "true" } else { "false" })],
    )
}

/// Neighbour shape updates read the loaded block tags: a pack that adds stone
/// to `#minecraft:supports_vegetation` keeps a poppy on stone alive
/// and one that makes a flower pot a wooden
/// fence joins an oak fence to it.
#[test]
fn datapack_block_tags_decide_survival_and_fence_connections() {
    let dimension = Java26BuiltinDimension::Overworld;
    let poppy = BlockPos { x: 2, y: 65, z: 2 };
    let fence = BlockPos { x: 6, y: 65, z: 2 };
    let pot = BlockPos { x: 7, ..fence };
    for (tags, joined) in [
        (&[][..], false),
        (
            &[
                (
                    "minecraft:supports_vegetation",
                    &["minecraft:stone"][..],
                    false,
                ),
                ("minecraft:fences", &["minecraft:flower_pot"][..], false),
                (
                    "minecraft:wooden_fences",
                    &["minecraft:flower_pot"][..],
                    false,
                ),
            ][..],
            true,
        ),
    ] {
        let candidate = restore_candidate(10, None, Vec::new());
        let mut world =
            ServerWorld::restore(super::datapack_config_with_block_tags(tags), &candidate).unwrap();
        for (pos, block) in [
            (BlockPos { y: 64, ..poppy }, "minecraft:stone"),
            (poppy, "minecraft:poppy"),
            (BlockPos { y: 64, ..fence }, "minecraft:stone"),
            (BlockPos { y: 64, ..pot }, "minecraft:stone"),
            (fence, "minecraft:oak_fence"),
            (pot, "minecraft:flower_pot"),
            (BlockPos { x: 3, ..poppy }, "minecraft:stone"),
        ] {
            assert!(world.set_block(
                dimension,
                pos,
                helpers::default_block_state(block),
                SET_BLOCK_UPDATE_ALL
            ));
        }
        let survived =
            live_block(&mut world, poppy) == Some(helpers::default_block_state("minecraft:poppy"));
        let east = live_block(&mut world, fence)
            .and_then(|state| crate::block_behavior::string_property(state, "east"));
        assert_eq!(survived, joined, "{tags:?}");
        assert_eq!(
            east.as_deref(),
            Some(if joined { "true" } else { "false" }),
            "{tags:?}"
        );
    }
}
