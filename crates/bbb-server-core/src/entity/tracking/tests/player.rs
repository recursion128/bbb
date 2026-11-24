use super::*;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_POSE_DATA_ID;

fn player_pairing_rows(entity: &ServerEntity, frame: &PlayerSyncFrame) -> Vec<EntityDataValue> {
    entity
        .send_player_pairing_data(
            uuid::Uuid::nil(),
            PlayerPairingData {
                metadata: player_metadata(frame, PlayerRows::NonDefault),
                ..Default::default()
            },
        )
        .into_iter()
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::SetEntityData(data) => Some(data.values),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn player_pose_metadata_is_dirty_once_and_pairing_keeps_the_latest_pose() {
    use bbb_protocol::entity_poses::{
        VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
        VANILLA_ENTITY_POSE_SWIMMING_ID,
    };

    let standing = player_sync_frame(1010);
    let mut entity = ServerEntity::for_player(&standing);
    assert!(player_pairing_rows(&entity, &standing).is_empty());

    let mut swimming = standing.clone();
    swimming.pose_id = VANILLA_ENTITY_POSE_SWIMMING_ID;
    swimming.pose_dirty = true;
    let published = entity.send_player_changes(swimming.clone());
    let updates: Vec<_> = published
        .ordered
        .iter()
        .filter_map(|item| match &item.packet {
            ClientboundPlayPacket::SetEntityData(data) => Some(data),
            _ => None,
        })
        .collect();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].values.len(), 1);
    assert_eq!(updates[0].values[0].data_id, VANILLA_ENTITY_POSE_DATA_ID);
    assert_eq!(
        updates[0].values[0].value,
        EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_SWIMMING_ID)
    );
    assert_eq!(player_pairing_rows(&entity, &swimming), updates[0].values);

    swimming.pose_dirty = false;
    assert!(entity
        .send_player_changes(swimming)
        .ordered
        .iter()
        .all(|item| { !matches!(item.packet, ClientboundPlayPacket::SetEntityData(_)) }));
    let mut crouching = standing.clone();
    crouching.pose_id = VANILLA_ENTITY_POSE_CROUCHING_ID;
    assert_eq!(
        player_pairing_rows(&ServerEntity::for_player(&crouching), &crouching)[0].value,
        EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_CROUCHING_ID)
    );
    let mut restored = standing;
    restored.pose_id = VANILLA_ENTITY_POSE_STANDING_ID;
    restored.pose_dirty = true;
    let published = entity.send_player_changes(restored);
    assert!(published.ordered.iter().any(|item| matches!(
        &item.packet,
        ClientboundPlayPacket::SetEntityData(data)
            if data.values.len() == 1
                && data.values[0].value == EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_STANDING_ID)
    )));
}

/// The `SetEntityData` rows one `send_player_changes` published, with their
/// audience.
fn published_rows(packets: &PlayerSyncPackets) -> Vec<(PlayerSyncAudience, Vec<EntityDataValue>)> {
    packets
        .ordered
        .iter()
        .filter_map(|item| match &item.packet {
            ClientboundPlayPacket::SetEntityData(update) => {
                Some((item.audience, update.values.clone()))
            }
            _ => None,
        })
        .collect()
}

/// `DATA_EFFECT_PARTICLES` and `DATA_EFFECT_AMBIENCE_ID` each follow their own
/// dirty bit into the packed rows (`SynchedEntityData.java:74-90`), which go
/// to the trackers and the session itself (`ServerEntity.java:336-341`), and a
/// row back at its define-time default leaves the pairing snapshot while the
/// other stays (`SynchedEntityData.java:92-106`).
#[test]
fn session_effect_rows_follow_their_own_dirty_bits_into_delta_and_pairing() {
    use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID;
    use bbb_protocol::entity_synched_data::{
        VANILLA_ENTITY_SHARED_FLAGS_DATA_ID, VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
        VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
        VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
    };
    let particle = bbb_protocol::packets::ParticlePayload {
        particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID,
        raw_options: vec![38, 0x33, 0xeb, 0xff],
    };
    let quiet = player_sync_frame(1013);
    let mut entity = ServerEntity::for_player(&quiet);

    let mut applied = quiet.clone();
    applied.shared_flags = VANILLA_ENTITY_SHARED_FLAG_INVISIBLE;
    applied.shared_flags_dirty = true;
    applied.effect_particles = vec![particle.clone()];
    applied.effect_particles_dirty = true;
    applied.effect_ambience = true;
    applied.effect_ambience_dirty = true;
    let rows = vec![
        EntityDataValue {
            data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(VANILLA_ENTITY_SHARED_FLAG_INVISIBLE),
        },
        EntityDataValue {
            data_id: VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID,
            value: EntityDataValueKind::Particles(vec![particle]),
        },
        EntityDataValue {
            data_id: VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        },
    ];
    assert_eq!(
        published_rows(&entity.send_player_changes(applied.clone())),
        vec![(PlayerSyncAudience::TrackingAndSelf, rows.clone())]
    );
    assert_eq!(player_pairing_rows(&entity, &applied), rows);

    let mut removed = applied;
    removed.shared_flags = 0;
    removed.effect_particles = Vec::new();
    removed.effect_ambience_dirty = false;
    let published = published_rows(&entity.send_player_changes(removed.clone()));
    assert_eq!(
        published,
        vec![(
            PlayerSyncAudience::TrackingAndSelf,
            vec![
                EntityDataValue {
                    value: EntityDataValueKind::Byte(0),
                    ..rows[0].clone()
                },
                EntityDataValue {
                    value: EntityDataValueKind::Particles(Vec::new()),
                    ..rows[1].clone()
                },
            ]
        )],
        "`removeEffectParticles` leaves the ambience item alone"
    );
    assert_eq!(
        player_pairing_rows(&entity, &removed),
        vec![rows[2].clone()]
    );
}

#[test]
fn player_server_entity_uses_catalog_policy_and_keeps_tick_counts_per_avatar() {
    let player = vanilla_entity_type(VANILLA_ENTITY_TYPE_PLAYER_ID).unwrap();
    assert_eq!(player.client_tracking_range, 32);
    assert_eq!(player.update_interval, 2);
    assert!(!player.track_deltas);

    let first_frame = player_sync_frame(1001);
    let second_frame = player_sync_frame(1002);
    let mut first = ServerEntity::for_player(&first_frame);
    let mut second = ServerEntity::for_player(&second_frame);
    assert_eq!(first.update_interval, 2);
    assert!(!first.track_delta);

    let first_tick_zero = first.send_player_changes(first_frame.clone());
    assert!(matches!(
        first_tick_zero.ordered.as_slice(),
        [PlayerSyncPacket {
            audience: PlayerSyncAudience::TrackingOnly,
            packet: ClientboundPlayPacket::MoveEntity(sync),
        }] if sync.id == 1001
                && sync.has_position
                && (sync.delta_x, sync.delta_y, sync.delta_z) == (0, 0, 0)
    ));
    assert!(first
        .send_player_changes(first_frame.clone())
        .ordered
        .is_empty());

    let second_tick_zero = second.send_player_changes(second_frame);
    assert!(matches!(
        second_tick_zero.ordered.as_slice(),
        [PlayerSyncPacket {
            audience: PlayerSyncAudience::TrackingOnly,
            packet: ClientboundPlayPacket::MoveEntity(sync),
        }] if sync.id == 1002
                && sync.has_position
                && (sync.delta_x, sync.delta_y, sync.delta_z) == (0, 0, 0)
    ));

    let mut moved = first_frame;
    moved.position.x += 0.25;
    let interval_tick = first.send_player_changes(moved);
    assert!(matches!(
        interval_tick.ordered.as_slice(),
        [PlayerSyncPacket {
            audience: PlayerSyncAudience::TrackingOnly,
            packet: ClientboundPlayPacket::MoveEntity(update),
        }] if update.id == 1001 && update.has_position
    ));
}

#[test]
fn player_server_entity_dirty_bypasses_interval_and_splits_recipients() {
    let initial = player_sync_frame(2001);
    let mut tracker = ServerEntity::for_player(&initial);
    tracker.send_player_changes(initial.clone());

    let mut dirty = initial.clone();
    dirty.position.x += 0.25;
    dirty.delta_movement.x = 4.0;
    dirty.y_head_rot = 90.0;
    dirty.shared_flags = 0x08;
    dirty.shared_flags_dirty = true;
    let mut scale = crate::entity::attributes::AttributeInstance::new(VANILLA_ATTRIBUTE_SCALE_ID);
    scale.set_base_value(1.25);
    dirty.dirty_attributes.push(attribute_snapshot(&scale));
    let packets = tracker.send_player_changes(dirty);
    assert_eq!(
        packets
            .ordered
            .iter()
            .map(|packet| (packet.audience, kind(&packet.packet)))
            .collect::<Vec<_>>(),
        [
            (PlayerSyncAudience::TrackingOnly, "move"),
            (PlayerSyncAudience::TrackingAndSelf, "data"),
            (PlayerSyncAudience::TrackingAndSelf, "attributes"),
            (PlayerSyncAudience::TrackingOnly, "head"),
        ]
    );
    assert!(packets
        .ordered
        .iter()
        .all(|packet| !matches!(&packet.packet, ClientboundPlayPacket::SetEntityMotion(_))));
    let ClientboundPlayPacket::SetEntityData(data) = &packets.ordered[1].packet else {
        panic!("shared flags")
    };
    assert_eq!(data.id, 2001);
    assert_eq!(data.values[0].data_id, VANILLA_ENTITY_SHARED_FLAGS_DATA_ID);
    assert_eq!(data.values[0].value, EntityDataValueKind::Byte(0x08));

    assert!(tracker
        .send_player_changes(initial)
        .ordered
        .iter()
        .all(|packet| packet.audience == PlayerSyncAudience::TrackingOnly));
}

#[test]
fn player_server_entity_riding_sends_only_rotation_and_updates_position_base() {
    let initial = player_sync_frame(3001);
    let mut tracker = ServerEntity::for_player(&initial);
    tracker.send_player_changes(initial.clone());

    let mut riding = initial.clone();
    riding.position.x = 8.0;
    riding.y_rot = 90.0;
    riding.x_rot = 20.0;
    riding.riding = true;
    riding.shared_flags = 1;
    riding.shared_flags_dirty = true;
    let packets = tracker.send_player_changes(riding.clone());
    assert!(matches!(
        packets.ordered.as_slice(),
        [
            PlayerSyncPacket {
                audience: PlayerSyncAudience::TrackingOnly,
                packet: ClientboundPlayPacket::MoveEntity(update),
            },
            PlayerSyncPacket {
                audience: PlayerSyncAudience::TrackingAndSelf,
                packet: ClientboundPlayPacket::SetEntityData(_),
            },
        ]
            if !update.has_position
                && update.y_rot == Some(90.0)
                && update.x_rot == Some(20.0)
    ));
    assert_eq!(tracker.position_base(), riding.position);

    let mut dismounted = riding;
    dismounted.riding = false;
    dismounted.shared_flags_dirty = false;
    let packets = tracker.send_player_changes(dismounted);
    assert!(matches!(
        packets.ordered.as_slice(),
        [PlayerSyncPacket {
            audience: PlayerSyncAudience::TrackingOnly,
            packet: ClientboundPlayPacket::EntityPositionSync(sync),
        }] if sync.position.x == 8.0
    ));
}

#[test]
fn client_information_metadata_defaults_and_dirty_rows_are_independent() {
    use bbb_protocol::entity_synched_data::{
        VANILLA_AVATAR_MAIN_HAND_DATA_ID, VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
    };
    use bbb_protocol::packets::ClientMainHand;
    let mut frame = player_sync_frame(1013);
    let mut entity = ServerEntity::for_player(&frame);
    assert!(player_pairing_rows(&entity, &frame).is_empty());
    frame.main_hand = ClientMainHand::Left;
    frame.main_hand_dirty = true;
    frame.model_customization = 0xff;
    frame.model_customization_dirty = true;
    let rows = published_rows(&entity.send_player_changes(frame.clone()));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, PlayerSyncAudience::TrackingAndSelf);
    assert_eq!(
        rows[0].1.iter().map(|row| row.data_id).collect::<Vec<_>>(),
        [
            VANILLA_AVATAR_MAIN_HAND_DATA_ID,
            VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID
        ]
    );
    assert_eq!(player_pairing_rows(&entity, &frame), rows[0].1);
    frame.main_hand_dirty = false;
    frame.model_customization_dirty = false;
    assert!(published_rows(&entity.send_player_changes(frame.clone())).is_empty());
    frame.main_hand = ClientMainHand::Right;
    frame.main_hand_dirty = true;
    let returned = published_rows(&entity.send_player_changes(frame.clone()));
    assert_eq!(returned[0].1.len(), 1);
    assert_eq!(
        returned[0].1[0].value,
        EntityDataValueKind::HumanoidArm(ClientMainHand::Right.id())
    );
    assert_eq!(
        player_pairing_rows(&entity, &frame),
        vec![rows[0].1[1].clone()]
    );
    frame.main_hand_dirty = false;
    frame.model_customization = 0;
    frame.model_customization_dirty = true;
    assert_eq!(
        published_rows(&entity.send_player_changes(frame.clone()))[0].1[0].value,
        EntityDataValueKind::Byte(0)
    );
    assert!(player_pairing_rows(&entity, &frame).is_empty());
}

#[test]
fn player_environment_metadata_defaults_pairing_and_dirty_order() {
    use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
    use bbb_protocol::entity_synched_data::{
        VANILLA_ENTITY_AIR_SUPPLY_DATA_ID, VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        VANILLA_ENTITY_SILENT_DATA_ID, VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
        VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
    };

    let mut frame = player_sync_frame(1014);
    let mut entity = ServerEntity::for_player(&frame);
    assert!(player_pairing_rows(&entity, &frame).is_empty());
    frame.shared_flags = 1;
    frame.shared_flags_dirty = true;
    frame.air_supply = -19;
    frame.air_supply_dirty = true;
    frame.silent = true;
    frame.silent_dirty = true;
    frame.pose_id = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID;
    frame.pose_dirty = true;
    frame.ticks_frozen = 140;
    frame.ticks_frozen_dirty = true;
    frame.living_entity_flags = 1;
    frame.living_entity_flags_dirty = true;
    let rows = published_rows(&entity.send_player_changes(frame.clone()));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, PlayerSyncAudience::TrackingAndSelf);
    assert_eq!(
        rows[0].1.iter().map(|row| row.data_id).collect::<Vec<_>>(),
        [
            VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
            VANILLA_ENTITY_AIR_SUPPLY_DATA_ID,
            VANILLA_ENTITY_SILENT_DATA_ID,
            VANILLA_ENTITY_POSE_DATA_ID,
            VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
            VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
        ]
    );
    for (index, value) in [(1, -19), (4, 140)] {
        assert_eq!(
            rows[0].1[index].serializer_id,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID
        );
        assert_eq!(rows[0].1[index].value, EntityDataValueKind::Int(value));
    }
    assert_eq!(player_pairing_rows(&entity, &frame), rows[0].1);
    frame.shared_flags_dirty = false;
    frame.air_supply_dirty = false;
    frame.silent_dirty = false;
    frame.pose_dirty = false;
    frame.ticks_frozen_dirty = false;
    frame.living_entity_flags_dirty = false;
    assert!(published_rows(&entity.send_player_changes(frame)).is_empty());
    let mut reset = player_sync_frame(1014);
    reset.shared_flags_dirty = true;
    reset.air_supply_dirty = true;
    reset.silent_dirty = true;
    reset.pose_dirty = true;
    reset.ticks_frozen_dirty = true;
    reset.living_entity_flags_dirty = true;
    entity.send_player_changes(reset.clone());
    assert!(player_pairing_rows(&entity, &reset).is_empty());
}

#[test]
fn a_gliding_player_publishes_its_known_movement_to_trackers() {
    let initial = player_sync_frame(2101);
    let mut tracker = ServerEntity::for_player(&initial);
    tracker.send_player_changes(initial.clone());
    tracker.send_player_changes(initial.clone());

    let mut gliding = initial.clone();
    gliding.shared_flags =
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
    gliding.known_movement = Vec3d {
        x: 1.5,
        y: -0.2,
        z: 0.0,
    };
    let packets = tracker.send_player_changes(gliding);
    let motion: Vec<_> = packets
        .ordered
        .iter()
        .filter_map(|packet| match &packet.packet {
            ClientboundPlayPacket::SetEntityMotion(motion) => Some((packet.audience, motion)),
            _ => None,
        })
        .collect();
    assert_eq!(motion.len(), 1);
    assert_eq!(motion[0].0, PlayerSyncAudience::TrackingOnly);
    assert_eq!(motion[0].1.delta_movement.x, 1.5);
    assert_eq!(motion[0].1.delta_movement.y, -0.2);
}
