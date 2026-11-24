use super::*;

use bbb_protocol::entity_anchors::EntityAnchor;
use bbb_protocol::packets::movement::PlayerPositionState;
use bbb_protocol::packets::{
    PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_DELTA_Z, PLAYER_RELATIVE_X,
    PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y, PLAYER_RELATIVE_Y_ROT, PLAYER_RELATIVE_Z,
};

fn destination(position: Vec3d, y_rot: f32, x_rot: f32) -> PlayerPositionState {
    PlayerPositionState {
        position,
        delta_movement: Vec3d::default(),
        y_rot,
        x_rot,
    }
}

/// `TeleportCommand.performTeleport` with typed coordinates: every relative bit
/// is clear, so `calculateAbsolute` takes the destination as it stands.
#[test]
fn an_absolute_teleport_moves_the_stand_and_refiles_it() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = crate::entity::spawn_armor_stand(
        &mut store,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 0.5,
        },
    )
    .expect("a fresh UUID");
    store
        .get_mut(id)
        .expect("the stand is live")
        .set_delta_movement(Vec3d {
            x: 0.25,
            y: 0.5,
            z: 0.75,
        });
    store.drain_lifecycle_events();

    let target = Vec3d {
        x: 33.5,
        y: 65.0,
        z: 0.5,
    };
    assert!(store.teleport_stored_entity(id, destination(target, 90.0, 45.0), 0));

    let stand = store.get(id).expect("the stand is still live");
    assert_eq!(stand.position(), target);
    assert_eq!(stand.old_position(), target);
    assert_eq!((stand.y_rot(), stand.x_rot()), (90.0, 45.0));
    // `ArmorStand.setYHeadRot` (`ArmorStand.java:439-442`) through
    // `teleportSetPosition`'s `setYHeadRot(yRot)` (`Entity.java:3083`).
    assert_eq!(stand.y_head_rot(), 90.0);
    // The transition's own delta movement is `Vec3.ZERO` (`Entity.java:3256`)
    // and no delta bit is relative, so the motion it had is dropped.
    assert_eq!(stand.delta_movement(), Vec3d::default());
    assert!(store
        .ids_in_section(crate::entity::SectionKey::new(2, 4, 0))
        .contains(&id));
    assert!(!store
        .ids_in_section(crate::entity::SectionKey::new(0, 4, 0))
        .contains(&id));
}

/// `/teleport ~ ~1 ~` : `Relative.direction` marks the delta axes and
/// `Relative.position` the position ones (`Relative.java:51-83`), so the
/// destination is an offset and the motion survives.
#[test]
fn a_relative_teleport_offsets_the_position_and_keeps_the_motion() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = crate::entity::spawn_armor_stand(
        &mut store,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 0.5,
        },
    )
    .expect("a fresh UUID");
    let motion = Vec3d {
        x: 0.25,
        y: 0.5,
        z: 0.75,
    };
    store
        .get_mut(id)
        .expect("the stand is live")
        .set_delta_movement(motion);
    store
        .get_mut(id)
        .expect("the stand is live")
        .set_rot(30.0, 0.0);

    let relatives = PLAYER_RELATIVE_X
        | PLAYER_RELATIVE_Y
        | PLAYER_RELATIVE_Z
        | PLAYER_RELATIVE_DELTA_X
        | PLAYER_RELATIVE_DELTA_Y
        | PLAYER_RELATIVE_DELTA_Z
        | PLAYER_RELATIVE_Y_ROT
        | PLAYER_RELATIVE_X_ROT;
    assert!(store.teleport_stored_entity(
        id,
        destination(
            Vec3d {
                x: 0.0,
                y: 1.0,
                z: 0.0
            },
            0.0,
            0.0
        ),
        relatives,
    ));

    let stand = store.get(id).expect("the stand is still live");
    assert_eq!(
        stand.position(),
        Vec3d {
            x: 1.5,
            y: 65.0,
            z: 0.5
        }
    );
    assert_eq!(stand.delta_movement(), motion);
    assert_eq!((stand.y_rot(), stand.x_rot()), (30.0, 0.0));
}

/// `Entity.lookAt` (`Entity.java:3570-3581`) from the source anchor a chat
/// command carries, which is `FEET`.
#[test]
fn look_at_turns_the_stand_towards_the_point() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = crate::entity::spawn_armor_stand(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    )
    .expect("a fresh UUID");
    let stand = store.get_mut(id).expect("the stand is live");
    stand.look_at(
        EntityAnchor::Feet,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 10.5,
        },
    );
    // Straight along +z is yaw zero, and a target at the same height is level.
    assert_eq!(stand.y_rot(), 0.0);
    assert_eq!(stand.x_rot(), 0.0);
    assert_eq!(stand.y_head_rot(), 0.0);
}

/// `BlockAttachedEntity.setPos` (`BlockAttachedEntity.java:141-145`) is what
/// `reapplyPosition` reaches for a hanging entity: the attachment block takes
/// the commanded position and the box is recalculated from it, which is what
/// leaves the frame somewhere its next survival check can refuse
/// (`BlockAttachedEntity.java:39-50`).
#[test]
fn a_block_attached_entity_takes_the_teleport_on_its_attachment() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let attached = BlockPos { x: 0, y: 64, z: 0 };
    let direction = bbb_protocol::packets::Direction::North;
    let frame = crate::entity::hanging::spawn_item_frame(&mut store, attached, direction, false)
        .expect("a fresh UUID");
    store
        .get_mut(frame)
        .expect("the frame is live")
        .set_needs_sync(false);

    assert!(store.teleport_stored_entity(
        frame,
        destination(
            Vec3d {
                x: 8.5,
                y: 65.0,
                z: 8.5,
            },
            0.0,
            0.0,
        ),
        0,
    ));

    let moved = BlockPos { x: 8, y: 65, z: 8 };
    let frame = store.get(frame).expect("the frame is live");
    let crate::entity::EntityData::ItemFrame(data) = frame.data() else {
        panic!("the spawned entity is an item frame")
    };
    assert_eq!(data.pose().attached(), moved);
    // `recalculateBoundingBox` puts the entity at the centre of the box the
    // attachment names, not at the commanded position.
    let expected = crate::entity::hanging::calculated_box(frame.data());
    assert_eq!(frame.position(), expected.center());
    assert_eq!(frame.bounding_box(), expected);
    assert!(frame.needs_sync());
}

/// `Entity.teleport` (`Entity.java:2971-3000`): a teleported passenger stops
/// riding, and a teleported vehicle carries its passengers at their offsets.
#[test]
fn a_teleport_dismounts_a_passenger_and_carries_a_vehicles_riders() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let at = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let pig = store
        .spawn_general_living(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID, at)
        .unwrap();
    let rider = crate::entity::spawn_armor_stand(&mut store, at).unwrap();
    assert!(store.link_existing_passenger(pig, rider));
    let offset = store.get(rider).unwrap().position().y - at.y;

    let target = Vec3d {
        x: 20.5,
        y: 70.0,
        z: 0.5,
    };
    assert!(store.teleport_stored_entity(pig, destination(target, 0.0, 0.0), 0));
    assert_eq!(store.vehicle(rider), Some(pig));
    assert_eq!(store.get(pig).unwrap().position(), target);
    let carried = store.get(rider).unwrap().position();
    assert_eq!(
        (carried.x, carried.y, carried.z),
        (20.5, 70.0 + offset, 0.5)
    );

    assert!(store.teleport_stored_entity(rider, destination(at, 0.0, 0.0), 0));
    assert_eq!(store.vehicle(rider), None);
    assert_eq!(store.get(rider).unwrap().position(), at);
}
