use super::*;
use bbb_protocol::packets::{RotateHead, SetPassengers};

fn cow_pose(world: &WorldStore) -> EntityModelInstance {
    entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 70)
        .unwrap()
}

#[test]
fn mob_body_pose_retains_spawn_yaw_then_follows_head_and_movement() {
    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(99));
    insert_loaded_entity_chunk(&mut world, ChunkPos { x: 0, z: -1 });
    world.apply_add_entity(protocol_add_entity_with_rotation(
        70,
        VANILLA_ENTITY_TYPE_COW_ID,
        [1.0, 64.0, -2.0],
        -180.0,
        -20.0,
        0.0,
    ));
    world.advance_entity_client_animations(25);
    let settled = cow_pose(&world);
    assert_eq!(settled.render_state.body_rot, 0.0);
    assert_eq!(settled.render_state.head_yaw, 0.0);
    assert_eq!(settled.render_state.head_pitch, -20.0);
    assert_eq!(world.probe_entity_transform(70).unwrap().y_rot, -180.0);

    assert!(world.apply_rotate_head(RotateHead {
        id: 70,
        y_head_rot: 90.0
    }));
    world.advance_entity_client_animations(3);
    let turning = cow_pose(&world).render_state;
    assert_eq!(turning.body_rot + turning.head_yaw, 90.0);
    assert!(turning.head_yaw > 0.0 && turning.head_yaw <= 75.0);
    world.advance_entity_client_animations(25);
    let facing = cow_pose(&world).render_state;
    assert_eq!(facing.body_rot, 90.0);
    assert_eq!(facing.head_yaw, 0.0);

    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 70,
        position: Vec3d {
            x: 5.0,
            y: 64.0,
            z: -2.0
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0
        },
        y_rot: -90.0,
        x_rot: -20.0,
        on_ground: true,
    }));
    assert!(world.apply_rotate_head(RotateHead {
        id: 70,
        y_head_rot: -90.0
    }));
    world.advance_entity_client_animations(4);
    let moving = cow_pose(&world);
    assert_eq!(moving.position, [5.0, 64.0, -2.0]);
    assert_eq!(moving.render_state.body_rot, -90.0);
    assert_eq!(moving.render_state.head_yaw, 0.0);
    assert_eq!(moving.render_state.head_pitch, -20.0);
}

#[test]
fn stationary_mob_body_does_not_turn_while_carrying_a_mob() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        70,
        VANILLA_ENTITY_TYPE_COW_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        71,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 70,
        passenger_ids: vec![71]
    }));
    assert!(world.apply_rotate_head(RotateHead {
        id: 70,
        y_head_rot: 90.0
    }));
    world.advance_entity_client_animations(30);
    let carrying = cow_pose(&world).render_state;
    assert_eq!(carrying.body_rot, 0.0);
    assert_eq!(carrying.head_yaw, 90.0);
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 70,
        passenger_ids: vec![]
    }));
    world.advance_entity_client_animations(30);
    let empty = cow_pose(&world).render_state;
    assert_eq!(empty.body_rot, 90.0);
    assert_eq!(empty.head_yaw, 0.0);
}
