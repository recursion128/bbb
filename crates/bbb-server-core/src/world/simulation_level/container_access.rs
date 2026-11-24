use super::*;
use crate::block_behavior::chest as chest_block;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

#[derive(Clone, Copy)]
pub(crate) struct BlockContainer {
    positions: [BlockPos; 2],
    sizes: [usize; 2],
    parts: usize,
}

impl BlockContainer {
    pub(crate) fn size(self) -> usize {
        self.sizes.iter().sum()
    }
    pub(crate) fn positions(&self) -> &[BlockPos] {
        &self.positions[..self.parts]
    }
    pub(crate) fn slot(self, slot: usize) -> Option<(BlockPos, usize)> {
        if slot < self.sizes[0] {
            Some((self.positions[0], slot))
        } else if self.parts == 2 && slot < self.size() {
            Some((self.positions[1], slot - self.sizes[0]))
        } else {
            None
        }
    }
}

impl LevelCtx<'_, '_> {
    pub(crate) fn chest_positions_in(
        &self,
        bounds: bbb_server_types::geometry::Aabb,
    ) -> impl Iterator<Item = BlockPos> + '_ {
        let entities = self.chunks.block_entities();
        let min_x = (bounds.min_x.floor() as i32).div_euclid(16);
        let max_x = (bounds.max_x.floor() as i32).div_euclid(16);
        let min_z = (bounds.min_z.floor() as i32).div_euclid(16);
        let max_z = (bounds.max_z.floor() as i32).div_euclid(16);
        (min_x..=max_x)
            .flat_map(move |x| (min_z..=max_z).flat_map(move |z| entities.chunk((x, z))))
            .filter_map(move |entity| {
                let pos = entity.block_pos();
                (entity.chest().is_some()
                    && f64::from(pos.x) >= bounds.min_x
                    && f64::from(pos.x) < bounds.max_x
                    && f64::from(pos.y) >= bounds.min_y
                    && f64::from(pos.y) < bounds.max_y
                    && f64::from(pos.z) >= bounds.min_z
                    && f64::from(pos.z) < bounds.max_z)
                    .then_some(pos)
            })
    }

    pub(crate) fn container_is_locked(&self, container: BlockContainer) -> bool {
        container.positions().iter().any(|&pos| {
            self.chunks
                .block_entities()
                .get(pos)
                .and_then(BlockEntity::chest)
                .is_none_or(|chest| chest.is_locked())
        })
    }

    pub(crate) fn block_container_unblocked(
        chunks: &crate::chunk::ChunkStore,
        pos: BlockPos,
    ) -> Option<BlockContainer> {
        let entity = chunks.block_entities().get(pos)?;
        let size = entity.container_slots()?.len();
        let state = crate::world::level_blocks::stored_block_state(chunks, pos);
        let combine = if entity.chest().is_some() && chest_block::is_chest_block(state) {
            chest_block::combine_with_blocking(
                pos,
                state,
                true,
                &mut |at| crate::world::level_blocks::stored_block_state(chunks, at),
                &mut |at| {
                    chunks
                        .block_entities()
                        .get(at)
                        .and_then(BlockEntity::chest)
                        .is_some()
                },
                &mut |_| false,
            )
        } else {
            chest_block::ChestCombine::Single(pos)
        };
        let (positions, sizes, parts) = match combine {
            chest_block::ChestCombine::None => return None,
            chest_block::ChestCombine::Single(at) => ([at, at], [size, 0], 1),
            chest_block::ChestCombine::Double(first, second) => ([first, second], [27, 27], 2),
        };
        Some(BlockContainer {
            positions,
            sizes,
            parts,
        })
    }

    pub(crate) fn block_container(
        &self,
        pos: BlockPos,
        ignore_being_blocked: bool,
    ) -> Option<BlockContainer> {
        if ignore_being_blocked {
            return Self::block_container_unblocked(self.chunks, pos);
        }
        let entity = self.chunks.block_entities().get(pos)?;
        let size = entity.container_slots()?.len();
        let state = self.block_state(pos);
        let combine = if entity.chest().is_some() && chest_block::is_chest_block(state) {
            chest_block::combine_with_blocking(
                pos,
                state,
                false,
                &mut |at| self.block_state(at),
                &mut |at| {
                    self.chunks
                        .block_entities()
                        .get(at)
                        .and_then(BlockEntity::chest)
                        .is_some()
                },
                &mut |at| self.cat_sitting_on_chest(at),
            )
        } else {
            chest_block::ChestCombine::Single(pos)
        };
        let (positions, sizes, parts) = match combine {
            chest_block::ChestCombine::None => return None,
            chest_block::ChestCombine::Single(at) => ([at, at], [size, 0], 1),
            chest_block::ChestCombine::Double(first, second) => ([first, second], [27, 27], 2),
        };
        Some(BlockContainer {
            positions,
            sizes,
            parts,
        })
    }

    // ItemCommands.java:372-378: commands use the block entity, not a compound chest.
    pub(in crate::world) fn get_block_entity_item(
        &mut self,
        pos: BlockPos,
        slot: usize,
    ) -> CoreResult<Option<RuntimeItemStack>> {
        let Some(container) = self.block_entity_container(pos) else {
            return Ok(None);
        };
        self.get_container_item(container, slot)
    }

    pub(in crate::world) fn block_entity_container(&self, pos: BlockPos) -> Option<BlockContainer> {
        let size = self
            .chunks
            .block_entities()
            .get(pos)?
            .container_slots()?
            .len();
        Some(BlockContainer {
            positions: [pos, pos],
            sizes: [size, 0],
            parts: 1,
        })
    }

    pub(crate) fn get_container_item(
        &mut self,
        container: BlockContainer,
        slot: usize,
    ) -> CoreResult<Option<RuntimeItemStack>> {
        let Some((pos, slot)) = container.slot(slot) else {
            return Ok(None);
        };
        let Some(entity) = self.chunks.block_entities().get(pos) else {
            return Ok(None);
        };
        // CrafterBlockEntity.java:156-169: getItem and isEmpty read raw slots.
        if entity.crafter().is_none() {
            self.unpack_container_loot(pos, None)?;
        }
        Ok(self
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::container_slots)
            .and_then(|items| items.get(slot).cloned()))
    }

    pub(in crate::world) fn container_is_empty(
        &mut self,
        container: BlockContainer,
    ) -> CoreResult<bool> {
        // CompoundContainer.java:22-24: leave the second half untouched when the first is nonempty.
        for &pos in container.positions() {
            if self
                .chunks
                .block_entities()
                .get(pos)
                .is_some_and(|entity| entity.crafter().is_none())
            {
                self.unpack_container_loot(pos, None)?;
            }
            if self
                .chunks
                .block_entities()
                .get(pos)
                .and_then(BlockEntity::container_slots)
                .is_some_and(|items| items.iter().any(|item| !item.is_empty()))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(in crate::world) fn remove_container_item(
        &mut self,
        container: BlockContainer,
        slot: usize,
    ) -> CoreResult<RuntimeItemStack> {
        let Some((pos, local)) = container.slot(slot) else {
            return Ok(RuntimeItemStack::empty());
        };
        let Some(mut item) = self.get_container_item(container, slot)? else {
            return Ok(RuntimeItemStack::empty());
        };
        // ContainerHelper.java:14-15: an empty slot is never mutated by removeItem.
        if item.is_empty() {
            return Ok(RuntimeItemStack::empty());
        }
        let removed = item.split(1);
        let Some(entity) = self.chunks.block_entities_mut().get_mut(pos) else {
            return Ok(RuntimeItemStack::empty());
        };
        let state = entity.block_state();
        let marks = entity.hopper().is_none() && entity.decorated_pot().is_none();
        entity.set_container_item_count(local, item.count())?;
        if marks {
            self.block_entity_set_changed(pos, state);
        }
        Ok(removed)
    }

    pub(crate) fn container_analog_output(&mut self, pos: BlockPos) -> i32 {
        let Some(container) = self.block_container(pos, false) else {
            return 0;
        };
        // AbstractContainerMenu.java:756-765: each getItem can fill and notify before the next read.
        crate::inventory::redstone_signal_from_container_access(
            container.size(),
            |slot| match self.get_container_item(container, slot) {
                Ok(item) => Ok(item.unwrap_or_else(RuntimeItemStack::empty)),
                Err(error) => {
                    self.record_item_failure(pos, error);
                    Err(())
                }
            },
            |_| (),
        )
        .unwrap_or(0)
    }
}
