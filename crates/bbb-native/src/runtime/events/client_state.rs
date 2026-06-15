use bbb_control::{NetVec3, PlayerPose};
use bbb_protocol::packets::PlayerPositionState;
use bbb_world::{LocalPlayerPoseState, WorldStore};

pub(super) fn apply_system_chat_update(
    world: &mut WorldStore,
    chat: bbb_protocol::packets::SystemChat,
) {
    world.apply_system_chat(chat);
}

pub(super) fn apply_action_bar_update(
    world: &mut WorldStore,
    text: bbb_protocol::packets::SetActionBarText,
) {
    world.apply_action_bar_text(text);
}

pub(super) fn apply_title_text_update(
    world: &mut WorldStore,
    text: bbb_protocol::packets::SetTitleText,
) {
    world.apply_title_text(text);
}

pub(super) fn apply_subtitle_text_update(
    world: &mut WorldStore,
    text: bbb_protocol::packets::SetSubtitleText,
) {
    world.apply_subtitle_text(text);
}

pub(super) fn apply_clear_titles_update(
    world: &mut WorldStore,
    clear: bbb_protocol::packets::ClearTitles,
) {
    world.apply_clear_titles(clear);
}

pub(super) fn apply_titles_animation_update(
    world: &mut WorldStore,
    animation: bbb_protocol::packets::SetTitlesAnimation,
) {
    world.apply_titles_animation(animation);
}

pub(super) fn apply_player_position_update(
    world: &mut WorldStore,
    update: bbb_protocol::packets::PlayerPositionUpdate,
) {
    world.apply_player_position(update);
}

pub(super) fn apply_player_rotation_update(
    world: &mut WorldStore,
    update: bbb_protocol::packets::PlayerRotationUpdate,
) {
    world.apply_player_rotation(update);
}

pub(super) fn apply_player_look_at_update(
    world: &mut WorldStore,
    update: bbb_protocol::packets::PlayerLookAt,
) {
    world.apply_player_look_at(update);
}

pub(crate) fn player_position_state_from_local_player_pose(
    player: LocalPlayerPoseState,
) -> PlayerPositionState {
    player.position_state()
}

pub(crate) fn player_pose_from_local_player_pose(player: LocalPlayerPoseState) -> PlayerPose {
    PlayerPose {
        position: net_vec3_from_protocol(player.position),
        delta_movement: net_vec3_from_protocol(player.delta_movement),
        y_rot: player.y_rot,
        x_rot: player.x_rot,
        last_teleport_id: player.last_teleport_id,
    }
}

pub(crate) fn local_player_pose_from_player_pose(player: PlayerPose) -> LocalPlayerPoseState {
    LocalPlayerPoseState {
        position: protocol_vec3_from_net(player.position),
        delta_movement: protocol_vec3_from_net(player.delta_movement),
        y_rot: player.y_rot,
        x_rot: player.x_rot,
        last_teleport_id: player.last_teleport_id,
    }
}

fn protocol_vec3_from_net(vec: NetVec3) -> bbb_protocol::packets::Vec3d {
    bbb_protocol::packets::Vec3d {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

fn net_vec3_from_protocol(vec: bbb_protocol::packets::Vec3d) -> NetVec3 {
    NetVec3 {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

#[cfg(test)]
mod tests;
