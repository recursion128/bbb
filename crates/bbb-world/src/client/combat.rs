use bbb_protocol::packets::{
    PlayerCombatEnd as ProtocolPlayerCombatEnd, PlayerCombatKill as ProtocolPlayerCombatKill,
};
use bbb_protocol::RichTextComponent;
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientCombatState {
    #[serde(default)]
    pub last_combat: Option<PlayerCombatEventState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCombatEventState {
    pub kind: String,
    pub duration: Option<i32>,
    pub player_id: Option<i32>,
    pub message: Option<RichTextComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientDeathScreenState {
    pub generation: u64,
    pub cause_of_death: RichTextComponent,
    pub score: i32,
    pub hardcore: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCombatKillApplyOutcome {
    Ignored,
    DeathScreenOpened,
    PerformRespawn,
}

impl WorldStore {
    pub fn apply_player_combat_enter(&mut self) {
        self.counters.player_combat_enter_packets += 1;
        self.client_combat.last_combat = Some(PlayerCombatEventState {
            kind: "enter".to_string(),
            duration: None,
            player_id: None,
            message: None,
        });
    }

    pub fn apply_player_combat_end(&mut self, packet: ProtocolPlayerCombatEnd) {
        self.counters.player_combat_end_packets += 1;
        self.client_combat.last_combat = Some(PlayerCombatEventState {
            kind: "end".to_string(),
            duration: Some(packet.duration),
            player_id: None,
            message: None,
        });
    }

    pub fn apply_player_combat_kill(
        &mut self,
        packet: ProtocolPlayerCombatKill,
    ) -> PlayerCombatKillApplyOutcome {
        self.counters.player_combat_kill_packets += 1;
        self.client_combat.last_combat = Some(PlayerCombatEventState {
            kind: "kill".to_string(),
            duration: None,
            player_id: Some(packet.player_id),
            message: Some(packet.message.clone()),
        });

        if self.local_player_id() != Some(packet.player_id) {
            return PlayerCombatKillApplyOutcome::Ignored;
        }
        if !self.gameplay.show_death_screen {
            return PlayerCombatKillApplyOutcome::PerformRespawn;
        }

        let score = self.local_player_score();
        let hardcore = self.is_hardcore();
        self.client_ui.replace_screen();
        self.client_ui.death_screen_generation =
            self.client_ui.death_screen_generation.saturating_add(1);
        self.client_ui.current_death_screen = Some(ClientDeathScreenState {
            generation: self.client_ui.death_screen_generation,
            cause_of_death: packet.message,
            score,
            hardcore,
        });
        PlayerCombatKillApplyOutcome::DeathScreenOpened
    }

    pub fn client_combat(&self) -> &ClientCombatState {
        &self.client_combat
    }

    pub fn last_player_combat(&self) -> Option<&PlayerCombatEventState> {
        self.client_combat.last_combat.as_ref()
    }

    pub fn death_screen(&self) -> Option<&ClientDeathScreenState> {
        self.client_ui.current_death_screen.as_ref()
    }

    pub fn close_death_screen(&mut self, generation: u64) -> bool {
        if self
            .client_ui
            .current_death_screen
            .as_ref()
            .is_none_or(|screen| screen.generation != generation)
        {
            return false;
        }
        self.client_ui.current_death_screen = None;
        true
    }

    pub fn clear_death_screen(&mut self) -> bool {
        self.client_ui.current_death_screen.take().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, EntityDataValue, EntityDataValueKind, SetEntityData,
        Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    #[test]
    fn tracks_last_player_combat_event_and_counters() {
        let mut store = WorldStore::new();

        store.apply_player_combat_enter();
        assert_eq!(
            store.last_player_combat(),
            Some(&PlayerCombatEventState {
                kind: "enter".to_string(),
                duration: None,
                player_id: None,
                message: None,
            })
        );

        store.apply_player_combat_end(ProtocolPlayerCombatEnd { duration: 37 });
        assert_eq!(
            store.last_player_combat(),
            Some(&PlayerCombatEventState {
                kind: "end".to_string(),
                duration: Some(37),
                player_id: None,
                message: None,
            })
        );

        store.apply_player_combat_kill(ProtocolPlayerCombatKill {
            player_id: 123,
            message: RichTextComponent::plain("You died"),
        });
        assert_eq!(
            store.last_player_combat(),
            Some(&PlayerCombatEventState {
                kind: "kill".to_string(),
                duration: None,
                player_id: Some(123),
                message: Some(RichTextComponent::plain("You died")),
            })
        );

        let counters = store.counters();
        assert_eq!(counters.player_combat_enter_packets, 1);
        assert_eq!(counters.player_combat_end_packets, 1);
        assert_eq!(counters.player_combat_kill_packets, 1);
    }

    #[test]
    fn player_combat_kill_only_opens_death_screen_for_local_player() {
        let mut store = local_player_world(7);
        store.gameplay.hardcore = true;

        assert_eq!(
            store.apply_player_combat_kill(ProtocolPlayerCombatKill {
                player_id: 8,
                message: RichTextComponent::plain("Remote death"),
            }),
            PlayerCombatKillApplyOutcome::Ignored
        );
        assert_eq!(store.death_screen(), None);

        assert_eq!(
            store.apply_player_combat_kill(ProtocolPlayerCombatKill {
                player_id: 7,
                message: RichTextComponent::plain("Local death"),
            }),
            PlayerCombatKillApplyOutcome::DeathScreenOpened
        );
        let screen = store.death_screen().expect("local death screen");
        assert_eq!(screen.generation, 1);
        assert_eq!(screen.cause_of_death.text(), "Local death");
        assert_eq!(screen.score, 42);
        assert!(screen.hardcore);
    }

    #[test]
    fn player_combat_kill_requests_immediate_respawn_when_screen_is_disabled() {
        let mut store = local_player_world(7);
        store.gameplay.show_death_screen = false;

        assert_eq!(
            store.apply_player_combat_kill(ProtocolPlayerCombatKill {
                player_id: 7,
                message: RichTextComponent::plain("Local death"),
            }),
            PlayerCombatKillApplyOutcome::PerformRespawn
        );
        assert_eq!(store.death_screen(), None);
    }

    #[test]
    fn death_screen_generation_guards_close_and_replacement() {
        let mut store = local_player_world(7);
        store.apply_player_combat_kill(ProtocolPlayerCombatKill {
            player_id: 7,
            message: RichTextComponent::plain("First"),
        });
        let first_generation = store.death_screen().unwrap().generation;
        store.apply_player_combat_kill(ProtocolPlayerCombatKill {
            player_id: 7,
            message: RichTextComponent::plain("Second"),
        });
        let second_generation = store.death_screen().unwrap().generation;

        assert!(second_generation > first_generation);
        assert!(!store.close_death_screen(first_generation));
        assert!(store.close_death_screen(second_generation));
        assert_eq!(store.death_screen(), None);
        assert!(!store.clear_death_screen());
    }

    #[test]
    fn death_screen_participates_in_central_screen_replacement() {
        let mut store = local_player_world(7);
        store.open_game_event_screen(|generation| crate::ClientGameEventScreenState::Win {
            generation,
        });
        store.apply_player_combat_kill(ProtocolPlayerCombatKill {
            player_id: 7,
            message: RichTextComponent::plain("Death replaces win"),
        });
        assert_eq!(store.game_event_screen(), None);
        assert!(store.death_screen().is_some());

        store.open_game_event_screen(|generation| crate::ClientGameEventScreenState::Win {
            generation,
        });
        assert_eq!(store.death_screen(), None);
        assert!(store.game_event_screen().is_some());
    }

    fn local_player_world(id: i32) -> WorldStore {
        let mut store = WorldStore::new();
        store.local_player_id = Some(id);
        store.apply_add_entity(ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        let (score_data_id, score_default) =
            crate::vanilla_entity_metadata_player_score_accessor_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            )
            .expect("generated Player score accessor");
        assert_eq!(score_default, 0);
        assert!(store.apply_set_entity_data(SetEntityData {
            id,
            values: vec![EntityDataValue {
                data_id: score_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(42),
            }],
        }));
        store
    }
}
