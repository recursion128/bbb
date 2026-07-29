use super::*;
use crate::LocalPlayerPoseState;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};

fn entity_pose(store: &WorldStore, entity_id: i32) -> i32 {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors");
    store
        .probe_entity(entity_id)
        .expect("entity state")
        .data_values
        .iter()
        .find(|value| value.data_id == accessors.pose_data_id)
        .map_or(accessors.pose_default, |value| match value.value {
            EntityDataValueKind::Pose(pose) => pose,
            ref value => panic!("unexpected pose metadata value {value:?}"),
        })
}

fn entity_pose_data(pose_id: i32) -> ProtocolEntityDataValue {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors");
    protocol_pose_data(accessors.pose_data_id, pose_id)
}

fn sleeping_pos_data(
    entity_type_id: i32,
    pos: Option<ProtocolBlockPos>,
) -> ProtocolEntityDataValue {
    let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
        .expect("generated inherited LivingEntity metadata accessors");
    ProtocolEntityDataValue {
        data_id: accessors.sleeping_pos_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
        value: EntityDataValueKind::OptionalBlockPos(pos),
    }
}

fn sleeping_pos(store: &WorldStore, entity_id: i32) -> Option<ProtocolBlockPos> {
    let entity_type_id = store.entity_type_id(entity_id).expect("entity type");
    let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
        .expect("generated inherited LivingEntity metadata accessors");
    store
        .probe_entity(entity_id)
        .expect("entity state")
        .data_values
        .iter()
        .find(|value| value.data_id == accessors.sleeping_pos_data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::OptionalBlockPos(pos) => pos,
            ref value => panic!("unexpected sleeping-position metadata value {value:?}"),
        })
}

fn occupied_bed_state_id(facing: &str) -> i32 {
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("facing".to_string(), facing.to_string());
    properties.insert("occupied".to_string(), "true".to_string());
    properties.insert("part".to_string(), "foot".to_string());
    crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:white_bed", &properties)
        .expect("occupied white-bed state")
        .id
}

fn stone_state_id() -> i32 {
    crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:stone", &std::collections::BTreeMap::new())
        .expect("stone default state")
        .id
}

fn sleeping_player(store: &mut WorldStore, entity_id: i32, bed_pos: ProtocolBlockPos) {
    store.apply_add_entity(protocol_add_entity_with_type(
        entity_id,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: entity_id,
        values: vec![
            entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID),
            sleeping_pos_data(VANILLA_ENTITY_TYPE_PLAYER_ID, Some(bed_pos)),
        ],
    }));
}

#[test]
fn player_wake_uses_safe_bed_position_and_updates_local_pose() {
    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    let bed_pos = ProtocolBlockPos { x: 2, y: 1, z: 2 };
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: bed_pos,
        block_state_id: occupied_bed_state_id("north"),
    }));
    // Facing NORTH and yaw 20 selects EAST first. The stone floor makes that
    // air cell the first safe `DismountHelper` result.
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 3, y: 0, z: 2 },
        block_state_id: stone_state_id(),
    }));
    sleeping_player(&mut store, 73, bed_pos);
    store.local_player_id = Some(73);
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 2.5,
            y: 1.6875,
            z: 2.5,
        },
        delta_movement: ProtocolVec3d {
            x: 0.25,
            y: -0.5,
            z: 0.75,
        },
        on_ground: true,
        horizontal_collision: true,
        minor_horizontal_collision: false,
        fall_distance: 3.0,
        sneaking: true,
        swimming: true,
        no_jump_delay_ticks: 0,
        y_rot: 20.0,
        x_rot: -10.0,
        last_teleport_id: 91,
    });

    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 73, action: 2 }));

    let entity = store.probe_entity(73).expect("player");
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 3.5,
            y: 1.0,
            z: 2.5,
        }
    );
    assert_eq!(entity.y_rot, 90.0);
    assert_eq!(entity.x_rot, 0.0);
    assert_eq!(entity_pose(&store, 73), VANILLA_ENTITY_POSE_STANDING_ID);
    assert_eq!(sleeping_pos(&store, 73), None);
    assert_eq!(store.player_sleep_counter(73), Some(100));
    assert_eq!(store.local_player_sleep_counter(), 100);

    let local = store.local_player_pose().expect("local pose");
    assert_eq!(
        local.position,
        ProtocolVec3d {
            x: 3.5,
            y: 1.0,
            z: 2.5,
        }
    );
    assert_eq!((local.y_rot, local.x_rot), (90.0, 0.0));
    assert!(!local.sneaking);
    assert!(!local.swimming);
    assert_eq!(
        local.delta_movement,
        ProtocolVec3d {
            x: 0.25,
            y: -0.5,
            z: 0.75,
        }
    );
    assert!(local.on_ground);
    assert!(local.horizontal_collision);
    assert_eq!(local.fall_distance, 3.0);
    assert_eq!(local.last_teleport_id, 91);
    assert_eq!(store.local_player_old_pose, Some(local));

    let bed = store
        .probe_block(BlockPos { x: 2, y: 1, z: 2 })
        .expect("loaded bed");
    assert_eq!(
        bed.block_properties.get("occupied").map(String::as_str),
        Some("false")
    );
}

#[test]
fn player_wake_uses_vanilla_fallback_when_no_safe_dismount_exists() {
    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    let bed_pos = ProtocolBlockPos { x: 2, y: 1, z: 2 };
    for y in 1..=2 {
        for x in 0..=4 {
            for z in 0..=4 {
                if y == bed_pos.y && x == bed_pos.x && z == bed_pos.z {
                    continue;
                }
                assert!(store.apply_block_update(ProtocolBlockUpdate {
                    pos: ProtocolBlockPos { x, y, z },
                    block_state_id: stone_state_id(),
                }));
            }
        }
    }
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: bed_pos,
        block_state_id: occupied_bed_state_id("south"),
    }));
    sleeping_player(&mut store, 74, bed_pos);

    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 74, action: 2 }));
    let entity = store.probe_entity(74).expect("player");
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 2.5,
            y: 2.1,
            z: 2.5,
        }
    );
    assert_eq!(entity.y_rot, -90.0);
    assert_eq!(entity.x_rot, 0.0);
    assert_eq!(store.player_sleep_counter(74), Some(100));
}

#[test]
fn player_wake_without_loaded_bed_keeps_transform_but_clears_sleep_state() {
    let mut store = WorldStore::new();
    let bed_pos = ProtocolBlockPos {
        x: 33,
        y: 70,
        z: -17,
    };
    sleeping_player(&mut store, 75, bed_pos);
    let before = store
        .probe_entity_transform(75)
        .expect("pre-wake transform");

    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 75, action: 2 }));
    let after = store
        .probe_entity_transform(75)
        .expect("post-wake transform");
    assert_eq!(after.position, before.position);
    assert_eq!((after.y_rot, after.x_rot), (before.y_rot, before.x_rot));
    assert_eq!(entity_pose(&store, 75), VANILLA_ENTITY_POSE_STANDING_ID);
    assert_eq!(sleeping_pos(&store, 75), None);
    assert_eq!(store.player_sleep_counter(75), Some(100));
    assert_eq!(store.local_player_sleep_counter(), 0);
}

#[test]
fn player_wake_ignores_non_player_semantics_and_absent_entities() {
    let mut store = WorldStore::new();
    let bed_pos = ProtocolBlockPos { x: 1, y: 2, z: 3 };
    store.apply_add_entity(protocol_add_entity_with_type(
        76,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 76,
        values: vec![
            entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID),
            sleeping_pos_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, Some(bed_pos)),
        ],
    }));

    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 76, action: 2 }));
    assert_eq!(entity_pose(&store, 76), VANILLA_ENTITY_POSE_SLEEPING_ID);
    assert_eq!(sleeping_pos(&store, 76), Some(bed_pos));
    assert_eq!(store.player_sleep_counter(76), None);
    assert_eq!(
        store.probe_entity(76).unwrap().last_animation_action,
        Some(2)
    );
    assert!(!store.apply_entity_animation(ProtocolEntityAnimation { id: 999, action: 2 }));
}

#[test]
fn player_wake_counter_ticks_and_survives_snapshots_but_not_readd() {
    let mut store = WorldStore::new();
    let bed_pos = ProtocolBlockPos { x: 1, y: 2, z: 3 };
    sleeping_player(&mut store, 77, bed_pos);

    store.advance_entity_client_animations_with_observation(120);
    assert_eq!(store.player_sleep_counter(77), Some(100));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 77, action: 2 }));
    store.advance_entity_client_animations_with_observation(9);
    assert_eq!(store.player_sleep_counter(77), Some(109));

    let snapshot = store.probe_entity(77).expect("player snapshot");
    assert_eq!(snapshot.player_sleep_counter, Some(109));
    let mut restored = WorldStore::new();
    restored.entities.insert_or_replace(snapshot);
    assert_eq!(restored.player_sleep_counter(77), Some(109));

    store.advance_entity_client_animations_with_observation(1);
    assert_eq!(store.player_sleep_counter(77), Some(0));
    store.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert_eq!(store.player_sleep_counter(77), Some(0));
}
