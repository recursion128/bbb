use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use bbb_protocol::packets::GameProfile as ProtocolGameProfile;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ChatReportEventKind, PlayerInfoProfileState, WorldStore};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServicesBlockListState {
    pub generation: u64,
    pub active: bool,
    pub blocked_players: Vec<Uuid>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSocialState {
    #[serde(default)]
    pub discovered_names_to_uuid: BTreeMap<String, Uuid>,
    #[serde(default)]
    pub server_label: String,
    #[serde(default)]
    pub services_block_list: ServicesBlockListState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialInteractionsPage {
    All,
    Hidden,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialInteractionsScreenState {
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialPlayerState {
    pub profile: PlayerInfoProfileState,
    pub removed: bool,
    pub recent_messages: bool,
    pub hidden: bool,
    pub blocked: bool,
    pub chat_reportable: bool,
}

impl WorldStore {
    pub(crate) fn record_social_player_profile(&mut self, profile: &ProtocolGameProfile) {
        self.player_social
            .discovered_names_to_uuid
            .insert(profile.name.clone(), profile.uuid);
    }

    pub fn player_social(&self) -> &PlayerSocialState {
        &self.player_social
    }

    pub fn social_discovered_uuid(&self, name: &str) -> Option<Uuid> {
        self.player_social
            .discovered_names_to_uuid
            .get(name)
            .copied()
    }

    pub fn set_social_server_label(&mut self, server_label: impl Into<String>) {
        self.player_social.server_label = server_label.into();
    }

    pub fn set_social_server_target(&mut self, host: &str, port: i32) {
        let host = if host.contains(':') && !(host.starts_with('[') && host.ends_with(']')) {
            format!("[{host}]")
        } else {
            host.to_string()
        };
        self.set_social_server_label(format!("{host}:{port}"));
    }

    pub fn social_player_is_hidden(&self, profile_id: Uuid) -> bool {
        self.client_chat
            .presentation
            .hidden_players
            .binary_search(&profile_id)
            .is_ok()
    }

    pub fn social_player_is_blocked(&self, profile_id: Uuid) -> bool {
        if profile_id.is_nil() || Some(profile_id) == self.local_player_profile_id() {
            return false;
        }
        self.client_chat
            .presentation
            .blocked_players
            .binary_search(&profile_id)
            .is_ok()
            || (self.player_social.services_block_list.active
                && self
                    .player_social
                    .services_block_list
                    .blocked_players
                    .binary_search(&profile_id)
                    .is_ok())
    }

    pub fn social_should_hide_message_from(&self, profile_id: Uuid) -> bool {
        if profile_id.is_nil() || Some(profile_id) == self.local_player_profile_id() {
            return false;
        }
        self.social_player_is_hidden(profile_id) || self.social_player_is_blocked(profile_id)
    }

    pub fn services_block_list(&self) -> &ServicesBlockListState {
        &self.player_social.services_block_list
    }

    pub fn begin_services_block_list_session(&mut self, generation: u64) {
        self.player_social.services_block_list = ServicesBlockListState {
            generation,
            active: true,
            blocked_players: Vec::new(),
        };
    }

    pub fn stop_services_block_list_session(&mut self, generation: u64) {
        self.player_social.services_block_list = ServicesBlockListState {
            generation,
            active: false,
            blocked_players: Vec::new(),
        };
    }

    pub fn replace_services_blocked_players(
        &mut self,
        generation: u64,
        blocked_players: impl IntoIterator<Item = Uuid>,
    ) -> bool {
        let state = &mut self.player_social.services_block_list;
        if !state.active || state.generation != generation {
            return false;
        }
        state.blocked_players = blocked_players.into_iter().collect();
        state.blocked_players.sort_unstable();
        state.blocked_players.dedup();
        true
    }

    pub fn hide_social_player(&mut self, profile_id: Uuid) -> bool {
        if self.social_player_is_hidden(profile_id) {
            return false;
        }
        let mut config = self.client_chat.presentation.clone();
        config.hidden_players.push(profile_id);
        self.set_chat_presentation_config(config);
        true
    }

    pub fn show_social_player(&mut self, profile_id: Uuid) -> bool {
        if !self.social_player_is_hidden(profile_id) {
            return false;
        }
        let mut config = self.client_chat.presentation.clone();
        config.hidden_players.retain(|hidden| *hidden != profile_id);
        self.set_chat_presentation_config(config);
        true
    }

    pub fn social_interactions_screen(&self) -> Option<SocialInteractionsScreenState> {
        self.client_ui.current_social_interactions_screen
    }

    pub fn open_social_interactions_screen(&mut self) -> SocialInteractionsScreenState {
        if let Some(screen) = self.social_interactions_screen() {
            return screen;
        }
        self.client_ui.replace_screen();
        self.client_ui.social_interactions_screen_generation = self
            .client_ui
            .social_interactions_screen_generation
            .saturating_add(1);
        let screen = SocialInteractionsScreenState {
            generation: self.client_ui.social_interactions_screen_generation,
        };
        self.client_ui.current_social_interactions_screen = Some(screen);
        screen
    }

    pub fn close_social_interactions_screen(&mut self, generation: u64) -> bool {
        if self
            .social_interactions_screen()
            .is_none_or(|screen| screen.generation != generation)
        {
            return false;
        }
        self.client_ui.current_social_interactions_screen = None;
        true
    }

    pub fn clear_social_interactions_screen(&mut self) -> bool {
        self.client_ui
            .current_social_interactions_screen
            .take()
            .is_some()
    }

    pub fn social_players(&self, page: SocialInteractionsPage) -> Vec<SocialPlayerState> {
        let recent_profiles = self.recent_social_profiles();
        let recent_ids = recent_profiles.keys().copied().collect::<BTreeSet<_>>();
        let mut players = BTreeMap::<Uuid, SocialPlayerState>::new();

        for (profile_id, info) in &self.player_info.entries {
            if page == SocialInteractionsPage::Hidden && !self.social_player_is_hidden(*profile_id)
            {
                continue;
            }
            if page == SocialInteractionsPage::Blocked
                && !self.social_player_is_blocked(*profile_id)
            {
                continue;
            }
            players.insert(
                *profile_id,
                SocialPlayerState {
                    profile: info.profile.clone(),
                    removed: false,
                    recent_messages: recent_ids.contains(profile_id),
                    hidden: self.social_player_is_hidden(*profile_id),
                    blocked: self.social_player_is_blocked(*profile_id),
                    chat_reportable: info.chat_session_present,
                },
            );
        }

        if page == SocialInteractionsPage::All {
            for (profile_id, info) in &self.player_info.seen_players {
                players
                    .entry(*profile_id)
                    .or_insert_with(|| SocialPlayerState {
                        profile: info.profile.clone(),
                        removed: true,
                        recent_messages: recent_ids.contains(profile_id),
                        hidden: self.social_player_is_hidden(*profile_id),
                        blocked: self.social_player_is_blocked(*profile_id),
                        chat_reportable: info.chat_session_present,
                    });
            }
            for (profile_id, profile) in recent_profiles {
                players
                    .entry(profile_id)
                    .or_insert_with(|| SocialPlayerState {
                        profile,
                        removed: true,
                        recent_messages: true,
                        hidden: self.social_player_is_hidden(profile_id),
                        blocked: self.social_player_is_blocked(profile_id),
                        chat_reportable: true,
                    });
            }
        }

        let local_player = self.local_player_profile_id();
        let mut players = players.into_values().collect::<Vec<_>>();
        players.sort_by(|left, right| compare_social_players(left, right, local_player));
        players
    }

    fn recent_social_profiles(&self) -> BTreeMap<Uuid, PlayerInfoProfileState> {
        let mut profiles = BTreeMap::new();
        for event in self.client_chat.report_log.events.iter().rev() {
            if event.kind != ChatReportEventKind::Player || event.signature.is_none() {
                continue;
            }
            let Some(profile_id) = event.sender else {
                continue;
            };
            profiles.entry(profile_id).or_insert_with(|| {
                let name = event
                    .sender_name
                    .clone()
                    .or_else(|| self.discovered_social_name(profile_id))
                    .unwrap_or_else(|| profile_id.to_string());
                PlayerInfoProfileState {
                    uuid: profile_id,
                    name,
                    properties: Vec::new(),
                }
            });
        }
        profiles
    }

    fn discovered_social_name(&self, profile_id: Uuid) -> Option<String> {
        self.player_social
            .discovered_names_to_uuid
            .iter()
            .find_map(|(name, discovered_id)| (*discovered_id == profile_id).then(|| name.clone()))
    }
}

fn compare_social_players(
    left: &SocialPlayerState,
    right: &SocialPlayerState,
    local_player: Option<Uuid>,
) -> Ordering {
    social_player_sort_category(left, local_player)
        .cmp(&social_player_sort_category(right, local_player))
        .then_with(|| {
            social_player_name_category(&left.profile.name)
                .cmp(&social_player_name_category(&right.profile.name))
        })
        .then_with(|| {
            left.profile
                .name
                .to_lowercase()
                .cmp(&right.profile.name.to_lowercase())
        })
        .then_with(|| left.profile.name.cmp(&right.profile.name))
        .then_with(|| left.profile.uuid.cmp(&right.profile.uuid))
}

fn social_player_sort_category(player: &SocialPlayerState, local_player: Option<Uuid>) -> u8 {
    if Some(player.profile.uuid) == local_player {
        0
    } else if player.profile.uuid.get_version_num() == 2 {
        4
    } else if player.recent_messages {
        2
    } else {
        3
    }
}

fn social_player_name_category(name: &str) -> u8 {
    match name.chars().next() {
        Some('_' | 'a'..='z' | 'A'..='Z' | '0'..='9') => 0,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        packets::{
            DialogHolder, GameProfileProperty, GameType, PlayerInfoAction, PlayerInfoEntry,
            PlayerInfoRemove, PlayerInfoUpdate, ShowDialog, Transfer,
        },
        NbtSummaryEntry, NbtSummaryValue,
    };

    use crate::{
        ChatReportEventState, ChatSignatureState, ChatTrustLevelState, DialogPreviousScreenState,
    };

    use super::*;

    #[test]
    fn player_social_discovery_hidden_blocked_and_sorting_share_chat_owner() {
        let mut store = WorldStore::new();
        let local_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let hidden_id = Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
        let blocked_id = Uuid::from_u128(0xcccccccccccccccccccccccccccccccc);
        store.set_local_player_profile_id(local_id);
        store.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![PlayerInfoAction::AddPlayer],
            entries: vec![
                player_info_entry(local_id, "Zulu"),
                player_info_entry(hidden_id, "Ada"),
                player_info_entry(blocked_id, "Grace"),
            ],
        });

        let mut chat_config = store.chat_presentation_config().clone();
        chat_config.blocked_players.push(blocked_id);
        store.set_chat_presentation_config(chat_config);
        assert!(store.hide_social_player(hidden_id));
        assert!(!store.hide_social_player(hidden_id));

        assert_eq!(store.social_discovered_uuid("Ada"), Some(hidden_id));
        assert_eq!(store.social_discovered_uuid("ada"), None);
        assert!(store.social_should_hide_message_from(hidden_id));
        assert!(store.social_should_hide_message_from(blocked_id));
        assert_eq!(
            store.chat_presentation_config().hidden_players,
            vec![hidden_id]
        );

        let all = store.social_players(SocialInteractionsPage::All);
        assert_eq!(
            all.iter()
                .map(|player| player.profile.uuid)
                .collect::<Vec<_>>(),
            vec![local_id, hidden_id, blocked_id]
        );
        assert!(all[1].hidden);
        assert!(all[2].blocked);
        assert_eq!(
            store
                .social_players(SocialInteractionsPage::Hidden)
                .iter()
                .map(|player| player.profile.uuid)
                .collect::<Vec<_>>(),
            vec![hidden_id]
        );
        assert_eq!(
            store
                .social_players(SocialInteractionsPage::Blocked)
                .iter()
                .map(|player| player.profile.uuid)
                .collect::<Vec<_>>(),
            vec![blocked_id]
        );

        assert!(store.show_social_player(hidden_id));
        assert!(!store.show_social_player(hidden_id));
        assert!(!store.social_player_is_hidden(hidden_id));
        assert!(store.social_player_is_blocked(blocked_id));
        assert!(store.chat_presentation_config().hidden_players.is_empty());

        store.clear_client_level();
        assert_eq!(store.social_discovered_uuid("Ada"), Some(hidden_id));
        assert!(store.social_player_is_blocked(blocked_id));
    }

    #[test]
    fn player_social_services_block_list_unions_configured_and_excludes_local_and_nil() {
        let mut store = WorldStore::new();
        let local_id = Uuid::from_u128(1);
        let configured_id = Uuid::from_u128(2);
        let services_id = Uuid::from_u128(3);
        store.set_local_player_profile_id(local_id);
        let mut config = store.chat_presentation_config().clone();
        config.blocked_players = vec![configured_id, local_id, Uuid::nil()];
        store.set_chat_presentation_config(config);

        store.begin_services_block_list_session(7);
        assert!(store.replace_services_blocked_players(
            7,
            [services_id, services_id, local_id, Uuid::nil()]
        ));

        assert!(store.social_player_is_blocked(configured_id));
        assert!(store.social_player_is_blocked(services_id));
        assert!(!store.social_player_is_blocked(local_id));
        assert!(!store.social_player_is_blocked(Uuid::nil()));
        assert!(store.social_should_hide_message_from(services_id));
        assert_eq!(
            store.services_block_list().blocked_players,
            vec![Uuid::nil(), local_id, services_id]
        );
    }

    #[test]
    fn player_social_services_block_list_rejects_stale_or_inactive_replacement() {
        let mut store = WorldStore::new();
        let blocked_id = Uuid::from_u128(4);
        store.begin_services_block_list_session(10);
        assert!(!store.replace_services_blocked_players(9, [blocked_id]));
        assert!(store.services_block_list().blocked_players.is_empty());

        assert!(store.replace_services_blocked_players(10, [blocked_id]));
        assert!(store.social_player_is_blocked(blocked_id));
        store.stop_services_block_list_session(11);
        assert!(!store.social_player_is_blocked(blocked_id));
        assert!(!store.replace_services_blocked_players(10, [blocked_id]));
        assert_eq!(
            store.services_block_list(),
            &ServicesBlockListState {
                generation: 11,
                active: false,
                blocked_players: Vec::new(),
            }
        );
    }

    #[test]
    fn player_info_remove_preserves_seen_and_recent_social_rows_as_removed() {
        let mut store = WorldStore::new();
        let seen_id = Uuid::from_u128(0x11111111111111111111111111111111);
        let recent_id = Uuid::from_u128(0x22222222222242228222222222222222);
        let forgotten_id = Uuid::from_u128(0x33333333333343338333333333333333);
        store.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![PlayerInfoAction::AddPlayer],
            entries: vec![
                player_info_entry(seen_id, "Seen"),
                player_info_entry(forgotten_id, "Forgotten"),
            ],
        });
        store.record_seen_player_info(seen_id);
        store.client_chat.report_log.events.push(chat_report_event(
            recent_id,
            "Recent",
            Some(ChatSignatureState {
                checksum: 7,
                bytes_len: 3,
                bytes: vec![1, 2, 3],
            }),
        ));

        assert_eq!(
            store.apply_player_info_remove(PlayerInfoRemove {
                profile_ids: vec![seen_id, forgotten_id],
            }),
            2
        );

        let players = store.social_players(SocialInteractionsPage::All);
        assert_eq!(
            players
                .iter()
                .map(|player| player.profile.uuid)
                .collect::<Vec<_>>(),
            vec![recent_id, seen_id]
        );
        let recent = players
            .iter()
            .find(|player| player.profile.uuid == recent_id)
            .unwrap();
        assert!(recent.removed);
        assert!(recent.recent_messages);
        assert!(recent.chat_reportable);
        let seen = players
            .iter()
            .find(|player| player.profile.uuid == seen_id)
            .unwrap();
        assert!(seen.removed);
        assert!(!seen.recent_messages);
        assert_eq!(seen.profile.properties.len(), 1);
        assert!(!players
            .iter()
            .any(|player| player.profile.uuid == forgotten_id));

        store.hide_social_player(seen_id);
        assert!(store
            .social_players(SocialInteractionsPage::Hidden)
            .is_empty());
    }

    #[test]
    fn social_screen_generation_rejects_stale_close_and_follows_replacement_lifecycle() {
        let mut store = WorldStore::new();
        let first = store.open_social_interactions_screen();
        assert_eq!(store.open_social_interactions_screen(), first);
        assert!(!store.close_social_interactions_screen(first.generation + 1));

        store.open_stats_screen();
        assert_eq!(store.social_interactions_screen(), None);
        let second = store.open_social_interactions_screen();
        assert!(second.generation > first.generation);

        store.apply_show_dialog(ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "type".to_string(),
                    value: NbtSummaryValue::String("minecraft:notice".to_string()),
                }]),
            },
        });
        assert_eq!(store.social_interactions_screen(), None);
        assert!(matches!(
            store.client_ui().dialog_previous_screen,
            Some(DialogPreviousScreenState::SocialInteractions { screen })
                if screen == second
        ));
        store.apply_clear_dialog();
        assert_eq!(store.social_interactions_screen(), Some(second));

        store.clear_client_level();
        assert_eq!(store.social_interactions_screen(), None);
        assert!(!store.close_social_interactions_screen(second.generation));
    }

    #[test]
    fn player_social_transfer_updates_repo_native_server_target() {
        let mut store = WorldStore::new();
        store.set_social_server_target("2001:db8::1", 25565);
        assert_eq!(store.player_social().server_label, "[2001:db8::1]:25565");

        store.apply_transfer(Transfer {
            host: "next.example.com".to_string(),
            port: 25566,
        });
        assert_eq!(store.player_social().server_label, "next.example.com:25566");
    }

    fn player_info_entry(profile_id: Uuid, name: &str) -> PlayerInfoEntry {
        PlayerInfoEntry {
            profile_id,
            profile: Some(ProtocolGameProfile {
                uuid: profile_id,
                name: name.to_string(),
                properties: vec![GameProfileProperty {
                    name: "textures".to_string(),
                    value: "skin-payload".to_string(),
                    signature: Some("skin-signature".to_string()),
                }],
            }),
            listed: false,
            latency: 0,
            game_mode: GameType::default(),
            display_name: None,
            show_hat: false,
            list_order: 0,
            chat_session: None,
        }
    }

    fn chat_report_event(
        sender: Uuid,
        sender_name: &str,
        signature: Option<ChatSignatureState>,
    ) -> ChatReportEventState {
        ChatReportEventState {
            id: 0,
            kind: ChatReportEventKind::Player,
            sender: Some(sender),
            sender_name: Some(sender_name.to_string()),
            signed_content: Some("hello".to_string()),
            unsigned_content: None,
            displayed_content: "hello".to_string(),
            received_epoch_millis: 1,
            message_timestamp_millis: Some(1),
            signed_message: None,
            trust_level: Some(ChatTrustLevelState::Secure),
            signature,
            filter_mask: None,
        }
    }
}
