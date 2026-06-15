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

#[cfg(test)]
mod tests;
