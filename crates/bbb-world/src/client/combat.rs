use bbb_protocol::packets::{
    PlayerCombatEnd as ProtocolPlayerCombatEnd, PlayerCombatKill as ProtocolPlayerCombatKill,
};
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
    pub message: Option<String>,
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

    pub fn apply_player_combat_kill(&mut self, packet: ProtocolPlayerCombatKill) {
        self.counters.player_combat_kill_packets += 1;
        self.client_combat.last_combat = Some(PlayerCombatEventState {
            kind: "kill".to_string(),
            duration: None,
            player_id: Some(packet.player_id),
            message: Some(packet.message),
        });
    }

    pub fn client_combat(&self) -> &ClientCombatState {
        &self.client_combat
    }

    pub fn last_player_combat(&self) -> Option<&PlayerCombatEventState> {
        self.client_combat.last_combat.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            message: "You died".to_string(),
        });
        assert_eq!(
            store.last_player_combat(),
            Some(&PlayerCombatEventState {
                kind: "kill".to_string(),
                duration: None,
                player_id: Some(123),
                message: Some("You died".to_string()),
            })
        );

        let counters = store.counters();
        assert_eq!(counters.player_combat_enter_packets, 1);
        assert_eq!(counters.player_combat_end_packets, 1);
        assert_eq!(counters.player_combat_kill_packets, 1);
    }
}
