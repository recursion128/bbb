use bbb_control::{NetCounters, NetVec3, PlayerPose};
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

pub(crate) fn sync_local_player_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.play_logins_received = world_counters.play_logins_received;
    counters.respawns_received = world_counters.respawns_received;
    counters.player_abilities_packets = world_counters.player_abilities_packets;
    counters.player_health_packets = world_counters.player_health_packets;
    counters.player_experience_packets = world_counters.player_experience_packets;
    counters.held_slot_packets = world_counters.held_slot_packets;
    counters.held_slot_updates_applied = world_counters.held_slot_updates_applied;
    counters.held_slot_updates_ignored = world_counters.held_slot_updates_ignored;
    counters.default_spawn_position_packets = world_counters.default_spawn_position_packets;
    counters.simulation_distance_packets = world_counters.simulation_distance_packets;
    counters.set_camera_packets = world_counters.set_camera_packets;
    counters.set_camera_updates_applied = world_counters.set_camera_updates_applied;
    counters.set_camera_updates_ignored = world_counters.set_camera_updates_ignored;
    counters.player_position_packets = world_counters.player_position_packets;
    counters.player_rotation_packets = world_counters.player_rotation_packets;
    counters.player_look_at_packets = world_counters.player_look_at_packets;
}

pub(super) fn apply_player_position_update(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    update: bbb_protocol::packets::PlayerPositionUpdate,
) {
    world.apply_player_position(update);
    sync_local_player_counters(counters, world);
}

pub(super) fn apply_player_rotation_update(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    update: bbb_protocol::packets::PlayerRotationUpdate,
) {
    world.apply_player_rotation(update);
    sync_local_player_counters(counters, world);
}

pub(super) fn apply_player_look_at_update(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    update: bbb_protocol::packets::PlayerLookAt,
) {
    world.apply_player_look_at(update);
    sync_local_player_counters(counters, world);
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
