//! `BrushableBlockEntity` (`BrushableBlockEntity.java:38-247`).
//!
//! Packed loot or the revealed stack, plus the optional brush hit face.
//! Brush count, reset tick and cooldown are runtime and are not saved.

use bbb_protocol::packets::Direction;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::bell::{direction_3d_data_value, direction_from_3d_data_value};
use super::value_input::{codec_byte, long_or};
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};

/// `BrushableBlockEntity.LOOT_TABLE_TAG` (`BrushableBlockEntity.java:40`).
const TAG_LOOT_TABLE: &str = "LootTable";
/// `BrushableBlockEntity.LOOT_TABLE_SEED_TAG` (`:41`).
const TAG_LOOT_TABLE_SEED: &str = "LootTableSeed";
/// `BrushableBlockEntity.HIT_DIRECTION_TAG` (`:42`).
const TAG_HIT_DIRECTION: &str = "hit_direction";
/// `BrushableBlockEntity.ITEM_TAG` (`:43`).
const TAG_ITEM: &str = "item";
/// `BrushableBlockEntity.BRUSH_COOLDOWN_TICKS` (`:44`).
const BRUSH_COOLDOWN_TICKS: i64 = 10;
/// `BrushableBlockEntity.BRUSH_RESET_TICKS` (`:45`).
const BRUSH_RESET_TICKS: i64 = 40;
/// `BrushableBlockEntity.REQUIRED_BRUSHES_TO_BREAK` (`:46`).
const REQUIRED_BRUSHES_TO_BREAK: i32 = 10;
/// `checkReset`'s `retractionSpeed` (`:157`).
const RETRACTION_TICKS: i64 = 4;

/// `BrushableBlockEntity` (`BrushableBlockEntity.java:38-53`).
#[derive(Debug, Clone, PartialEq)]
pub struct BrushableBlockEntity {
    item: RuntimeItemStack,
    hit_direction: Option<Direction>,
    loot_table: Option<NbtSummaryValue>,
    loot_table_seed: i64,
    brush_count: i32,
    brush_count_resets_at_tick: i64,
    cool_down_ends_at_tick: i64,
}

impl Default for BrushableBlockEntity {
    fn default() -> Self {
        Self {
            item: RuntimeItemStack::empty(),
            hit_direction: None,
            loot_table: None,
            loot_table_seed: 0,
            brush_count: 0,
            brush_count_resets_at_tick: 0,
            cool_down_ends_at_tick: 0,
        }
    }
}

/// What one brush stroke did once its loot was unpacked
/// (`BrushableBlockEntity.java:71-85`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushStroke {
    /// The tenth stroke: `brushingCompleted` runs.
    Completed,
    /// Any other: the `DUSTED` value the stroke moved the block to, if it moved.
    Dusted(Option<i32>),
}

/// What `checkReset` (`BrushableBlockEntity.java:148-168`) asks of the level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrushReset {
    /// The `DUSTED` value to write back, when the retraction changed it.
    pub dusted: Option<i32>,
    /// Whether another tick is scheduled.
    pub reschedule: bool,
}

impl BrushableBlockEntity {
    #[must_use]
    pub fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    #[must_use]
    pub const fn hit_direction(&self) -> Option<Direction> {
        self.hit_direction
    }

    #[must_use]
    pub fn loot_table(&self) -> Option<&NbtSummaryValue> {
        self.loot_table.as_ref()
    }

    #[must_use]
    pub const fn loot_table_seed(&self) -> i64 {
        self.loot_table_seed
    }

    pub fn set_item(&mut self, item: RuntimeItemStack) {
        self.item = if item.is_empty() {
            RuntimeItemStack::empty()
        } else {
            item
        };
        self.loot_table = None;
        self.loot_table_seed = 0;
    }

    /// `BrushableBlockEntity.setLootTable` (`BrushableBlockEntity.java:225-228`).
    pub fn set_loot_table(&mut self, loot_table: NbtSummaryValue, seed: i64) {
        self.loot_table = Some(loot_table);
        self.loot_table_seed = seed;
        self.item = RuntimeItemStack::empty();
    }

    pub fn set_hit_direction(&mut self, hit_direction: Option<Direction>) {
        self.hit_direction = hit_direction;
    }

    /// Takes the revealed stack for `dropContent` (`:130-146`), leaving the
    /// entity empty.
    pub fn take_item(&mut self) -> RuntimeItemStack {
        std::mem::replace(&mut self.item, RuntimeItemStack::empty())
    }

    /// The part of `brush` (`BrushableBlockEntity.java:59-69`) before the loot
    /// unpack: false while the previous stroke's cooldown still runs.
    pub fn begin_brush(&mut self, game_time: i64, direction: Direction) -> bool {
        if self.hit_direction.is_none() {
            self.hit_direction = Some(direction);
        }
        self.brush_count_resets_at_tick = game_time + BRUSH_RESET_TICKS;
        if game_time < self.cool_down_ends_at_tick {
            return false;
        }
        self.cool_down_ends_at_tick = game_time + BRUSH_COOLDOWN_TICKS;
        true
    }

    /// The part of `brush` after the loot unpack (`:71-85`).
    pub fn stroke(&mut self) -> BrushStroke {
        let previous = self.completion_state();
        self.brush_count += 1;
        if self.brush_count >= REQUIRED_BRUSHES_TO_BREAK {
            return BrushStroke::Completed;
        }
        let current = self.completion_state();
        BrushStroke::Dusted((current != previous).then_some(current))
    }

    /// `BrushableBlockEntity.checkReset` (`:148-168`).
    pub fn check_reset(&mut self, game_time: i64) -> BrushReset {
        let mut dusted = None;
        if self.brush_count != 0 && game_time >= self.brush_count_resets_at_tick {
            let previous = self.completion_state();
            self.brush_count = (self.brush_count - 2).max(0);
            let current = self.completion_state();
            if current != previous {
                dusted = Some(current);
            }
            self.brush_count_resets_at_tick = game_time + RETRACTION_TICKS;
        }
        let reschedule = self.brush_count != 0;
        if !reschedule {
            self.hit_direction = None;
            self.brush_count_resets_at_tick = 0;
            self.cool_down_ends_at_tick = 0;
        }
        BrushReset { dusted, reschedule }
    }

    /// `BrushableBlockEntity.getCompletionState` (`:230-238`).
    fn completion_state(&self) -> i32 {
        match self.brush_count {
            0 => 0,
            1..=2 => 1,
            3..=5 => 2,
            _ => 3,
        }
    }

    /// `BrushableBlockEntity.saveAdditional` (`:218-222`): persists either
    /// `LootTable`/`LootTableSeed` or non-empty `item` (never `hit_direction`).
    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        if let Some(loot_table) = &self.loot_table {
            entries.push(entry(TAG_LOOT_TABLE, loot_table.clone()));
            if self.loot_table_seed != 0 {
                entries.push(entry(
                    TAG_LOOT_TABLE_SEED,
                    NbtSummaryValue::Long(self.loot_table_seed),
                ));
            }
        } else if !self.item.is_empty() {
            // `output.store("item", ItemStack.CODEC, this.item)` (`:221`) leaves
            // the field out when the encode fails, because `ItemStack.CODEC`
            // never answers a partial value for
            // `TagValueOutput.store`'s partial arm (`TagValueOutput.java:43`).
            if let Some(item) = save.write(&self.item)? {
                entries.push(entry(TAG_ITEM, item));
            }
        }
        Ok(entries)
    }

    /// `BrushableBlockEntity.getUpdateTag` (`:190-198`): sends `hit_direction`
    /// and non-empty `item` to clients without leaking `LootTable`/`LootTableSeed`.
    pub(super) fn save_update_tag(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        if let Some(direction) = self.hit_direction {
            entries.push(entry(
                TAG_HIT_DIRECTION,
                NbtSummaryValue::Byte(direction_3d_data_value(direction) as i8),
            ));
        }
        if !self.item.is_empty() {
            if let Some(item) = save.write(&self.item)? {
                entries.push(entry(TAG_ITEM, item));
            }
        }
        Ok(entries)
    }

    /// `BrushableBlockEntity.loadAdditional` (`BrushableBlockEntity.java:206-215`).
    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        self.loot_table = unique_field(record, TAG_LOOT_TABLE).cloned();
        self.loot_table_seed = long_or(record, TAG_LOOT_TABLE_SEED, 0);
        self.item = if self.loot_table.is_some() {
            RuntimeItemStack::empty()
        } else {
            match unique_field(record, TAG_ITEM) {
                Some(value) => load_item(value, load)?.unwrap_or_else(RuntimeItemStack::empty),
                None => RuntimeItemStack::empty(),
            }
        };
        self.hit_direction = unique_field(record, TAG_HIT_DIRECTION).and_then(load_hit_direction);
        Ok(())
    }
}

/// `Direction.LEGACY_ID_CODEC` (`Direction.java:45`): a byte `get3DDataValue`.
fn load_hit_direction(value: &NbtSummaryValue) -> Option<Direction> {
    let id = i32::from(codec_byte(value)?);
    Some(direction_from_3d_data_value(
        id.wrapping_rem(6).unsigned_abs() as i32,
    ))
}

fn load_item(
    value: &NbtSummaryValue,
    load: &mut BlockEntityItemLoad<'_>,
) -> bbb_server_types::Result<Option<RuntimeItemStack>> {
    load.read(value)
}

#[cfg(test)]
mod tests {
    use bbb_protocol::NbtSummaryValue;

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_SUSPICIOUS_SAND_ID;
    use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
    use bbb_protocol::packets::{BlockPos, Direction, ItemStackSummary};

    use crate::block_entity::{new_block_entity, unique_field, BlockEntityRemovalDrops};

    use super::*;

    fn pos() -> BlockPos {
        BlockPos { x: 1, y: 2, z: 3 }
    }

    fn default_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_SUSPICIOUS_SAND_ID)
            .expect("minecraft:suspicious_sand has a default state")
    }

    fn stone() -> RuntimeItemStack {
        RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: Default::default(),
        })
    }

    fn loot_table() -> NbtSummaryValue {
        NbtSummaryValue::string("minecraft:archaeology/desert_well")
    }

    #[test]
    fn a_brushable_block_round_trips_its_item_and_splits_update_tag() {
        let mut built =
            new_block_entity(pos(), default_state()).expect("suspicious sand builds an object");
        {
            let brushable = built.brushable_mut().expect("payload is a brushable block");
            brushable.set_item(stone());
            brushable.set_hit_direction(Some(Direction::East));
        }
        let record = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture save");
        assert!(unique_field(&record, TAG_ITEM).is_some());
        assert_eq!(unique_field(&record, TAG_LOOT_TABLE), None);
        assert_eq!(unique_field(&record, TAG_HIT_DIRECTION), None);
        let update_tag = built
            .update_tag_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture update tag");
        assert!(unique_field(&update_tag, TAG_ITEM).is_some());
        assert_eq!(unique_field(&update_tag, TAG_LOOT_TABLE), None);
        assert_eq!(
            unique_field(&update_tag, TAG_HIT_DIRECTION),
            Some(&NbtSummaryValue::Byte(5))
        );
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture load")
            .expect("the record names a brushable block");
        let brushable = loaded.brushable().expect("payload is a brushable block");
        assert_eq!(brushable.item().item_id(), Some(VANILLA_ITEM_STONE_ID));
        assert_eq!(brushable.hit_direction(), None);
        let again = crate::block_entity::load_test_fixture(
            pos(),
            default_state(),
            &loaded
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
                .expect("fixture save"),
        )
        .expect("fixture load")
        .expect("the round-trip still names a brushable block");
        let again = again.brushable().expect("payload is a brushable block");
        assert_eq!(again.item().item_id(), Some(VANILLA_ITEM_STONE_ID));
        assert_eq!(again.hit_direction(), None);
        assert_eq!(
            loaded.pre_remove_side_effects(loaded.block_state()),
            BlockEntityRemovalDrops::None
        );

        let mut with_loot =
            new_block_entity(pos(), default_state()).expect("suspicious sand builds an object");
        with_loot
            .brushable_mut()
            .expect("payload is a brushable block")
            .set_loot_table(loot_table(), 42);
        let loot_update = with_loot
            .update_tag_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture update tag");
        assert_eq!(unique_field(&loot_update, TAG_LOOT_TABLE), None);
        assert_eq!(unique_field(&loot_update, TAG_LOOT_TABLE_SEED), None);
    }

    #[test]
    fn a_packed_loot_table_skips_the_item() {
        let mut built =
            new_block_entity(pos(), default_state()).expect("suspicious sand builds an object");
        built
            .brushable_mut()
            .expect("payload is a brushable block")
            .set_loot_table(loot_table(), -12345);
        let record = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture save");
        assert_eq!(unique_field(&record, TAG_LOOT_TABLE), Some(&loot_table()));
        assert_eq!(
            unique_field(&record, TAG_LOOT_TABLE_SEED),
            Some(&NbtSummaryValue::Long(-12345))
        );
        assert_eq!(unique_field(&record, TAG_ITEM), None);
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture load")
            .expect("the record names a brushable block");
        let brushable = loaded.brushable().expect("payload is a brushable block");
        assert!(brushable.item().is_empty());
        assert_eq!(brushable.loot_table(), Some(&loot_table()));
        assert_eq!(brushable.loot_table_seed(), -12345);
    }
}
