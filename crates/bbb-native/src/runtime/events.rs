use bbb_control::NetCounters;

mod client_state;
mod control_state;
mod dispatcher;

pub(crate) use client_state::player_position_state_from_pose;
pub(super) use dispatcher::drain_net_events;

fn apply_world_time_update(counters: &mut NetCounters, time: bbb_protocol::packets::PlayTime) {
    let day_time = time
        .clock_updates
        .first()
        .map(|clock| clock.total_ticks)
        .unwrap_or(time.game_time);
    counters.world_time = Some(bbb_control::WorldTime {
        game_time: time.game_time,
        day_time,
        clock_updates: time.clock_updates.len(),
    });
    counters.world_time_packets += 1;
}

fn apply_game_event(counters: &mut NetCounters, event: bbb_protocol::packets::GameEvent) {
    counters.weather.last_game_event_id = Some(event.event_id);
    counters.weather.last_game_event_param = event.param;
    counters.game_event_packets += 1;

    match event.event_id {
        1 => {
            counters.weather.raining = true;
            counters.weather.rain_level = counters.weather.rain_level.max(1.0);
        }
        2 => {
            counters.weather.raining = false;
            counters.weather.rain_level = 0.0;
            counters.weather.thunder_level = 0.0;
        }
        7 => {
            counters.weather.rain_level = event.param.clamp(0.0, 1.0);
            counters.weather.raining = counters.weather.rain_level > 0.0;
        }
        8 => {
            counters.weather.thunder_level = event.param.clamp(0.0, 1.0);
        }
        _ => {}
    }
}

fn apply_block_changed_ack(
    counters: &mut NetCounters,
    ack: bbb_protocol::packets::BlockChangedAck,
) {
    counters.block_changed_ack_packets += 1;
    counters.last_block_changed_ack_sequence = Some(ack.sequence);
}

#[cfg(test)]
mod tests;
