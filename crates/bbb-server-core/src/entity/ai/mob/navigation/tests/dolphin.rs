use super::water::tick;
use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID;

#[test]
fn a_dolphin_jumps_above_the_water_and_returns_to_its_pool() {
    let mut native = NativeLevel::new();
    let chunks = (0..=3)
        .flat_map(|x| (0..=3).map(move |z| bbb_protocol::packets::ChunkPos { x, z }))
        .collect::<Vec<_>>();
    native.world.set_chunks_forced(
        crate::entity::tests::native_level::DIMENSION.into(),
        true,
        &chunks,
    );
    for x in 0..=63 {
        for z in 0..=63 {
            native.set(BlockPos { x, y: 68, z }, state("minecraft:stone"));
            if x == 0 || x == 63 || z == 0 || z == 63 {
                for y in 69..=72 {
                    native.set(BlockPos { x, y, z }, state("minecraft:stone"));
                }
            } else {
                native.set(BlockPos { x, y: 69, z }, state("minecraft:water"));
            }
        }
    }
    let mut store = EntityStore::new(107, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d {
                x: 31.5,
                y: 69.0,
                z: 31.5,
            },
        )
        .unwrap();
    let mut jumped = false;
    let mut returned = false;
    let mut tilted_up = false;
    let mut peak = 69.0_f64;
    for _ in 0..1200 {
        tick(&mut native, &mut store);
        let body = store.get(id).unwrap();
        peak = peak.max(body.position().y);
        jumped |= body.position().y > 70.5;
        if body.position().y > 70.5 {
            tilted_up |= body.x_rot() < -10.0;
        }
        if jumped && body.is_in_water() {
            returned = true;
            break;
        }
    }
    assert!(
        jumped && returned && tilted_up,
        "dolphin jumps, tilts and returns; peak={peak}, tilted_up={tilted_up}, position={:?}",
        store.get(id).unwrap().position()
    );
}

#[test]
fn a_low_air_dolphin_swims_to_the_surface_and_breathes_before_drowning() {
    let mut native = NativeLevel::new();
    for x in 0..=15 {
        for y in 64..=69 {
            for z in 0..=15 {
                native.set(BlockPos { x, y, z }, state("minecraft:water"));
            }
        }
    }
    let mut store = EntityStore::new(97, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d {
                x: 8.5,
                y: 66.0,
                z: 8.5,
            },
        )
        .unwrap();
    store.get_mut(id).unwrap().set_air_supply(130);
    let before = store.get(id).unwrap().living().unwrap().health();
    let mut breathed = false;
    for _ in 0..200 {
        tick(&mut native, &mut store);
        let entity = store.get(id).unwrap();
        if entity.air_supply() == 4800 {
            breathed = true;
            break;
        }
    }
    let entity = store.get(id).unwrap();
    assert!(
        breathed,
        "position {:?}, air {}",
        entity.position(),
        entity.air_supply()
    );
    assert_eq!(entity.living().unwrap().health(), before);
    assert!(entity.position().y > 69.0);
}

#[test]
fn a_stranded_dolphin_moves_across_dry_ground_to_nearby_water() {
    let mut native = NativeLevel::new();
    for x in 2..=11 {
        for z in 1..=10 {
            native.set(BlockPos { x, y: 64, z }, state("minecraft:stone"));
            if x >= 7 {
                for y in 65..=68 {
                    native.set(BlockPos { x, y, z }, state("minecraft:water"));
                }
            }
        }
    }
    let mut store = EntityStore::new(101, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d {
                x: 5.5,
                y: 65.0,
                z: 5.5,
            },
        )
        .unwrap();
    store.get_mut(id).unwrap().on_ground = true;
    let mut returned = false;
    for _ in 0..400 {
        tick(&mut native, &mut store);
        if store.get(id).unwrap().is_in_water() {
            returned = true;
            break;
        }
    }
    assert!(returned, "position {:?}", store.get(id).unwrap().position());
}

#[test]
fn dolphin_look_turns_are_retained_and_published_after_a_real_tick() {
    use crate::connection::ClientboundPlayPacket;
    use crate::entity::ai::mob::LookTarget;
    use crate::entity::tracking::ServerEntity;

    let mut native = super::water::pool();
    let mut store = EntityStore::new(103, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            Vec3d {
                x: 3.5,
                y: 66.0,
                z: 0.5,
            },
        )
        .unwrap();
    tick(&mut native, &mut store);
    let entity = store.get_mut(id).unwrap();
    entity.set_rot(0.0, 30.0);
    entity.set_delta_movement(Vec3d::default());
    let living = entity.living_mut().unwrap();
    living.y_head_rot = 0.0;
    living.y_body_rot = 0.0;
    let target = LookTarget {
        x: entity.position().x,
        y: entity.position().y + f64::from(entity.eye_height()),
        z: entity.position().z + 5.0,
    };
    let mut tracker = ServerEntity::new(entity, 1, true);
    let runtime = store.mob_ai_mut(id).unwrap();
    runtime.goal_selector = Default::default();
    runtime.target_selector = Default::default();
    runtime.stop_navigation();
    runtime.look_control.as_mut().unwrap().set_look_at(target);

    tick(&mut native, &mut store);
    tick(&mut native, &mut store);

    let entity = store.get(id).unwrap();
    assert_eq!(entity.x_rot(), 28.0);
    assert_eq!(entity.y_head_rot(), 20.0);
    assert!(entity.living().unwrap().y_body_rot > 0.0);
    let packets = tracker.send_changes(store.get_mut(id).unwrap());
    assert!(packets.iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::RotateHead(head) if head.id == id && head.y_head_rot == 20.0
    )));
}
