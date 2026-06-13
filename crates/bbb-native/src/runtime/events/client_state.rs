use bbb_control::{
    ActionBarText, CameraState, DefaultSpawn, NetCounters, NetVec3, PlayerAbilities,
    PlayerExperience, PlayerHealth, PlayerPose, SystemChatLine,
};
use bbb_protocol::packets::PlayerPositionState;
use bbb_world::{BlockPos, WorldStore};

pub(super) fn apply_player_abilities_update(
    counters: &mut NetCounters,
    abilities: bbb_protocol::packets::PlayerAbilities,
) {
    counters.player_abilities = Some(PlayerAbilities {
        invulnerable: abilities.invulnerable,
        flying: abilities.flying,
        can_fly: abilities.can_fly,
        instabuild: abilities.instabuild,
        flying_speed: abilities.flying_speed,
        walking_speed: abilities.walking_speed,
    });
    counters.player_abilities_packets += 1;
}

pub(super) fn apply_default_spawn_update(
    counters: &mut NetCounters,
    spawn: bbb_protocol::packets::SetDefaultSpawnPosition,
) {
    counters.default_spawn = Some(DefaultSpawn {
        dimension: spawn.dimension,
        pos: BlockPos {
            x: spawn.pos.x,
            y: spawn.pos.y,
            z: spawn.pos.z,
        },
        yaw: spawn.yaw,
        pitch: spawn.pitch,
    });
    counters.default_spawn_position_packets += 1;
}

pub(super) fn apply_simulation_distance_update(
    counters: &mut NetCounters,
    distance: bbb_protocol::packets::SetSimulationDistance,
) {
    counters.simulation_distance = Some(distance.distance);
    counters.simulation_distance_packets += 1;
}

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

pub(super) fn apply_set_camera_update(
    counters: &mut NetCounters,
    world: &WorldStore,
    camera: bbb_protocol::packets::SetCamera,
) {
    counters.set_camera_packets += 1;
    let follows_player = counters.player_entity_id == Some(camera.camera_id);
    if follows_player || world.probe_entity(camera.camera_id).is_some() {
        counters.camera = CameraState {
            entity_id: Some(camera.camera_id),
            follows_player,
            entity_known: true,
        };
    }
}

pub(super) fn apply_player_health_update(
    counters: &mut NetCounters,
    health: bbb_protocol::packets::PlayerHealth,
) {
    counters.player_health = Some(PlayerHealth {
        health: health.health,
        food: health.food,
        saturation: health.saturation,
    });
    counters.player_health_packets += 1;
}

pub(super) fn apply_player_experience_update(
    counters: &mut NetCounters,
    experience: bbb_protocol::packets::PlayerExperience,
) {
    counters.player_experience = Some(PlayerExperience {
        progress: experience.progress,
        level: experience.level,
        total: experience.total,
    });
    counters.player_experience_packets += 1;
}

pub(super) fn apply_held_slot_update(
    counters: &mut NetCounters,
    slot: bbb_protocol::packets::SetHeldSlot,
) {
    if (0..=8).contains(&slot.slot) {
        counters.selected_hotbar_slot = slot.slot as u8;
    }
    counters.held_slot_packets += 1;
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

pub(crate) fn player_position_state_from_pose(player: PlayerPose) -> PlayerPositionState {
    PlayerPositionState {
        position: protocol_vec3_from_net(player.position),
        delta_movement: protocol_vec3_from_net(player.delta_movement),
        y_rot: player.y_rot,
        x_rot: player.x_rot,
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
