use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::entity::{tick_one_entity, EntityIdCounter, EntityStore, TargetLevelFacts};

/// How far a zombie walks toward an occupied village bed in 160 ticks.
fn walked_toward_bed(bright_outside: bool) -> f64 {
    let mut native = NativeLevel::new();
    native
        .world
        .load_entity_chunk_for_test(DIMENSION, (0, 0))
        .unwrap();
    let properties = [("facing", "north"), ("occupied", "false"), ("part", "head")]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let bed =
        vanilla_block_state_id_by_name_and_properties("minecraft:red_bed", &properties).unwrap();
    let home = BlockPos { x: 14, y: 64, z: 8 };
    native.set(home, bed);
    native.with_level(|level| {
        assert!(level.poi_mut().acquire_ticket([home.x, home.y, home.z]));
    });
    let mut store = EntityStore::new(5, EntityIdCounter::new());
    store.set_target_level_facts(TargetLevelFacts {
        bright_outside: Some(bright_outside),
        ..TargetLevelFacts::default()
    });
    let start = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 8.5,
    };
    let zombie = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, start)
        .unwrap();
    for _ in 0..160 {
        native.with_level(|level| {
            tick_one_entity(&mut store, level, zombie);
        });
    }
    store.get(zombie).unwrap().position().x - start.x
}

#[test]
fn zombies_walk_into_villages_only_at_night() {
    let night = walked_toward_bed(false);
    assert!(night > 7.0, "{night}");
    let day = walked_toward_bed(true);
    assert!(day < 7.0, "{day}");
}

/// `MoveThroughVillageGoal.canUse` leaves the zombie's navigation opening
/// doors, so after a village walk
/// even a zombie that cannot break doors paths up to a closed wooden door.
#[test]
fn after_a_village_walk_a_zombie_paths_up_to_a_closed_door() {
    use crate::entity::tests::native_level::state;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID;

    let mut native = NativeLevel::new();
    native
        .world
        .load_entity_chunk_for_test(DIMENSION, (0, 0))
        .unwrap();
    let stone = state("minecraft:stone");
    for y in 64..=66 {
        for x in 6..=14 {
            for z in -4..=20 {
                if x == 6 || x == 14 || z == -4 || z == 20 {
                    native.set(BlockPos { x, y, z }, stone);
                }
            }
        }
    }
    for (y, half) in [(64, "lower"), (65, "upper")] {
        let properties = [
            ("facing", "east"),
            ("half", half),
            ("hinge", "left"),
            ("open", "false"),
            ("powered", "false"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
        let door = vanilla_block_state_id_by_name_and_properties("minecraft:oak_door", &properties)
            .unwrap();
        native.set(BlockPos { x: 6, y, z: 2 }, door);
    }
    let properties = [("facing", "north"), ("occupied", "false"), ("part", "head")]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let bed =
        vanilla_block_state_id_by_name_and_properties("minecraft:red_bed", &properties).unwrap();
    let home = BlockPos { x: 12, y: 64, z: 8 };
    native.set(home, bed);
    native.with_level(|level| {
        assert!(level.poi_mut().acquire_ticket([home.x, home.y, home.z]));
    });
    let mut store = EntityStore::new(5, EntityIdCounter::new());
    store.set_target_level_facts(TargetLevelFacts {
        bright_outside: Some(false),
        ..TargetLevelFacts::default()
    });
    let zombie = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 1.5,
                y: 64.0,
                z: 8.5,
            },
        )
        .unwrap();
    let mut tick = |store: &mut EntityStore, ticks: usize| {
        for _ in 0..ticks {
            native.with_level(|level| {
                tick_one_entity(store, level, zombie);
            });
        }
    };
    tick(&mut store, 120);
    let villager = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            Vec3d {
                x: 10.5,
                y: 64.0,
                z: 12.5,
            },
        )
        .unwrap();
    let crate::entity::EntityData::GeneralLiving(data) = &mut store.get_mut(villager).unwrap().data
    else {
        unreachable!("a villager is living")
    };
    data.set_no_ai(true);
    tick(&mut store, 300);
    let at = store.get(zombie).unwrap().position();
    assert!((at.z - 2.5).abs() < 2.0 && at.x > 3.5, "{at:?}");
}
