use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    GameProfile as ProtocolGameProfile, GameProfileProperty as ProtocolGameProfileProperty,
    GameType as ProtocolGameType, PlayerInfoAction as ProtocolPlayerInfoAction,
    PlayerInfoChatSession as ProtocolPlayerInfoChatSession,
    PlayerInfoRemove as ProtocolPlayerInfoRemove, PlayerInfoUpdate as ProtocolPlayerInfoUpdate,
};
use bbb_protocol::RichTextComponent;
use rsa::{
    pkcs8::{DecodePublicKey, EncodePublicKey},
    Pkcs1v15Sign, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::{client::chat::RemoteChatSessionState, WorldStore};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoState {
    pub entries: BTreeMap<Uuid, PlayerInfoEntryState>,
    pub listed_players: BTreeSet<Uuid>,
    /// Player info retained after a matching player entity has existed. This
    /// survives active-list removal and connection reconfiguration like
    /// vanilla's TransferState.seenPlayers map.
    #[serde(default)]
    pub seen_players: BTreeMap<Uuid, PlayerInfoEntryState>,
    /// UUIDs whose retained entry still aliases the current active PlayerInfo.
    /// Vanilla stores object references; this set preserves that distinction
    /// when canonical state stores owned snapshots.
    #[serde(default)]
    current_seen_players: BTreeSet<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoEntryState {
    pub profile: PlayerInfoProfileState,
    pub listed: bool,
    pub latency: i32,
    pub game_mode: String,
    pub display_name: Option<RichTextComponent>,
    pub show_hat: bool,
    pub list_order: i32,
    /// True only after the advertised profile key validates against the
    /// active Minecraft Services key set.
    pub chat_session_present: bool,
    /// Validated public session plus its per-sender message validator. This is
    /// runtime trust state rather than a control/serde projection.
    #[serde(skip)]
    pub(crate) chat_session: Option<RemoteChatSessionState>,
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
            chat_session: None,
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
        if !self.preflight_packet_components(&packet, "player info update") {
            return 0;
        }

        if packet
            .actions
            .contains(&ProtocolPlayerInfoAction::AddPlayer)
        {
            for entry in &packet.entries {
                let Some(profile) = &entry.profile else {
                    continue;
                };
                if !self.player_info.entries.contains_key(&entry.profile_id) {
                    self.record_social_player_profile(profile);
                    self.player_info
                        .entries
                        .insert(entry.profile_id, PlayerInfoEntryState::new(profile));
                }
            }
        }

        let mut applied = 0;
        for entry in packet.entries {
            let validated_chat_session = packet
                .actions
                .contains(&ProtocolPlayerInfoAction::InitializeChat)
                .then(|| {
                    entry.chat_session.as_ref().and_then(|session| {
                        validated_remote_chat_session(
                            &self.services_profile_keys,
                            entry.profile_id,
                            session,
                        )
                    })
                })
                .flatten();
            let Some(info) = self.player_info.entries.get_mut(&entry.profile_id) else {
                continue;
            };
            applied += 1;
            for action in &packet.actions {
                match action {
                    ProtocolPlayerInfoAction::AddPlayer => {}
                    ProtocolPlayerInfoAction::InitializeChat => {
                        info.chat_session_present = validated_chat_session.is_some();
                        info.chat_session = validated_chat_session.clone();
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
            if self
                .player_info
                .current_seen_players
                .contains(&entry.profile_id)
            {
                self.player_info
                    .seen_players
                    .insert(entry.profile_id, info.clone());
            }
        }

        self.update_player_info_counts();
        if applied > 0 {
            self.advance_scoreboard_display_revision();
        }
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
            self.player_info.current_seen_players.remove(&profile_id);
        }
        self.update_player_info_counts();
        if removed > 0 {
            self.advance_scoreboard_display_revision();
        }
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

    pub fn player_list_display_revision(&self) -> u64 {
        self.player_list_display_revision
    }

    pub fn seen_players(&self) -> &BTreeMap<Uuid, PlayerInfoEntryState> {
        &self.player_info.seen_players
    }

    pub fn seen_player_info_entry(&self, profile_id: Uuid) -> Option<&PlayerInfoEntryState> {
        self.player_info.seen_players.get(&profile_id)
    }

    pub(crate) fn record_seen_player_info(&mut self, profile_id: Uuid) {
        let Some(info) = self.player_info.entries.get(&profile_id).cloned() else {
            return;
        };
        self.player_info.seen_players.insert(profile_id, info);
        self.player_info.current_seen_players.insert(profile_id);
    }

    pub(crate) fn clear_active_player_info(&mut self) {
        let display_changed = !self.player_info.entries.is_empty();
        self.player_info.entries.clear();
        self.player_info.listed_players.clear();
        self.player_info.current_seen_players.clear();
        self.update_player_info_counts();
        if display_changed {
            self.advance_scoreboard_display_revision();
        }
    }

    fn update_player_info_counts(&mut self) {
        self.counters.player_info_entries_tracked = self.player_info.entries.len();
        self.counters.listed_players_tracked = self.player_info.listed_players.len();
    }
}

fn validated_remote_chat_session(
    services_profile_keys: &[Vec<u8>],
    profile_id: Uuid,
    chat_session: &ProtocolPlayerInfoChatSession,
) -> Option<RemoteChatSessionState> {
    let Ok(player_public_key) = RsaPublicKey::from_public_key_der(&chat_session.public_key) else {
        return None;
    };
    let Ok(canonical_player_key) = player_public_key.to_public_key_der() else {
        return None;
    };
    let mut payload = Vec::with_capacity(24 + canonical_player_key.as_bytes().len());
    payload.extend_from_slice(profile_id.as_bytes());
    payload.extend_from_slice(&chat_session.expires_at_epoch_millis.to_be_bytes());
    payload.extend_from_slice(canonical_player_key.as_bytes());
    let digest = Sha1::digest(payload);

    services_profile_keys
        .iter()
        .any(|key| {
            RsaPublicKey::from_public_key_der(key).is_ok_and(|key| {
                key.verify(
                    Pkcs1v15Sign::new::<Sha1>(),
                    &digest,
                    &chat_session.key_signature,
                )
                .is_ok()
            })
        })
        .then(|| {
            RemoteChatSessionState::new(
                chat_session.session_id,
                chat_session.expires_at_epoch_millis,
                canonical_player_key.as_bytes().to_vec(),
            )
        })
}

fn player_info_game_mode_name(game_mode: ProtocolGameType) -> &'static str {
    game_mode.serialized_name()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, PlayerInfoEntry as ProtocolPlayerInfoEntry,
        Transfer as ProtocolTransfer, Vec3d as ProtocolVec3d,
    };
    use rand::SeedableRng;
    use rsa::{pkcs8::EncodePublicKey, RsaPrivateKey};

    #[test]
    fn player_info_adds_player_with_profile_and_fields() {
        let mut store = WorldStore::new();
        assert_eq!(store.scoreboard_display_revision(), 0);
        assert_eq!(store.player_list_display_revision(), 0);
        let id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let mut entry = protocol_player_info_entry(id);
        entry.profile = Some(protocol_game_profile(id, "Ada"));
        entry.listed = true;
        entry.latency = 42;
        entry.game_mode = ProtocolGameType::Creative;
        entry.display_name = Some(RichTextComponent::plain("{\"text\":\"Ada Lovelace\"}"));
        entry.show_hat = true;
        entry.list_order = 7;
        let (services_key, chat_session) = signed_player_info_chat_session(id);
        store.replace_services_profile_keys(vec![services_key]);
        entry.chat_session = Some(chat_session);

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
            info.display_name.as_ref().map(RichTextComponent::text),
            Some("{\"text\":\"Ada Lovelace\"}")
        );
        assert!(info.show_hat);
        assert_eq!(info.list_order, 7);
        assert!(info.chat_session_present);
        assert!(info.chat_session.is_some());
        assert_eq!(store.listed_players(), &BTreeSet::from([id]));
        assert_eq!(store.scoreboard_display_revision(), 1);
        assert_eq!(store.player_list_display_revision(), 1);

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
        assert_eq!(store.scoreboard_display_revision(), 0);
        assert_eq!(store.player_list_display_revision(), 0);
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
        let (services_key, chat_session) = signed_player_info_chat_session(id);
        store.replace_services_profile_keys(vec![services_key]);
        with_chat.chat_session = Some(chat_session);
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::InitializeChat,
            ],
            entries: vec![with_chat],
        });
        assert!(store.player_info_entry(id).unwrap().chat_session_present);
        assert!(store.player_info_entry(id).unwrap().chat_session.is_some());

        let mut without_chat = protocol_player_info_entry(id);
        without_chat.chat_session = None;
        let applied = store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![ProtocolPlayerInfoAction::InitializeChat],
            entries: vec![without_chat],
        });

        assert_eq!(applied, 1);
        assert!(!store.player_info_entry(id).unwrap().chat_session_present);
        assert!(store.player_info_entry(id).unwrap().chat_session.is_none());
        assert_eq!(store.counters().player_info_update_packets, 2);
    }

    #[test]
    fn player_info_rejects_unvalidated_profile_keys() {
        let id = Uuid::from_u128(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee);
        let (services_key, chat_session) = signed_player_info_chat_session(id);

        let mut without_services_key = WorldStore::new();
        let mut entry = listed_player_info_entry(id, "Margaret", false);
        entry.chat_session = Some(chat_session.clone());
        without_services_key.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::InitializeChat,
            ],
            entries: vec![entry],
        });
        assert!(
            !without_services_key
                .player_info_entry(id)
                .unwrap()
                .chat_session_present
        );
        assert!(without_services_key
            .player_info_entry(id)
            .unwrap()
            .chat_session
            .is_none());

        let mut invalid_signature = WorldStore::new();
        invalid_signature.replace_services_profile_keys(vec![services_key]);
        let mut entry = listed_player_info_entry(id, "Margaret", false);
        let mut chat_session = chat_session;
        chat_session.key_signature[0] ^= 0xff;
        entry.chat_session = Some(chat_session);
        invalid_signature.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::InitializeChat,
            ],
            entries: vec![entry],
        });
        assert!(
            !invalid_signature
                .player_info_entry(id)
                .unwrap()
                .chat_session_present
        );
        assert!(invalid_signature
            .player_info_entry(id)
            .unwrap()
            .chat_session
            .is_none());
    }

    #[test]
    fn seen_players_follow_current_info_and_survive_transfer_reset() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xfedcba98765432100123456789abcdef);
        let mut initial = listed_player_info_entry(id, "Ada", true);
        initial.latency = 12;
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::UpdateListed,
                ProtocolPlayerInfoAction::UpdateLatency,
            ],
            entries: vec![initial],
        });

        store.apply_add_entity(protocol_player_entity(7, id));
        assert_eq!(store.seen_player_info_entry(id).unwrap().latency, 12);

        let mut latency_update = protocol_player_info_entry(id);
        latency_update.latency = 24;
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![ProtocolPlayerInfoAction::UpdateLatency],
            entries: vec![latency_update],
        });
        assert_eq!(store.seen_player_info_entry(id).unwrap().latency, 24);

        store.apply_player_info_remove(ProtocolPlayerInfoRemove {
            profile_ids: vec![id],
        });
        assert!(store.player_info_entry(id).is_none());
        assert_eq!(store.seen_player_info_entry(id).unwrap().latency, 24);

        store.apply_transfer(ProtocolTransfer {
            host: "next.example.com".to_string(),
            port: 25566,
        });
        store.clear_client_level();
        assert!(store.player_info().entries.is_empty());
        assert_eq!(store.seen_player_info_entry(id).unwrap().latency, 24);

        let mut replacement = listed_player_info_entry(id, "Ada New", false);
        replacement.latency = 36;
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::UpdateLatency,
            ],
            entries: vec![replacement],
        });
        // The replacement PlayerInfo does not overwrite the historical object
        // until its matching player entity is actually added.
        assert_eq!(store.seen_player_info_entry(id).unwrap().latency, 24);

        store.apply_add_entity(protocol_player_entity(8, id));
        let seen = store.seen_player_info_entry(id).unwrap();
        assert_eq!(seen.profile.name, "Ada New");
        assert_eq!(seen.latency, 36);
        assert_eq!(store.seen_players().len(), 1);
    }

    #[test]
    fn player_list_loaded_player_entity_query_tracks_entity_without_active_info() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0x0123456789abcdeffedcba9876543210);

        assert!(!store.has_loaded_player_entity(id));
        store.apply_add_entity(protocol_player_entity(9, id));

        assert!(store.seen_players().is_empty());
        assert!(store.has_loaded_player_entity(id));
        store.apply_remove_entities(bbb_protocol::packets::RemoveEntities {
            entity_ids: vec![9],
        });
        assert!(!store.has_loaded_player_entity(id));
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

    fn signed_player_info_chat_session(
        profile_id: Uuid,
    ) -> (Vec<u8>, ProtocolPlayerInfoChatSession) {
        let mut random = rand::rngs::StdRng::seed_from_u64(0x5e7a_1ce5);
        let services_private_key = RsaPrivateKey::new(&mut random, 1024).unwrap();
        let player_private_key = RsaPrivateKey::new(&mut random, 1024).unwrap();
        let player_public_key = RsaPublicKey::from(&player_private_key)
            .to_public_key_der()
            .unwrap();
        let expires_at_epoch_millis = 1_700_000_000_000i64;
        let mut payload = Vec::with_capacity(24 + player_public_key.as_bytes().len());
        payload.extend_from_slice(profile_id.as_bytes());
        payload.extend_from_slice(&expires_at_epoch_millis.to_be_bytes());
        payload.extend_from_slice(player_public_key.as_bytes());
        let key_signature = services_private_key
            .sign(Pkcs1v15Sign::new::<Sha1>(), &Sha1::digest(payload))
            .unwrap();
        let services_public_key = RsaPublicKey::from(&services_private_key)
            .to_public_key_der()
            .unwrap()
            .as_bytes()
            .to_vec();

        (
            services_public_key,
            ProtocolPlayerInfoChatSession {
                session_id: Uuid::from_u128(0x12345678123456781234567812345678),
                expires_at_epoch_millis,
                public_key: player_public_key.as_bytes().to_vec(),
                key_signature,
            },
        )
    }

    fn protocol_player_entity(id: i32, uuid: Uuid) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid,
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }
}
