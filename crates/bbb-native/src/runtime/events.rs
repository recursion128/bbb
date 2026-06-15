use bbb_control::NetCounters;
use bbb_world::WorldStore;

mod client_state;
mod control_state;
mod dispatcher;

pub(crate) use client_state::{
    local_player_pose_from_player_pose, player_pose_from_local_player_pose,
    player_position_state_from_local_player_pose, sync_local_player_counters,
};
pub(super) use dispatcher::drain_net_events_with_sinks;
#[cfg(test)]
pub(super) use dispatcher::{drain_net_events, drain_net_events_with_audio};

fn sync_world_time_counters(counters: &mut NetCounters, world: &WorldStore) {
    counters.world_time_packets = world.counters().world_time_packets;
}

fn sync_weather_counters(counters: &mut NetCounters, world: &WorldStore) {
    counters.game_event_packets = world.counters().game_event_packets;
}

#[cfg(test)]
mod tests;
