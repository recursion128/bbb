use std::collections::VecDeque;

use bbb_protocol::packets::AdvancementDisplaySummary;
use serde::{Deserialize, Serialize};

use crate::WorldStore;

pub const MAX_CLIENT_TOAST_EVENTS: usize = 4096;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientToastState {
    #[serde(default)]
    pub events: VecDeque<ClientToastEvent>,
    #[serde(default)]
    pub next_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientToastEvent {
    pub sequence: u64,
    pub kind: ClientToastEventKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientToastEventKind {
    Advancement {
        advancement_id: String,
        display: AdvancementDisplaySummary,
    },
}

impl ClientToastEvent {
    pub fn advancement(&self) -> Option<(&str, &AdvancementDisplaySummary)> {
        match &self.kind {
            ClientToastEventKind::Advancement {
                advancement_id,
                display,
            } => Some((advancement_id, display)),
        }
    }
}

impl WorldStore {
    pub fn client_toasts(&self) -> &ClientToastState {
        &self.client_toasts
    }

    pub fn client_toast_events(&self) -> &VecDeque<ClientToastEvent> {
        &self.client_toasts.events
    }

    pub(crate) fn push_advancement_toast_event(
        &mut self,
        advancement_id: String,
        display: AdvancementDisplaySummary,
    ) {
        self.push_client_toast_event(ClientToastEventKind::Advancement {
            advancement_id,
            display,
        });
        self.counters.advancement_toast_events =
            self.counters.advancement_toast_events.saturating_add(1);
    }

    pub(crate) fn reset_client_toasts_for_new_level(&mut self) {
        self.client_toasts = ClientToastState::default();
    }

    fn push_client_toast_event(&mut self, kind: ClientToastEventKind) {
        let sequence = self.client_toasts.next_sequence;
        self.client_toasts.next_sequence = self
            .client_toasts
            .next_sequence
            .checked_add(1)
            .expect("client toast sequence overflow");
        if self.client_toasts.events.len() == MAX_CLIENT_TOAST_EVENTS {
            if matches!(
                self.client_toasts.events.front().map(|event| &event.kind),
                Some(ClientToastEventKind::Advancement { .. })
            ) {
                self.counters.advancement_toast_events_dropped = self
                    .counters
                    .advancement_toast_events_dropped
                    .saturating_add(1);
            }
            self.client_toasts.events.pop_front();
        }
        self.client_toasts
            .events
            .push_back(ClientToastEvent { sequence, kind });
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{
        AdvancementDisplaySummary, AdvancementFrameType, AdvancementIconSummary,
    };

    use super::*;

    #[test]
    fn client_toast_handoff_is_globally_sequenced_bounded_and_level_scoped() {
        let mut store = WorldStore::new();
        let display = advancement_display();

        for index in 0..=MAX_CLIENT_TOAST_EVENTS {
            store.push_advancement_toast_event(format!("minecraft:test/{index}"), display.clone());
        }

        assert_eq!(store.client_toast_events().len(), MAX_CLIENT_TOAST_EVENTS);
        assert_eq!(store.client_toast_events()[0].sequence, 1);
        assert_eq!(
            store.client_toasts().next_sequence,
            u64::try_from(MAX_CLIENT_TOAST_EVENTS + 1).unwrap()
        );
        assert_eq!(store.counters().advancement_toast_events_dropped, 1);

        store.reset_client_toasts_for_new_level();
        assert!(store.client_toast_events().is_empty());
        assert_eq!(store.client_toasts().next_sequence, 0);
        assert_eq!(
            store.counters().advancement_toast_events,
            MAX_CLIENT_TOAST_EVENTS + 1
        );
        assert_eq!(store.counters().advancement_toast_events_dropped, 1);
    }

    fn advancement_display() -> AdvancementDisplaySummary {
        AdvancementDisplaySummary {
            title: bbb_protocol::RichTextComponent::plain("Toast"),
            description: bbb_protocol::RichTextComponent::plain("Description"),
            icon: AdvancementIconSummary {
                item_id: 1,
                count: 1,
                component_patch: Default::default(),
            },
            frame_type: AdvancementFrameType::Task,
            background: None,
            show_toast: true,
            hidden: false,
            x: 0.0,
            y: 0.0,
        }
    }
}
