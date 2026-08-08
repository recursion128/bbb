use std::collections::VecDeque;

use bbb_protocol::packets::{AdvancementDisplaySummary, ItemStackSummary};
use serde::{Deserialize, Serialize};

use crate::{ResourcePackDownloadState, TutorialToastKind, WorldStore};

pub const MAX_CLIENT_TOAST_EVENTS: usize = 4096;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientToastState {
    #[serde(default)]
    pub events: VecDeque<ClientToastEvent>,
    #[serde(default)]
    pub next_sequence: u64,
    #[serde(default)]
    pub generation: u64,
    /// Canonical handoff for Vanilla's independent `NowPlayingToast` instance.
    /// This is deliberately not part of the bounded five-slot event queue:
    /// `ToastManager.clear()` leaves the music instance intact.
    #[serde(default)]
    pub now_playing: ClientNowPlayingToastState,
    /// Vanilla `Options.joinedFirstServer`, consumed by the only production
    /// owner that creates a duration-bound `TutorialToast`.
    #[serde(default)]
    pub joined_first_server: bool,
    /// Retains Minecraft's `socialInteractionsToast` handle even after its
    /// natural timeout so the key path can issue the same idempotent hide.
    #[serde(default)]
    pub social_interactions_tutorial_owned: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientNowPlayingToastState {
    /// Actual resolved sound resource selected by `SoundManager.play`, not the
    /// music event id. `NowPlayingToast` derives its translation key from this
    /// identity in Vanilla.
    #[serde(default)]
    pub sound_name: Option<String>,
    #[serde(default)]
    pub wanted_visible: bool,
    #[serde(default)]
    pub mutation_sequence: u64,
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
    Tutorial(ClientTutorialToastEvent),
    System(ClientSystemToastEvent),
}

/// Stable identity for a tutorial toast independent of the shared queue's
/// sequence. Step-owned ids may overlap the one process-owned social toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientTutorialToastToken {
    Step(u64),
    SocialInteractions,
}

/// Ordered mutations for Vanilla `TutorialToast` objects. Lifetime and slide
/// state remain native because both are measured in wall-clock milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientTutorialToastEvent {
    Show {
        token: ClientTutorialToastToken,
        kind: TutorialToastKind,
        time_to_display_ms: u32,
    },
    Hide {
        token: ClientTutorialToastToken,
    },
}

/// One call-site-ordered system-toast mutation.  The native ToastManager uses
/// the variant to apply Vanilla's per-owner `add` versus `addOrUpdate`
/// semantics without reconstructing an order from unrelated counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientSystemToastEvent {
    NarratorToggle {
        enabled: bool,
        status_name: String,
    },
    InsecureServerWarning,
    ResourcePackLoadFailure,
    LowDiskSpace,
    ResourcePackDownload {
        session_generation: u64,
        state: ResourcePackDownloadState,
    },
}

impl ClientToastEvent {
    pub fn advancement(&self) -> Option<(&str, &AdvancementDisplaySummary)> {
        match &self.kind {
            ClientToastEventKind::Advancement {
                advancement_id,
                display,
            } => Some((advancement_id, display)),
            ClientToastEventKind::Recipe { .. }
            | ClientToastEventKind::Tutorial(_)
            | ClientToastEventKind::System(_) => None,
        }
    }

    pub fn recipe(&self) -> Option<(i32, &ItemStackSummary, &ItemStackSummary)> {
        match &self.kind {
            ClientToastEventKind::Recipe {
                recipe_id,
                category_item,
                unlocked_item,
            } => Some((*recipe_id, category_item, unlocked_item)),
            ClientToastEventKind::Advancement { .. }
            | ClientToastEventKind::Tutorial(_)
            | ClientToastEventKind::System(_) => None,
        }
    }

    pub fn tutorial(&self) -> Option<ClientTutorialToastEvent> {
        match self.kind {
            ClientToastEventKind::Tutorial(event) => Some(event),
            ClientToastEventKind::Advancement { .. }
            | ClientToastEventKind::Recipe { .. }
            | ClientToastEventKind::System(_) => None,
        }
    }

    pub fn system(&self) -> Option<&ClientSystemToastEvent> {
        match &self.kind {
            ClientToastEventKind::System(event) => Some(event),
            ClientToastEventKind::Advancement { .. }
            | ClientToastEventKind::Recipe { .. }
            | ClientToastEventKind::Tutorial(_) => None,
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

    pub fn now_playing_toast(&self) -> &ClientNowPlayingToastState {
        &self.client_toasts.now_playing
    }

    /// Publish a successfully started music instance and reset the independent
    /// Now Playing toast animation, matching `ToastManager.showNowPlayingToast`.
    pub fn show_now_playing_music(&mut self, sound_name: String) {
        let state = &mut self.client_toasts.now_playing;
        state.sound_name = Some(sound_name);
        state.wanted_visible = true;
        state.mutation_sequence = state
            .mutation_sequence
            .checked_add(1)
            .expect("now-playing mutation sequence overflow");
    }

    /// Natural completion removes `MusicManager.currentMusic`, so extraction
    /// stops drawing the song without forcing the toast visibility state.
    pub fn clear_now_playing_music(&mut self) {
        self.client_toasts.now_playing.sound_name = None;
    }

    /// `MusicManager.stopPlaying()` explicitly asks the independent toast to
    /// slide out in addition to clearing the current sound identity.
    pub fn hide_now_playing_music(&mut self) {
        let state = &mut self.client_toasts.now_playing;
        state.sound_name = None;
        state.wanted_visible = false;
        state.mutation_sequence = state
            .mutation_sequence
            .checked_add(1)
            .expect("now-playing mutation sequence overflow");
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
        self.push_client_toast_event(ClientToastEventKind::Tutorial(
            ClientTutorialToastEvent::Show {
                token: ClientTutorialToastToken::Step(toast_id),
                kind,
                time_to_display_ms: 0,
            },
        ));
        self.counters.tutorial_toast_events = self.counters.tutorial_toast_events.saturating_add(1);
    }

    /// Apply the `Minecraft.tick` first-multiplayer-server call site before
    /// the ordinary tutorial owner. The caller uses the return value to queue
    /// the same-tick `Options.save` effect on the platform thread.
    pub fn show_social_interactions_tutorial_if_first_join(&mut self) -> bool {
        if self.level.is_none() || self.client_toasts.joined_first_server {
            return false;
        }
        self.client_toasts.joined_first_server = true;
        self.client_toasts.social_interactions_tutorial_owned = true;
        self.push_client_toast_event(ClientToastEventKind::Tutorial(
            ClientTutorialToastEvent::Show {
                token: ClientTutorialToastToken::SocialInteractions,
                kind: TutorialToastKind::SocialInteractions,
                time_to_display_ms: 8_000,
            },
        ));
        self.counters.tutorial_toast_events = self.counters.tutorial_toast_events.saturating_add(1);
        true
    }

    pub fn set_joined_first_server(&mut self, joined: bool) {
        self.client_toasts.joined_first_server = joined;
    }

    /// `Minecraft.handleKeybinds` hides the retained toast handle before it
    /// installs `SocialInteractionsScreen`.
    pub fn hide_social_interactions_tutorial(&mut self) -> bool {
        if !self.client_toasts.social_interactions_tutorial_owned {
            return false;
        }
        self.client_toasts.social_interactions_tutorial_owned = false;
        self.push_client_toast_event(ClientToastEventKind::Tutorial(
            ClientTutorialToastEvent::Hide {
                token: ClientTutorialToastToken::SocialInteractions,
            },
        ));
        true
    }

    pub(crate) fn push_system_toast_event(&mut self, event: ClientSystemToastEvent) {
        self.push_client_toast_event(ClientToastEventKind::System(event));
    }

    pub(crate) fn clear_client_toasts_for_disconnect(&mut self) {
        let now_playing = std::mem::take(&mut self.client_toasts.now_playing);
        let joined_first_server = self.client_toasts.joined_first_server;
        let social_interactions_tutorial_owned =
            self.client_toasts.social_interactions_tutorial_owned;
        self.client_toasts = ClientToastState {
            generation: self.client_toasts.generation.saturating_add(1),
            now_playing,
            joined_first_server,
            social_interactions_tutorial_owned,
            ..ClientToastState::default()
        };
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
                Some(ClientToastEventKind::Tutorial(_)) => {
                    self.counters.tutorial_toast_events_dropped = self
                        .counters
                        .tutorial_toast_events_dropped
                        .saturating_add(1);
                }
                Some(ClientToastEventKind::System(_)) => {}
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

        store.clear_client_toasts_for_disconnect();
        assert!(store.client_toast_events().is_empty());
        assert_eq!(store.client_toasts().next_sequence, 0);
        assert_eq!(store.client_toasts().generation, 1);
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
    fn all_toast_families_share_one_call_site_order() {
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
        store.push_system_toast_event(ClientSystemToastEvent::LowDiskSpace);
        store
            .push_advancement_toast_event("minecraft:test/last".to_string(), advancement_display());

        assert_eq!(
            store
                .client_toast_events()
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
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
            store.client_toast_events()[2].system(),
            Some(&ClientSystemToastEvent::LowDiskSpace)
        );
        assert_eq!(
            store.client_toast_events()[3]
                .advancement()
                .expect("advancement")
                .0,
            "minecraft:test/last"
        );
    }

    #[test]
    fn now_playing_is_independent_from_the_bounded_queue_and_normal_clear() {
        let mut store = WorldStore::new();
        store.show_now_playing_music("minecraft:music/game/creative/creative1".to_string());
        assert_eq!(
            store.now_playing_toast(),
            &ClientNowPlayingToastState {
                sound_name: Some("minecraft:music/game/creative/creative1".to_string()),
                wanted_visible: true,
                mutation_sequence: 1,
            }
        );

        store.clear_client_toasts_for_disconnect();
        assert_eq!(
            store.now_playing_toast().sound_name.as_deref(),
            Some("minecraft:music/game/creative/creative1")
        );
        assert!(store.now_playing_toast().wanted_visible);

        store.clear_now_playing_music();
        assert_eq!(store.now_playing_toast().sound_name, None);
        assert_eq!(store.now_playing_toast().mutation_sequence, 1);
        store.hide_now_playing_music();
        assert!(!store.now_playing_toast().wanted_visible);
        assert_eq!(store.now_playing_toast().mutation_sequence, 2);
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
