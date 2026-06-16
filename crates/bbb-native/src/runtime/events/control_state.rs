use bbb_control::NetCounters;
use bbb_net::NetEvent;

pub(super) fn apply_control_projection_event(event: &NetEvent, counters: &mut NetCounters) {
    match event {
        NetEvent::Connected => {
            counters.connected = true;
            counters.last_error = None;
        }
        NetEvent::Disconnected { reason } => {
            counters.connected = false;
            counters.last_error = reason.clone();
        }
        NetEvent::StateChanged { state } => {
            counters.state = Some(format!("{state:?}"));
        }
        NetEvent::CompressionSet { threshold } => {
            counters.compression_threshold = Some(*threshold);
        }
        NetEvent::PacketSeen { .. } => {
            counters.packets_seen += 1;
        }
        NetEvent::UnsupportedPacket {
            state,
            packet_id,
            len,
        } => {
            counters.unsupported_packets += 1;
            counters.last_unsupported_packet_state = Some(format!("{state:?}"));
            counters.last_unsupported_packet_id = Some(*packet_id);
            counters.last_unsupported_packet_len = Some(*len);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_projection_updates_runtime_status_without_consuming_events() {
        let mut counters = NetCounters::default();

        apply_control_projection_event(&NetEvent::Connected, &mut counters);
        assert!(counters.connected);
        assert!(counters.last_error.is_none());

        let state_changed = NetEvent::StateChanged {
            state: bbb_net::ConnectionState::Play,
        };
        apply_control_projection_event(&state_changed, &mut counters);
        assert!(matches!(
            state_changed,
            NetEvent::StateChanged {
                state: bbb_net::ConnectionState::Play,
            }
        ));
        assert_eq!(counters.state.as_deref(), Some("Play"));

        apply_control_projection_event(&NetEvent::CompressionSet { threshold: 256 }, &mut counters);
        assert_eq!(counters.compression_threshold, Some(256));

        apply_control_projection_event(
            &NetEvent::PacketSeen {
                state: bbb_net::ConnectionState::Play,
                packet_id: 0x24,
                len: 17,
            },
            &mut counters,
        );
        assert_eq!(counters.packets_seen, 1);

        apply_control_projection_event(
            &NetEvent::UnsupportedPacket {
                state: bbb_net::ConnectionState::Play,
                packet_id: 0x7f,
                len: 12,
            },
            &mut counters,
        );
        assert_eq!(counters.unsupported_packets, 1);
        assert_eq!(
            counters.last_unsupported_packet_state.as_deref(),
            Some("Play")
        );
        assert_eq!(counters.last_unsupported_packet_id, Some(0x7f));
        assert_eq!(counters.last_unsupported_packet_len, Some(12));

        apply_control_projection_event(
            &NetEvent::Disconnected {
                reason: Some("done".to_string()),
            },
            &mut counters,
        );
        assert!(!counters.connected);
        assert_eq!(counters.last_error.as_deref(), Some("done"));

        apply_control_projection_event(&NetEvent::ResetChat, &mut counters);
        assert_eq!(counters.packets_seen, 1);
    }
}
