use bbb_client_render::{CameraHurtEffect, CameraPose, CameraViewBobEffect};
use bbb_client_world::WorldStore;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CameraEyeHeightTickState {
    eye_height_old: f32,
    eye_height: f32,
}

impl CameraEyeHeightTickState {
    /// Vanilla `Camera.tick`: retain the previous endpoint and ease halfway
    /// toward the active camera entity's current pose eye height.
    pub(crate) fn advance_for_world(&mut self, ticks: u32, world: &WorldStore) {
        for _ in 0..ticks {
            let Some(target) =
                camera_pose_from_world_at_partial_tick(world, 1.0).map(|pose| pose.eye_height)
            else {
                continue;
            };
            self.eye_height_old = self.eye_height;
            self.eye_height += (target - self.eye_height) * 0.5;
        }
    }

    pub(crate) fn frame_eye_height(self, partial_ticks: f32) -> f32 {
        let partial_ticks = partial_ticks.clamp(0.0, 1.0);
        self.eye_height_old + (self.eye_height - self.eye_height_old) * partial_ticks
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum CameraType {
    #[default]
    FirstPerson,
    ThirdPersonBack,
    ThirdPersonFront,
}

impl CameraType {
    pub(crate) fn cycle(self) -> Self {
        match self {
            Self::FirstPerson => Self::ThirdPersonBack,
            Self::ThirdPersonBack => Self::ThirdPersonFront,
            Self::ThirdPersonFront => Self::FirstPerson,
        }
    }

    pub(crate) fn is_first_person(self) -> bool {
        matches!(self, Self::FirstPerson)
    }

    pub(crate) fn is_mirrored(self) -> bool {
        matches!(self, Self::ThirdPersonFront)
    }
}

pub(crate) fn current_post_effect_id_from_world(world: &WorldStore) -> Option<&'static str> {
    let camera_entity_id = world.camera_entity_id()?;
    let entity_type_id = world.entity_type_id(camera_entity_id)?;
    bbb_client_world::vanilla_entity_camera_post_effect_id(entity_type_id)
}

pub(crate) fn camera_pose_from_world(world: &WorldStore) -> Option<CameraPose> {
    camera_pose_from_world_at_partial_tick(world, 1.0)
}

pub(crate) fn camera_pose_from_world_at_partial_tick(
    world: &WorldStore,
    partial_ticks: f32,
) -> Option<CameraPose> {
    camera_entity_transform(world, partial_ticks).map(|(pose, _, _)| pose)
}

fn camera_entity_transform(
    world: &WorldStore,
    partial_ticks: f32,
) -> Option<(CameraPose, [f64; 3], Option<i32>)> {
    let camera = world.local_player().camera;
    let remote = camera
        .entity_id
        .filter(|_| !camera.follows_player)
        .and_then(|id| world.probe_entity_camera_pose_at_partial_tick(id, partial_ticks));
    let (position, y_rot, x_rot, eye_height, id) = if let Some(entity) = remote {
        (
            entity.position,
            entity.y_rot,
            entity.x_rot,
            entity.eye_height,
            Some(entity.id),
        )
    } else {
        let player = world.local_player_pose_at_partial_tick(partial_ticks)?;
        (
            player.position,
            player.y_rot,
            player.x_rot,
            world.local_player_eye_height(player) as f32,
            world.local_player_id(),
        )
    };
    let position = [position.x, position.y, position.z];
    Some((
        CameraPose {
            position: position.map(|coordinate| coordinate as f32),
            y_rot,
            x_rot,
            eye_height,
        },
        position,
        id,
    ))
}

#[derive(Clone, Copy)]
pub(crate) struct FrameCamera {
    pub(crate) entity_pose: CameraPose,
    pub(crate) pose: CameraPose,
    pub(crate) eye: [f64; 3],
}

pub(crate) fn frame_camera_from_world_at_partial_tick(
    world: &WorldStore,
    partial_ticks: f32,
    camera_type: CameraType,
    eye_height_ticks: CameraEyeHeightTickState,
) -> Option<FrameCamera> {
    let (mut camera_pose, mut eye, entity_id) = camera_entity_transform(world, partial_ticks)?;
    camera_pose.eye_height = eye_height_ticks.frame_eye_height(partial_ticks);
    eye[1] += f64::from(camera_pose.eye_height);
    let entity_pose = camera_pose;
    if camera_type.is_first_person() {
        if let Some(y_rot) = entity_id.and_then(|id| world.sleeping_camera_y_rot(id)) {
            // Camera.alignWithEntity: face along the bed, then move(0, 0.3, 0).
            camera_pose.y_rot = y_rot;
            camera_pose.x_rot = 0.0;
            camera_pose.position[1] += 0.3;
            eye[1] += f64::from(0.3_f32);
        }
    } else {
        let requested_distance = world.camera_entity_third_person_distance().unwrap_or(4.0);
        (camera_pose, eye) = detached_camera_pose_with_clip(
            camera_pose,
            eye,
            camera_type,
            requested_distance,
            |from, to| world.camera_visual_block_hit(from, to),
        );
    }
    Some(FrameCamera {
        entity_pose,
        pose: camera_pose,
        eye,
    })
}

pub(crate) fn render_camera_pose_from_world_at_partial_tick(
    world: &WorldStore,
    partial_ticks: f32,
    camera_type: CameraType,
    eye_height_ticks: CameraEyeHeightTickState,
) -> Option<CameraPose> {
    frame_camera_from_world_at_partial_tick(world, partial_ticks, camera_type, eye_height_ticks)
        .map(|camera| camera.pose)
}

fn detached_camera_pose_with_clip(
    mut camera_pose: CameraPose,
    base: [f64; 3],
    camera_type: CameraType,
    requested_distance: f32,
    mut clip: impl FnMut([f64; 3], [f64; 3]) -> Option<[f64; 3]>,
) -> (CameraPose, [f64; 3]) {
    if camera_type.is_mirrored() {
        camera_pose.y_rot += 180.0;
        camera_pose.x_rot = -camera_pose.x_rot;
    }
    let forward = camera_forward_vector(camera_pose);
    let mut distance = if requested_distance.is_finite() {
        requested_distance.max(0.0)
    } else {
        4.0
    };
    for corner in 0..8 {
        let offset = [
            f64::from((corner & 1) * 2 - 1) * 0.1,
            f64::from(((corner >> 1) & 1) * 2 - 1) * 0.1,
            f64::from(((corner >> 2) & 1) * 2 - 1) * 0.1,
        ];
        let from = std::array::from_fn(|axis| base[axis] + offset[axis]);
        let to =
            std::array::from_fn(|axis| from[axis] - f64::from(forward[axis]) * f64::from(distance));
        let Some(hit) = clip(from, to) else {
            continue;
        };
        let hit_distance = hit
            .into_iter()
            .zip(base)
            .map(|(hit, base)| (hit - base).powi(2))
            .sum::<f64>()
            .sqrt() as f32;
        if hit_distance < distance {
            distance = hit_distance;
        }
    }
    let eye = std::array::from_fn(|axis| base[axis] - f64::from(forward[axis] * distance));
    camera_pose.position = eye.map(|coordinate| coordinate as f32);
    camera_pose.eye_height = 0.0;
    (camera_pose, eye)
}

pub(crate) fn camera_eye_position(camera_pose: CameraPose) -> [f32; 3] {
    [
        camera_pose.position[0],
        camera_pose.position[1] + camera_pose.eye_height,
        camera_pose.position[2],
    ]
}

pub(crate) fn camera_forward_vector(camera_pose: CameraPose) -> [f32; 3] {
    let yaw = camera_pose.y_rot.to_radians();
    let pitch = camera_pose.x_rot.to_radians();
    let cos_pitch = pitch.cos();
    [-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch]
}

pub(crate) fn camera_hurt_effect_from_world(
    world: &WorldStore,
    partial_ticks: f32,
    damage_tilt_strength: f32,
) -> Option<CameraHurtEffect> {
    let camera = world.local_player().camera;
    let camera_entity_id = if camera.follows_player {
        world.local_player_id()?
    } else {
        camera.entity_id?
    };
    let state = world.probe_entity_camera_hurt_state(camera_entity_id, partial_ticks)?;
    Some(CameraHurtEffect {
        hurt_time: state.hurt_time,
        hurt_duration: state.hurt_duration,
        hurt_dir: state.hurt_dir,
        death_time: state.death_time,
        is_dead_or_dying: state.is_dead_or_dying,
        damage_tilt_strength,
    })
}

pub(crate) fn camera_view_bob_effect_from_world(
    world: &WorldStore,
    partial_ticks: f32,
    enabled: bool,
) -> Option<CameraViewBobEffect> {
    if !enabled {
        return None;
    }
    let camera = world.local_player().camera;
    let camera_entity_id = if camera.follows_player {
        world.local_player_id()?
    } else {
        camera.entity_id?
    };
    let state = world.probe_entity_camera_view_bob_state(camera_entity_id, partial_ticks)?;
    Some(CameraViewBobEffect {
        backwards_interpolated_walk_distance: state.backwards_interpolated_walk_distance,
        bob: state.bob,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_world::{LocalPlayerInputState, LocalPlayerPoseState};
    use bbb_protocol::packets::Vec3d;

    #[test]
    fn render_camera_uses_local_player_tick_endpoints() {
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            y_rot: 170.0,
            ..LocalPlayerPoseState::default()
        });
        let current = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    mouse_delta_x: 200.0,
                    ..LocalPlayerInputState::default()
                },
                0.05,
            )
            .unwrap();

        let old = camera_pose_from_world_at_partial_tick(&world, 0.0).unwrap();
        let mid = camera_pose_from_world_at_partial_tick(&world, 0.5).unwrap();
        let end = camera_pose_from_world(&world).unwrap();
        assert_eq!(old.position, [0.0, 64.0, 0.0]);
        assert_eq!(end.position[2], current.position.z as f32);
        assert_eq!(mid.position[2], end.position[2] * 0.5);
        assert_eq!(old.y_rot, 170.0);
        assert_eq!(mid.y_rot, 185.0);
        assert_eq!(end.y_rot, current.y_rot);
    }

    #[test]
    fn camera_perspective_detached_pose_matches_back_and_mirrored_front_offsets() {
        let base = CameraPose {
            position: [1.0, 64.0, 2.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.5,
        };
        let eye = [1.0, 65.5, 2.0];
        let (back, _) =
            detached_camera_pose_with_clip(base, eye, CameraType::ThirdPersonBack, 4.0, |_, _| {
                None
            });
        assert_eq!(back.position, [1.0, 65.5, -2.0]);
        assert_eq!(back.y_rot, 0.0);
        assert_eq!(back.x_rot, 0.0);
        assert_eq!(back.eye_height, 0.0);

        let (front, _) =
            detached_camera_pose_with_clip(base, eye, CameraType::ThirdPersonFront, 4.0, |_, _| {
                None
            });
        assert!((front.position[0] - 1.0).abs() < 1.0e-5);
        assert_eq!(front.position[1], 65.5);
        assert!((front.position[2] - 6.0).abs() < 1.0e-5);
        assert_eq!(front.y_rot, 180.0);
        assert_eq!(front.x_rot, -0.0);
        assert_eq!(front.eye_height, 0.0);
    }

    #[test]
    fn camera_perspective_visual_clip_uses_eight_jittered_rays_and_shortest_hit() {
        let base = CameraPose {
            position: [24_000_000.0, 8.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.0,
        };
        let mut rays = Vec::new();
        let eye = [24_000_000.9, 9.0, 0.0];
        let (clipped, clipped_eye) = detached_camera_pose_with_clip(
            base,
            eye,
            CameraType::ThirdPersonBack,
            4.0,
            |from, to| {
                rays.push((from, to));
                (rays.len() == 1).then_some([eye[0], 9.0, -2.0])
            },
        );

        assert_eq!(rays.len(), 8);
        assert_eq!(rays[0].0, [eye[0] - 0.1, 8.9, -0.1]);
        assert_eq!(rays[0].1, [eye[0] - 0.1, 8.9, -4.1]);
        assert_eq!(clipped_eye, [eye[0], 9.0, -2.0]);
        assert_eq!(clipped.position, [24_000_000.0, 9.0, -2.0]);
        assert_eq!(rays[1].1[2] - rays[1].0[2], -2.0);
    }

    #[test]
    fn camera_eye_height_tick_state_eases_and_interpolates_pose_transitions() {
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        let mut eye_height = CameraEyeHeightTickState::default();

        eye_height.advance_for_world(1, &world);
        assert!((eye_height.frame_eye_height(0.0) - 0.0).abs() < 1.0e-6);
        assert!((eye_height.frame_eye_height(0.5) - 0.405).abs() < 1.0e-6);
        assert!((eye_height.frame_eye_height(1.0) - 0.81).abs() < 1.0e-6);

        world.set_local_player_pose(LocalPlayerPoseState {
            swimming: true,
            ..LocalPlayerPoseState::default()
        });
        eye_height.advance_for_world(1, &world);
        assert!((eye_height.frame_eye_height(0.0) - 0.81).abs() < 1.0e-6);
        assert!((eye_height.frame_eye_height(0.5) - 0.7075).abs() < 1.0e-6);
        assert!((eye_height.frame_eye_height(1.0) - 0.605).abs() < 1.0e-6);
    }

    fn logged_in_world() -> WorldStore {
        use bbb_protocol::packets::{CommonPlayerSpawnInfo, PlayLogin};
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 7,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        world
    }

    #[test]
    fn elytra_gliding_lowers_the_camera_to_the_fall_flying_eye_height() {
        use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, SetEntityData};
        let mut world = logged_in_world();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        let accessors = bbb_client_world::vanilla_entity_metadata_entity_accessors().unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 7,
            values: vec![EntityDataValue {
                data_id: accessors.shared_flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.fall_flying_mask),
            }],
        }));

        // Avatar.POSES FALL_FLYING: eye height 0.4 on a 0.6 box.
        let camera = camera_pose_from_world_at_partial_tick(&world, 1.0).unwrap();
        assert_eq!(camera.eye_height, 0.4);
        let pose = world.local_player_pose().unwrap();
        assert_eq!(world.local_player_dimensions(pose).height, 0.6);
    }

    #[test]
    fn sleeping_first_person_camera_follows_the_local_or_remote_sleeping_entity() {
        use bbb_protocol::packets::{
            AddEntity, BlockPos, EntityDataValue, EntityDataValueKind, SetCamera, SetEntityData,
        };
        let mut world = logged_in_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 37.0,
            x_rot: 20.0,
            ..LocalPlayerPoseState::default()
        });
        let living =
            bbb_client_world::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            )
            .unwrap();
        let sleeping = EntityDataValue {
            data_id: living.sleeping_pos_data_id,
            serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
            value: EntityDataValueKind::OptionalBlockPos(Some(BlockPos { x: 4, y: 64, z: -3 })),
        };
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 7,
            values: vec![sleeping.clone()]
        }));
        let mut eye_height = CameraEyeHeightTickState::default();
        eye_height.advance_for_world(1, &world);

        // Camera.alignWithEntity off a bed block: yaw 0, pitch 0, then 0.3 up;
        // setPosToBed put the feet at bed + (0.5, 0.6875, 0.5).
        let camera = render_camera_pose_from_world_at_partial_tick(
            &world,
            1.0,
            CameraType::FirstPerson,
            eye_height,
        )
        .unwrap();
        assert_eq!((camera.y_rot, camera.x_rot), (0.0, 0.0));
        assert_eq!(camera.position, [4.5, 64.6875 + 0.3, -2.5]);
        let pose = world.local_player_pose().unwrap();
        assert_eq!(world.local_player_eye_height(pose), f64::from(0.2_f32));

        let remote_bed = BlockPos {
            x: 24_000_000,
            y: 64,
            z: -3,
        };
        world.insert_decoded_chunk(bbb_client_world::ChunkColumn {
            pos: bbb_client_world::ChunkPos {
                x: remote_bed.x.div_euclid(16),
                z: remote_bed.z.div_euclid(16),
            },
            state: bbb_client_world::ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections:
                (0..24)
                    .map(|_| {
                        std::sync::Arc::new(bbb_client_world::ChunkSection {
                            non_empty_block_count: 0,
                            fluid_count: 0,
                            block_states: bbb_client_world::PalettedContainerData {
                                domain: bbb_client_world::PaletteDomain::BlockStates,
                                bits_per_entry: 0,
                                palette_kind: bbb_client_world::PaletteKind::SingleValue,
                                palette_global_ids: vec![
                                    bbb_client_world::vanilla_air_block_state_id(),
                                ],
                                packed_data: Vec::new(),
                                entry_count: 4096,
                            },
                            biomes: bbb_client_world::PalettedContainerData {
                                domain: bbb_client_world::PaletteDomain::Biomes,
                                bits_per_entry: 0,
                                palette_kind: bbb_client_world::PaletteKind::SingleValue,
                                palette_global_ids: vec![0],
                                packed_data: Vec::new(),
                                entry_count: 64,
                            },
                        })
                    })
                    .collect(),
            block_entities: std::sync::Arc::default(),
            light: bbb_client_world::LightData::default(),
        });
        let bed_state_id = world
            .registries()
            .block_state_id_by_name_and_properties(
                "minecraft:white_bed",
                &std::collections::BTreeMap::from([
                    ("facing".to_string(), "east".to_string()),
                    ("occupied".to_string(), "true".to_string()),
                    ("part".to_string(), "head".to_string()),
                ]),
            )
            .unwrap();
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: remote_bed,
                block_state_id: bed_state_id,
            })
        );

        world.apply_add_entity(AddEntity {
            id: 8,
            uuid: uuid::Uuid::from_u128(8),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID,
            position: Vec3d {
                x: 24_000_000.9,
                y: 64.0,
                z: -3.0,
            },
            delta_movement: Vec3d::default(),
            x_rot: 20.0,
            y_rot: 37.0,
            y_head_rot: 37.0,
            data: 0,
        });
        assert!(world.apply_set_camera(SetCamera { camera_id: 8 }));
        let mut remote_eye_height = CameraEyeHeightTickState::default();
        remote_eye_height.advance_for_world(1, &world);
        let awake = frame_camera_from_world_at_partial_tick(
            &world,
            1.0,
            CameraType::FirstPerson,
            remote_eye_height,
        )
        .unwrap();
        assert_eq!(awake.pose, awake.entity_pose);
        assert_eq!((awake.pose.y_rot, awake.pose.x_rot), (37.0, 20.0));
        assert_eq!(awake.eye[0], 24_000_000.9);

        let entity_accessors =
            bbb_client_world::vanilla_entity_metadata_entity_accessors().unwrap();
        let villager_living =
            bbb_client_world::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID,
            )
            .unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 8,
            values: vec![
                EntityDataValue {
                    data_id: entity_accessors.pose_data_id,
                    serializer_id:
                        bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                    value: EntityDataValueKind::Pose(
                        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID,
                    ),
                },
                EntityDataValue {
                    data_id: villager_living.sleeping_pos_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                    value: EntityDataValueKind::OptionalBlockPos(Some(remote_bed)),
                },
            ],
        }));
        remote_eye_height = CameraEyeHeightTickState::default();
        remote_eye_height.advance_for_world(1, &world);
        let asleep = frame_camera_from_world_at_partial_tick(
            &world,
            1.0,
            CameraType::FirstPerson,
            remote_eye_height,
        )
        .unwrap();
        let feet = world
            .probe_entity_camera_pose_at_partial_tick(8, 1.0)
            .unwrap()
            .position;
        assert_eq!([feet.x, feet.y, feet.z], [24_000_000.5, 64.6875, -2.5]);
        assert!((asleep.entity_pose.eye_height - 0.1).abs() < 1.0e-6);
        assert_eq!((asleep.pose.y_rot, asleep.pose.x_rot), (90.0, 0.0));
        assert_eq!(
            asleep.eye,
            [
                24_000_000.5,
                64.6875 + f64::from(asleep.entity_pose.eye_height) + f64::from(0.3_f32),
                -2.5,
            ]
        );
    }

    #[test]
    fn third_person_camera_keeps_the_world_eye_after_height_and_distance_offsets() {
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d {
                x: 24_000_000.9,
                y: 0.0,
                z: 5.25,
            },
            ..LocalPlayerPoseState::default()
        });
        let mut eye_height = CameraEyeHeightTickState::default();
        eye_height.advance_for_world(1, &world);

        let first_person = frame_camera_from_world_at_partial_tick(
            &world,
            1.0,
            CameraType::FirstPerson,
            eye_height,
        )
        .unwrap();
        assert!((first_person.pose.eye_height - 0.81).abs() < 1.0e-6);
        assert_eq!(
            first_person.eye,
            [24_000_000.9, f64::from(first_person.pose.eye_height), 5.25]
        );

        let third_person = frame_camera_from_world_at_partial_tick(
            &world,
            1.0,
            CameraType::ThirdPersonBack,
            eye_height,
        )
        .unwrap();
        assert!((third_person.pose.position[1] - 0.81).abs() < 1.0e-6);
        assert_eq!(third_person.pose.eye_height, 0.0);
        assert_eq!(third_person.eye, [24_000_000.9, first_person.eye[1], 1.25]);
    }
}
