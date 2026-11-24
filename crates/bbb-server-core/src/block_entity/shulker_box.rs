//! `ShulkerBoxBlockEntity` (`ShulkerBoxBlockEntity.java:35-270`).
//!
//! Twenty-seven slots, the `Items` schema, the opener count that drives
//! `OPENING`/`CLOSING`, and `WorldlyContainer.canPlaceItemThroughFace`
//! (`:238-240`) which refuses nested shulker boxes.
//! Contents do not scatter on break (`preRemoveSideEffects` is empty,
//! `:163-164`); they leave through the block's loot `CONTENTS` drop. Entity
//! push while the lid moves is an entity-slice question.

use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SHULKER_BOX_ID;
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::Aabb;
use bbb_server_types::Result;

use super::chest::{load_items, saved_items, TAG_ITEMS};
use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::container_fields::{ContainerLootTable, ContainerNameAndLock};
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};
use crate::inventory::redstone_signal_from_container;
use bbb_server_types::item_stack::RuntimeItemStack;

/// `ShulkerBoxBlockEntity.CONTAINER_SIZE` (`ShulkerBoxBlockEntity.java:38`).
pub const SHULKER_BOX_CONTAINER_SIZE: usize = 27;
/// `ShulkerBoxBlockEntity.EVENT_SET_OPEN_COUNT` (`:39`).
pub const EVENT_SET_OPEN_COUNT: i32 = 1;

/// `ShulkerBoxBlockEntity.AnimationStatus` (`:264-269`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationStatus {
    Closed,
    Opening,
    Opened,
    Closing,
}

/// `ShulkerBoxBlockEntity` (`ShulkerBoxBlockEntity.java:35-50`).
#[derive(Debug, Clone, PartialEq)]
pub struct ShulkerBoxBlockEntity {
    items: Vec<RuntimeItemStack>,
    base: ContainerNameAndLock,
    loot_table: ContainerLootTable,
    open_count: i32,
    animation_status: AnimationStatus,
    progress: f32,
    progress_old: f32,
}

impl Default for ShulkerBoxBlockEntity {
    fn default() -> Self {
        Self {
            items: vec![RuntimeItemStack::empty(); SHULKER_BOX_CONTAINER_SIZE],
            base: ContainerNameAndLock::default(),
            loot_table: ContainerLootTable::default(),
            open_count: 0,
            animation_status: AnimationStatus::Closed,
            progress: 0.0,
            progress_old: 0.0,
        }
    }
}

impl ShulkerBoxBlockEntity {
    #[must_use]
    pub fn items(&self) -> Vec<RuntimeItemStack> {
        self.items.to_vec()
    }

    pub fn original_items(&self) -> &[RuntimeItemStack] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut [RuntimeItemStack] {
        &mut self.items
    }

    pub fn set_items(&mut self, items: Vec<RuntimeItemStack>) -> bool {
        if items.len() != self.items.len() {
            return false;
        }
        self.items = items.into_iter().collect();
        true
    }

    /// `BaseContainerBlockEntity.isEmpty` (`BaseContainerBlockEntity.java:89-97`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.iter().all(|item| item.is_empty())
    }

    /// `RandomizableContainerBlockEntity.collectImplicitComponents`
    /// (`RandomizableContainerBlockEntity.java:107-112`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        self.base.collect(components);
        components.set_container(&self.items);
        self.loot_table.collect(components);
    }

    /// `RandomizableContainerBlockEntity.applyImplicitComponents` (`RandomizableContainerBlockEntity.java:97-104`).
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        self.base.apply_components(components)?;
        components.copy_container_into(&mut self.items)?;
        self.loot_table.apply_components(components)
    }

    pub(super) fn randomizable_parts_mut(
        &mut self,
    ) -> (&mut ContainerLootTable, &mut [RuntimeItemStack]) {
        (&mut self.loot_table, &mut self.items)
    }

    /// Whether `unpackLootTable` still has a table to fill.
    #[must_use]
    pub const fn has_loot_table(&self) -> bool {
        self.loot_table.is_set()
    }

    pub(super) const fn loot_table(&self) -> &ContainerLootTable {
        &self.loot_table
    }

    pub fn redstone_signal(shulker: &Self) -> bbb_server_types::Result<i32> {
        redstone_signal_from_container(&shulker.items())
    }

    #[must_use]
    pub fn custom_name(&self) -> Option<&NbtSummaryValue> {
        self.base.custom_name()
    }

    pub fn can_open_record(
        &self,
        is_spectator: bool,
        key: &RuntimeItemStack,
        registries: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> bbb_server_types::Result<bool> {
        if self.loot_table.is_set() && is_spectator {
            return Ok(false);
        }
        Ok(self.base.can_unlock(is_spectator, key, registries))
    }

    #[must_use]
    pub const fn animation_status(&self) -> AnimationStatus {
        self.animation_status
    }

    #[must_use]
    pub const fn is_closed(&self) -> bool {
        matches!(self.animation_status, AnimationStatus::Closed)
    }

    /// The lid is off the closed full-block shape (`getBoundingBox` at
    /// `ShulkerBoxBlockEntity.java:107-110`).
    #[must_use]
    pub fn lid_moved(&self) -> bool {
        self.progress > 0.0
    }

    #[must_use]
    pub const fn open_count(&self) -> i32 {
        self.open_count
    }

    /// ShulkerBoxBlockEntity.java:107-110; Shulker.java:246-263.
    #[must_use]
    pub fn collision_box(&self, facing: Direction) -> Aabb {
        let extent = f64::from(0.5_f32 * self.progress);
        let (x, y, z) = match facing {
            Direction::Down => (0.0, -extent, 0.0),
            Direction::Up => (0.0, extent, 0.0),
            Direction::North => (0.0, 0.0, -extent),
            Direction::South => (0.0, 0.0, extent),
            Direction::West => (-extent, 0.0, 0.0),
            Direction::East => (extent, 0.0, 0.0),
        };
        Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 }).expand_towards(Vec3d { x, y, z })
    }

    /// The band an opening lid swept this tick and the push it gives
    /// (`ShulkerBoxBlockEntity.java:112-131`, `Shulker.java:250-263`). The
    /// band uses the raw progress, not the halved lid extent.
    #[must_use]
    pub fn lid_push(&self, facing: Direction, pos: BlockPos) -> (Aabb, Vec3d) {
        let (from, to) = (f64::from(self.progress_old), f64::from(self.progress));
        let (low, high) = (from.min(to), from.max(to));
        let block = Aabb::of_block(pos);
        let push = high - low + 0.01;
        match facing {
            Direction::Up => (
                Aabb {
                    min_y: block.max_y + low,
                    max_y: block.max_y + high,
                    ..block
                },
                Vec3d {
                    x: 0.0,
                    y: push,
                    z: 0.0,
                },
            ),
            Direction::Down => (
                Aabb {
                    min_y: block.min_y - high,
                    max_y: block.min_y - low,
                    ..block
                },
                Vec3d {
                    x: 0.0,
                    y: -push,
                    z: 0.0,
                },
            ),
            Direction::East => (
                Aabb {
                    min_x: block.max_x + low,
                    max_x: block.max_x + high,
                    ..block
                },
                Vec3d {
                    x: push,
                    y: 0.0,
                    z: 0.0,
                },
            ),
            Direction::West => (
                Aabb {
                    min_x: block.min_x - high,
                    max_x: block.min_x - low,
                    ..block
                },
                Vec3d {
                    x: -push,
                    y: 0.0,
                    z: 0.0,
                },
            ),
            Direction::South => (
                Aabb {
                    min_z: block.max_z + low,
                    max_z: block.max_z + high,
                    ..block
                },
                Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: push,
                },
            ),
            Direction::North => (
                Aabb {
                    min_z: block.min_z - high,
                    max_z: block.min_z - low,
                    ..block
                },
                Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: -push,
                },
            ),
        }
    }

    /// `ShulkerBoxBlockEntity.startOpen` (`:167-181`) as the count half.
    pub fn start_open(&mut self) -> i32 {
        if self.open_count < 0 {
            self.open_count = 0;
        }
        self.open_count += 1;
        self.open_count
    }

    /// `ShulkerBoxBlockEntity.stopOpen` (`:184-194`) as the count half.
    pub fn stop_open(&mut self) -> i32 {
        self.open_count -= 1;
        self.open_count
    }

    /// `ShulkerBoxBlockEntity.triggerEvent` (`:140-155`).
    #[must_use]
    pub fn trigger_event(&mut self, param_a: i32, param_b: i32) -> bool {
        if param_a != EVENT_SET_OPEN_COUNT {
            return false;
        }
        self.open_count = param_b;
        if param_b == 0 {
            self.animation_status = AnimationStatus::Closing;
        }
        if param_b == 1 {
            self.animation_status = AnimationStatus::Opening;
        }
        true
    }

    /// `ShulkerBoxBlockEntity.updateAnimation` (`:66-101`) without
    /// `moveCollidedEntities`, which the world runs from [`Self::lid_push`].
    ///
    /// Answers whether `doNeighborUpdates` (`:157-160`) should run.
    pub fn tick_animation(&mut self) -> bool {
        self.progress_old = self.progress;
        match self.animation_status {
            AnimationStatus::Closed => {
                self.progress = 0.0;
                false
            }
            AnimationStatus::Opening => {
                let started = self.progress_old == 0.0;
                self.progress += 0.1;
                if self.progress >= 1.0 {
                    self.animation_status = AnimationStatus::Opened;
                    self.progress = 1.0;
                    true
                } else {
                    started
                }
            }
            AnimationStatus::Opened => {
                self.progress = 1.0;
                false
            }
            AnimationStatus::Closing => {
                let started = self.progress_old == 1.0;
                self.progress -= 0.1;
                if self.progress <= 0.0 {
                    self.animation_status = AnimationStatus::Closed;
                    self.progress = 0.0;
                    true
                } else {
                    started
                }
            }
        }
    }

    /// `ShulkerBoxBlockEntity.saveAdditional` (`:208-213`), whose
    /// `saveAllItems(..., false)` discards an empty `Items` list
    /// (`ContainerHelper.java:25-38`).
    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        self.base.save(&mut entries);
        if !self.loot_table.try_save(&mut entries) {
            let items = saved_items(&self.items, save)?;
            if !matches!(&items, NbtSummaryValue::List(list) if list.is_empty()) {
                entries.push(entry(TAG_ITEMS, items));
            }
        }
        Ok(entries)
    }

    /// `ShulkerBoxBlockEntity.loadAdditional` (`:202-205`) through `loadFromTag`
    /// (`:215-220`).
    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> Result<()> {
        self.base = ContainerNameAndLock::load(record);
        self.items = vec![RuntimeItemStack::empty(); SHULKER_BOX_CONTAINER_SIZE];
        self.loot_table = ContainerLootTable::try_load(record);
        if !self.loot_table.is_set() {
            load_items(unique_field(record, TAG_ITEMS), &mut self.items, load)?;
        }
        Ok(())
    }
}

/// `BlockItem.canFitInsideContainerItems` (`BlockItem.java:194-196`) and
/// `ShulkerBoxBlockEntity.canPlaceItemThroughFace` (`:238-240`): a shulker
/// box item does not go inside another shulker box.
#[must_use]
pub fn item_is_shulker_box(item_id: Option<i32>) -> bool {
    let Some(item_id) = item_id else {
        return false;
    };
    let Some(item) = vanilla_item_type(item_id) else {
        return false;
    };
    let Some(behavior) = item.block_item_behavior else {
        return false;
    };
    vanilla_block_type(behavior.block_type_id).and_then(|block| block.block_entity_type_id)
        == Some(VANILLA_BLOCK_ENTITY_TYPE_SHULKER_BOX_ID)
}

#[cfg(test)]
mod collision_tests {
    use super::*;

    #[test]
    fn shulker_box_collision_tracks_opening_and_closing_in_all_directions() {
        let cube = Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 });
        for facing in [Direction::Up, Direction::West] {
            let mut shulker = ShulkerBoxBlockEntity::default();
            assert_eq!(shulker.collision_box(facing), cube);
            assert!(shulker.trigger_event(EVENT_SET_OPEN_COUNT, 1));
            assert_eq!(shulker.collision_box(facing), cube);
            shulker.tick_animation();
            let first = shulker.collision_box(facing);
            let expected = f64::from(0.05_f32);
            assert!(
                (first.max_x - first.min_x + first.max_y - first.min_y + first.max_z
                    - first.min_z
                    - 3.0
                    - expected)
                    .abs()
                    < 1e-14
            );
            for _ in 0..9 {
                shulker.tick_animation();
            }
            assert_eq!(shulker.animation_status(), AnimationStatus::Opened);
            let mut full = cube;
            match facing {
                Direction::Down => full.min_y = -0.5,
                Direction::Up => full.max_y = 1.5,
                Direction::North => full.min_z = -0.5,
                Direction::South => full.max_z = 1.5,
                Direction::West => full.min_x = -0.5,
                Direction::East => full.max_x = 1.5,
            }
            assert_eq!(shulker.collision_box(facing), full);
            assert!(shulker.trigger_event(EVENT_SET_OPEN_COUNT, 0));
            assert_eq!(shulker.collision_box(facing), full);
            shulker.tick_animation();
            let closing = shulker.collision_box(facing);
            assert_ne!(closing, full);
            assert_ne!(closing, cube);
            for _ in 0..9 {
                shulker.tick_animation();
            }
            assert_eq!(shulker.animation_status(), AnimationStatus::Closed);
            assert_eq!(shulker.collision_box(facing), cube);
        }
    }
}
