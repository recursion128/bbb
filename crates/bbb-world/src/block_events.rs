use bbb_protocol::packets::{
    BlockDestruction as ProtocolBlockDestruction, BlockEvent as ProtocolBlockEvent,
    LevelEvent as ProtocolLevelEvent,
};
use serde::{Deserialize, Serialize};

use crate::{protocol_block_pos, BlockPos, WorldStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDestructionProgress {
    pub id: i32,
    pub pos: BlockPos,
    pub progress: u8,
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

impl WorldStore {
    pub fn apply_block_destruction(&mut self, update: ProtocolBlockDestruction) -> bool {
        self.counters.block_destructions_received += 1;
        if update.progress < 10 {
            let progress = BlockDestructionProgress {
                id: update.id,
                pos: protocol_block_pos(update.pos),
                progress: update.progress,
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
        self.counters.block_destructions_tracked = self.block_destructions.len();
        removed > 0
    }

    pub fn apply_block_event(&mut self, event: ProtocolBlockEvent) {
        self.counters.block_events_received += 1;
        self.block_events.push(BlockEventRecord {
            pos: protocol_block_pos(event.pos),
            b0: event.b0,
            b1: event.b1,
            block_id: event.block_id,
        });
        self.counters.block_events_tracked = self.block_events.len();
    }

    pub fn apply_level_event(&mut self, event: ProtocolLevelEvent) {
        self.counters.level_events_received += 1;
        self.level_events.push(LevelEventRecord {
            event_type: event.event_type,
            pos: protocol_block_pos(event.pos),
            data: event.data,
            global: event.global,
        });
        self.counters.level_events_tracked = self.level_events.len();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        BlockPos as ProtocolBlockPos, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
        PlayLogin as ProtocolPlayLogin,
    };

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
