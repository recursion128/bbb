//! Random-tick position sampling over loaded chunk sections.

use crate::random::GameRandom;
use crate::random_ticks::{random_block_pos, RANDOM_TICK_Y_MASK, SECTION_SIDE};
use bbb_protocol::packets::BlockPos;

use bbb_server_types::chunk_data::ChunkColumn;

/// Positions sampled from sections containing randomly ticking states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomTickSampler {
    min_x: i32,
    min_z: i32,
    tick_speed: i32,
    section_index: usize,
    remaining_in_section: i32,
}

impl RandomTickSampler {
    #[must_use]
    pub const fn new(chunk_x: i32, chunk_z: i32, tick_speed: i32) -> Self {
        Self {
            min_x: chunk_x * SECTION_SIDE,
            min_z: chunk_z * SECTION_SIDE,
            tick_speed,
            section_index: 0,
            remaining_in_section: 0,
        }
    }

    /// Draws one position, reading a section gate only when entering it.
    pub fn next_position(&mut self, column: &ChunkColumn, random: &GameRandom) -> Option<BlockPos> {
        if self.tick_speed <= 0 {
            return None;
        }

        loop {
            if self.section_index >= column.section_count() {
                return None;
            }
            if self.remaining_in_section == 0 {
                if !column
                    .chunk_section(self.section_index)
                    .is_randomly_ticking()
                {
                    self.section_index += 1;
                    continue;
                }
                self.remaining_in_section = self.tick_speed;
            }

            let min_section_y = column.min_y() >> 4;
            let min_y = (min_section_y + self.section_index as i32) * SECTION_SIDE;
            let pos = random_block_pos(random, self.min_x, min_y, self.min_z, RANDOM_TICK_Y_MASK);
            self.remaining_in_section -= 1;
            if self.remaining_in_section == 0 {
                self.section_index += 1;
            }
            return Some(pos);
        }
    }
}

#[cfg(test)]
pub mod tests;
