use super::*;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::{EntityData, EntityIdCounter};
use crate::world::WorldEvent;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};

#[test]
fn a_level_that_is_not_thundering_never_strikes() {
    let mut native = NativeLevel::new();
    assert!(native
        .world
        .tick_thunder_for_test(DIMENSION, (0, 0))
        .is_empty());
}

#[test]
fn the_heightmap_supplies_the_column_floor_when_no_living_entity_is_nearby() {
    let mut native = NativeLevel::new();
    native
        .world
        .block_state(DIMENSION, BlockPos { x: 8, y: 64, z: 8 })
        .unwrap();
    let store = EntityStore::new(1, EntityIdCounter::new());
    let target = native.with_level(|level| {
        find_lightning_target_around(&store, level, BlockPos { x: 8, y: 0, z: 8 })
    });
    assert_eq!(target, Some(BlockPos { x: 8, y: 64, z: 8 }));
}

#[test]
fn lightning_targets_use_living_health_range_and_actual_sky_access() {
    for (x, y, alive, expected) in [
        (10.5, 65.0, true, (10, 65, 9)),
        (10.5, 65.0, false, (8, 64, 8)),
        (13.5, 65.0, true, (8, 64, 8)),
        (8.5, 200.0, true, (8, 200, 9)),
    ] {
        let mut native = NativeLevel::new();
        native
            .world
            .block_state(DIMENSION, BlockPos { x: 8, y: 64, z: 8 })
            .unwrap();
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        let id = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d { x, y, z: 9.5 })
            .unwrap();
        if !alive {
            store
                .get_mut(id)
                .unwrap()
                .living_mut()
                .unwrap()
                .set_health(0.0);
        }
        let target = native.with_level(|level| {
            find_lightning_target_around(&store, level, BlockPos { x: 8, y: 0, z: 8 })
        });
        assert_eq!(
            target,
            Some(BlockPos {
                x: expected.0,
                y: expected.1,
                z: expected.2
            })
        );
    }
}

/// `ServerLevel.findLightningRod` (`ServerLevel.java:596-606`): a rod that
/// tops its column within 128 blocks takes the strike over a living target.
#[test]
fn a_rod_topping_its_column_draws_the_strike_over_living_targets() {
    let mut native = NativeLevel::new();
    for x in [8, 40] {
        native
            .world
            .block_state(DIMENSION, BlockPos { x, y: 64, z: 8 })
            .unwrap();
    }
    native.set(
        BlockPos { x: 40, y: 64, z: 8 },
        state("minecraft:lightning_rod"),
    );
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 10.5,
                y: 65.0,
                z: 9.5,
            },
        )
        .unwrap();
    let target = native.with_level(|level| {
        find_lightning_target_around(&store, level, BlockPos { x: 8, y: 0, z: 8 })
    });
    assert_eq!(target, Some(BlockPos { x: 40, y: 65, z: 8 }));
}

fn storm(spawn_mobs: bool, rod: bool, difficulty: &str) -> NativeLevel {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 1);
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        0,
        "gamerule minecraft:spawn_mobs false",
    );
    crate::tests::console_command(&sender, &receiver, &mut native.world, 1, difficulty);
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        2,
        "weather thunder 40000t",
    );
    for _ in 0..200 {
        if native.world.level_weather(DIMENSION).is_thundering() {
            break;
        }
        native.world.tick(receiver.drain());
    }
    assert!(native.world.level_weather(DIMENSION).is_thundering());
    native.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 20.5,
            y: 64.0,
            z: 8.5,
        },
    );
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        3,
        if spawn_mobs {
            "gamerule minecraft:spawn_mobs true"
        } else {
            "gamerule minecraft:spawn_mobs false"
        },
    );
    for x in 0..4 {
        for z in 0..4 {
            assert!(native.world.set_noise_biome_for_test(
                DIMENSION,
                (x, 16, z),
                "minecraft:plains"
            ));
        }
    }
    assert!(native
        .world
        .is_raining_at(DIMENSION, BlockPos { x: 8, y: 64, z: 8 }));
    if rod {
        for x in 0..16 {
            for z in 0..16 {
                native.set(BlockPos { x, y: 63, z }, state("minecraft:lightning_rod"));
            }
        }
    }
    native
}

/// A seed that stages a strike: the `nextInt(100000)` roll wins, the column
/// draw is free, and the trap's `nextDouble()` is under hard's 2.25%.
fn striking_trap_seed() -> i64 {
    (0..)
        .find(|&seed| {
            let random = crate::random::GameRandom::with_seed(seed);
            random.next_int(100_000) == 0 && {
                random.next_u64();
                random.next_double() < 0.005
            }
        })
        .unwrap()
}

#[test]
fn native_thunder_traps_publish_horses_and_visual_bolts_only_when_allowed() {
    for (spawn_mobs, rod, difficulty, trap) in [
        (true, false, "difficulty hard", true),
        (false, false, "difficulty hard", false),
        (true, true, "difficulty hard", false),
        (true, false, "difficulty peaceful", false),
    ] {
        let mut native = storm(spawn_mobs, rod, difficulty);
        native.world.random.reseed(striking_trap_seed());
        let mut events = native.world.tick_thunder_for_test(DIMENSION, (0, 0));
        events.extend(native.world.tick(Vec::new()).events);
        let bolt_id = events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::AddEntity(add),
                    ..
                }
                | WorldEvent::Broadcast {
                    packet: ClientboundPlayPacket::AddEntity(add),
                    ..
                } if add.entity_type_id == VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID => Some(add.id),
                _ => None,
            })
            .unwrap_or_else(|| panic!("the real storm publishes a bolt: spawn_mobs={spawn_mobs}, rod={rod}, events={}", events.len()));
        let bolt = native
            .world
            .stored_entity_for_test(DIMENSION, bolt_id)
            .unwrap();
        let bolt_position = bolt.position();
        assert_eq!(bolt_position.y, 64.0);
        let EntityData::LightningBolt(data) = bolt.data() else {
            unreachable!()
        };
        assert_eq!(data.visual_only(), trap);
        let horses: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::AddEntity(add),
                    ..
                }
                | WorldEvent::Broadcast {
                    packet: ClientboundPlayPacket::AddEntity(add),
                    ..
                } if add.entity_type_id == VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => Some(add.id),
                _ => None,
            })
            .collect();
        assert_eq!(!horses.is_empty(), trap);
        for id in horses {
            let horse = native.world.stored_entity_for_test(DIMENSION, id).unwrap();
            assert_eq!(
                horse.position(),
                Vec3d {
                    x: bolt_position.x.floor(),
                    y: bolt_position.y,
                    z: bolt_position.z.floor()
                }
            );
            assert!(crate::entity::skeleton_horse::is_trap(horse));
        }
    }
}
