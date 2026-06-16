mod client_state;
mod control_state;
mod dispatcher;

pub(super) use dispatcher::drain_net_events_with_sinks;
#[cfg(test)]
pub(super) use dispatcher::{drain_net_events, drain_net_events_with_audio};

#[cfg(test)]
mod tests;
