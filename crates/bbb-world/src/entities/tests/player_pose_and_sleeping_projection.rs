use super::*;

#[test]
fn entity_model_sources_project_elytra_animation_state() {
    const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
    const ENTITY_FLAG_FALL_FLYING: i8 = 0x80_u8 as i8;
    const POSE_CROUCHING: i32 = 5;
    const EPSILON: f32 = 1.0e-6;
    const DEFAULT_X: f32 = std::f32::consts::PI / 12.0;
    const DEFAULT_Y: f32 = 0.0;
    const DEFAULT_Z: f32 = -std::f32::consts::PI / 12.0;

    let elytra = |store: &WorldStore, id: i32, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (
            source.elytra_rot_x,
            source.elytra_rot_y,
            source.elytra_rot_z,
        )
    };
    let assert_close = |actual: f32, expected: f32, label: &str| {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "{label}: expected {expected}, got {actual}"
        );
    };
    let add_player = || {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(
            76,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
        ));
        store
    };

    // Source rows preserve the renderer's steady non-flying elytra default before
    // the first living-entity animation tick.
    let mut standing = add_player();
    assert_eq!(
        elytra(&standing, 76, 1.0),
        (DEFAULT_X, DEFAULT_Y, DEFAULT_Z)
    );
    standing.advance_entity_client_animations(1);
    let (x0, y0, z0) = elytra(&standing, 76, 0.0);
    assert_close(x0, DEFAULT_X, "standing old x");
    assert_close(y0, DEFAULT_Y, "standing old y");
    assert_close(z0, DEFAULT_Z, "standing old z");
    let (x, y, z) = elytra(&standing, 76, 1.0);
    assert_close(x, DEFAULT_X, "standing x stays at PI/12");
    assert_close(y, DEFAULT_Y, "standing y stays zero");
    assert_close(z, DEFAULT_Z, "standing z stays at -PI/12");

    // Vanilla `Entity.isCrouching()` makes the target (2PI/9, 5deg, -PI/4).
    let mut crouching = add_player();
    assert!(crouching.apply_set_entity_data(ProtocolSetEntityData {
        id: 76,
        values: vec![protocol_pose_data(
            super::dimensions::ENTITY_DATA_POSE_ID,
            POSE_CROUCHING
        )],
    }));
    crouching.advance_entity_client_animations(1);
    let (x, y, z) = elytra(&crouching, 76, 1.0);
    let crouching_target_x = std::f32::consts::PI * 2.0 / 9.0;
    let crouching_target_y = 0.087_266_46;
    let crouching_target_z = -std::f32::consts::PI / 4.0;
    assert_close(
        x,
        DEFAULT_X + (crouching_target_x - DEFAULT_X) * 0.3,
        "crouching x target",
    );
    assert_close(
        y,
        DEFAULT_Y + (crouching_target_y - DEFAULT_Y) * 0.3,
        "crouching y target",
    );
    assert_close(
        z,
        DEFAULT_Z + (crouching_target_z - DEFAULT_Z) * 0.3,
        "crouching z target",
    );

    // Vanilla `isFallFlying()` reads shared flag bit 7 and then derives the
    // X/Z target from the normalized downward velocity.
    let mut fall_flying = add_player();
    assert!(fall_flying.apply_set_entity_data(ProtocolSetEntityData {
        id: 76,
        values: vec![ProtocolEntityDataValue {
            data_id: ENTITY_SHARED_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(ENTITY_FLAG_FALL_FLYING),
        }],
    }));
    assert!(
        fall_flying.apply_set_entity_motion(ProtocolSetEntityMotion {
            id: 76,
            delta_movement: ProtocolVec3d {
                x: 1.0,
                y: -1.0,
                z: 0.0,
            },
        })
    );
    fall_flying.advance_entity_client_animations(1);
    let normalized_down = 1.0_f32 / 2.0_f32.sqrt();
    let ratio = 1.0 - normalized_down.powf(1.5);
    let target_x = std::f32::consts::PI / 12.0
        + ratio * (std::f32::consts::PI / 9.0 - std::f32::consts::PI / 12.0);
    let target_z = -std::f32::consts::PI / 12.0
        + ratio * (-std::f32::consts::PI / 2.0 + std::f32::consts::PI / 12.0);
    let expected_x = DEFAULT_X + (target_x - DEFAULT_X) * 0.3;
    let expected_z = DEFAULT_Z + (target_z - DEFAULT_Z) * 0.3;
    let (x, y, z) = elytra(&fall_flying, 76, 1.0);
    assert_close(x, expected_x, "fall-flying x velocity target");
    assert_close(y, DEFAULT_Y, "fall-flying y target");
    assert_close(z, expected_z, "fall-flying z velocity target");
    let (mid_x, _, mid_z) = elytra(&fall_flying, 76, 0.5);
    assert_close(
        mid_x,
        DEFAULT_X + (expected_x - DEFAULT_X) * 0.5,
        "fall-flying x partial lerp",
    );
    assert_close(
        mid_z,
        DEFAULT_Z + (expected_z - DEFAULT_Z) * 0.5,
        "fall-flying z partial lerp",
    );

    // Non-living entities never tick `LivingEntity.elytraAnimationState`; source
    // rows still preserve the renderer's steady elytra defaults.
    let mut boat = WorldStore::new();
    boat.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(boat.apply_set_entity_data(ProtocolSetEntityData {
        id: 77,
        values: vec![ProtocolEntityDataValue {
            data_id: ENTITY_SHARED_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(ENTITY_FLAG_FALL_FLYING),
        }],
    }));
    boat.advance_entity_client_animations(1);
    assert_eq!(elytra(&boat, 77, 1.0), (DEFAULT_X, DEFAULT_Y, DEFAULT_Z));
}

#[test]
fn entity_model_sources_project_player_cape_cloak_state() {
    const EPSILON: f32 = 1.0e-5;

    let sync_position = |store: &mut WorldStore, x: f64, y: f64, z: f64| {
        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 78,
                position: ProtocolVec3d { x, y, z },
                delta_movement: ProtocolVec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                // Facing vanilla north: forwardX = 0, forwardZ = -1.
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: true,
            })
        );
    };
    let cape = |store: &WorldStore, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 78)
            .unwrap();
        (
            source.player_cape_flap,
            source.player_cape_lean,
            source.player_cape_lean2,
        )
    };
    let assert_close = |actual: f32, expected: f32, label: &str| {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "{label}: expected {expected}, got {actual}"
        );
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    // The first cloak tick initializes both entity and cloak positions to the
    // player position, matching vanilla's teleport-safe startup behavior.
    sync_position(&mut store, 0.0, 64.0, 0.0);
    store.advance_entity_client_animations(1);
    assert_eq!(cape(&store, 1.0), (0.0, 0.0, 0.0));

    // Vanilla `ClientAvatarState.moveCloak`: each axis follows the player by 25%
    // per tick when the delta stays within 10 blocks. Moving +1y/+1z leaves the
    // cloak 0.75 blocks behind on both axes at partial=1:
    // flap = clamp(-0.75 * 10, -6, 32) = -6,
    // lean = (-0.75 * forwardZ=-1) * 100 * (1 - 1^2/100) = 74.25,
    // lean2 = 0.
    sync_position(&mut store, 0.0, 65.0, 1.0);
    store.advance_entity_client_animations(1);
    let (flap, lean, lean2) = cape(&store, 1.0);
    assert_close(flap, -6.0, "full-tick cape flap clamp");
    assert_close(lean, 74.25, "full-tick forward cape lean");
    assert_close(lean2, 0.0, "full-tick side cape lean");

    // Partial tick lerps both entity and cloak positions before the same
    // projection: the lag is half as large at partial=0.5.
    let (flap, lean, lean2) = cape(&store, 0.5);
    assert_close(flap, -3.75, "mid-tick cape flap");
    assert_close(lean, 37.40625, "mid-tick forward cape lean");
    assert_close(lean2, 0.0, "mid-tick side cape lean");
}

#[test]
fn entity_model_sources_project_dinnerbone_upside_down() {
    const VANILLA_ENTITY_CUSTOM_NAME_DATA_ID: u8 = 2;
    const OPTIONAL_COMPONENT_SERIALIZER_ID: i32 = 6;

    let source = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };
    let set_custom_name = |store: &mut WorldStore, id: i32, name: Option<&str>| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_ENTITY_CUSTOM_NAME_DATA_ID,
                serializer_id: OPTIONAL_COMPONENT_SERIALIZER_ID,
                value: EntityDataValueKind::OptionalComponent(name.map(str::to_string)),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    // A normally-named living entity is upright but still carries a real bb height.
    assert!(!source(&store, 80).is_upside_down);
    assert!(source(&store, 80).bounding_box_height > 0.0);

    // Vanilla LivingEntityRenderer.isUpsideDownName: "Dinnerbone" and "Grumm" flip.
    assert!(set_custom_name(&mut store, 80, Some("Dinnerbone")));
    assert!(source(&store, 80).is_upside_down);
    assert!(set_custom_name(&mut store, 80, Some("Grumm")));
    assert!(source(&store, 80).is_upside_down);
    // Any other name (or clearing it) leaves the entity upright.
    assert!(set_custom_name(&mut store, 80, Some("Dinnerbon")));
    assert!(!source(&store, 80).is_upside_down);
    assert!(set_custom_name(&mut store, 80, None));
    assert!(!source(&store, 80).is_upside_down);

    // A non-living entity (boat) named Dinnerbone is never flipped: only
    // LivingEntityRenderer reads the easter egg.
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(set_custom_name(&mut store, 81, Some("Dinnerbone")));
    assert!(!source(&store, 81).is_upside_down);

    // The player path keys off the GameProfile name + cape part (AvatarRenderer),
    // not the custom name, so a player with only a "Dinnerbone" custom name (no
    // player-info profile, no shown cape) stays upright. The profile-driven player
    // flip is covered by `entity_model_sources_project_player_upside_down`.
    store.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(set_custom_name(&mut store, 82, Some("Dinnerbone")));
    assert!(!source(&store, 82).is_upside_down);
}

#[test]
fn entity_model_sources_project_player_upside_down() {
    // Vanilla AvatarRenderer.isEntityUpsideDown: a Player is flipped only when its
    // cape model part is shown (DATA_PLAYER_MODE_CUSTOMISATION id 16, CAPE bit 0x01)
    // AND its GameProfile name (from the player-info list, not the custom name) is
    // "Dinnerbone"/"Grumm".
    const VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID: u8 = 16;
    const VANILLA_AVATAR_CAPE_PART_MASK: i8 = 0x01;

    let upside_down = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .is_upside_down
    };
    let set_customization = |store: &mut WorldStore, id: i32, mask: i8| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(mask),
            }],
        })
    };
    let add_player = |store: &mut WorldStore, id: i32, uuid: Uuid| {
        let mut add = protocol_add_entity_with_type(id, VANILLA_ENTITY_TYPE_PLAYER_ID);
        add.uuid = uuid;
        store.apply_add_entity(add);
    };
    let add_profile = |store: &mut WorldStore, uuid: Uuid, name: &str| {
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![ProtocolPlayerInfoAction::AddPlayer],
            entries: vec![ProtocolPlayerInfoEntry {
                profile_id: uuid,
                profile: Some(ProtocolGameProfile {
                    uuid,
                    name: name.to_string(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: ProtocolGameType::Survival,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            }],
        });
    };

    let mut store = WorldStore::new();
    add_player(&mut store, 90, default_entity_uuid());

    // A shown cape but no player-info profile yet: the GameProfile name is unknown,
    // so the player stays upright.
    assert!(set_customization(
        &mut store,
        90,
        VANILLA_AVATAR_CAPE_PART_MASK
    ));
    assert!(!upside_down(&store, 90));

    // Dinnerbone profile + shown cape: flipped.
    add_profile(&mut store, default_entity_uuid(), "Dinnerbone");
    assert!(upside_down(&store, 90));

    // Hiding the cape (CAPE bit clear) suppresses the flip even for Dinnerbone.
    assert!(set_customization(&mut store, 90, 0));
    assert!(!upside_down(&store, 90));

    // Other customization bits without the cape bit also do not flip.
    assert!(set_customization(
        &mut store,
        90,
        !VANILLA_AVATAR_CAPE_PART_MASK
    ));
    assert!(!upside_down(&store, 90));

    // Showing the cape again restores the flip.
    assert!(set_customization(
        &mut store,
        90,
        VANILLA_AVATAR_CAPE_PART_MASK
    ));
    assert!(upside_down(&store, 90));

    // A cape-showing player whose profile name is not Dinnerbone/Grumm is upright.
    let steve_uuid = Uuid::from_u128(0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA);
    add_player(&mut store, 91, steve_uuid);
    add_profile(&mut store, steve_uuid, "Steve");
    assert!(set_customization(
        &mut store,
        91,
        VANILLA_AVATAR_CAPE_PART_MASK
    ));
    assert!(!upside_down(&store, 91));

    // The other easter-egg name, "Grumm", flips too (cape shown).
    let grumm_uuid = Uuid::from_u128(0xBBBB_BBBB_BBBB_BBBB_BBBB_BBBB_BBBB_BBBB);
    add_player(&mut store, 92, grumm_uuid);
    add_profile(&mut store, grumm_uuid, "Grumm");
    assert!(set_customization(
        &mut store,
        92,
        VANILLA_AVATAR_CAPE_PART_MASK
    ));
    assert!(upside_down(&store, 92));
}

#[test]
fn entity_model_sources_project_player_extra_ears_from_profile_name() {
    // Vanilla `AbstractClientPlayer.showExtraEars` is an exact, lowercase
    // GameProfile-name check for `"deadmau5"`; it is not driven by custom names.
    const CHICKEN_TYPE_ID: i32 = 26;
    let show_extra_ears = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .show_extra_ears
    };
    let add_player = |store: &mut WorldStore, id: i32, uuid: Uuid| {
        let mut add = protocol_add_entity_with_type(id, VANILLA_ENTITY_TYPE_PLAYER_ID);
        add.uuid = uuid;
        store.apply_add_entity(add);
    };
    let add_profile = |store: &mut WorldStore, uuid: Uuid, name: &str| {
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![ProtocolPlayerInfoAction::AddPlayer],
            entries: vec![ProtocolPlayerInfoEntry {
                profile_id: uuid,
                profile: Some(ProtocolGameProfile {
                    uuid,
                    name: name.to_string(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: ProtocolGameType::Survival,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            }],
        });
    };

    let mut store = WorldStore::new();
    let deadmau5_uuid = Uuid::from_u128(0xCCCC_CCCC_CCCC_CCCC_CCCC_CCCC_CCCC_CCCC);
    add_player(&mut store, 93, deadmau5_uuid);
    assert!(!show_extra_ears(&store, 93));

    add_profile(&mut store, deadmau5_uuid, "deadmau5");
    assert!(show_extra_ears(&store, 93));

    let mixed_case_uuid = Uuid::from_u128(0xDDDD_DDDD_DDDD_DDDD_DDDD_DDDD_DDDD_DDDD);
    add_player(&mut store, 94, mixed_case_uuid);
    add_profile(&mut store, mixed_case_uuid, "Deadmau5");
    assert!(!show_extra_ears(&store, 94));

    let mut chicken = protocol_add_entity_with_type(95, CHICKEN_TYPE_ID);
    chicken.uuid = deadmau5_uuid;
    store.apply_add_entity(chicken);
    assert!(!show_extra_ears(&store, 95));
}

#[test]
fn sleeping_bed_yaw_and_offset_matches_vanilla() {
    let eye = 2.0_f32;
    let ho = eye - 0.1;
    let bed = |facing: &str| {
        let mut props = std::collections::BTreeMap::new();
        props.insert("facing".to_string(), facing.to_string());
        super::sleeping_bed_yaw_and_offset("minecraft:white_bed", &props, eye)
    };
    // Vanilla LivingEntityRenderer.sleepDirectionToRotation + Direction.getStepX/Z;
    // the head-offset translate is [-stepX * (eye - 0.1), -stepZ * (eye - 0.1)].
    assert_eq!(bed("south"), Some((90.0, [0.0, -ho])));
    assert_eq!(bed("west"), Some((0.0, [ho, 0.0])));
    assert_eq!(bed("north"), Some((270.0, [0.0, ho])));
    assert_eq!(bed("east"), Some((180.0, [-ho, 0.0])));

    // A non-bed block, or a bed without a facing, never resolves.
    let mut props = std::collections::BTreeMap::new();
    props.insert("facing".to_string(), "north".to_string());
    assert_eq!(
        super::sleeping_bed_yaw_and_offset("minecraft:stone", &props, eye),
        None
    );
    assert_eq!(
        super::sleeping_bed_yaw_and_offset(
            "minecraft:white_bed",
            &std::collections::BTreeMap::new(),
            eye,
        ),
        None
    );
}

#[test]
fn entity_model_sources_project_player_shoulder_parrots_from_optional_unsigned_int() {
    // Vanilla `Player.DATA_SHOULDER_PARROT_LEFT/RIGHT` are `OPTIONAL_UNSIGNED_INT` accessors after
    // Player absorption/score: ids 19 and 20. The decoded value is the `Parrot.Variant` id.
    const PLAYER_SHOULDER_PARROT_LEFT_DATA_ID: u8 = 19;
    const PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID: u8 = 20;
    const CHICKEN_TYPE_ID: i32 = 26;

    let shoulders = |store: &WorldStore, id: i32| {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (
            source.player_left_shoulder_parrot,
            source.player_right_shoulder_parrot,
        )
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        101,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(102, CHICKEN_TYPE_ID));

    assert_eq!(shoulders(&store, 101), (None, None));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 101,
        values: vec![
            protocol_optional_unsigned_int_data(PLAYER_SHOULDER_PARROT_LEFT_DATA_ID, Some(4)),
            protocol_optional_unsigned_int_data(PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID, Some(1)),
        ],
    }));
    assert_eq!(shoulders(&store, 101), (Some(4), Some(1)));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 101,
        values: vec![protocol_optional_unsigned_int_data(
            PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
            None,
        )],
    }));
    assert_eq!(shoulders(&store, 101), (None, Some(1)));

    // Non-player entities ignore the same data ids even if the metadata packet carries them.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 102,
        values: vec![
            protocol_optional_unsigned_int_data(PLAYER_SHOULDER_PARROT_LEFT_DATA_ID, Some(2)),
            protocol_optional_unsigned_int_data(PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID, Some(3)),
        ],
    }));
    assert_eq!(shoulders(&store, 102), (None, None));
}

#[test]
fn entity_model_sources_gate_sleeping_pose_on_living_entities() {
    const POSE_STANDING: i32 = 0;
    const POSE_SLEEPING: i32 = 2;

    let source = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };
    let set_pose = |store: &mut WorldStore, id: i32, pose: i32| {
        store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![protocol_pose_data(
                super::dimensions::ENTITY_DATA_POSE_ID,
                pose,
            )],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    // An awake (standing) entity is not sleeping.
    assert!(!source(&store, 90).is_sleeping);

    // Vanilla Pose.SLEEPING marks the entity sleeping; with no bed resolved the bed
    // yaw/offset stay at the no-bed fallback (the renderer uses the body yaw).
    assert!(set_pose(&mut store, 90, POSE_SLEEPING));
    let asleep = source(&store, 90);
    assert!(asleep.is_sleeping);
    assert_eq!(asleep.sleeping_bed_yaw, None);
    assert_eq!(asleep.sleeping_bed_offset, [0.0, 0.0]);

    // Standing again clears it.
    assert!(set_pose(&mut store, 90, POSE_STANDING));
    assert!(!source(&store, 90).is_sleeping);

    // A non-living entity (boat) with a SLEEPING pose never sleeps: only
    // LivingEntityRenderer lays entities down.
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(set_pose(&mut store, 91, POSE_SLEEPING));
    assert!(!source(&store, 91).is_sleeping);
}

#[test]
fn entity_model_sources_resolve_sleeping_bed_orientation() {
    const POSE_SLEEPING: i32 = 2;
    const SLEEPING_POS_DATA_ID: u8 = 14;
    const OPTIONAL_BLOCK_POS_SERIALIZER_ID: i32 = 11;

    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());

    // Place a north-facing bed and point the entity's sleeping position at it.
    let mut bed_props = std::collections::BTreeMap::new();
    bed_props.insert("facing".to_string(), "north".to_string());
    bed_props.insert("occupied".to_string(), "false".to_string());
    bed_props.insert("part".to_string(), "foot".to_string());
    let bed_id = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:white_bed", &bed_props)
        .expect("vanilla 26.1 north white_bed state exists")
        .id;
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 2, y: 1, z: 2 },
        block_state_id: bed_id,
    }));

    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 92,
        values: vec![
            protocol_pose_data(super::dimensions::ENTITY_DATA_POSE_ID, POSE_SLEEPING),
            ProtocolEntityDataValue {
                data_id: SLEEPING_POS_DATA_ID,
                serializer_id: OPTIONAL_BLOCK_POS_SERIALIZER_ID,
                value: EntityDataValueKind::OptionalBlockPos(Some(ProtocolBlockPos {
                    x: 2,
                    y: 1,
                    z: 2,
                })),
            },
        ],
    }));

    let source = store
        .entity_model_sources_at_partial_tick(0.0)
        .into_iter()
        .find(|source| source.entity_id == 92)
        .unwrap();
    assert!(source.is_sleeping);
    // Vanilla BedBlock.getBedOrientation reads FACING; sleepDirectionToRotation(NORTH) = 270.
    assert_eq!(source.sleeping_bed_yaw, Some(270.0));
    // headOffset = standingEyeHeight - 0.1 > 0; the NORTH step (0, -1) lifts the
    // offset to [0, +headOffset].
    assert!(source.sleeping_bed_offset[0].abs() < 1e-6);
    assert!(source.sleeping_bed_offset[1] > 0.0);
}

#[test]
fn entity_model_sources_project_scale_attribute() {
    const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;

    let scale = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .scale
    };
    let set_scale = |store: &mut WorldStore, id: i32, value: f64| {
        store.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: id,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
                base: value,
                modifiers: Vec::new(),
            }],
        })
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        95,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    // No SCALE attribute synced -> vanilla getScale() default of 1.0.
    assert_eq!(scale(&store, 95), 1.0);

    // Vanilla LivingEntity.getScale() is the SCALE attribute value.
    assert!(set_scale(&mut store, 95, 1.5));
    assert_eq!(scale(&store, 95), 1.5);

    // The SCALE attribute is clamped to [0.0625, 16.0].
    assert!(set_scale(&mut store, 95, 20.0));
    assert_eq!(scale(&store, 95), 16.0);
    assert!(set_scale(&mut store, 95, 0.001));
    assert_eq!(scale(&store, 95), 0.0625);

    // A non-living entity (boat) is gated out of the living render scale (the same
    // `vanilla_living_entity_type` gate as the other render-state projections).
    store.apply_add_entity(protocol_add_entity_with_type(
        96,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert_eq!(scale(&store, 96), 1.0);
}

#[test]
fn death_animation_gates_on_living_entity_health() {
    const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;
    const FLOAT_SERIALIZER_ID: i32 = 3;

    let zero_health = vec![ProtocolEntityDataValue {
        data_id: VANILLA_ENTITY_HEALTH_DATA_ID,
        serializer_id: FLOAT_SERIALIZER_ID,
        value: EntityDataValueKind::Float(0.0),
    }];

    // A non-living entity (item) is not a LivingEntity, so a stray float at the
    // health id never starts the death animation.
    let mut item = EntityClientAnimationState::default();
    item.sync_targets_from_metadata(VANILLA_ENTITY_TYPE_ITEM_ID, &zero_health);
    assert!(item.death.is_none());

    // A living entity at zero health begins it (deathTime 0 until the first tick).
    let mut chicken = EntityClientAnimationState::default();
    chicken.sync_targets_from_metadata(VANILLA_ENTITY_TYPE_CHICKEN_ID, &zero_health);
    assert!(chicken.death.is_some());
    assert_eq!(chicken.death_time(0.0), 0.0);
    assert!(!chicken.has_red_overlay());
}
