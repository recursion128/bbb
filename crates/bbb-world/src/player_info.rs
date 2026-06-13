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
