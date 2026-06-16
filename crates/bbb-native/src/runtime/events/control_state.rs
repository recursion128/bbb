use bbb_control::NetCounters;
use bbb_net::NetEvent;

pub(super) fn apply_control_projection_event(
    event: NetEvent,
    counters: &mut NetCounters,
) -> Option<NetEvent> {
    match event {
        NetEvent::Connected => {
            counters.connected = true;
            counters.last_error = None;
        }
        NetEvent::Disconnected { reason } => {
            counters.connected = false;
            counters.last_error = reason;
        }
        NetEvent::StateChanged { state } => {
            counters.state = Some(format!("{state:?}"));
        }
        NetEvent::CompressionSet { threshold } => {
            counters.compression_threshold = Some(threshold);
        }
        NetEvent::PacketSeen { .. } => {
            counters.packets_seen += 1;
        }
        other => return Some(other),
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_projection_consumes_only_runtime_status_events() {
        let mut counters = NetCounters::default();

        assert!(apply_control_projection_event(NetEvent::Connected, &mut counters).is_none());
        assert!(counters.connected);
        assert!(counters.last_error.is_none());

        assert!(apply_control_projection_event(
            NetEvent::StateChanged {
                state: bbb_net::ConnectionState::Play,
            },
            &mut counters,
        )
        .is_none());
        assert_eq!(counters.state.as_deref(), Some("Play"));

        assert!(apply_control_projection_event(
            NetEvent::CompressionSet { threshold: 256 },
            &mut counters,
        )
        .is_none());
        assert_eq!(counters.compression_threshold, Some(256));

        assert!(apply_control_projection_event(
            NetEvent::PacketSeen {
                state: bbb_net::ConnectionState::Play,
                packet_id: 0x24,
                len: 17,
            },
            &mut counters,
        )
        .is_none());
        assert_eq!(counters.packets_seen, 1);

        assert!(apply_control_projection_event(
            NetEvent::Disconnected {
                reason: Some("done".to_string()),
            },
            &mut counters,
        )
        .is_none());
        assert!(!counters.connected);
        assert_eq!(counters.last_error.as_deref(), Some("done"));

        let world_event = apply_control_projection_event(NetEvent::ResetChat, &mut counters);
        assert!(matches!(world_event, Some(NetEvent::ResetChat)));
    }
}
