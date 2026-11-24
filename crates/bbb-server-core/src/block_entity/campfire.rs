//! `CampfireBlockEntity` (`CampfireBlockEntity.java:41-222`).
//!
//! Four cooking slots and their progress arrays. `cookTick` needs
//! `CampfireCookingRecipe` and stays outside this slice.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::chest::{load_items, saved_items, TAG_ITEMS};
use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};

/// `CampfireBlockEntity.NUM_SLOTS` (`CampfireBlockEntity.java:44`).
pub const NUM_SLOTS: usize = 4;
/// `CampfireBlockEntity.BURN_COOL_SPEED` (`CampfireBlockEntity.java:42`).
const COOL_SPEED: i32 = 2;
const TAG_COOKING_TIMES: &str = "CookingTimes";
const TAG_COOKING_TOTAL_TIMES: &str = "CookingTotalTimes";

/// What one `cookTick` pass produced: the slots that finished this tick, in
/// slot order, and whether any slot advanced at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampfireCookTick {
    pub finished: Vec<usize>,
    pub changed: bool,
}

/// `CampfireBlockEntity` (`CampfireBlockEntity.java:41-47`).
#[derive(Debug, Clone, PartialEq)]
pub struct CampfireBlockEntity {
    items: Vec<RuntimeItemStack>,
    cooking_progress: [i32; NUM_SLOTS],
    cooking_time: [i32; NUM_SLOTS],
}

impl Default for CampfireBlockEntity {
    fn default() -> Self {
        Self {
            items: vec![RuntimeItemStack::empty(); NUM_SLOTS],
            cooking_progress: [0; NUM_SLOTS],
            cooking_time: [0; NUM_SLOTS],
        }
    }
}

impl CampfireBlockEntity {
    /// `CampfireBlockEntity.collectImplicitComponents` (`CampfireBlockEntity.java:213-216`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        components.set_container(&self.items);
    }

    /// `CampfireBlockEntity.applyImplicitComponents` (`CampfireBlockEntity.java:207-210`).
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        components.copy_container_into(&mut self.items)
    }

    pub(super) fn dropped_slots_mut(&mut self) -> &mut [RuntimeItemStack] {
        &mut self.items
    }

    #[must_use]
    pub fn items(&self) -> Vec<RuntimeItemStack> {
        self.items.to_vec()
    }

    pub fn original_items(&self) -> &[RuntimeItemStack] {
        &self.items
    }

    #[must_use]
    pub fn cooking_progress(&self) -> &[i32] {
        &self.cooking_progress
    }

    #[must_use]
    pub fn cooking_time(&self) -> &[i32] {
        &self.cooking_time
    }

    pub fn set_slot(&mut self, slot: usize, item: RuntimeItemStack) -> bool {
        if slot >= self.items.len() {
            return false;
        }
        self.items[slot] = if item.is_empty() {
            RuntimeItemStack::empty()
        } else {
            item
        };
        true
    }

    pub fn set_cooking(&mut self, slot: usize, progress: i32, total: i32) -> bool {
        if slot >= NUM_SLOTS {
            return false;
        }
        self.cooking_progress[slot] = progress;
        self.cooking_time[slot] = total;
        true
    }

    /// `CampfireBlockEntity.cookTick` (`CampfireBlockEntity.java:53-83`) down to
    /// the state it owns: every non-empty slot advances one step, and the slots
    /// that reached their cooking time come back for the level to finish.
    ///
    /// `changed` is vanilla's own flag: a tick that found no item at all leaves
    /// the block entity unmarked.
    pub fn cook_tick(&mut self) -> CampfireCookTick {
        let mut finished = Vec::new();
        let mut changed = false;
        for slot in 0..self.items.len() {
            if self.items[slot].is_empty() {
                continue;
            }
            changed = true;
            self.cooking_progress[slot] += 1;
            if self.cooking_progress[slot] >= self.cooking_time[slot] {
                finished.push(slot);
            }
        }
        CampfireCookTick { finished, changed }
    }

    /// `CampfireBlockEntity.cooldownTick` (`:87-99`): an unlit campfire loses
    /// two progress steps per slot, clamped to that slot's cooking time.
    pub fn cooldown_tick(&mut self) -> bool {
        let mut changed = false;
        for slot in 0..self.items.len() {
            if self.cooking_progress[slot] > 0 {
                changed = true;
                // `Mth.clamp(cookingProgress - 2, 0, cookingTime)`: the floor is
                // zero, not a saturating subtraction, and the ceiling is the
                // slot's own cooking time (`CampfireBlockEntity.java:89-91`).
                self.cooking_progress[slot] =
                    (self.cooking_progress[slot] - COOL_SPEED).clamp(0, self.cooking_time[slot]);
            }
        }
        changed
    }

    /// The completion half of `cookTick` (`:67-75`): the finished slot is
    /// emptied, and the caller drops what it held.
    pub fn take_finished_slot(&mut self, slot: usize) -> RuntimeItemStack {
        if slot >= self.items.len() {
            return RuntimeItemStack::empty();
        }
        std::mem::replace(&mut self.items[slot], RuntimeItemStack::empty()).clone()
    }

    /// `CampfireBlockEntity.getUpdateTag` (`CampfireBlockEntity.java:159-164`):
    /// `saveAllItems` only, through the resolver-free accumulator the wire
    /// carriers already sent.
    pub(super) fn save_items(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        Ok(vec![entry(TAG_ITEMS, saved_items(&self.items, save)?)])
    }

    /// `CampfireBlockEntity.saveAdditional` (`CampfireBlockEntity.java:147-151`).
    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut entries = vec![entry(TAG_ITEMS, saved_items(&self.items, save)?)];
        entries.push(entry(
            TAG_COOKING_TIMES,
            NbtSummaryValue::IntArray(self.cooking_progress.to_vec()),
        ));
        entries.push(entry(
            TAG_COOKING_TOTAL_TIMES,
            NbtSummaryValue::IntArray(self.cooking_time.to_vec()),
        ));
        Ok(entries)
    }

    /// `CampfireBlockEntity.loadAdditional` (`CampfireBlockEntity.java:130-143`).
    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        self.items = vec![RuntimeItemStack::empty(); NUM_SLOTS];
        load_items(unique_field(record, TAG_ITEMS), &mut self.items, load)?;
        load_int_array(
            unique_field(record, TAG_COOKING_TIMES),
            &mut self.cooking_progress,
        );
        load_int_array(
            unique_field(record, TAG_COOKING_TOTAL_TIMES),
            &mut self.cooking_time,
        );
        Ok(())
    }
}

/// `ValueInput.getIntArray` then `System.arraycopy` (`:134-143`).
fn load_int_array(value: Option<&NbtSummaryValue>, dest: &mut [i32]) {
    match value {
        Some(NbtSummaryValue::IntArray(values)) => {
            let n = dest.len().min(values.len());
            dest[..n].copy_from_slice(&values[..n]);
        }
        _ => dest.fill(0),
    }
}

/// Contents `preRemoveSideEffects` drops (`CampfireBlockEntity.java:200-204`).
#[must_use]
pub fn drop_contents(
    campfire: &CampfireBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    campfire
        .original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}

#[cfg(test)]
mod tests {
    use bbb_protocol::NbtSummaryValue;

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_CAMPFIRE_ID;
    use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
    use bbb_protocol::packets::{BlockPos, ItemStackSummary};

    use crate::block_entity::{new_block_entity, unique_field, BlockEntityRemovalDrops};

    use super::*;

    fn pos() -> BlockPos {
        BlockPos { x: 1, y: 2, z: 3 }
    }

    fn default_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_CAMPFIRE_ID)
            .expect("minecraft:campfire has a default state")
    }

    fn stone() -> RuntimeItemStack {
        RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: Default::default(),
        })
    }

    /// `CampfireBlockEntity.cookTick` (`CampfireBlockEntity.java:53-83`): only
    /// non-empty slots advance, and a slot that reaches its cooking time comes
    /// back for the level to finish.
    #[test]
    fn cook_tick_advances_only_filled_slots_and_reports_the_finished_ones() {
        let mut campfire = CampfireBlockEntity::default();
        let empty = campfire.cook_tick();
        assert_eq!(empty.finished, Vec::<usize>::new());
        assert!(!empty.changed);

        assert!(campfire.set_slot(1, stone()));
        assert!(campfire.set_slot(3, stone()));
        assert!(campfire.set_cooking(1, 0, 3));
        assert!(campfire.set_cooking(3, 0, 2));
        let first = campfire.cook_tick();
        assert_eq!(first.finished, Vec::<usize>::new());
        assert!(first.changed);
        assert_eq!(campfire.cooking_progress(), &[0, 1, 0, 1]);
        let second = campfire.cook_tick();
        assert_eq!(second.finished, vec![3]);
        assert_eq!(campfire.cooking_progress(), &[0, 2, 0, 2]);
        let third = campfire.cook_tick();
        assert_eq!(third.finished, vec![1, 3]);
        assert_eq!(campfire.cooking_progress(), &[0, 3, 0, 3]);
    }

    /// `CampfireBlockEntity.cooldownTick` (`:87-99`): two steps per tick, never
    /// below zero and never above the slot's own cooking time.
    #[test]
    fn cooldown_tick_cools_by_two_and_clamps() {
        let mut campfire = CampfireBlockEntity::default();
        assert!(!campfire.cooldown_tick());
        assert!(campfire.set_cooking(0, 3, 600));
        assert!(campfire.set_cooking(1, 1, 2));
        assert!(campfire.set_cooking(2, 601, 600));
        assert!(campfire.cooldown_tick());
        assert_eq!(campfire.cooking_progress(), &[1, 0, 599, 0]);
        assert!(campfire.cooldown_tick());
        assert_eq!(campfire.cooking_progress(), &[0, 0, 597, 0]);
        // Only a campfire whose slots are all at zero answers `changed` false.
        assert!(campfire.set_cooking(2, 1, 600));
        assert!(campfire.cooldown_tick());
        assert_eq!(campfire.cooking_progress(), &[0, 0, 0, 0]);
        assert!(!campfire.cooldown_tick());
    }

    #[test]
    fn a_campfire_round_trips_items_and_cooking_arrays() {
        let mut built =
            new_block_entity(pos(), default_state()).expect("a campfire builds an object");
        {
            let campfire = built.campfire_mut().expect("payload is a campfire");
            assert!(campfire.set_slot(1, stone()));
            assert!(campfire.set_cooking(1, 40, 100));
            assert!(campfire.set_cooking(3, 7, 20));
        }
        let record = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture save");
        assert!(unique_field(&record, TAG_ITEMS).is_some());
        assert_eq!(
            unique_field(&record, TAG_COOKING_TIMES),
            Some(&NbtSummaryValue::IntArray(vec![0, 40, 0, 7]))
        );
        assert_eq!(
            unique_field(&record, TAG_COOKING_TOTAL_TIMES),
            Some(&NbtSummaryValue::IntArray(vec![0, 100, 0, 20]))
        );
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture load")
            .expect("the record names a campfire");
        let campfire = loaded.campfire().expect("payload is a campfire");
        assert_eq!(campfire.items()[1].item_id(), Some(VANILLA_ITEM_STONE_ID));
        assert_eq!(campfire.items()[1].count(), 1);
        assert!(campfire.items()[0].is_empty());
        assert_eq!(campfire.cooking_progress(), &[0, 40, 0, 7]);
        assert_eq!(campfire.cooking_time(), &[0, 100, 0, 20]);
        let again = crate::block_entity::load_test_fixture(
            pos(),
            default_state(),
            &loaded
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
                .expect("fixture save"),
        )
        .expect("fixture load")
        .expect("the round-trip still names a campfire");
        let again = again.campfire().expect("payload is a campfire");
        assert_eq!(again.items()[1].item_id(), Some(VANILLA_ITEM_STONE_ID));
        assert_eq!(again.cooking_progress(), campfire.cooking_progress());
        assert_eq!(again.cooking_time(), campfire.cooking_time());
        let drops = loaded.pre_remove_side_effects(loaded.block_state());
        let BlockEntityRemovalDrops::ContainerSlots(slots) = drops else {
            panic!("a campfire is a container");
        };
        assert_eq!(slots.len(), 4);
        assert!(slots[0].is_empty());
        assert_eq!(slots[1].summary(), stone().summary(),);
    }

    #[test]
    fn a_campfire_update_tag_is_items_only() {
        let mut built =
            new_block_entity(pos(), default_state()).expect("a campfire builds an object");
        built
            .campfire_mut()
            .expect("payload is a campfire")
            .set_slot(0, stone());
        built
            .campfire_mut()
            .expect("payload is a campfire")
            .set_cooking(0, 3, 10);
        let tag = built
            .update_tag_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture update tag");
        assert!(unique_field(&tag, TAG_ITEMS).is_some());
        assert_eq!(unique_field(&tag, TAG_COOKING_TIMES), None);
        assert_eq!(unique_field(&tag, TAG_COOKING_TOTAL_TIMES), None);
        let saved = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture save");
        match unique_field(&saved, TAG_ITEMS) {
            Some(NbtSummaryValue::List(_)) => {}
            other => panic!("alwaysSave still writes Items: {other:?}"),
        }
    }
}
