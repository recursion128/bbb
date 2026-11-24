use super::*;
use crate::entities::vanilla_entity_metadata_entity_accessors;
use crate::{
    ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
    PalettedContainerData, WorldDimension,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_BAT_ID,
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
    VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_SHULKER_ID, VANILLA_ENTITY_TYPE_WARDEN_ID,
};
use bbb_protocol::packets::SetBorderSize as ProtocolSetBorderSize;

mod clips_and_particles;
mod shapes;
mod supporting_and_collidability;

fn entity_shift_key_down_data() -> bbb_protocol::packets::EntityDataValue {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert_eq!(accessors.shared_flags_default, 0);
    bbb_protocol::packets::EntityDataValue {
        data_id: accessors.shared_flags_data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: bbb_protocol::packets::EntityDataValueKind::Byte(accessors.shift_key_down_mask),
    }
}

fn entity_pose_data(pose_id: i32) -> bbb_protocol::packets::EntityDataValue {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    bbb_protocol::packets::EntityDataValue {
        data_id: accessors.pose_data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
        value: bbb_protocol::packets::EntityDataValueKind::Pose(pose_id),
    }
}

#[test]
fn remote_powder_snow_walkability_matches_generated_entity_type_tag() {
    let mut world = empty_world();
    for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
        let entity_id = 1_000 + entry.id;
        add_collision_test_entity(&mut world, entity_id, entry.id, [0.0; 3]);
        assert_eq!(
            world.remote_entity_can_walk_on_powder_snow(entity_id),
            entry.powder_snow_walkable,
            "entity type {}",
            entry.resource_id
        );
    }
}

#[test]
fn remote_supporting_block_context_uses_type_boots_descending_and_falling_block() {
    const BOOTS_ITEM_ID: i32 = 43;

    let mut world = empty_world();
    world.set_powder_snow_walkable_foot_item_ids(std::collections::BTreeSet::from([BOOTS_ITEM_ID]));
    let powder_snow = vanilla_block_state_id("minecraft:powder_snow", []);
    let tagged_entity_type_id = bbb_protocol::entity_types::VANILLA_ENTITY_TYPES
        .iter()
        .find(|entry| entry.powder_snow_walkable)
        .map(|entry| entry.id)
        .expect("generated powder-snow-walkable entity type");
    let cases = [
        (51, tagged_entity_type_id, 0, true),
        (
            52,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            2,
            true,
        ),
        (
            53,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            4,
            false,
        ),
        (54, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, 6, true),
        (
            55,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            8,
            false,
        ),
    ];
    for (entity_id, entity_type_id, x, _) in cases {
        set_block(&mut world, BlockPos { x, y: 0, z: 0 }, powder_snow);
        add_collision_test_entity(
            &mut world,
            entity_id,
            entity_type_id,
            [f64::from(x) + 0.5, 1.0, 0.5],
        );
    }
    for entity_id in [52, 53] {
        assert!(
            world.apply_set_equipment(bbb_protocol::packets::SetEquipment {
                entity_id,
                slots: vec![bbb_protocol::packets::EquipmentSlotUpdate {
                    slot: ProtocolEquipmentSlot::Feet,
                    item: bbb_protocol::packets::ItemStackSummary {
                        item_id: Some(BOOTS_ITEM_ID),
                        count: 1,
                        component_patch: Default::default(),
                    },
                }],
            })
        );
    }
    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 53,
            values: vec![entity_shift_key_down_data()],
        })
    );

    for (entity_id, _, x, expected_support) in cases {
        assert!(
            world.apply_entity_position_sync(bbb_protocol::packets::EntityPositionSync {
                id: entity_id,
                position: bbb_protocol::packets::Vec3d {
                    x: f64::from(x) + 0.5,
                    y: 1.0,
                    z: 0.5,
                },
                delta_movement: bbb_protocol::packets::Vec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: true,
            })
        );
        assert_eq!(
            world.entity_main_supporting_block_pos(entity_id),
            expected_support.then_some(BlockPos { x, y: 0, z: 0 }),
            "entity {entity_id} powder-snow context"
        );
    }
}

#[test]
fn collision_debug_collidability_matches_generated_class_dispatch() {
    let mut world = empty_world();
    world.set_local_player_pose(LocalPlayerPoseState::default());
    let camera = world
        .debug_camera_collision_query()
        .expect("local-player collision query");
    let target_box = WorldCollisionBox {
        min: [0.0; 3],
        max: [1.0; 3],
    };
    for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
        let entity_id = 1_000 + entry.id;
        add_collision_test_entity(&mut world, entity_id, entry.id, [0.5, 0.0, 0.5]);
        let expected = matches!(
            vanilla_entity_metadata_type_collidability_kind(entry.id),
            Some(VanillaEntityCollidabilityKind::Always | VanillaEntityCollidabilityKind::Alive)
        );
        assert_eq!(
            world.debug_entity_can_be_collided_with(camera, entity_id, entry.id, target_box,),
            expected,
            "entity type {}",
            entry.resource_id
        );
    }
    assert!(!world.debug_entity_can_be_collided_with(camera, i32::MAX, -1, target_box,));
}

#[test]
fn happy_ghast_collision_requires_adult_alive_and_player_above_or_stays_still() {
    let mut world = empty_world();
    let accessors = vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    )
    .expect("generated HappyGhast collision accessors");
    let (baby_data_id, baby_default) =
        crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        )
        .expect("generated HappyGhast baby accessor");
    assert!(!baby_default);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 0.0,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    add_collision_test_entity(
        &mut world,
        20,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        [2.5, 0.0, 0.5],
    );
    assert!(world.debug_camera_collision_boxes(6.0).is_empty());

    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 4.0,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    assert_eq!(world.debug_camera_collision_boxes(6.0).len(), 1);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 0.0,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });

    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 20,
            values: vec![bbb_protocol::packets::EntityDataValue {
                data_id: accessors.is_leash_holder_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: bbb_protocol::packets::EntityDataValueKind::Boolean(true),
            }],
        })
    );
    assert!(world.debug_camera_collision_boxes(6.0).is_empty());

    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 20,
            values: vec![bbb_protocol::packets::EntityDataValue {
                data_id: accessors.stays_still_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: bbb_protocol::packets::EntityDataValueKind::Boolean(true),
            }],
        })
    );
    assert_eq!(world.debug_camera_collision_boxes(6.0).len(), 1);

    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 20,
            values: vec![bbb_protocol::packets::EntityDataValue {
                data_id: baby_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: bbb_protocol::packets::EntityDataValueKind::Boolean(true),
            }],
        })
    );
    assert!(world.debug_camera_collision_boxes(6.0).is_empty());
}

#[test]
fn collision_debug_pushability_matches_generated_class_dispatch() {
    let mut world = empty_world();
    for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
        let entity_id = 1_000 + entry.id;
        add_collision_test_entity(&mut world, entity_id, entry.id, [0.5, 0.0, 0.5]);
        let expected = vanilla_entity_metadata_type_pushability_kind(entry.id)
            .is_some_and(|kind| kind != VanillaEntityPushabilityKind::Never);
        assert_eq!(
            world.debug_entity_is_pushable(entity_id, entry.id, [0.5, 0.0, 0.5]),
            expected,
            "entity type {}",
            entry.resource_id
        );
    }
    assert!(!world.debug_entity_is_pushable(i32::MAX, -1, [0.5, 0.0, 0.5]));
}

#[test]
fn turtle_collision_uses_living_pushability_and_never_direct_collidability() {
    let turtle_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID;
    assert_eq!(
        vanilla_entity_metadata_type_pushability_kind(turtle_type),
        Some(VanillaEntityPushabilityKind::Living)
    );
    assert_eq!(
        vanilla_entity_metadata_type_collidability_kind(turtle_type),
        Some(VanillaEntityCollidabilityKind::Never)
    );

    let mut world = empty_world();
    add_collision_test_entity(&mut world, 39, turtle_type, [0.5, 0.0, 0.5]);
    assert!(world.debug_entity_is_pushable(39, turtle_type, [0.5, 0.0, 0.5]));

    let ladder = vanilla_block_state_id(
        "minecraft:ladder",
        [("facing", "north"), ("waterlogged", "false")],
    );
    set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, ladder);
    assert!(!world.debug_entity_is_pushable(39, turtle_type, [0.5, 0.0, 0.5]));
}

#[test]
fn generated_pushability_kinds_preserve_warden_and_creaking_instance_gates() {
    let mut world = empty_world();
    let (creaking_can_move_data_id, creaking_can_move_default) =
        vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
        )
        .expect("generated Creaking can-move accessor");
    assert!(creaking_can_move_default);
    add_collision_test_entity(
        &mut world,
        41,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
        [0.5, 0.0, 0.5],
    );
    add_collision_test_entity(
        &mut world,
        42,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
        [1.5, 0.0, 0.5],
    );
    assert!(world.debug_entity_is_pushable(41, VANILLA_ENTITY_TYPE_WARDEN_ID, [0.5, 0.0, 0.5],));
    assert!(world.debug_entity_is_pushable(42, VANILLA_ENTITY_TYPE_CREAKING_ID, [1.5, 0.0, 0.5],));

    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 41,
            values: vec![entity_pose_data(VANILLA_ENTITY_POSE_DIGGING_ID)],
        })
    );
    assert!(!world.debug_entity_is_pushable(41, VANILLA_ENTITY_TYPE_WARDEN_ID, [0.5, 0.0, 0.5],));

    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 42,
            values: vec![bbb_protocol::packets::EntityDataValue {
                data_id: creaking_can_move_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: bbb_protocol::packets::EntityDataValueKind::Boolean(false),
            }],
        })
    );
    assert!(!world.debug_entity_is_pushable(42, VANILLA_ENTITY_TYPE_CREAKING_ID, [1.5, 0.0, 0.5],));
}

fn axis_name(axis: CollisionAxis) -> &'static str {
    match axis {
        CollisionAxis::X => "x",
        CollisionAxis::Y => "y",
        CollisionAxis::Z => "z",
    }
}

fn add_collision_test_entity(
    world: &mut WorldStore,
    id: i32,
    entity_type_id: i32,
    position: [f64; 3],
) {
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id,
        uuid: uuid::Uuid::from_u128(id as u128 + 1),
        entity_type_id,
        position: bbb_protocol::packets::Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        delta_movement: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
}

fn world_collision_box_from_pick_target(target: crate::EntityPickTargetState) -> WorldCollisionBox {
    WorldCollisionBox {
        min: [
            target.position.x + f64::from(target.bounds.min[0]),
            target.position.y + f64::from(target.bounds.min[1]),
            target.position.z + f64::from(target.bounds.min[2]),
        ],
        max: [
            target.position.x + f64::from(target.bounds.max[0]),
            target.position.y + f64::from(target.bounds.max[1]),
            target.position.z + f64::from(target.bounds.max[2]),
        ],
    }
}

fn empty_world() -> WorldStore {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![std::sync::Arc::new(ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
            biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
        })],
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    });
    world
}

fn single_value_container(
    domain: PaletteDomain,
    entry_count: usize,
    global_id: i32,
) -> PalettedContainerData {
    PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn set_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
    // Scene setup tolerates the vanilla identical-state no-op; the block
    // must hold the requested state either way.
    world.apply_block_update(bbb_protocol::packets::BlockUpdate {
        pos: bbb_protocol::packets::BlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        block_state_id,
    });
    assert_eq!(
        world.probe_block(pos).map(|block| block.block_state_id),
        Some(block_state_id)
    );
}

fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
    crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties(name, &string_props(props))
        .unwrap_or_else(|| panic!("vanilla 26.1 block state exists for {name}"))
        .id
}

fn string_props<const N: usize>(entries: [(&str, &str); N]) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn assert_f64_near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1.0e-12,
        "actual {actual} expected {expected}",
    );
}
