use bbb_renderer::{CameraHurtEffect, CameraPose};
use bbb_world::{LocalPlayerPoseState, WorldStore};

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
    let camera_entity_id = world.local_player().camera.entity_id?;
    let entity_type_id = world.entity_type_id(camera_entity_id)?;
    bbb_world::vanilla_entity_camera_post_effect_id(entity_type_id)
}

pub(crate) fn camera_pose_from_world(world: &WorldStore) -> Option<CameraPose> {
    camera_pose_from_world_at_partial_tick(world, 1.0)
}

pub(crate) fn camera_pose_from_world_at_partial_tick(
    world: &WorldStore,
    partial_ticks: f32,
) -> Option<CameraPose> {
    let camera = world.local_player().camera;
    if let Some(camera_id) = camera.entity_id {
        if !camera.follows_player {
            if let Some(camera_pose) =
                world.probe_entity_camera_pose_at_partial_tick(camera_id, partial_ticks)
            {
                return Some(CameraPose {
                    position: [
                        camera_pose.position.x as f32,
                        camera_pose.position.y as f32,
                        camera_pose.position.z as f32,
                    ],
                    y_rot: camera_pose.y_rot,
                    x_rot: camera_pose.x_rot,
                    eye_height: camera_pose.eye_height,
                });
            }
        }
    }

    world
        .local_player_pose_at_partial_tick(partial_ticks)
        .map(camera_pose_from_local_player_pose)
}

pub(crate) fn render_camera_pose_from_world_at_partial_tick(
    world: &WorldStore,
    partial_ticks: f32,
    camera_type: CameraType,
) -> Option<CameraPose> {
    let camera_pose = camera_pose_from_world_at_partial_tick(world, partial_ticks)?;
    if camera_type.is_first_person() {
        return Some(camera_pose);
    }
    let requested_distance = world.camera_entity_third_person_distance().unwrap_or(4.0);
    Some(detached_camera_pose_with_clip(
        camera_pose,
        camera_type,
        requested_distance,
        |from, to| world.camera_visual_block_hit(from, to),
    ))
}

fn detached_camera_pose_with_clip(
    mut camera_pose: CameraPose,
    camera_type: CameraType,
    requested_distance: f32,
    mut clip: impl FnMut([f64; 3], [f64; 3]) -> Option<[f64; 3]>,
) -> CameraPose {
    if camera_type.is_mirrored() {
        camera_pose.y_rot += 180.0;
        camera_pose.x_rot = -camera_pose.x_rot;
    }
    let base = camera_eye_position(camera_pose);
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
        let from = std::array::from_fn(|axis| f64::from(base[axis]) + offset[axis]);
        let to =
            std::array::from_fn(|axis| from[axis] - f64::from(forward[axis]) * f64::from(distance));
        let Some(hit) = clip(from, to) else {
            continue;
        };
        let hit_distance = hit
            .into_iter()
            .zip(base)
            .map(|(hit, base)| (hit - f64::from(base)).powi(2))
            .sum::<f64>()
            .sqrt() as f32;
        if hit_distance < distance {
            distance = hit_distance;
        }
    }
    camera_pose.position = std::array::from_fn(|axis| base[axis] - forward[axis] * distance);
    camera_pose.eye_height = 0.0;
    camera_pose
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

fn camera_pose_from_local_player_pose(player: LocalPlayerPoseState) -> CameraPose {
    CameraPose {
        position: [
            player.position.x as f32,
            player.position.y as f32,
            player.position.z as f32,
        ],
        y_rot: player.y_rot,
        x_rot: player.x_rot,
        eye_height: player.eye_height() as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::Vec3d;
    use bbb_world::LocalPlayerInputState;

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
        assert_eq!(mid.y_rot, 182.0);
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
        let back =
            detached_camera_pose_with_clip(base, CameraType::ThirdPersonBack, 4.0, |_, _| None);
        assert_eq!(back.position, [1.0, 65.5, -2.0]);
        assert_eq!(back.y_rot, 0.0);
        assert_eq!(back.x_rot, 0.0);
        assert_eq!(back.eye_height, 0.0);

        let front =
            detached_camera_pose_with_clip(base, CameraType::ThirdPersonFront, 4.0, |_, _| None);
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
            position: [0.0, 8.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.0,
        };
        let mut rays = Vec::new();
        let clipped =
            detached_camera_pose_with_clip(base, CameraType::ThirdPersonBack, 4.0, |from, to| {
                rays.push((from, to));
                (rays.len() == 1).then_some([0.0, 9.0, -2.0])
            });

        assert_eq!(rays.len(), 8);
        assert_eq!(rays[0].0, [-0.1, 8.9, -0.1]);
        assert_eq!(rays[0].1, [-0.1, 8.9, -4.1]);
        assert_eq!(clipped.position, [0.0, 9.0, -2.0]);
        assert_eq!(rays[1].1[2] - rays[1].0[2], -2.0);
    }
}
