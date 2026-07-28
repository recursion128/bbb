use std::collections::VecDeque;

use bbb_protocol::packets::{AdvancementDisplaySummary, ItemStackSummary};
use serde::{Deserialize, Serialize};

use crate::{TutorialToastKind, WorldStore};

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
    Recipe {
        recipe_id: i32,
        category_item: ItemStackSummary,
        unlocked_item: ItemStackSummary,
    },
    Tutorial {
        toast_id: u64,
        kind: TutorialToastKind,
    },
}

impl ClientToastEvent {
    pub fn advancement(&self) -> Option<(&str, &AdvancementDisplaySummary)> {
        match &self.kind {
            ClientToastEventKind::Advancement {
                advancement_id,
                display,
            } => Some((advancement_id, display)),
            ClientToastEventKind::Recipe { .. } | ClientToastEventKind::Tutorial { .. } => None,
        }
    }

    pub fn recipe(&self) -> Option<(i32, &ItemStackSummary, &ItemStackSummary)> {
        match &self.kind {
            ClientToastEventKind::Recipe {
                recipe_id,
                category_item,
                unlocked_item,
            } => Some((*recipe_id, category_item, unlocked_item)),
            ClientToastEventKind::Advancement { .. } | ClientToastEventKind::Tutorial { .. } => {
                None
            }
        }
    }

    pub fn tutorial(&self) -> Option<(u64, TutorialToastKind)> {
        match self.kind {
            ClientToastEventKind::Tutorial { toast_id, kind } => Some((toast_id, kind)),
            ClientToastEventKind::Advancement { .. } | ClientToastEventKind::Recipe { .. } => None,
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

    pub(crate) fn push_recipe_toast_event(
        &mut self,
        recipe_id: i32,
        category_item: ItemStackSummary,
        unlocked_item: ItemStackSummary,
    ) {
        self.push_client_toast_event(ClientToastEventKind::Recipe {
            recipe_id,
            category_item,
            unlocked_item,
        });
    }

    pub(crate) fn push_tutorial_toast_event(&mut self, toast_id: u64, kind: TutorialToastKind) {
        self.push_client_toast_event(ClientToastEventKind::Tutorial { toast_id, kind });
        self.counters.tutorial_toast_events = self.counters.tutorial_toast_events.saturating_add(1);
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
            match self.client_toasts.events.front().map(|event| &event.kind) {
                Some(ClientToastEventKind::Advancement { .. }) => {
                    self.counters.advancement_toast_events_dropped = self
                        .counters
                        .advancement_toast_events_dropped
                        .saturating_add(1);
                }
                Some(ClientToastEventKind::Recipe { .. }) => {
                    self.counters.recipe_book_notification_events_dropped = self
                        .counters
                        .recipe_book_notification_events_dropped
                        .saturating_add(1);
                }
                Some(ClientToastEventKind::Tutorial { .. }) => {
                    self.counters.tutorial_toast_events_dropped = self
                        .counters
                        .tutorial_toast_events_dropped
                        .saturating_add(1);
                }
                None => {}
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

        store.push_recipe_toast_event(7, ItemStackSummary::empty(), ItemStackSummary::empty());
        for index in 0..MAX_CLIENT_TOAST_EVENTS {
            store.push_advancement_toast_event(
                format!("minecraft:test/after_reset_{index}"),
                display.clone(),
            );
        }
        assert_eq!(store.counters().recipe_book_notification_events_dropped, 1);
        assert_eq!(store.counters().advancement_toast_events_dropped, 1);
    }

    #[test]
    fn advancement_and_recipe_events_share_one_packet_order() {
        let mut store = WorldStore::new();
        store.push_advancement_toast_event(
            "minecraft:test/first".to_string(),
            advancement_display(),
        );
        store.push_recipe_toast_event(
            7,
            ItemStackSummary {
                item_id: Some(1),
                count: 1,
                component_patch: Default::default(),
            },
            ItemStackSummary {
                item_id: Some(2),
                count: 1,
                component_patch: Default::default(),
            },
        );
        store
            .push_advancement_toast_event("minecraft:test/last".to_string(), advancement_display());

        assert_eq!(
            store
                .client_toast_events()
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            store.client_toast_events()[0]
                .advancement()
                .expect("advancement")
                .0,
            "minecraft:test/first"
        );
        assert_eq!(
            store.client_toast_events()[1].recipe().expect("recipe").0,
            7
        );
        assert_eq!(
            store.client_toast_events()[2]
                .advancement()
                .expect("advancement")
                .0,
            "minecraft:test/last"
        );
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
