use super::super::simulation_level::container_access::BlockContainer;
use super::*;
use crate::block_entity::hopper::MOVE_ITEM_SPEED;
mod minecart;
mod single_slot;
mod transfer;
use crate::entity::item_slots;
use crate::world::LevelId;
use bbb_server_types::geometry::Aabb;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;
use bbb_server_types::ServerWorldError;

impl ServerWorld {
    pub(in crate::world) fn set_command_block_item(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        slot: usize,
        item: RuntimeItemStack,
    ) -> CoreResult<()> {
        let Some(container) = self.with_level(dimension, |level| level.block_entity_container(pos))
        else {
            return Ok(());
        };
        WorldContainerAccess {
            world: self,
            dimension: *dimension,
            pos,
            facing: bbb_protocol::packets::Direction::Down,
        }
        .set_item(WorldContainer::Block(container), slot, item)
    }

    pub(crate) fn distribute_command_block_loot(
        &mut self,
        dimension: &bbb_server_types::save::Java26LevelKey,
        pos: BlockPos,
        replace: Option<(i32, usize)>,
        drops: &[RuntimeItemStack],
    ) -> std::result::Result<Vec<RuntimeItemStack>, bbb_protocol::NbtSummaryValue> {
        let dimension = self.require_level_id(dimension);
        if let Some(body) = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .filter(|body| body.jukebox().is_some())
        {
            crate::commands::container_slot(
                Some(1),
                pos,
                replace.map_or(0, |(slot, _)| slot),
                true,
            )?;
            let state = body.block_state();
            let mut used = Vec::new();
            let candidates =
                &drops[..replace.map_or(drops.len(), |(_, count)| count.min(1).min(drops.len()))];
            for item in candidates {
                let empty = self
                    .level_by_id(&dimension)
                    .chunks
                    .block_entities()
                    .get(pos)
                    .and_then(BlockEntity::jukebox)
                    .is_some_and(|jukebox| jukebox.item().is_empty());
                if !empty {
                    break;
                }
                if item.is_empty() || !item.has_component(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
                ).map_err(|error| bbb_protocol::component_literal_nbt(error.to_string()))? {
                    continue;
                }
                if let Some(new_state) =
                    crate::block_behavior::jukebox::with_has_record(state, true)
                {
                    if self.set_the_jukebox_item(dimension, pos, new_state, item.clone()) {
                        used.push(item.clone());
                    }
                }
            }
            return Ok(used);
        }
        let container = self.with_level(dimension, |level| level.block_entity_container(pos));
        crate::commands::container_slot(
            container.map(BlockContainer::size),
            pos,
            replace.map_or(0, |(slot, _)| slot),
            true,
        )?;
        let container = WorldContainer::Block(container.expect("validated command container"));
        let resolver = self.item_stack_resolver();
        let mut access = WorldContainerAccess {
            world: self,
            dimension,
            pos,
            facing: bbb_protocol::packets::Direction::Down,
        };
        let mut used = Vec::new();
        let result = (|| -> CoreResult<()> {
            if let Some((start, count)) = replace {
                for offset in 0..count.min(access.size(container) - start as usize) {
                    let slot = start as usize + offset;
                    let item = drops.get(offset).cloned().unwrap_or_default();
                    if access.can_place(container, slot, &item, None)? {
                        access.set_item(container, slot, item.clone())?;
                        used.push(item);
                    }
                }
            } else {
                for drop in drops {
                    let mut item = drop.clone();
                    let mut changed = false;
                    for slot in 0..access.size(container) {
                        if item.is_empty() {
                            break;
                        }
                        let mut current = access.get_item(container, slot)?;
                        if !access.can_place(container, slot, &item, None)? {
                            continue;
                        }
                        // `LootCommand.distributeToContainer` / `canMergeItems`
                        // (LootCommand.java:275-298, :329-331).
                        if current.is_empty() {
                            access.set_item(container, slot, item)?;
                            changed = true;
                            break;
                        }
                        if current.count() <= current.max_stack_size()?
                            && current.same_item_same_effective_components(&item, &resolver)?
                        {
                            let count = item.count().min(item.max_stack_size()? - current.count());
                            item.shrink(count);
                            current.grow(count);
                            access.set_count(container, slot, current.count())?;
                            changed = true;
                        }
                    }
                    if changed {
                        used.push(drop.clone());
                    }
                }
            }
            if !used.is_empty() {
                access.set_changed(container);
            }
            Ok(())
        })();
        result.map_err(|error| bbb_protocol::component_literal_nbt(error.to_string()))?;
        Ok(used)
    }

    fn worldly_container_face(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        face: bbb_protocol::packets::Direction,
    ) -> Option<WorldlyContainerFace> {
        if let Some(face) = self.furnace_hopper_face(dimension, pos, face) {
            return Some(WorldlyContainerFace::Furnace(face));
        }
        self.brewing_stand_hopper_face(dimension, pos, face)
            .map(WorldlyContainerFace::BrewingStand)
    }

    pub(super) fn tick_hopper(&mut self, dimension: &LevelId, pos: BlockPos) {
        let game_time = self.game_time();
        let Some(hopper) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::hopper_mut)
        else {
            return;
        };
        if hopper.begin_transfer_tick(game_time) {
            self.try_move_hopper_items(dimension, pos, None);
        }
    }

    pub(in crate::world) fn hopper_item_inside(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        item_id: i32,
    ) {
        let bounds = crate::block_entity::hopper::SUCK_AABB.moved(bbb_protocol::packets::Vec3d {
            x: f64::from(pos.x),
            y: f64::from(pos.y),
            z: f64::from(pos.z),
        });
        if self.level_by_id(dimension).entities.get(item_id).is_some_and(|entity| {
            entity.is_alive() && entity.bounding_box().intersects(bounds)
                && matches!(entity.data(), crate::entity::EntityData::Item(item) if !item.item().is_empty())
        }) {
            self.try_move_hopper_items(dimension, pos, Some(item_id));
        }
    }

    fn try_move_hopper_items(&mut self, dimension: &LevelId, pos: BlockPos, item_id: Option<i32>) {
        let Some(state) = self
            .level_by_id(dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)
        else {
            return;
        };
        if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
            != Some(bbb_protocol::block_types::VANILLA_BLOCK_HOPPER_ID)
            || bbb_protocol::block_state_types::vanilla_block_state_property(state, "enabled")
                == Some("false")
            || self
                .level_by_id(dimension)
                .chunks
                .block_entities()
                .get(pos)
                .and_then(BlockEntity::hopper)
                .is_none_or(|hopper| hopper.is_on_cooldown())
        {
            return;
        }
        let Some(container) =
            LevelCtx::block_container_unblocked(&self.level_by_id(dimension).chunks, pos)
        else {
            return;
        };
        let mut access = WorldContainerAccess {
            world: self,
            dimension: *dimension,
            pos,
            facing: hopper_facing(state),
        };
        let container = WorldContainer::Block(container);
        let changed = match access.transfer_items(container, item_id) {
            Ok(changed) => changed,
            Err(error) => {
                access
                    .world
                    .with_level(dimension, |level| level.record_item_failure(pos, error));
                return;
            }
        };
        if changed {
            if let Some(hopper) = access
                .world
                .level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::hopper_mut)
            {
                hopper.set_cooldown(MOVE_ITEM_SPEED);
            }
            access.set_changed(container);
        }
    }
}

#[derive(Clone, Copy)]
pub(in crate::world) enum WorldContainer {
    Block(BlockContainer),
    Entity(i32),
    /// The composter state the container was taken from.
    Composter {
        pos: BlockPos,
        state: i32,
    },
    Jukebox(BlockPos),
}

pub(in crate::world) struct WorldContainerAccess<'a> {
    pub(in crate::world) world: &'a mut ServerWorld,
    pub(in crate::world) dimension: LevelId,
    pub(in crate::world) pos: BlockPos,
    pub(in crate::world) facing: bbb_protocol::packets::Direction,
}

impl WorldContainerAccess<'_> {
    pub(in crate::world) fn container_at(&mut self, pos: BlockPos) -> Option<WorldContainer> {
        self.container_at_center(
            pos,
            bbb_protocol::packets::Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
        )
    }

    fn container_at_center(
        &mut self,
        pos: BlockPos,
        center: bbb_protocol::packets::Vec3d,
    ) -> Option<WorldContainer> {
        if let Some(container) = LevelCtx::block_container_unblocked(
            &self.world.level_by_id(&self.dimension).chunks,
            pos,
        ) {
            return Some(WorldContainer::Block(container));
        }
        if let Some(container) = self.single_slot_at(pos) {
            return Some(container);
        }
        let bounds = Aabb {
            min_x: center.x - 0.5,
            min_y: center.y - 0.5,
            min_z: center.z - 0.5,
            max_x: center.x + 0.5,
            max_y: center.y + 0.5,
            max_z: center.z + 0.5,
        };
        let level = self.world.level_by_id(&self.dimension);
        let candidates: Vec<_> = level
            .entities
            .ids_in_box_sections(bounds)
            .into_iter()
            .filter(|id| {
                level.entities.get(*id).is_some_and(|entity| {
                    entity.is_alive()
                        && entity.bounding_box().intersects(bounds)
                        && item_slots::container_size(entity).is_some()
                })
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let index = self
            .world
            .level_mut_by_id(&self.dimension)
            .random
            .next_int(candidates.len() as i32) as usize;
        Some(WorldContainer::Entity(candidates[index]))
    }

    pub(in crate::world) fn is_crafter(&self, container: WorldContainer) -> bool {
        let WorldContainer::Block(container) = container else {
            return false;
        };
        self.world
            .level_by_id(&self.dimension)
            .chunks
            .block_entities()
            .get(container.positions()[0])
            .is_some_and(|entity| entity.crafter().is_some())
    }

    pub(in crate::world) fn max_stack_size(
        &self,
        container: WorldContainer,
        item: &RuntimeItemStack,
    ) -> CoreResult<i32> {
        let max = crate::inventory::container_max_stack_size(item)?;
        if Self::single_slot_size(container).is_some() {
            return Ok(max.min(1));
        }
        let WorldContainer::Block(container) = container else {
            return Ok(max);
        };
        if self
            .world
            .level_by_id(&self.dimension)
            .chunks
            .block_entities()
            .get(container.positions()[0])
            .is_some_and(|entity| entity.chiseled_bookshelf().is_some())
        {
            return Ok(max.min(1));
        }
        Ok(max)
    }

    fn unpack_entity_loot(&mut self, id: i32, slot: usize) -> CoreResult<()> {
        let Some(entity) = self.world.level_by_id(&self.dimension).entities.get(id) else {
            return Ok(());
        };
        if item_slots::container_size(entity).is_none_or(|size| slot >= size) {
            return Ok(());
        }
        let uuid = entity.uuid();
        self.world
            .unpack_item_command_vehicle(&self.dimension, uuid, slot as i32)
            .map_err(ServerWorldError::InvalidConfiguration)
    }

    fn raw_entity_item(&self, id: i32, slot: usize) -> CoreResult<RuntimeItemStack> {
        let Some(entity) = self.world.level_by_id(&self.dimension).entities.get(id) else {
            return Ok(RuntimeItemStack::empty());
        };
        if item_slots::container_size(entity).is_none_or(|size| slot >= size) {
            return Ok(RuntimeItemStack::empty());
        }
        Ok(item_slots::read(entity, slot as i32)
            .map_err(ServerWorldError::InvalidConfiguration)?
            .unwrap_or_else(RuntimeItemStack::empty))
    }
}

impl WorldContainerAccess<'_> {
    pub(in crate::world) fn attached(
        &mut self,
    ) -> Option<(WorldContainer, bbb_protocol::packets::Direction)> {
        let pos = bbb_server_types::directions::relative(self.pos, self.facing);
        self.container_at(pos).map(|container| {
            (
                container,
                bbb_server_types::directions::opposite(self.facing),
            )
        })
    }
    fn source(&mut self) -> Option<WorldContainer> {
        let pos = crate::fluid::above(self.pos);
        self.container_at(pos)
    }
    fn size(&self, container: WorldContainer) -> usize {
        match container {
            WorldContainer::Block(container) => container.size(),
            WorldContainer::Composter { .. } | WorldContainer::Jukebox(_) => {
                Self::single_slot_size(container).unwrap_or(0)
            }
            WorldContainer::Entity(id) => self
                .world
                .level_by_id(&self.dimension)
                .entities
                .get(id)
                .and_then(item_slots::container_size)
                .unwrap_or(0),
        }
    }
    fn slots(
        &self,
        container: WorldContainer,
        direction: Option<bbb_protocol::packets::Direction>,
    ) -> Vec<usize> {
        if let Some(slots) = Self::single_slot_faces(container, direction) {
            return slots;
        }
        let WorldContainer::Block(container) = container else {
            return (0..self.size(container)).collect();
        };
        if container.positions().len() == 1 {
            if let Some(face) = direction.and_then(|direction| {
                self.world.worldly_container_face(
                    &self.dimension,
                    container.positions()[0],
                    direction,
                )
            }) {
                return face.slots().to_vec();
            }
        }
        (0..container.size()).collect()
    }
    fn get_item(&mut self, container: WorldContainer, slot: usize) -> CoreResult<RuntimeItemStack> {
        if let Some(item) = self.single_slot_item(container) {
            return Ok(item);
        }
        let WorldContainer::Block(container) = container else {
            let WorldContainer::Entity(id) = container else {
                unreachable!()
            };
            self.unpack_entity_loot(id, slot)?;
            return self.raw_entity_item(id, slot);
        };
        let Some((pos, local)) = container.slot(slot) else {
            return Ok(RuntimeItemStack::empty());
        };
        let needs_unpack = {
            let chunks = &self.world.level_by_id(&self.dimension).chunks;
            let Some(entity) = chunks.block_entities().get(pos) else {
                return Ok(RuntimeItemStack::empty());
            };
            if entity.crafter().is_some() || entity.container_loot_table_id().is_none() {
                return Ok(entity
                    .container_slots()
                    .and_then(|items| items.get(local).cloned())
                    .unwrap_or_else(RuntimeItemStack::empty));
            }
            true
        };
        debug_assert!(needs_unpack);
        Ok(self
            .world
            .with_level(self.dimension, |level| {
                level.get_container_item(container, slot)
            })?
            .unwrap_or_else(RuntimeItemStack::empty))
    }
    pub(in crate::world) fn set_item(
        &mut self,
        container: WorldContainer,
        slot: usize,
        mut item: RuntimeItemStack,
    ) -> CoreResult<()> {
        if Self::single_slot_size(container).is_some() {
            self.set_single_slot_item(container, item);
            return Ok(());
        }
        let WorldContainer::Block(container) = container else {
            let WorldContainer::Entity(id) = container else {
                unreachable!()
            };
            self.unpack_entity_loot(id, slot)?;
            if let Some(entity) = self
                .world
                .level_mut_by_id(&self.dimension)
                .entities
                .get_mut(id)
            {
                item_slots::set_container_item(entity, slot, item)
                    .map_err(ServerWorldError::InvalidConfiguration)?;
            }
            return Ok(());
        };
        let Some((pos, local)) = container.slot(slot) else {
            return Ok(());
        };
        let randomizable = self
            .world
            .level_mut_by_id(&self.dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .is_some_and(|entity| entity.randomizable_container_mut().is_some());
        if randomizable {
            let max = crate::inventory::container_max_stack_size(&item)?;
            item.set_count(item.count().min(max));
            return self.world.with_level(self.dimension, |level| {
                level.set_randomizable_container_item(pos, local, item)
            });
        }
        if self
            .world
            .level_by_id(&self.dimension)
            .chunks
            .block_entities()
            .get(pos)
            .is_some_and(|entity| entity.furnace().is_some())
        {
            let cooking_time = self
                .world
                .furnace_input_cooking_time(&self.dimension, pos, Some(&item))?
                .unwrap_or(200);
            let resolver = self.world.item_stack_resolver();
            let marks = if let Some(furnace) = self
                .world
                .level_mut_by_id(&self.dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::furnace_mut)
            {
                furnace.set_item(&resolver, local, item, cooking_time)?
            } else {
                false
            };
            if marks {
                self.set_changed(WorldContainer::Block(container));
            }
            return Ok(());
        }
        let Some(mut items) = self.world.container_items_at_by_key(&self.dimension, pos) else {
            return Ok(());
        };
        // `ShelfBlockEntity` inherits `ListBackedContainer.setItemNoUpdate`
        // (ListBackedContainer.java:66-69) which calls `limitSize`;
        // `ChiseledBookShelfBlockEntity.setItem` (ChiseledBookShelfBlockEntity.java:90-97)
        // overrides `setItem` without `limitSize`.
        if self
            .world
            .level_by_id(&self.dimension)
            .chunks
            .block_entities()
            .get(pos)
            .is_some_and(|entity| entity.shelf().is_some())
        {
            item.set_count(
                item.count()
                    .min(self.max_stack_size(WorldContainer::Block(container), &item)?),
            );
        }
        items[local] = item;
        self.world
            .write_container_items_at_by_key(&self.dimension, pos, items)?;
        if self
            .world
            .level_by_id(&self.dimension)
            .chunks
            .block_entities()
            .get(pos)
            .is_some_and(|entity| entity.chiseled_bookshelf().is_none())
        {
            self.set_changed(WorldContainer::Block(container));
        }
        Ok(())
    }
    fn set_count(&mut self, container: WorldContainer, slot: usize, count: i32) -> CoreResult<()> {
        if Self::single_slot_size(container).is_some() {
            if count <= 0 {
                self.set_single_slot_item(container, RuntimeItemStack::empty());
            }
            return Ok(());
        }
        let WorldContainer::Block(container) = container else {
            let WorldContainer::Entity(id) = container else {
                unreachable!()
            };
            if let Some(entity) = self
                .world
                .level_mut_by_id(&self.dimension)
                .entities
                .get_mut(id)
            {
                item_slots::set_container_item_count(entity, slot, count)
                    .map_err(ServerWorldError::InvalidConfiguration)?;
            }
            return Ok(());
        };
        if let Some((pos, local)) = container.slot(slot) {
            if let Some(entity) = self
                .world
                .level_mut_by_id(&self.dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
            {
                entity.set_container_item_count(local, count)?;
            }
        }
        Ok(())
    }
    fn extracts_entire_slot(&self, container: WorldContainer, slot: usize) -> bool {
        let WorldContainer::Block(block) = container else {
            return false;
        };
        let Some((pos, _)) = block.slot(slot) else {
            return false;
        };
        self.world
            .level_by_id(&self.dimension)
            .chunks
            .block_entities()
            .get(pos)
            .is_some_and(|entity| entity.chiseled_bookshelf().is_some())
    }

    fn remove_one(
        &mut self,
        container: WorldContainer,
        slot: usize,
    ) -> CoreResult<RuntimeItemStack> {
        let WorldContainer::Block(block) = container else {
            let mut item = self.get_item(container, slot)?;
            if item.is_empty() {
                return Ok(RuntimeItemStack::empty());
            }
            let removed = item.split(1);
            self.set_count(container, slot, item.count())?;
            return Ok(removed);
        };
        let Some((pos, local)) = block.slot(slot) else {
            return Ok(RuntimeItemStack::empty());
        };
        if self.extracts_entire_slot(container, slot) {
            let item = self.get_item(container, slot)?;
            self.set_item(container, slot, RuntimeItemStack::empty())?;
            return Ok(item);
        }
        // Fast path for hoppers / decorated pots with no pending loot table:
        // neither needs `block_entity_set_changed` (`remove_container_item`).
        let fast_removed = {
            let chunks = &mut self.world.level_mut_by_id(&self.dimension).chunks;
            if let Some(entity) = chunks.block_entities_mut().get_mut(pos) {
                if (entity.hopper().is_some() || entity.decorated_pot().is_some())
                    && entity.container_loot_table_id().is_none()
                {
                    let current = entity
                        .container_slots()
                        .and_then(|items| items.get(local).cloned())
                        .unwrap_or_else(RuntimeItemStack::empty);
                    if current.is_empty() {
                        Some(Ok(RuntimeItemStack::empty()))
                    } else {
                        let mut updated = current;
                        let removed = updated.split(1);
                        Some(
                            entity
                                .set_container_item_count(local, updated.count())
                                .map(|()| removed),
                        )
                    }
                } else {
                    None
                }
            } else {
                Some(Ok(RuntimeItemStack::empty()))
            }
        };
        if let Some(result) = fast_removed {
            return result;
        }
        self.world.with_level(self.dimension, |level| {
            level.remove_container_item(block, slot)
        })
    }
    fn is_empty(&mut self, container: WorldContainer) -> CoreResult<bool> {
        if let Some(item) = self.single_slot_item(container) {
            return Ok(item.is_empty());
        }
        let WorldContainer::Block(container) = container else {
            let WorldContainer::Entity(id) = container else {
                unreachable!()
            };
            // An emptiness check must preserve pending entity loot.
            for slot in 0..self.size(container) {
                if !self.raw_entity_item(id, slot)?.is_empty() {
                    return Ok(false);
                }
            }
            return Ok(true);
        };
        {
            let chunks = &self.world.level_by_id(&self.dimension).chunks;
            let mut needs_unpack = false;
            let mut empty = true;
            for &pos in container.positions() {
                let Some(entity) = chunks.block_entities().get(pos) else {
                    continue;
                };
                if entity.crafter().is_none() && entity.container_loot_table_id().is_some() {
                    needs_unpack = true;
                    break;
                }
                if entity
                    .container_slots()
                    .is_some_and(|items| items.iter().any(|item| !item.is_empty()))
                {
                    empty = false;
                    break;
                }
            }
            if !needs_unpack {
                return Ok(empty);
            }
        }
        self.world
            .with_level(self.dimension, |level| level.container_is_empty(container))
    }
    fn can_place(
        &self,
        container: WorldContainer,
        slot: usize,
        item: &RuntimeItemStack,
        direction: Option<bbb_protocol::packets::Direction>,
    ) -> CoreResult<bool> {
        if let Some(answer) = self.single_slot_can_place(container, item, direction)? {
            return Ok(answer);
        }
        let WorldContainer::Block(block) = container else {
            return Ok(slot < self.size(container));
        };
        let Some((pos, local)) = block.slot(slot) else {
            return Ok(false);
        };
        if let Some(face) = self.world.worldly_container_face(
            &self.dimension,
            pos,
            direction.unwrap_or(bbb_protocol::packets::Direction::Up),
        ) {
            let item = item.summary();
            let is_fuel = self
                .world
                .fuel_values()
                .is_some_and(|values| values.is_fuel(item));
            return Ok(if direction.is_some() {
                face.allows_insert(local, item, is_fuel)
            } else {
                face.allows_place(local, item, is_fuel)
            });
        }
        let Some(entity) = self
            .world
            .level_by_id(&self.dimension)
            .chunks
            .block_entities()
            .get(pos)
        else {
            return Ok(false);
        };
        if entity.shulker_box().is_some() && direction.is_some() {
            return Ok(!shulker_box::item_is_shulker_box(item.item_id()));
        }
        if entity.chiseled_bookshelf().is_some() || entity.shelf().is_some() {
            if entity.chiseled_bookshelf().is_some() {
                let tags = self
                    .world
                    .level_by_id(&self.dimension)
                    .chunks
                    .registry_tags();
                if !crate::block_entity::chiseled_bookshelf::item_is_bookshelf_book_with(
                    item,
                    |registry, tag, id| {
                        tags.map_or_else(
                            || {
                                crate::entity::EntityRegistryTagView::Builtin26_1
                                    .contains(registry, tag, id)
                            },
                            |tags| tags.contains(registry, tag, id),
                        )
                    },
                ) {
                    return Ok(false);
                }
            }
            let current = &entity.container_slots().expect("list-backed container")[local];
            return Ok(
                current.is_empty() || current.count() < self.max_stack_size(container, item)?
            );
        }
        if self.is_crafter(container) {
            return entity
                .crafter()
                .expect("crafter container")
                .can_place_item(&self.world.item_stack_resolver(), local);
        }
        Ok(true)
    }
    fn can_take(
        &mut self,
        container: WorldContainer,
        slot: usize,
        item: &RuntimeItemStack,
        into: WorldContainer,
    ) -> CoreResult<bool> {
        if let Some(answer) = self.single_slot_can_take(container, into)? {
            return Ok(answer);
        }
        let WorldContainer::Block(container) = container else {
            return Ok(slot < self.size(container));
        };
        let Some((pos, local)) = container.slot(slot) else {
            return Ok(false);
        };
        if let Some(face) = self.world.worldly_container_face(
            &self.dimension,
            pos,
            bbb_protocol::packets::Direction::Down,
        ) {
            return Ok(face.allows_take(local, item.summary()));
        }
        if self
            .world
            .level_by_id(&self.dimension)
            .chunks
            .block_entities()
            .get(pos)
            .is_some_and(|entity| entity.chiseled_bookshelf().is_some())
        {
            let resolver = self.world.item_stack_resolver();
            for slot in 0..self.size(into) {
                let current = self.get_item(into, slot)?;
                if current.is_empty()
                    || current.same_item_same_effective_components(item, &resolver)?
                        && current.count() + item.count() <= self.max_stack_size(into, &current)?
                {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        Ok(true)
    }
    fn set_changed(&mut self, container: WorldContainer) {
        let WorldContainer::Block(container) = container else {
            return;
        };
        self.world.with_level(self.dimension, |level| {
            for &pos in container.positions() {
                let Some((state, shelf)) = level
                    .chunks
                    .block_entities()
                    .get(pos)
                    .map(|body| (body.block_state(), body.shelf().is_some()))
                else {
                    continue;
                };
                level.block_entity_set_changed(pos, state);
                // ShelfBlockEntity.java:87-100.
                if shelf {
                    level.event_sink.game_event(
                        crate::level_event::VanillaGameEvent::BLOCK_ACTIVATE,
                        pos,
                        Some(state),
                    );
                    level.send_block_updated(pos, state, state);
                }
            }
        });
    }
    fn receiving_hopper_cooldown(&mut self, from: Option<WorldContainer>, into: WorldContainer) {
        let WorldContainer::Block(into) = into else {
            return;
        };
        let pos = into.positions()[0];
        let from_time = from.and_then(|from| {
            let WorldContainer::Block(from) = from else {
                return None;
            };
            self.world
                .level_by_id(&self.dimension)
                .chunks
                .block_entities()
                .get(from.positions()[0])
                .and_then(BlockEntity::hopper)
                .map(|hopper| hopper.ticked_game_time())
        });
        if let Some(hopper) = self
            .world
            .level_mut_by_id(&self.dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::hopper_mut)
        {
            if hopper.cooldown_time() <= MOVE_ITEM_SPEED {
                let skip =
                    from_time.is_some_and(|from_time| hopper.ticked_game_time() >= from_time);
                hopper.set_cooldown(MOVE_ITEM_SPEED - i32::from(skip));
            }
        }
    }
}

#[cfg(test)]
mod tests;

enum WorldlyContainerFace {
    Furnace(FurnaceHopperFace),
    BrewingStand(BrewingStandHopperFace),
}
impl WorldlyContainerFace {
    fn slots(&self) -> &[usize] {
        match self {
            Self::Furnace(face) => face.slots(),
            Self::BrewingStand(face) => face.slots(),
        }
    }
    fn allows_insert(&self, slot: usize, item: &ItemStackSummary, is_furnace_fuel: bool) -> bool {
        match self {
            Self::Furnace(face) => face.allows_insert(slot, item, is_furnace_fuel),
            Self::BrewingStand(face) => face.allows_insert(slot, item),
        }
    }
    fn allows_place(&self, slot: usize, item: &ItemStackSummary, is_furnace_fuel: bool) -> bool {
        match self {
            Self::Furnace(face) => face.allows_place(slot, item, is_furnace_fuel),
            Self::BrewingStand(face) => face.allows_place(slot, item),
        }
    }
    fn allows_take(&self, slot: usize, item: &ItemStackSummary) -> bool {
        match self {
            Self::Furnace(face) => face.allows_take(slot, item),
            Self::BrewingStand(face) => face.allows_take(slot, item),
        }
    }
}
