use super::water::{pool, tick};
use super::*;
use bbb_protocol::entity_synched_data::VANILLA_GUARDIAN_MOVING_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, VANILLA_ENTITY_TYPE_GUARDIAN_ID,
};
use bbb_protocol::packets::EntityDataValueKind;

fn moving(store: &EntityStore, id: i32) -> bool {
    let values = crate::entity::tracking::synched_values(store.get(id).unwrap());
    let row = values
        .iter()
        .find(|row| row.value.data_id == VANILLA_GUARDIAN_MOVING_DATA_ID)
        .unwrap();
    let EntityDataValueKind::Boolean(moving) = row.value.value else {
        panic!("the guardian moving flag is boolean");
    };
    moving
}

#[test]
fn guardian_navigation_moves_both_sizes_over_a_wall_then_stops_and_sinks() {
    for kind in [
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID,
    ] {
        let mut native = pool();
        for y in 64..=66 {
            for z in -3..=4 {
                native.set(BlockPos { x: 5, y, z }, state("minecraft:stone"));
            }
        }
        let mut store = EntityStore::new(47, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                kind,
                Vec3d {
                    x: 3.5,
                    y: 65.0,
                    z: 2.5,
                },
            )
            .unwrap();
        tick(&mut native, &mut store);
        store.mob_ai_mut(id).unwrap().goal_selector = Default::default();
        let mob = MobPathingSnapshot::from_entity(
            store.get(id).unwrap(),
            false,
            false,
            None,
            PathfindingMalus::for_entity_type(kind),
        )
        .unwrap();
        assert!(
            native.with_level(|level| store.mob_ai_mut(id).unwrap().navigate_to(
                level,
                mob,
                BlockPos { x: 8, y: 68, z: 1 },
                0,
                1.0
            ))
        );
        let mut crossed = false;
        let mut moved = false;
        for _ in 0..300 {
            tick(&mut native, &mut store);
            moved |= moving(&store, id);
            let pos = store.get(id).unwrap().position();
            if pos.x > 6.2 && pos.y > 66.9 {
                crossed = true;
                break;
            }
        }
        assert!(crossed, "{kind}: {:?}", store.get(id).unwrap().position());
        assert!(moved);
        store.mob_ai_mut(id).unwrap().stop_navigation();
        let before = Vec3d {
            y: 68.5,
            ..store.get(id).unwrap().position()
        };
        let entity = store.get_mut(id).unwrap();
        entity.set_pos(before);
        entity.set_delta_movement(Vec3d::default());
        for _ in 0..16 {
            tick(&mut native, &mut store);
        }
        assert!(!moving(&store, id));
        assert!(store.get(id).unwrap().position().y < before.y - 0.1);
    }
}

#[test]
fn persistent_guardians_swim_autonomously_on_real_ticks() {
    for kind in [
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID,
    ] {
        let mut native = pool();
        let mut store = EntityStore::new(53, EntityIdCounter::new());
        let start = Vec3d {
            x: 3.5,
            y: 66.0,
            z: 2.5,
        };
        let id = store.spawn_general_living(kind, start).unwrap();
        let crate::entity::EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data
        else {
            unreachable!();
        };
        data.mob_retained_mut()
            .unwrap()
            .set_persistence_required(true);
        let mut swam = false;
        for _ in 0..4000 {
            tick(&mut native, &mut store);
            let pos = store.get(id).unwrap().position();
            if (pos.x - start.x).hypot(pos.z - start.z) > 1.0 {
                swam = true;
                break;
            }
        }
        assert!(swam, "{kind}: {:?}", store.get(id).unwrap().position());
    }
}

#[test]
fn guardians_return_to_an_existing_home_without_replacing_it() {
    for kind in [
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID,
    ] {
        let mut native = pool();
        let mut store = EntityStore::new(73, EntityIdCounter::new());
        let home = BlockPos { x: 3, y: 66, z: 2 };
        let id = store
            .spawn_general_living(
                kind,
                Vec3d {
                    x: 8.5,
                    y: 66.0,
                    z: 2.5,
                },
            )
            .unwrap();
        store.get_mut(id).unwrap().set_mob_home(home, 3);
        let mut returned = false;
        for _ in 0..400 {
            tick(&mut native, &mut store);
            let pos = store.get(id).unwrap().block_position();
            let dx = pos.x - home.x;
            let dy = pos.y - home.y;
            let dz = pos.z - home.z;
            if dx * dx + dy * dy + dz * dz < 9 {
                returned = true;
                break;
            }
        }
        assert!(returned, "{kind}: {:?}", store.get(id).unwrap().position());
        assert_eq!(store.get(id).unwrap().mob_home(), Some((home, 3)));
    }
}

#[test]
fn only_elder_guardians_establish_their_home_on_real_ai_ticks() {
    let mut native = pool();
    let mut store = EntityStore::new(79, EntityIdCounter::new());
    let start = Vec3d {
        x: 3.5,
        y: 66.0,
        z: 2.5,
    };
    let guardian = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_GUARDIAN_ID, start)
        .unwrap();
    let elder = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, start)
        .unwrap();
    tick(&mut native, &mut store);
    assert_eq!(store.get(guardian).unwrap().mob_home(), None);
    assert_eq!(
        store.get(elder).unwrap().mob_home(),
        Some((containing_block(start), 16))
    );
}

#[test]
fn guardians_refill_air_in_water_and_flop_on_dry_ground() {
    for kind in [
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID,
    ] {
        let mut wet = pool();
        let mut store = EntityStore::new(83, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                kind,
                Vec3d {
                    x: 3.5,
                    y: 66.0,
                    z: 2.5,
                },
            )
            .unwrap();
        store.get_mut(id).unwrap().set_air_supply(12);
        tick(&mut wet, &mut store);
        assert_eq!(store.get(id).unwrap().air_supply(), 300);

        let mut dry = NativeLevel::new();
        for x in 2..=4 {
            for z in 1..=3 {
                dry.set(BlockPos { x, y: 64, z }, state("minecraft:stone"));
            }
        }
        let mut grounded = EntityStore::new(89, EntityIdCounter::new());
        let id = grounded
            .spawn_general_living(
                kind,
                Vec3d {
                    x: 3.5,
                    y: 65.0,
                    z: 2.5,
                },
            )
            .unwrap();
        grounded.get_mut(id).unwrap().on_ground = true;
        tick(&mut dry, &mut grounded);
        let entity = grounded.get(id).unwrap();
        assert!(
            entity.position().y > 65.2,
            "{kind}: {:?}",
            entity.position()
        );
        assert!(!entity.on_ground());
    }
}
