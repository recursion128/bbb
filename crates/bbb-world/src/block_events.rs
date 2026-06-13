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
