use bbb_control::{
    ActionBarText, CameraState, DefaultSpawn, NetCounters, NetVec3, PlayerAbilities,
    PlayerExperience, PlayerHealth, PlayerLookAtState, PlayerPose, SystemChatLine,
};
use bbb_protocol::packets::PlayerPositionState;
use bbb_world::WorldStore;

const STANDING_EYE_HEIGHT: f64 = 1.62;

pub(super) fn apply_system_chat_update(
    counters: &mut NetCounters,
    chat: bbb_protocol::packets::SystemChat,
) {
    counters.last_system_chat = Some(SystemChatLine {
        content: chat.content,
        overlay: chat.overlay,
    });
    counters.system_chat_packets += 1;
}

pub(super) fn apply_action_bar_update(
    counters: &mut NetCounters,
    text: bbb_protocol::packets::SetActionBarText,
) {
    counters.last_action_bar = Some(ActionBarText {
        content: text.content,
        display_ticks: 60,
    });
    counters.action_bar_packets += 1;
}

pub(super) fn apply_title_text_update(
    counters: &mut NetCounters,
    text: bbb_protocol::packets::SetTitleText,
) {
    counters.title.title = Some(text.content);
    counters.title.title_time = title_total_ticks(&counters.title);
    counters.title_text_packets += 1;
}

pub(super) fn apply_subtitle_text_update(
    counters: &mut NetCounters,
    text: bbb_protocol::packets::SetSubtitleText,
) {
    counters.title.subtitle = Some(text.content);
    counters.subtitle_text_packets += 1;
}

pub(super) fn apply_clear_titles_update(
    counters: &mut NetCounters,
    clear: bbb_protocol::packets::ClearTitles,
) {
    counters.title.title = None;
    counters.title.subtitle = None;
    counters.title.title_time = 0;
    if clear.reset_times {
        let defaults = bbb_control::TitleState::default();
        counters.title.fade_in = defaults.fade_in;
        counters.title.stay = defaults.stay;
        counters.title.fade_out = defaults.fade_out;
    }
    counters.clear_titles_packets += 1;
}

pub(super) fn apply_titles_animation_update(
    counters: &mut NetCounters,
    animation: bbb_protocol::packets::SetTitlesAnimation,
) {
    if animation.fade_in >= 0 {
        counters.title.fade_in = animation.fade_in;
    }
    if animation.stay >= 0 {
        counters.title.stay = animation.stay;
    }
    if animation.fade_out >= 0 {
        counters.title.fade_out = animation.fade_out;
    }
    if counters.title.title_time > 0 {
        counters.title.title_time = title_total_ticks(&counters.title);
    }
    counters.titles_animation_packets += 1;
}

fn title_total_ticks(title: &bbb_control::TitleState) -> i32 {
    title
        .fade_in
        .saturating_add(title.stay)
        .saturating_add(title.fade_out)
}

pub(super) fn apply_ticking_state_update(
    counters: &mut NetCounters,
    ticking: bbb_protocol::packets::TickingState,
) {
    counters.ticking.tick_rate = ticking.clamped_tick_rate();
    counters.ticking.frozen = ticking.frozen;
    counters.ticking_state_packets += 1;
}

pub(super) fn apply_ticking_step_update(
    counters: &mut NetCounters,
    step: bbb_protocol::packets::TickingStep,
) {
    counters.ticking.frozen_ticks_to_run = step.tick_steps;
    counters.ticking_step_packets += 1;
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

    let world_counters = world.counters();
    counters.player_abilities_packets = world_counters.player_abilities_packets;
    counters.player_health_packets = world_counters.player_health_packets;
    counters.player_experience_packets = world_counters.player_experience_packets;
    counters.held_slot_packets = world_counters.held_slot_packets;
    counters.default_spawn_position_packets = world_counters.default_spawn_position_packets;
    counters.simulation_distance_packets = world_counters.simulation_distance_packets;
    counters.set_camera_packets = world_counters.set_camera_packets;
}

pub(super) fn apply_player_position_update(
    counters: &mut NetCounters,
    update: bbb_protocol::packets::PlayerPositionUpdate,
) {
    let current = counters
        .player_pose
        .map(player_position_state_from_pose)
        .unwrap_or_default();
    let state = update.apply_to_state(current);

    counters.player_pose = Some(PlayerPose {
        position: net_vec3_from_protocol(state.position),
        delta_movement: net_vec3_from_protocol(state.delta_movement),
        y_rot: state.y_rot,
        x_rot: state.x_rot,
        last_teleport_id: update.id,
    });
    counters.player_position_packets += 1;
}

pub(super) fn apply_player_rotation_update(
    counters: &mut NetCounters,
    update: bbb_protocol::packets::PlayerRotationUpdate,
) {
    let current = counters
        .player_pose
        .map(player_position_state_from_pose)
        .unwrap_or_default();
    let state = update.apply_to_state(current);
    let last_teleport_id = counters
        .player_pose
        .map(|pose| pose.last_teleport_id)
        .unwrap_or_default();

    counters.player_pose = Some(PlayerPose {
        position: net_vec3_from_protocol(state.position),
        delta_movement: net_vec3_from_protocol(state.delta_movement),
        y_rot: state.y_rot,
        x_rot: state.x_rot,
        last_teleport_id,
    });
    counters.player_rotation_packets += 1;
}

pub(super) fn apply_player_look_at_update(
    counters: &mut NetCounters,
    update: bbb_protocol::packets::PlayerLookAt,
) {
    counters.last_player_look_at = Some(PlayerLookAtState {
        from_anchor: update.from_anchor.as_str().to_string(),
        position: net_vec3_from_protocol(update.position),
        target_entity_id: update.target.map(|target| target.entity_id),
        to_anchor: update
            .target
            .map(|target| target.to_anchor.as_str().to_string()),
    });

    if let Some(pose) = counters.player_pose {
        counters.player_pose = Some(apply_look_at_to_pose(pose, update));
    }

    counters.player_look_at_packets += 1;
}

pub(crate) fn player_position_state_from_pose(player: PlayerPose) -> PlayerPositionState {
    PlayerPositionState {
        position: protocol_vec3_from_net(player.position),
        delta_movement: protocol_vec3_from_net(player.delta_movement),
        y_rot: player.y_rot,
        x_rot: player.x_rot,
    }
}

fn apply_look_at_to_pose(
    pose: PlayerPose,
    update: bbb_protocol::packets::PlayerLookAt,
) -> PlayerPose {
    let from_y = match update.from_anchor {
        bbb_protocol::packets::EntityAnchor::Feet => pose.position.y,
        bbb_protocol::packets::EntityAnchor::Eyes => pose.position.y + STANDING_EYE_HEIGHT,
    };
    let dx = update.position.x - pose.position.x;
    let dy = update.position.y - from_y;
    let dz = update.position.z - pose.position.z;
    let horizontal = (dx * dx + dz * dz).sqrt();
    let x_rot = wrap_degrees_f32(-(dy.atan2(horizontal).to_degrees() as f32));
    let y_rot = wrap_degrees_f32(dz.atan2(dx).to_degrees() as f32 - 90.0);

    PlayerPose {
        position: pose.position,
        delta_movement: pose.delta_movement,
        y_rot,
        x_rot,
        last_teleport_id: pose.last_teleport_id,
    }
}

fn wrap_degrees_f32(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
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
