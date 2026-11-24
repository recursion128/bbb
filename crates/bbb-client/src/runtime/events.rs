#[cfg(test)]
mod client_state;
mod control_state;
mod dispatcher;

pub(crate) use bbb_client_world::LevelEventSoundRandomState;
pub(super) use dispatcher::audio_registry_snapshot_from_world;
#[cfg(test)]
pub(super) use dispatcher::drain_net_events_with_sinks;
pub(crate) use dispatcher::queue_chat_acknowledgement;
pub(super) use dispatcher::{
    dispatch_local_level_event, dispatch_pending_client_particle_spawns,
    drain_net_events_with_sinks_and_debug, NetEventDispatchSinks,
};
#[cfg(test)]
pub(super) use dispatcher::{
    drain_net_events, drain_net_events_with_audio, drain_net_events_with_input,
    drain_net_events_with_server_list,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) use tests::minimal_non_empty_synchronized_registry_data;
