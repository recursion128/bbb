//! Adapts saved Java upgrade work to the block-owned executor.

use std::collections::BTreeSet;

use crate::random::GameRandom;
use crate::upgrade_data::{execute_upgrade_data, BlockUpgradeData, UpgradeSection};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::packets::BlockPos;
use bbb_server_persistence::chunk_schema::Java26UpgradeData;
use bbb_server_types::block_queries::block_state_has_block_entity;
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, SavedTick};
use bbb_server_types::save::SaveResult;

use super::{invalid_carvers_region, split_world_coord, ChunkStore};

impl ChunkStore {
    pub(super) fn execute_pending_upgrade_data(&mut self, chunk: (i32, i32)) -> SaveResult<()> {
        let Some(saved) = self
            .pending_legacy_generation_work(chunk.0, chunk.1)
            .and_then(|work| work.upgrade_data())
            .cloned()
        else {
            return Ok(());
        };
        let input = translate_upgrade_data(self, chunk, &saved)?;
        let mut level = ChunkUpgradeLevel::new(self);
        execute_upgrade_data(&mut level, &input);
        let ChunkUpgradeLevel {
            mut touched_chunks, ..
        } = level;
        self.complete_pending_upgrade_data(chunk);
        // The cleared UpgradeData must reach disk even when nothing changed.
        touched_chunks.insert(chunk);
        for rewrite in touched_chunks {
            self.mark_whole_rewrite_pending(rewrite)?;
        }
        Ok(())
    }
}

fn translate_upgrade_data(
    store: &ChunkStore,
    chunk: (i32, i32),
    saved: &Java26UpgradeData,
) -> SaveResult<BlockUpgradeData> {
    let column = store.column_at(chunk).ok_or_else(|| {
        invalid_carvers_region(
            [chunk.0, chunk.1],
            "saved UpgradeData requires its target column",
        )
    })?;
    let min_section = column.min_y().div_euclid(16);
    let height = i32::try_from(column.section_count())
        .ok()
        .and_then(|sections| sections.checked_mul(16))
        .ok_or_else(|| {
            invalid_carvers_region([chunk.0, chunk.1], "UpgradeData build height overflows")
        })?;
    let max_y = column
        .min_y()
        .checked_add(height)
        .and_then(|exclusive| exclusive.checked_sub(1))
        .ok_or_else(|| {
            invalid_carvers_region([chunk.0, chunk.1], "UpgradeData maximum Y overflows")
        })?;
    let mut inside = Vec::new();
    if let Some(indices) = saved.indices() {
        inside.reserve(indices.len());
        for (&relative, packed) in indices {
            let relative = i32::try_from(relative).map_err(|_| {
                invalid_carvers_region(
                    [chunk.0, chunk.1],
                    "UpgradeData section index does not fit i32",
                )
            })?;
            let section_y = min_section.checked_add(relative).ok_or_else(|| {
                invalid_carvers_region(
                    [chunk.0, chunk.1],
                    "UpgradeData absolute section Y overflows",
                )
            })?;
            inside.push(UpgradeSection {
                section_y,
                indices: packed.clone(),
            });
        }
    }
    let mut sides = [false; 8];
    for (index, side) in sides.iter_mut().enumerate() {
        *side = saved.sides() & (1_u8 << index) != 0;
    }
    let neighbor_block_ticks = saved
        .neighbor_block_ticks()
        .unwrap_or_default()
        .iter()
        .map(SavedTick::from_saved)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            invalid_carvers_region(
                [chunk.0, chunk.1],
                format!("invalid UpgradeData block tick: {error}"),
            )
        })?;
    let neighbor_fluid_ticks = saved
        .neighbor_fluid_ticks()
        .unwrap_or_default()
        .iter()
        .map(SavedTick::from_saved)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            invalid_carvers_region(
                [chunk.0, chunk.1],
                format!("invalid UpgradeData fluid tick: {error}"),
            )
        })?;
    Ok(BlockUpgradeData {
        chunk_x: chunk.0,
        chunk_z: chunk.1,
        min_y: column.min_y(),
        max_y,
        inside,
        sides,
        neighbor_block_ticks,
        neighbor_fluid_ticks,
    })
}

pub(crate) struct ChunkUpgradeLevel<'a> {
    store: &'a mut ChunkStore,
    touched_chunks: BTreeSet<(i32, i32)>,

    random: GameRandom,
}

impl<'a> ChunkUpgradeLevel<'a> {
    pub(crate) fn get_min_y(&self) -> i32 {
        self.store.min_y()
    }

    fn new(store: &'a mut ChunkStore) -> Self {
        Self {
            store,
            touched_chunks: BTreeSet::new(),

            random: GameRandom::with_seed(0),
        }
    }

    // Loading a chunk has no live level to receive drops or neighbor updates.
    pub(crate) fn replace_state(&mut self, pos: BlockPos, state: i32) -> Option<bool> {
        vanilla_block_state_block_type_id(state)?;
        let (chunk_x, local_x) = split_world_coord(pos.x);
        let (chunk_z, local_z) = split_world_coord(pos.z);
        let chunk = (chunk_x, chunk_z);
        let old = self
            .store
            .column_at(chunk)?
            .get_block_state(local_x, pos.y, local_z)?;
        vanilla_block_state_block_type_id(old)?;
        if old == state {
            return Some(false);
        }
        let entity = self.store.block_entities().get(pos);
        let old_has_entity = block_state_has_block_entity(old);
        let new_has_entity = block_state_has_block_entity(state);
        if old_has_entity != entity.is_some()
            || new_has_entity != entity.is_some()
            || entity.is_some_and(|entity| !entity.is_valid_block_state(state))
        {
            return None;
        }
        self.store
            .chunk_mut(chunk)?
            .set_block_state(local_x, pos.y, local_z, state)?;
        self.store
            .update_poi_on_block_change([pos.x, pos.y, pos.z], old, state);
        if new_has_entity {
            self.store
                .block_entities_mut()
                .rebind_block_state(pos, state);
        }
        self.touched_chunks.insert(chunk);
        Some(true)
    }

    pub(crate) fn append_block_tick(&mut self, tick: SavedTick<BlockTickType>) -> Option<()> {
        let (chunk_x, _) = split_world_coord(tick.pos.x);
        let (chunk_z, _) = split_world_coord(tick.pos.z);
        let chunk = (chunk_x, chunk_z);
        self.store
            .get_block_state(tick.pos.x, tick.pos.y, tick.pos.z)?;
        let column = &mut self.store.chunk_mut(chunk)?.column;
        if column.block_ticks_mut().push_saved_tick(tick) {
            self.touched_chunks.insert(chunk);
        }
        Some(())
    }

    pub(crate) fn append_fluid_tick(&mut self, tick: SavedTick<FluidTickType>) -> Option<()> {
        let (chunk_x, _) = split_world_coord(tick.pos.x);
        let (chunk_z, _) = split_world_coord(tick.pos.z);
        let chunk = (chunk_x, chunk_z);
        self.store
            .get_block_state(tick.pos.x, tick.pos.y, tick.pos.z)?;
        let column = &mut self.store.chunk_mut(chunk)?.column;
        if column.fluid_ticks_mut().push_saved_tick(tick) {
            self.touched_chunks.insert(chunk);
        }
        Some(())
    }

    pub(crate) fn state_at(&self, pos: BlockPos) -> Option<i32> {
        self.store.get_block_state(pos.x, pos.y, pos.z)
    }
}

impl ChunkUpgradeLevel<'_> {
    pub(crate) fn swap_chest_contents(&mut self, first: BlockPos, second: BlockPos) -> Option<()> {
        let first_items = self
            .store
            .block_entities()
            .get(first)?
            .chest()?
            .items()
            .to_vec();
        let second_items = self
            .store
            .block_entities()
            .get(second)?
            .chest()?
            .items()
            .to_vec();
        if !self
            .store
            .block_entities_mut()
            .get_mut(first)?
            .chest_mut()?
            .set_items(second_items)
            || !self
                .store
                .block_entities_mut()
                .get_mut(second)?
                .chest_mut()?
                .set_items(first_items)
        {
            return None;
        }
        self.store.block_entities_mut().set_changed(first);
        self.store.block_entities_mut().set_changed(second);
        self.touched_chunks
            .insert((first.x.div_euclid(16), first.z.div_euclid(16)));
        self.touched_chunks
            .insert((second.x.div_euclid(16), second.z.div_euclid(16)));

        Some(())
    }
}

impl ChunkUpgradeLevel<'_> {
    pub(crate) fn level_random(&self) -> &GameRandom {
        &self.random
    }
}

#[cfg(test)]
mod tests;
