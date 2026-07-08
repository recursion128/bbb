use bbb_protocol::packets::{
    BlockChangedAck as ProtocolBlockChangedAck, BlockDestruction as ProtocolBlockDestruction,
    BlockEvent as ProtocolBlockEvent, LevelEvent as ProtocolLevelEvent, Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use crate::{protocol_block_pos, BlockPos, JukeboxLevelEventState, WorldStore};

const BLOCK_DESTRUCTION_EXPIRY_SCAN_INTERVAL_TICKS: u32 = 20;
const BLOCK_DESTRUCTION_EXPIRY_TICKS: u32 = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDestructionProgress {
    pub id: i32,
    pub pos: BlockPos,
    pub progress: u8,
    #[serde(default)]
    pub updated_render_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEventRecord {
    pub pos: BlockPos,
    pub b0: u8,
    pub b1: u8,
    pub block_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelEventRecord {
    pub event_type: i32,
    pub pos: BlockPos,
    pub data: i32,
    pub global: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockChangedAckState {
    pub sequence: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalBlockPredictionState {
    pub sequence: i32,
    pub pos: BlockPos,
    pub server_block_state_id: i32,
    pub predicted_block_state_id: i32,
    #[serde(default)]
    pub player_position: Option<ProtocolVec3d>,
}

impl WorldStore {
    pub fn apply_block_changed_ack(&mut self, ack: ProtocolBlockChangedAck) {
        self.counters.block_changed_ack_packets += 1;
        self.last_block_changed_ack = Some(BlockChangedAckState {
            sequence: ack.sequence,
        });
        let reconciled = self.sync_ended_local_block_predictions(ack.sequence);
        self.counters.local_block_predictions_reconciled_by_ack += reconciled;
    }

    pub fn apply_block_destruction(&mut self, update: ProtocolBlockDestruction) -> bool {
        self.counters.block_destructions_received += 1;
        if update.progress < 10 {
            let progress = BlockDestructionProgress {
                id: update.id,
                pos: protocol_block_pos(update.pos),
                progress: update.progress,
                updated_render_tick: self.block_destruction_render_tick,
            };
            if let Some(existing) = self
                .block_destructions
                .iter_mut()
                .find(|existing| existing.id == update.id)
            {
                *existing = progress;
            } else {
                self.block_destructions.push(progress);
            }
            self.counters.block_destructions_tracked = self.block_destructions.len();
            return true;
        }

        let before = self.block_destructions.len();
        self.block_destructions
            .retain(|progress| progress.id != update.id);
        let removed = before - self.block_destructions.len();
        self.counters.block_destructions_removed += removed;
        if removed == 0 {
            self.counters.block_destructions_ignored += 1;
        }
        self.counters.block_destructions_tracked = self.block_destructions.len();
        removed > 0
    }

    pub fn advance_block_destruction_render_ticks(&mut self, ticks: u32) -> usize {
        if ticks == 0 {
            return 0;
        }

        let previous_tick = self.block_destruction_render_tick;
        self.block_destruction_render_tick =
            self.block_destruction_render_tick.saturating_add(ticks);
        if !crossed_block_destruction_expiry_scan(previous_tick, self.block_destruction_render_tick)
        {
            return 0;
        }

        let before = self.block_destructions.len();
        let scan_tick = block_destruction_expiry_scan_tick(self.block_destruction_render_tick);
        self.block_destructions.retain(|progress| {
            scan_tick.saturating_sub(progress.updated_render_tick) <= BLOCK_DESTRUCTION_EXPIRY_TICKS
        });
        let expired = before - self.block_destructions.len();
        self.counters.block_destructions_expired += expired;
        self.counters.block_destructions_tracked = self.block_destructions.len();
        expired
    }

    pub fn apply_block_event(&mut self, event: ProtocolBlockEvent) {
        self.counters.block_events_received += 1;
        let pos = protocol_block_pos(event.pos);
        self.block_events.push(BlockEventRecord {
            pos,
            b0: event.b0,
            b1: event.b1,
            block_id: event.block_id,
        });
        self.counters.block_events_tracked = self.block_events.len();
        self.update_chest_lid_from_block_event(pos, event.b0, event.b1);
        self.update_bell_shake_from_block_event(pos, event.b0, event.b1);
        self.update_shulker_box_lid_from_block_event(pos, event.b0, event.b1);
        self.update_decorated_pot_wobble_from_block_event(pos, event.b0, event.b1);
        self.update_end_gateway_from_block_event(pos, event.b0);
    }

    pub fn apply_level_event(
        &mut self,
        event: ProtocolLevelEvent,
    ) -> Option<JukeboxLevelEventState> {
        self.counters.level_events_received += 1;
        self.level_events.push(LevelEventRecord {
            event_type: event.event_type,
            pos: protocol_block_pos(event.pos),
            data: event.data,
            global: event.global,
        });
        self.counters.level_events_tracked = self.level_events.len();
        self.record_jukebox_level_event(event)
    }

    pub fn block_destructions(&self) -> &[BlockDestructionProgress] {
        &self.block_destructions
    }

    pub fn block_destruction(&self, id: i32) -> Option<&BlockDestructionProgress> {
        self.block_destructions
            .iter()
            .find(|progress| progress.id == id)
    }

    pub fn block_events(&self) -> &[BlockEventRecord] {
        &self.block_events
    }

    pub fn level_events(&self) -> &[LevelEventRecord] {
        &self.level_events
    }

    pub fn last_block_changed_ack(&self) -> Option<&BlockChangedAckState> {
        self.last_block_changed_ack.as_ref()
    }

    pub fn local_block_predictions(&self) -> &[LocalBlockPredictionState] {
        &self.local_block_predictions
    }

    pub(crate) fn record_local_block_prediction(
        &mut self,
        sequence: i32,
        pos: BlockPos,
        server_block_state_id: i32,
        predicted_block_state_id: i32,
        player_position: Option<ProtocolVec3d>,
    ) {
        self.local_block_predictions
            .retain(|prediction| prediction.sequence != sequence && prediction.pos != pos);
        self.local_block_predictions
            .push(LocalBlockPredictionState {
                sequence,
                pos,
                server_block_state_id,
                predicted_block_state_id,
                player_position,
            });
        self.local_block_predictions
            .sort_by_key(|prediction| prediction.sequence);
        self.counters.local_block_predictions_created += 1;
        self.update_local_block_prediction_count();
    }

    pub(crate) fn update_local_block_prediction_server_state(
        &mut self,
        pos: BlockPos,
        server_block_state_id: i32,
    ) -> bool {
        if server_block_state_id < 0 {
            return false;
        }
        let Some(prediction) = self
            .local_block_predictions
            .iter_mut()
            .find(|prediction| prediction.pos == pos)
        else {
            return false;
        };
        prediction.server_block_state_id = server_block_state_id;
        true
    }

    pub(crate) fn take_local_block_predictions_through_sequence(
        &mut self,
        sequence: i32,
    ) -> Vec<LocalBlockPredictionState> {
        let mut ended = Vec::new();
        self.local_block_predictions.retain(|prediction| {
            if prediction.sequence <= sequence {
                ended.push(*prediction);
                false
            } else {
                true
            }
        });
        self.update_local_block_prediction_count();
        ended
    }

    pub(crate) fn update_local_block_prediction_count(&mut self) {
        self.counters.local_block_predictions_tracked = self.local_block_predictions.len();
    }
}

fn crossed_block_destruction_expiry_scan(previous_tick: u32, current_tick: u32) -> bool {
    previous_tick / BLOCK_DESTRUCTION_EXPIRY_SCAN_INTERVAL_TICKS
        != current_tick / BLOCK_DESTRUCTION_EXPIRY_SCAN_INTERVAL_TICKS
}

fn block_destruction_expiry_scan_tick(current_tick: u32) -> u32 {
    (current_tick / BLOCK_DESTRUCTION_EXPIRY_SCAN_INTERVAL_TICKS)
        * BLOCK_DESTRUCTION_EXPIRY_SCAN_INTERVAL_TICKS
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        BlockChangedAck, BlockPos as ProtocolBlockPos, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
        PlayLogin as ProtocolPlayLogin,
    };

    #[test]
    fn tracks_block_changed_ack_sequence_and_counter() {
        let mut store = WorldStore::new();

        store.apply_block_changed_ack(BlockChangedAck { sequence: 17 });

        assert_eq!(
            store.last_block_changed_ack(),
            Some(&BlockChangedAckState { sequence: 17 })
        );
        assert_eq!(store.counters().block_changed_ack_packets, 1);

        store.apply_block_changed_ack(BlockChangedAck { sequence: 18 });

        assert_eq!(
            store.last_block_changed_ack(),
            Some(&BlockChangedAckState { sequence: 18 })
        );
        assert_eq!(store.counters().block_changed_ack_packets, 2);
    }

    #[test]
    fn tracks_block_destruction_progress_by_id() {
        let mut store = WorldStore::new();

        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 7,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 3,
        }));
        assert_eq!(
            store.block_destruction(7),
            Some(&BlockDestructionProgress {
                id: 7,
                pos: BlockPos {
                    x: 12,
                    y: 64,
                    z: -5,
                },
                progress: 3,
                updated_render_tick: 0,
            })
        );
        assert_eq!(store.counters().block_destructions_received, 1);
        assert_eq!(store.counters().block_destructions_tracked, 1);
        assert_eq!(store.counters().block_destructions_removed, 0);

        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 7,
            pos: ProtocolBlockPos {
                x: 13,
                y: 65,
                z: -6,
            },
            progress: 9,
        }));
        assert_eq!(store.block_destructions().len(), 1);
        assert_eq!(
            store.block_destruction(7),
            Some(&BlockDestructionProgress {
                id: 7,
                pos: BlockPos {
                    x: 13,
                    y: 65,
                    z: -6,
                },
                progress: 9,
                updated_render_tick: 0,
            })
        );

        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 7,
            pos: ProtocolBlockPos {
                x: 13,
                y: 65,
                z: -6,
            },
            progress: 10,
        }));
        assert!(store.block_destructions().is_empty());
        assert_eq!(store.counters().block_destructions_received, 3);
        assert_eq!(store.counters().block_destructions_tracked, 0);
        assert_eq!(store.counters().block_destructions_removed, 1);

        assert!(!store.apply_block_destruction(ProtocolBlockDestruction {
            id: 99,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            progress: 255,
        }));
        assert_eq!(store.counters().block_destructions_received, 4);
        assert_eq!(store.counters().block_destructions_removed, 1);
        assert_eq!(store.counters().block_destructions_ignored, 1);
    }

    #[test]
    fn block_destruction_progress_expires_after_vanilla_render_tick_window() {
        let mut store = WorldStore::new();
        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 7,
            pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
            progress: 3,
        }));

        assert_eq!(store.advance_block_destruction_render_ticks(399), 0);
        assert_eq!(store.block_destructions().len(), 1);
        assert_eq!(store.advance_block_destruction_render_ticks(1), 0);
        assert_eq!(store.block_destructions().len(), 1);
        assert_eq!(store.advance_block_destruction_render_ticks(20), 1);

        assert!(store.block_destructions().is_empty());
        assert_eq!(store.counters().block_destructions_expired, 1);
        assert_eq!(store.counters().block_destructions_tracked, 0);
        assert_eq!(store.counters().block_destructions_removed, 0);
    }

    #[test]
    fn block_destruction_batched_tick_advance_expires_at_scan_tick() {
        let mut store = WorldStore::new();
        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 7,
            pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
            progress: 3,
        }));

        assert_eq!(store.advance_block_destruction_render_ticks(401), 0);
        assert_eq!(store.block_destructions().len(), 1);
        assert_eq!(store.advance_block_destruction_render_ticks(19), 1);

        assert!(store.block_destructions().is_empty());
        assert_eq!(store.counters().block_destructions_expired, 1);
    }

    #[test]
    fn block_destruction_update_refreshes_expiry_tick() {
        let mut store = WorldStore::new();
        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 7,
            pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
            progress: 3,
        }));
        assert_eq!(store.advance_block_destruction_render_ticks(399), 0);

        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 7,
            pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
            progress: 6,
        }));
        assert_eq!(
            store
                .block_destruction(7)
                .map(|progress| progress.updated_render_tick),
            Some(399)
        );

        assert_eq!(store.advance_block_destruction_render_ticks(20), 0);
        assert_eq!(
            store.block_destruction(7).map(|progress| progress.progress),
            Some(6)
        );
        assert_eq!(store.advance_block_destruction_render_ticks(400), 1);
        assert!(store.block_destructions().is_empty());
    }

    #[test]
    fn tracks_transient_block_and_level_events() {
        let mut store = WorldStore::new();

        store.apply_block_event(ProtocolBlockEvent {
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            b0: 1,
            b1: 5,
            block_id: 123,
        });
        store.apply_level_event(ProtocolLevelEvent {
            event_type: 2001,
            pos: ProtocolBlockPos {
                x: 13,
                y: 65,
                z: -6,
            },
            data: 9,
            global: true,
        });

        assert_eq!(
            store.block_events(),
            &[BlockEventRecord {
                pos: BlockPos {
                    x: 12,
                    y: 64,
                    z: -5,
                },
                b0: 1,
                b1: 5,
                block_id: 123,
            }]
        );
        assert_eq!(
            store.level_events(),
            &[LevelEventRecord {
                event_type: 2001,
                pos: BlockPos {
                    x: 13,
                    y: 65,
                    z: -6,
                },
                data: 9,
                global: true,
            }]
        );
        assert_eq!(store.counters().block_events_received, 1);
        assert_eq!(store.counters().block_events_tracked, 1);
        assert_eq!(store.counters().level_events_received, 1);
        assert_eq!(store.counters().level_events_tracked, 1);

        store.apply_login(&ProtocolPlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:the_nether".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 12345,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 32,
            },
            enforces_secure_chat: true,
        });

        assert!(store.block_events().is_empty());
        assert!(store.level_events().is_empty());
        assert_eq!(store.counters().block_events_tracked, 0);
        assert_eq!(store.counters().level_events_tracked, 0);
    }
}
