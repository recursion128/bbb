use bbb_control::NetCounters;
use bbb_world::WorldStore;

mod client_state;
mod control_state;
mod dispatcher;

pub(crate) use client_state::{
    local_player_pose_from_player_pose, player_pose_from_local_player_pose,
    player_position_state_from_local_player_pose,
};
pub(super) use dispatcher::{drain_net_events, drain_net_events_with_audio};

fn sync_world_time_counters(counters: &mut NetCounters, world: &WorldStore) {
    counters.world_time = world.world_time().map(|time| bbb_control::WorldTime {
        game_time: time.game_time,
        day_time: time.day_time,
        clock_updates: time.clock_updates.len(),
    });
    counters.world_time_packets = world.counters().world_time_packets;
}

fn sync_weather_counters(counters: &mut NetCounters, world: &WorldStore) {
    let weather = world.weather();
    counters.weather.last_game_event_id = weather.last_game_event_id;
    counters.weather.last_game_event_param = weather.last_game_event_param;
    counters.weather.raining = weather.raining;
    counters.weather.rain_level = weather.rain_level;
    counters.weather.thunder_level = weather.thunder_level;
    counters.game_event_packets = world.counters().game_event_packets;
}

#[cfg(test)]
mod tests;
