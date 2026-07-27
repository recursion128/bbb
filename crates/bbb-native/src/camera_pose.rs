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
}
