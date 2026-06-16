use bbb_renderer::CameraPose;
use bbb_world::{LocalPlayerPoseState, WorldStore};

pub(crate) fn camera_pose_from_world(world: &WorldStore) -> Option<CameraPose> {
    let camera = world.local_player().camera;
    if let Some(camera_id) = camera.entity_id {
        if !camera.follows_player {
            if let Some(camera_pose) = world.probe_entity_camera_pose(camera_id) {
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
        .local_player_pose()
        .map(camera_pose_from_local_player_pose)
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
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    }
}
