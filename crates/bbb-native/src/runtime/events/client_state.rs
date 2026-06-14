use bbb_control::{
    ActionBarText, CameraState, DefaultSpawn, NetCounters, NetVec3, PlayerAbilities,
    PlayerExperience, PlayerHealth, PlayerLookAtState, PlayerPose, SystemChatLine,
};
use bbb_protocol::packets::PlayerPositionState;
use bbb_world::{LocalPlayerLookAtState, LocalPlayerPoseState, WorldStore};

pub(super) fn apply_system_chat_update(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    chat: bbb_protocol::packets::SystemChat,
) {
    world.apply_system_chat(chat);
    sync_hud_text_counters(counters, world);
}

pub(super) fn apply_action_bar_update(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    text: bbb_protocol::packets::SetActionBarText,
) {
    world.apply_action_bar_text(text);
    sync_hud_text_counters(counters, world);
}

pub(super) fn apply_title_text_update(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    text: bbb_protocol::packets::SetTitleText,
) {
    world.apply_title_text(text);
    sync_hud_text_counters(counters, world);
}

pub(super) fn apply_subtitle_text_update(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    text: bbb_protocol::packets::SetSubtitleText,
) {
    world.apply_subtitle_text(text);
    sync_hud_text_counters(counters, world);
}

pub(super) fn apply_clear_titles_update(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    clear: bbb_protocol::packets::ClearTitles,
) {
    world.apply_clear_titles(clear);
    sync_hud_text_counters(counters, world);
}

pub(super) fn apply_titles_animation_update(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    animation: bbb_protocol::packets::SetTitlesAnimation,
) {
    world.apply_titles_animation(animation);
    sync_hud_text_counters(counters, world);
}

pub(super) fn sync_hud_text_counters(counters: &mut NetCounters, world: &WorldStore) {
    let hud = world.client_hud();
    counters.last_system_chat = hud.system_chat.as_ref().map(|line| SystemChatLine {
        content: line.content.clone(),
        overlay: line.overlay,
    });
    counters.last_action_bar = hud.action_bar.as_ref().map(|action_bar| ActionBarText {
        content: action_bar.content.clone(),
        display_ticks: action_bar.display_ticks,
    });
    counters.title = bbb_control::TitleState {
        title: hud.title.title.clone(),
        subtitle: hud.title.subtitle.clone(),
        fade_in: hud.title.fade_in,
        stay: hud.title.stay,
        fade_out: hud.title.fade_out,
        title_time: hud.title.title_time,
    };

    let world_counters = world.counters();
    counters.system_chat_packets = world_counters.system_chat_packets;
    counters.action_bar_packets = world_counters.action_bar_packets;
    counters.title_text_packets = world_counters.title_text_packets;
    counters.subtitle_text_packets = world_counters.subtitle_text_packets;
    counters.clear_titles_packets = world_counters.clear_titles_packets;
    counters.titles_animation_packets = world_counters.titles_animation_packets;
}

pub(super) fn sync_ticking_counters(counters: &mut NetCounters, world: &WorldStore) {
    let ticking = world.ticking();
    counters.ticking.tick_rate = ticking.tick_rate;
    counters.ticking.frozen = ticking.frozen;
    counters.ticking.frozen_ticks_to_run = ticking.frozen_ticks_to_run;
    let world_counters = world.counters();
    counters.ticking_state_packets = world_counters.ticking_state_packets;
    counters.ticking_step_packets = world_counters.ticking_step_packets;
}

pub(super) fn sync_local_player_counters(counters: &mut NetCounters, world: &WorldStore) {
    let local = world.local_player();
    counters.player_entity_id = world.local_player_id();
    counters.player_abilities = local.abilities.map(|abilities| PlayerAbilities {
        invulnerable: abilities.invulnerable,
        flying: abilities.flying,
        can_fly: abilities.can_fly,
        instabuild: abilities.instabuild,
        flying_speed: abilities.flying_speed,
        walking_speed: abilities.walking_speed,
    });
    counters.player_health = local.health.map(|health| PlayerHealth {
        health: health.health,
        food: health.food,
        saturation: health.saturation,
    });
    counters.player_experience = local.experience.map(|experience| PlayerExperience {
        progress: experience.progress,
        level: experience.level,
        total: experience.total,
    });
    counters.selected_hotbar_slot = local.selected_hotbar_slot;
    counters.default_spawn = local.default_spawn.as_ref().map(|spawn| DefaultSpawn {
        dimension: spawn.dimension.clone(),
        pos: spawn.pos,
        yaw: spawn.yaw,
        pitch: spawn.pitch,
    });
    counters.simulation_distance = local.simulation_distance;
    counters.camera = CameraState {
        entity_id: local.camera.entity_id,
        follows_player: local.camera.follows_player,
        entity_known: local.camera.entity_known,
    };
    counters.player_pose = local.pose.map(player_pose_from_local_player_pose);
    counters.last_player_look_at = local.last_look_at.map(control_player_look_at);

    let world_counters = world.counters();
    counters.play_logins_received = world_counters.play_logins_received;
    counters.respawns_received = world_counters.respawns_received;
    counters.player_abilities_packets = world_counters.player_abilities_packets;
    counters.player_health_packets = world_counters.player_health_packets;
    counters.player_experience_packets = world_counters.player_experience_packets;
    counters.held_slot_packets = world_counters.held_slot_packets;
    counters.default_spawn_position_packets = world_counters.default_spawn_position_packets;
    counters.simulation_distance_packets = world_counters.simulation_distance_packets;
    counters.set_camera_packets = world_counters.set_camera_packets;
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

fn control_player_look_at(look_at: LocalPlayerLookAtState) -> PlayerLookAtState {
    PlayerLookAtState {
        from_anchor: look_at.from_anchor.as_str().to_string(),
        position: net_vec3_from_protocol(look_at.position),
        target_entity_id: look_at.target_entity_id,
        to_anchor: look_at.to_anchor.map(|anchor| anchor.as_str().to_string()),
    }
}

#[cfg(test)]
mod tests;
