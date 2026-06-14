use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    GameProfile as ProtocolGameProfile, GameProfileProperty as ProtocolGameProfileProperty,
    GameType as ProtocolGameType, PlayerInfoAction as ProtocolPlayerInfoAction,
    PlayerInfoRemove as ProtocolPlayerInfoRemove, PlayerInfoUpdate as ProtocolPlayerInfoUpdate,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoState {
    pub entries: BTreeMap<Uuid, PlayerInfoEntryState>,
    pub listed_players: BTreeSet<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoEntryState {
    pub profile: PlayerInfoProfileState,
    pub listed: bool,
    pub latency: i32,
    pub game_mode: String,
    pub display_name: Option<String>,
    pub show_hat: bool,
    pub list_order: i32,
    pub chat_session_present: bool,
}

impl PlayerInfoEntryState {
    fn new(profile: &ProtocolGameProfile) -> Self {
        Self {
            profile: PlayerInfoProfileState::from(profile),
            listed: false,
            latency: 0,
            game_mode: player_info_game_mode_name(ProtocolGameType::default()).to_string(),
            display_name: None,
            show_hat: false,
            list_order: 0,
            chat_session_present: false,
        }
    }

    pub fn is_spectator(&self) -> bool {
        self.game_mode == "spectator"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoProfileState {
    pub uuid: Uuid,
    pub name: String,
    pub properties: Vec<ProtocolGameProfileProperty>,
}

impl From<&ProtocolGameProfile> for PlayerInfoProfileState {
    fn from(profile: &ProtocolGameProfile) -> Self {
        Self {
            uuid: profile.uuid,
            name: profile.name.clone(),
            properties: profile.properties.clone(),
        }
    }
}

impl WorldStore {
    pub fn apply_player_info_update(&mut self, packet: ProtocolPlayerInfoUpdate) -> usize {
        self.counters.player_info_update_packets += 1;

        if packet
            .actions
            .contains(&ProtocolPlayerInfoAction::AddPlayer)
        {
            for entry in &packet.entries {
                let Some(profile) = &entry.profile else {
                    continue;
                };
                self.player_info
                    .entries
                    .entry(entry.profile_id)
                    .or_insert_with(|| PlayerInfoEntryState::new(profile));
            }
        }

        let mut applied = 0;
        for entry in packet.entries {
            let Some(info) = self.player_info.entries.get_mut(&entry.profile_id) else {
                continue;
            };
            applied += 1;
            for action in &packet.actions {
                match action {
                    ProtocolPlayerInfoAction::AddPlayer => {}
                    ProtocolPlayerInfoAction::InitializeChat => {
                        info.chat_session_present = entry.chat_session.is_some();
                    }
                    ProtocolPlayerInfoAction::UpdateGameMode => {
                        info.game_mode = player_info_game_mode_name(entry.game_mode).to_string();
                    }
                    ProtocolPlayerInfoAction::UpdateListed => {
                        info.listed = entry.listed;
                        if entry.listed {
                            self.player_info.listed_players.insert(entry.profile_id);
                        } else {
                            self.player_info.listed_players.remove(&entry.profile_id);
                        }
                    }
                    ProtocolPlayerInfoAction::UpdateLatency => {
                        info.latency = entry.latency;
                    }
                    ProtocolPlayerInfoAction::UpdateDisplayName => {
                        info.display_name = entry.display_name.clone();
                    }
                    ProtocolPlayerInfoAction::UpdateHat => {
                        info.show_hat = entry.show_hat;
                    }
                    ProtocolPlayerInfoAction::UpdateListOrder => {
                        info.list_order = entry.list_order;
                    }
                }
            }
        }

        self.update_player_info_counts();
        applied
    }

    pub fn apply_player_info_remove(&mut self, packet: ProtocolPlayerInfoRemove) -> usize {
        self.counters.player_info_remove_packets += 1;
        let mut removed = 0;
        for profile_id in packet.profile_ids {
            if self.player_info.entries.remove(&profile_id).is_some() {
                removed += 1;
            }
            self.player_info.listed_players.remove(&profile_id);
        }
        self.update_player_info_counts();
        removed
    }

    pub fn player_info(&self) -> &PlayerInfoState {
        &self.player_info
    }

    pub fn player_info_entry(&self, profile_id: Uuid) -> Option<&PlayerInfoEntryState> {
        self.player_info.entries.get(&profile_id)
    }

    pub fn listed_players(&self) -> &BTreeSet<Uuid> {
        &self.player_info.listed_players
    }

    fn update_player_info_counts(&mut self) {
        self.counters.player_info_entries_tracked = self.player_info.entries.len();
        self.counters.listed_players_tracked = self.player_info.listed_players.len();
    }
}

fn player_info_game_mode_name(game_mode: ProtocolGameType) -> &'static str {
    match game_mode {
        ProtocolGameType::Survival => "survival",
        ProtocolGameType::Creative => "creative",
        ProtocolGameType::Adventure => "adventure",
        ProtocolGameType::Spectator => "spectator",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        PlayerInfoChatSession as ProtocolPlayerInfoChatSession,
        PlayerInfoEntry as ProtocolPlayerInfoEntry,
    };

    #[test]
    fn player_info_adds_player_with_profile_and_fields() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let mut entry = protocol_player_info_entry(id);
        entry.profile = Some(protocol_game_profile(id, "Ada"));
        entry.listed = true;
        entry.latency = 42;
        entry.game_mode = ProtocolGameType::Creative;
        entry.display_name = Some("{\"text\":\"Ada Lovelace\"}".to_string());
        entry.show_hat = true;
        entry.list_order = 7;
        entry.chat_session = Some(protocol_player_info_chat_session());

        let applied = store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::InitializeChat,
                ProtocolPlayerInfoAction::UpdateGameMode,
                ProtocolPlayerInfoAction::UpdateListed,
                ProtocolPlayerInfoAction::UpdateLatency,
                ProtocolPlayerInfoAction::UpdateDisplayName,
                ProtocolPlayerInfoAction::UpdateHat,
                ProtocolPlayerInfoAction::UpdateListOrder,
            ],
            entries: vec![entry],
        });

        assert_eq!(applied, 1);
        let info = store.player_info_entry(id).unwrap();
        assert_eq!(info.profile.uuid, id);
        assert_eq!(info.profile.name, "Ada");
        assert_eq!(info.profile.properties.len(), 1);
        assert!(info.listed);
        assert_eq!(info.latency, 42);
        assert_eq!(info.game_mode, "creative");
        assert_eq!(
            info.display_name.as_deref(),
            Some("{\"text\":\"Ada Lovelace\"}")
        );
        assert!(info.show_hat);
        assert_eq!(info.list_order, 7);
        assert!(info.chat_session_present);
        assert_eq!(store.listed_players(), &BTreeSet::from([id]));

        let counters = store.counters();
        assert_eq!(counters.player_info_update_packets, 1);
        assert_eq!(counters.player_info_entries_tracked, 1);
        assert_eq!(counters.listed_players_tracked, 1);
    }

    #[test]
    fn player_info_update_ignores_unknown_uuid() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
        let mut entry = protocol_player_info_entry(id);
        entry.listed = true;
        entry.latency = 99;

        let applied = store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::UpdateListed,
                ProtocolPlayerInfoAction::UpdateLatency,
            ],
            entries: vec![entry],
        });

        assert_eq!(applied, 0);
        assert!(store.player_info().entries.is_empty());
        assert!(store.listed_players().is_empty());
        assert_eq!(store.counters().player_info_update_packets, 1);
        assert_eq!(store.counters().player_info_entries_tracked, 0);
        assert_eq!(store.counters().listed_players_tracked, 0);
    }

    #[test]
    fn player_info_remove_clears_entry_and_listed_tracking() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xcccccccccccccccccccccccccccccccc);
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::UpdateListed,
            ],
            entries: vec![listed_player_info_entry(id, "Grace", true)],
        });

        assert!(store.player_info_entry(id).is_some());
        assert!(store.listed_players().contains(&id));

        let removed = store.apply_player_info_remove(ProtocolPlayerInfoRemove {
            profile_ids: vec![id],
        });

        assert_eq!(removed, 1);
        assert!(store.player_info_entry(id).is_none());
        assert!(store.listed_players().is_empty());
        let counters = store.counters();
        assert_eq!(counters.player_info_remove_packets, 1);
        assert_eq!(counters.player_info_entries_tracked, 0);
        assert_eq!(counters.listed_players_tracked, 0);
    }

    #[test]
    fn player_info_listed_false_removes_from_listed_set() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xdddddddddddddddddddddddddddddddd);
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::UpdateListed,
            ],
            entries: vec![listed_player_info_entry(id, "Katherine", true)],
        });
        assert!(store.listed_players().contains(&id));

        let mut unlisted = protocol_player_info_entry(id);
        unlisted.listed = false;
        let applied = store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![ProtocolPlayerInfoAction::UpdateListed],
            entries: vec![unlisted],
        });

        assert_eq!(applied, 1);
        assert!(!store.player_info_entry(id).unwrap().listed);
        assert!(store.listed_players().is_empty());
        assert_eq!(store.counters().listed_players_tracked, 0);
    }

    #[test]
    fn player_info_chat_session_present_flag_can_set_and_clear() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee);
        let mut with_chat = listed_player_info_entry(id, "Margaret", false);
        with_chat.chat_session = Some(protocol_player_info_chat_session());
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::InitializeChat,
            ],
            entries: vec![with_chat],
        });
        assert!(store.player_info_entry(id).unwrap().chat_session_present);

        let mut without_chat = protocol_player_info_entry(id);
        without_chat.chat_session = None;
        let applied = store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![ProtocolPlayerInfoAction::InitializeChat],
            entries: vec![without_chat],
        });

        assert_eq!(applied, 1);
        assert!(!store.player_info_entry(id).unwrap().chat_session_present);
        assert_eq!(store.counters().player_info_update_packets, 2);
    }

    fn protocol_game_profile(uuid: Uuid, name: &str) -> ProtocolGameProfile {
        ProtocolGameProfile {
            uuid,
            name: name.to_string(),
            properties: vec![ProtocolGameProfileProperty {
                name: "textures".to_string(),
                value: "skin-payload".to_string(),
                signature: Some("skin-signature".to_string()),
            }],
        }
    }

    fn protocol_player_info_entry(profile_id: Uuid) -> ProtocolPlayerInfoEntry {
        ProtocolPlayerInfoEntry {
            profile_id,
            profile: None,
            listed: false,
            latency: 0,
            game_mode: ProtocolGameType::default(),
            display_name: None,
            show_hat: false,
            list_order: 0,
            chat_session: None,
        }
    }

    fn listed_player_info_entry(
        profile_id: Uuid,
        name: &str,
        listed: bool,
    ) -> ProtocolPlayerInfoEntry {
        let mut entry = protocol_player_info_entry(profile_id);
        entry.profile = Some(protocol_game_profile(profile_id, name));
        entry.listed = listed;
        entry
    }

    fn protocol_player_info_chat_session() -> ProtocolPlayerInfoChatSession {
        ProtocolPlayerInfoChatSession {
            session_id: Uuid::from_u128(0x12345678123456781234567812345678),
            expires_at_epoch_millis: 1_700_000_000_000,
            public_key: vec![1, 2, 3],
            key_signature: vec![4, 5, 6],
        }
    }
}
