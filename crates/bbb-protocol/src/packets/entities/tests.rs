use super::*;
use crate::{
    codec::Encoder,
    ids,
    packets::{
        decode_play_clientbound, DataComponentPatchSummary, ParticlePayload, PlayClientbound,
        PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_X,
    },
};

// WIRE_SAMPLE_SERIALIZER_IDS: numeric `serializer_id` values in this file are intentional
// decoder evidence for the raw varints written by these tests, not semantic consumer identities.

#[test]
fn decodes_entity_lifecycle_packets() {
    let uuid = Uuid::from_u128(0x12345678123456781234567812345678);
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_uuid(uuid);
    payload.write_var_i32(7);
    payload.write_f64(1.0);
    payload.write_f64(64.0);
    payload.write_f64(-2.0);
    payload.write_bytes(&lp_vec3_axis_x());
    payload.write_i8(-64);
    payload.write_i8(64);
    payload.write_i8(32);
    payload.write_var_i32(99);
    let mut bytes = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_ADD_ENTITY, &bytes).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::AddEntity(AddEntity {
            id: 123,
            uuid,
            entity_type_id: 7,
            position: Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: Vec3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: -90.0,
            y_rot: 90.0,
            y_head_rot: 45.0,
            data: 99,
        })
    );
    bytes.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_ADD_ENTITY, &bytes)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after add entity packet")
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_f64(2.0);
    payload.write_f64(65.0);
    payload.write_f64(-3.0);
    payload.write_f64(0.0);
    payload.write_f64(0.25);
    payload.write_f64(0.0);
    payload.write_f32(180.0);
    payload.write_f32(30.0);
    payload.write_bool(true);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_ENTITY_POSITION_SYNC,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::EntityPositionSync(EntityPositionSync {
            id: 123,
            position: Vec3d {
                x: 2.0,
                y: 65.0,
                z: -3.0,
            },
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.25,
                z: 0.0,
            },
            y_rot: 180.0,
            x_rot: 30.0,
            on_ground: true,
        })
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(124);
    payload.write_i16(0);
    payload.write_i16(0);
    payload.write_i16(0);
    payload.write_bool(true);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_MOVE_ENTITY_POS,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::MoveEntity(EntityMove {
            id: 124,
            has_position: true,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground: true,
        })
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_i16(4096);
    payload.write_i16(0);
    payload.write_i16(-2048);
    payload.write_i8(-64);
    payload.write_i8(64);
    payload.write_bool(false);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::MoveEntity(EntityMove {
            id: 123,
            has_position: true,
            delta_x: 4096,
            delta_y: 0,
            delta_z: -2048,
            y_rot: Some(-90.0),
            x_rot: Some(90.0),
            on_ground: false,
        })
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(77);
    payload.write_var_i32(2);
    payload.write_f64(1.0);
    payload.write_f64(64.125);
    payload.write_f64(-2.0);
    payload.write_f64(0.25);
    payload.write_f64(0.0);
    payload.write_f64(-0.25);
    payload.write_i8(32);
    payload.write_i8(-16);
    payload.write_f32(0.5);
    payload.write_f64(1.5);
    payload.write_f64(64.25);
    payload.write_f64(-2.5);
    payload.write_f64(0.5);
    payload.write_f64(0.0);
    payload.write_f64(-0.5);
    payload.write_i8(64);
    payload.write_i8(16);
    payload.write_f32(1.25);
    let payload = payload.into_inner();
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_MOVE_MINECART_ALONG_TRACK, &payload)
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::MoveMinecartAlongTrack(MoveMinecartAlongTrack {
            entity_id: 77,
            lerp_steps: vec![
                MinecartStep {
                    position: Vec3d {
                        x: 1.0,
                        y: 64.125,
                        z: -2.0,
                    },
                    movement: Vec3d {
                        x: 0.25,
                        y: 0.0,
                        z: -0.25,
                    },
                    y_rot: 45.0,
                    x_rot: -22.5,
                    weight: 0.5,
                },
                MinecartStep {
                    position: Vec3d {
                        x: 1.5,
                        y: 64.25,
                        z: -2.5,
                    },
                    movement: Vec3d {
                        x: 0.5,
                        y: 0.0,
                        z: -0.5,
                    },
                    y_rot: 90.0,
                    x_rot: 22.5,
                    weight: 1.25,
                },
            ],
        })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_MOVE_MINECART_ALONG_TRACK, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after move minecart along track packet")
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_bytes(&[0]);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_ENTITY_MOTION,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetEntityMotion(SetEntityMotion {
            id: 123,
            delta_movement: Vec3d::default(),
        })
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_f64(0.5);
    payload.write_f64(1.0);
    payload.write_f64(-0.5);
    payload.write_f64(0.0);
    payload.write_f64(0.1);
    payload.write_f64(0.0);
    payload.write_f32(15.0);
    payload.write_f32(-5.0);
    payload.write_i32(PLAYER_RELATIVE_X | PLAYER_RELATIVE_DELTA_Y);
    payload.write_bool(true);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_TELEPORT_ENTITY,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::TeleportEntity(TeleportEntity {
            id: 123,
            position: Vec3d {
                x: 0.5,
                y: 1.0,
                z: -0.5,
            },
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.1,
                z: 0.0,
            },
            y_rot: 15.0,
            x_rot: -5.0,
            relatives_mask: PLAYER_RELATIVE_X | PLAYER_RELATIVE_DELTA_Y,
            on_ground: true,
        })
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_i8(64);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_ROTATE_HEAD, &payload.into_inner()).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::RotateHead(RotateHead {
            id: 123,
            y_head_rot: 90.0,
        })
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_var_i32(123);
    payload.write_var_i32(456);
    let payload = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_REMOVE_ENTITIES, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::RemoveEntities(RemoveEntities {
            entity_ids: vec![123, 456],
        })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_REMOVE_ENTITIES, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after remove entities packet")
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(321);
    payload.write_var_i32(654);
    payload.write_var_i32(3);
    let payload = payload.into_inner();
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_TAKE_ITEM_ENTITY, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::TakeItemEntity(TakeItemEntity {
            item_id: 321,
            player_id: 654,
            amount: 3,
        })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_TAKE_ITEM_ENTITY, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after take item entity packet")
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_u8(0);
    payload.write_var_i32(0);
    payload.write_i8(0x20);
    payload.write_u8(2);
    payload.write_var_i32(1);
    payload.write_var_i32(300);
    payload.write_u8(3);
    payload.write_var_i32(5);
    payload.write_bytes(&nbt_string_root("Name"));
    payload.write_u8(4);
    payload.write_var_i32(6);
    payload.write_bool(false);
    payload.write_u8(5);
    payload.write_var_i32(9);
    payload.write_f32(1.0);
    payload.write_f32(2.0);
    payload.write_f32(3.0);
    payload.write_u8(8);
    payload.write_var_i32(7);
    payload.write_var_i32(3);
    payload.write_var_i32(42);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    payload.write_u8(0xff);
    let payload = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_SET_ENTITY_DATA, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetEntityData(SetEntityData {
            id: 123,
            values: vec![
                EntityDataValue {
                    data_id: 0,
                    serializer_id: 0,
                    value: EntityDataValueKind::Byte(0x20),
                },
                EntityDataValue {
                    data_id: 2,
                    serializer_id: 1,
                    value: EntityDataValueKind::Int(300),
                },
                EntityDataValue {
                    data_id: 3,
                    serializer_id: 5,
                    value: EntityDataValueKind::Component("Name".to_string()),
                },
                EntityDataValue {
                    data_id: 4,
                    serializer_id: 6,
                    value: EntityDataValueKind::OptionalComponent(None),
                },
                EntityDataValue {
                    data_id: 5,
                    serializer_id: 9,
                    value: EntityDataValueKind::Rotations {
                        x: 1.0,
                        y: 2.0,
                        z: 3.0,
                    },
                },
                EntityDataValue {
                    data_id: 8,
                    serializer_id: 7,
                    value: EntityDataValueKind::ItemStack(ItemStackSummary {
                        item_id: Some(42),
                        count: 3,
                        component_patch: Default::default(),
                    }),
                },
            ],
        })
    );

    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SET_ENTITY_DATA, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after set entity data packet")
    );
}

#[test]
fn bounds_set_entity_data_values_to_accessor_id_space() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    for data_id in 0..=254u8 {
        payload.write_u8(data_id);
        payload.write_var_i32(0);
        payload.write_i8(0);
    }
    payload.write_u8(0xff);

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_ENTITY_DATA,
        &payload.into_inner(),
    )
    .unwrap();
    let PlayClientbound::SetEntityData(packet) = packet else {
        panic!("expected set entity data packet");
    };
    assert_eq!(packet.values.len(), MAX_ENTITY_DATA_VALUES);

    let mut oversized = Encoder::new();
    oversized.write_var_i32(123);
    for data_id in 0..=254u8 {
        oversized.write_u8(data_id);
        oversized.write_var_i32(0);
        oversized.write_i8(0);
    }
    oversized.write_u8(0);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_ENTITY_DATA,
            &oversized.into_inner()
        ),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_ENTITY_DATA_VALUES + 1 && max == MAX_ENTITY_DATA_VALUES
    ));
}

#[test]
fn decodes_move_entity_variants_strictly() {
    let mut pos = Encoder::new();
    pos.write_var_i32(7);
    pos.write_i16(4096);
    pos.write_i16(0);
    pos.write_i16(-2048);
    pos.write_bool(true);

    let mut pos_rot = Encoder::new();
    pos_rot.write_var_i32(8);
    pos_rot.write_i16(0);
    pos_rot.write_i16(2048);
    pos_rot.write_i16(0);
    pos_rot.write_i8(64);
    pos_rot.write_i8(-32);
    pos_rot.write_bool(false);

    let mut rot = Encoder::new();
    rot.write_var_i32(9);
    rot.write_i8(-64);
    rot.write_i8(32);
    rot.write_bool(true);

    for (packet_id, payload, has_position) in [
        (
            ids::play::CLIENTBOUND_MOVE_ENTITY_POS,
            pos.into_inner(),
            true,
        ),
        (
            ids::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT,
            pos_rot.into_inner(),
            true,
        ),
        (
            ids::play::CLIENTBOUND_MOVE_ENTITY_ROT,
            rot.into_inner(),
            false,
        ),
    ] {
        let PlayClientbound::MoveEntity(packet) =
            decode_play_clientbound(packet_id, &payload).unwrap()
        else {
            panic!("expected move entity packet");
        };
        assert_eq!(packet.has_position, has_position);
        let mut trailing = payload;
        trailing.push(0);
        let error = decode_play_clientbound(packet_id, &trailing).unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after move entity packet"));
    }
}

#[test]
fn decodes_absolute_entity_corrections_strictly() {
    let mut position_sync = Encoder::new();
    position_sync.write_var_i32(7);
    for value in [1.0, 64.0, -2.0, 0.1, 0.2, 0.3] {
        position_sync.write_f64(value);
    }
    position_sync.write_f32(90.0);
    position_sync.write_f32(-15.0);
    position_sync.write_bool(true);

    let mut teleport = Encoder::new();
    teleport.write_var_i32(8);
    for value in [0.5, 1.0, -0.5, 0.0, 0.1, 0.0] {
        teleport.write_f64(value);
    }
    teleport.write_f32(15.0);
    teleport.write_f32(-5.0);
    teleport.write_i32(PLAYER_RELATIVE_X | PLAYER_RELATIVE_DELTA_Y);
    teleport.write_bool(false);

    for (packet_id, payload, error_message) in [
        (
            ids::play::CLIENTBOUND_ENTITY_POSITION_SYNC,
            position_sync.into_inner(),
            "trailing bytes after entity position sync packet",
        ),
        (
            ids::play::CLIENTBOUND_TELEPORT_ENTITY,
            teleport.into_inner(),
            "trailing bytes after teleport entity packet",
        ),
    ] {
        assert!(decode_play_clientbound(packet_id, &payload).is_ok());
        let mut trailing = payload;
        trailing.push(0);
        let error = decode_play_clientbound(packet_id, &trailing).unwrap_err();
        assert!(error.to_string().contains(error_message));
    }
}

#[test]
fn decodes_entity_motion_and_head_rotation_strictly() {
    let mut motion = Encoder::new();
    motion.write_var_i32(7);
    motion.write_i16(800);
    motion.write_i16(-400);
    motion.write_i16(0);

    let mut head = Encoder::new();
    head.write_var_i32(8);
    head.write_i8(64);

    for (packet_id, payload, error_message) in [
        (
            ids::play::CLIENTBOUND_SET_ENTITY_MOTION,
            motion.into_inner(),
            "trailing bytes after set entity motion packet",
        ),
        (
            ids::play::CLIENTBOUND_ROTATE_HEAD,
            head.into_inner(),
            "trailing bytes after rotate head packet",
        ),
    ] {
        assert!(decode_play_clientbound(packet_id, &payload).is_ok());
        let mut trailing = payload;
        trailing.push(0);
        let error = decode_play_clientbound(packet_id, &trailing).unwrap_err();
        assert!(error.to_string().contains(error_message));
    }
}

#[test]
fn decodes_additional_entity_data_serializers() {
    let owner = Uuid::from_u128(0xaaaaaaaa11111111bbbbbbbb22222222);
    let profile_id = Uuid::from_u128(0xbbbbbbbb22222222cccccccc33333333);
    let mut dust_options = Encoder::new();
    dust_options.write_i32(0x00ff00);
    dust_options.write_f32(0.5);
    let dust_options = dust_options.into_inner();
    let mut block_particle_options = Encoder::new();
    block_particle_options.write_var_i32(1234);
    let block_particle_options = block_particle_options.into_inner();

    let mut payload = Encoder::new();
    payload.write_var_i32(123);

    payload.write_u8(9);
    payload.write_var_i32(13);
    payload.write_bool(true);
    payload.write_uuid(owner);

    payload.write_u8(10);
    payload.write_var_i32(21);
    payload.write_var_i32(7);

    payload.write_u8(11);
    payload.write_var_i32(33);
    payload.write_bool(true);
    payload.write_string("overworld");
    payload.write_i64(pack_block_pos(34, 65, -12));

    payload.write_u8(12);
    payload.write_var_i32(39);
    payload.write_f32(1.25);
    payload.write_f32(-2.5);
    payload.write_f32(3.75);

    payload.write_u8(13);
    payload.write_var_i32(40);
    payload.write_f32(0.0);
    payload.write_f32(0.25);
    payload.write_f32(0.5);
    payload.write_f32(1.0);

    payload.write_u8(14);
    payload.write_var_i32(42);
    payload.write_var_i32(1);

    payload.write_u8(15);
    payload.write_var_i32(35);
    payload.write_var_i32(2);

    payload.write_u8(16);
    payload.write_var_i32(34);
    payload.write_var_i32(13);

    payload.write_u8(17);
    payload.write_var_i32(34);
    payload.write_var_i32(0);
    payload.write_var_i32(16);
    payload.write_var_i32(16);
    payload.write_string("minecraft:kebab");
    payload.write_bool(true);
    payload.write_bytes(&nbt_string_root("Kebab"));
    payload.write_bool(true);
    payload.write_bytes(&nbt_string_root("Artist"));

    payload.write_u8(18);
    payload.write_var_i32(41);
    payload.write_bool(true);
    payload.write_uuid(profile_id);
    payload.write_string("Player");
    payload.write_var_i32(1);
    payload.write_string("textures");
    payload.write_string("texture-value");
    payload.write_bool(true);
    payload.write_string("signature");
    write_skin_patch(&mut payload);

    payload.write_u8(19);
    payload.write_var_i32(16);
    payload.write_var_i32(14);
    payload.write_bytes(&dust_options);

    payload.write_u8(20);
    payload.write_var_i32(17);
    payload.write_var_i32(2);
    payload.write_var_i32(45);
    payload.write_var_i32(1);
    payload.write_bytes(&block_particle_options);

    payload.write_u8(0xff);

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_ENTITY_DATA,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetEntityData(SetEntityData {
            id: 123,
            values: vec![
                EntityDataValue {
                    data_id: 9,
                    serializer_id: 13,
                    value: EntityDataValueKind::OptionalLivingEntityReference(Some(owner)),
                },
                EntityDataValue {
                    data_id: 10,
                    serializer_id: 21,
                    value: EntityDataValueKind::RegistryId {
                        serializer: EntityDataRegistryHolder::CatVariant,
                        id: 7,
                    },
                },
                EntityDataValue {
                    data_id: 11,
                    serializer_id: 33,
                    value: EntityDataValueKind::OptionalGlobalPos(Some(GlobalPosData {
                        dimension: "minecraft:overworld".to_string(),
                        pos: BlockPos {
                            x: 34,
                            y: 65,
                            z: -12,
                        },
                    })),
                },
                EntityDataValue {
                    data_id: 12,
                    serializer_id: 39,
                    value: EntityDataValueKind::Vector3f {
                        x: 1.25,
                        y: -2.5,
                        z: 3.75,
                    },
                },
                EntityDataValue {
                    data_id: 13,
                    serializer_id: 40,
                    value: EntityDataValueKind::Quaternionf {
                        x: 0.0,
                        y: 0.25,
                        z: 0.5,
                        w: 1.0,
                    },
                },
                EntityDataValue {
                    data_id: 14,
                    serializer_id: 42,
                    value: EntityDataValueKind::HumanoidArm(1),
                },
                EntityDataValue {
                    data_id: 15,
                    serializer_id: 35,
                    value: EntityDataValueKind::EnumId {
                        serializer: EntityDataEnumSerializer::SnifferState,
                        id: 2,
                    },
                },
                EntityDataValue {
                    data_id: 16,
                    serializer_id: 34,
                    value: EntityDataValueKind::PaintingVariant(PaintingVariantData {
                        registry_id: Some(12),
                        direct: None,
                    }),
                },
                EntityDataValue {
                    data_id: 17,
                    serializer_id: 34,
                    value: EntityDataValueKind::PaintingVariant(PaintingVariantData {
                        registry_id: None,
                        direct: Some(DirectPaintingVariantData {
                            width: 16,
                            height: 16,
                            asset_id: "minecraft:kebab".to_string(),
                            title: Some("Kebab".to_string()),
                            author: Some("Artist".to_string()),
                        }),
                    }),
                },
                EntityDataValue {
                    data_id: 18,
                    serializer_id: 41,
                    value: EntityDataValueKind::ResolvableProfile(ResolvableProfileData {
                        profile: ResolvableProfileKind::GameProfile {
                            id: profile_id,
                            name: "Player".to_string(),
                            properties: 1,
                        },
                        skin_patch: PlayerSkinPatchData {
                            body: Some("minecraft:entity/player/wide/steve".to_string()),
                            cape: None,
                            elytra: Some("minecraft:entity/elytra".to_string()),
                            slim_model: Some(true),
                        },
                    }),
                },
                EntityDataValue {
                    data_id: 19,
                    serializer_id: 16,
                    value: EntityDataValueKind::Particle(ParticlePayload {
                        particle_type_id: 14,
                        raw_options: dust_options,
                    }),
                },
                EntityDataValue {
                    data_id: 20,
                    serializer_id: 17,
                    value: EntityDataValueKind::Particles(vec![
                        ParticlePayload {
                            particle_type_id: 45,
                            raw_options: Vec::new(),
                        },
                        ParticlePayload {
                            particle_type_id: 1,
                            raw_options: block_particle_options,
                        },
                    ]),
                },
            ],
        })
    );
}

#[test]
fn rejects_invalid_entity_data_global_pos_dimension() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_u8(11);
    payload.write_var_i32(33);
    payload.write_bool(true);
    payload.write_string("minecraft:Overworld");

    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_ENTITY_DATA,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn rejects_invalid_direct_painting_variant_asset_id() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_u8(9);
    payload.write_var_i32(34);
    payload.write_var_i32(0);
    payload.write_var_i32(1);
    payload.write_var_i32(1);
    payload.write_string("minecraft:Painting/Kebab");

    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_ENTITY_DATA,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn decodes_registered_entity_variant_data_serializers() {
    let registry_cases = [
        (21, EntityDataRegistryHolder::CatVariant),
        (22, EntityDataRegistryHolder::CatSoundVariant),
        (23, EntityDataRegistryHolder::CowVariant),
        (24, EntityDataRegistryHolder::CowSoundVariant),
        (25, EntityDataRegistryHolder::WolfVariant),
        (26, EntityDataRegistryHolder::WolfSoundVariant),
        (27, EntityDataRegistryHolder::FrogVariant),
        (28, EntityDataRegistryHolder::PigVariant),
        (29, EntityDataRegistryHolder::PigSoundVariant),
        (30, EntityDataRegistryHolder::ChickenVariant),
        (31, EntityDataRegistryHolder::ChickenSoundVariant),
        (32, EntityDataRegistryHolder::ZombieNautilusVariant),
    ];
    let enum_cases = [
        (35, EntityDataEnumSerializer::SnifferState),
        (36, EntityDataEnumSerializer::ArmadilloState),
        (37, EntityDataEnumSerializer::CopperGolemState),
        (38, EntityDataEnumSerializer::WeatheringCopperState),
    ];

    let mut payload = Encoder::new();
    payload.write_var_i32(321);
    let mut data_id = 0u8;
    for (serializer_id, _) in registry_cases {
        payload.write_u8(data_id);
        payload.write_var_i32(serializer_id);
        payload.write_var_i32(100 + serializer_id);
        data_id += 1;
    }
    for (serializer_id, _) in enum_cases {
        payload.write_u8(data_id);
        payload.write_var_i32(serializer_id);
        payload.write_var_i32(200 + serializer_id);
        data_id += 1;
    }
    payload.write_u8(0xff);

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_ENTITY_DATA,
        &payload.into_inner(),
    )
    .unwrap();
    let PlayClientbound::SetEntityData(packet) = packet else {
        panic!("expected set entity data packet");
    };
    assert_eq!(packet.id, 321);
    assert_eq!(packet.values.len(), registry_cases.len() + enum_cases.len());

    for ((serializer_id, serializer), value) in registry_cases.iter().zip(packet.values.iter()) {
        assert_eq!(value.serializer_id, *serializer_id);
        assert_eq!(
            value.value,
            EntityDataValueKind::RegistryId {
                serializer: *serializer,
                id: 100 + serializer_id,
            }
        );
    }

    let enum_offset = registry_cases.len();
    for ((serializer_id, serializer), value) in
        enum_cases.iter().zip(packet.values[enum_offset..].iter())
    {
        assert_eq!(value.serializer_id, *serializer_id);
        assert_eq!(
            value.value,
            EntityDataValueKind::EnumId {
                serializer: *serializer,
                id: 200 + serializer_id,
            }
        );
    }
}

#[test]
fn decodes_entity_transient_event_packets() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_u8(3);
    let payload = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_ANIMATE, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::EntityAnimation(EntityAnimation { id: 123, action: 3 })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_ANIMATE, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after entity animation packet")
    );

    let mut payload = Encoder::new();
    payload.write_i32(123);
    payload.write_i8(35);
    let payload = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_ENTITY_EVENT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::EntityEvent(EntityEvent {
            entity_id: 123,
            event_id: 35,
        })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_ENTITY_EVENT, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after entity event packet")
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_f32(45.5);
    let payload = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_HURT_ANIMATION, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::HurtAnimation(HurtAnimation { id: 123, yaw: 45.5 })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_HURT_ANIMATION, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after hurt animation packet")
    );
}

#[test]
fn decodes_set_equipment_slots_and_item_stacks() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_u8(EquipmentSlot::MainHand.ordinal() | 0x80);
    payload.write_var_i32(0);
    payload.write_u8(EquipmentSlot::Head.ordinal());
    payload.write_var_i32(1);
    payload.write_var_i32(42);
    payload.write_var_i32(0);
    payload.write_var_i32(0);

    let payload = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_SET_EQUIPMENT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetEquipment(SetEquipment {
            entity_id: 123,
            slots: vec![
                EquipmentSlotUpdate {
                    slot: EquipmentSlot::MainHand,
                    item: ItemStackSummary::empty(),
                },
                EquipmentSlotUpdate {
                    slot: EquipmentSlot::Head,
                    item: ItemStackSummary {
                        item_id: Some(42),
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                },
            ],
        })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SET_EQUIPMENT, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after set equipment packet")
    );
}

#[test]
fn decodes_update_attributes_packet() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_var_i32(2);
    payload.write_var_i32(21);
    payload.write_f64(20.0);
    payload.write_var_i32(1);
    payload.write_string("health_bonus");
    payload.write_f64(4.0);
    // AttributeModifier.Operation.STREAM_CODEC uses ByIdMap ZERO fallback.
    payload.write_var_i32(99);
    payload.write_var_i32(26);
    payload.write_f64(0.7);
    payload.write_var_i32(0);

    let payload = payload.into_inner();
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::UpdateAttributes(UpdateAttributes {
            entity_id: 123,
            attributes: vec![
                AttributeSnapshot {
                    attribute_id: 21,
                    base: 20.0,
                    modifiers: vec![AttributeModifier {
                        id: "minecraft:health_bonus".to_string(),
                        amount: 4.0,
                        operation_id: 0,
                    }],
                },
                AttributeSnapshot {
                    attribute_id: 26,
                    base: 0.7,
                    modifiers: Vec::new(),
                },
            ],
        })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after update attributes packet")
    );
}

#[test]
fn update_attributes_rejects_holders_outside_the_builtin_registry() {
    let invalid_attribute_id = crate::attribute_types::VANILLA_ATTRIBUTE_TYPES.len() as i32;
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_var_i32(1);
    payload.write_var_i32(invalid_attribute_id);

    let error = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(error.to_string().contains(&format!(
        "invalid vanilla attribute holder id {invalid_attribute_id}"
    )));
}

#[test]
fn rejects_invalid_attribute_modifier_id() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_var_i32(1);
    payload.write_var_i32(21);
    payload.write_f64(20.0);
    payload.write_var_i32(1);
    payload.write_string("minecraft:HealthBonus");

    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn decodes_set_passengers_packet() {
    let mut payload = Encoder::new();
    payload.write_var_i32(7);
    payload.write_var_i32(2);
    payload.write_var_i32(123);
    payload.write_var_i32(456);

    let payload = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_SET_PASSENGERS, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetPassengers(SetPassengers {
            vehicle_id: 7,
            passenger_ids: vec![123, 456],
        })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SET_PASSENGERS, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after set passengers packet")
    );
}

#[test]
fn decodes_set_entity_link_packet() {
    let mut payload = Encoder::new();
    payload.write_i32(123);
    payload.write_i32(456);

    let payload = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_SET_ENTITY_LINK, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetEntityLink(SetEntityLink {
            source_id: 123,
            dest_id: 456,
        })
    );
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SET_ENTITY_LINK, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after set entity link packet")
    );
}

fn nbt_string_root(text: &str) -> Vec<u8> {
    let mut payload = vec![8];
    payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
    payload.extend_from_slice(text.as_bytes());
    payload
}

fn write_skin_patch(payload: &mut Encoder) {
    payload.write_bool(true);
    payload.write_string("minecraft:entity/player/wide/steve");
    payload.write_bool(false);
    payload.write_bool(true);
    payload.write_string("minecraft:entity/elytra");
    payload.write_bool(true);
    payload.write_bool(true);
}

fn lp_vec3_axis_x() -> [u8; 6] {
    let buffer = 1u64 | (32766u64 << 3) | (16383u64 << 18) | (16383u64 << 33);
    [
        buffer as u8,
        (buffer >> 8) as u8,
        ((buffer >> 16) >> 24) as u8,
        ((buffer >> 16) >> 16) as u8,
        ((buffer >> 16) >> 8) as u8,
        (buffer >> 16) as u8,
    ]
}

fn pack_block_pos(x: i32, y: i32, z: i32) -> i64 {
    (((x as i64) & 0x3ffffff) << 38) | (((z as i64) & 0x3ffffff) << 12) | ((y as i64) & 0xfff)
}
