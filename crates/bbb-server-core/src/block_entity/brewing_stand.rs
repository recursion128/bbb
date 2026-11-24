//! `BrewingStandBlockEntity` (`BrewingStandBlockEntity.java:28-252`).
//!
//! Five slots, `BrewTime`, `Fuel` and the `serverTick` pass (`:97-139`).

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::chest::{load_items, saved_items, TAG_ITEMS};
use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::container_fields::ContainerNameAndLock;
use super::value_input::{byte_or, short_or};
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};
use crate::potion_brewing::PotionBrewing;

/// `NonNullList.withSize(5, ...)` (`BrewingStandBlockEntity.java:41`).
pub const BREWING_STAND_CONTAINER_SIZE: usize = 5;
/// `BrewingStandBlockEntity.INGREDIENT_SLOT` (`:29`).
pub const SLOT_INGREDIENT: usize = 3;
/// `BrewingStandBlockEntity.FUEL_SLOT` (`:30`).
pub const SLOT_FUEL: usize = 4;
/// `BrewingStandBlockEntity.FUEL_USES` (`:34`).
pub const FUEL_USES: i32 = 20;
/// The 400 ticks a brew takes (`:120`), `PotionBrewing.BREWING_TIME_SECONDS`
/// (`PotionBrewing.java:17`) times twenty.
const BREW_TIME: i32 = 400;
const TAG_BREW_TIME: &str = "BrewTime";
const TAG_FUEL: &str = "Fuel";

/// What one `serverTick` pass (`BrewingStandBlockEntity.java:97-139`) reads off
/// the level.
pub struct BrewingStandTickFacts<'a> {
    /// `level.potionBrewing()` (`:105`, `:175`).
    pub brewing: &'a PotionBrewing,
    /// `fuel.is(ItemTags.BREWING_FUEL)` (`:99`) over the live item tags.
    pub fuel_is_brewing_fuel: bool,
    /// `ingredient.getItem().getCraftingRemainder()` (`:182`).
    pub ingredient_remainder: Option<RuntimeItemStack>,
}

/// What one pass leaves for the level to carry out.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BrewingStandTick {
    /// The bottle bits `level.setBlock(pos, state, 2)` writes (`:125-138`).
    pub potion_bits: Option<[bool; 3]>,
    /// The remainder `Containers.dropItemStack` spawns because the ingredient
    /// slot still held something (`:187`).
    pub dropped_remainder: Option<RuntimeItemStack>,
    /// `level.levelEvent(1035, pos, 0)` (`:192`).
    pub brewed: bool,
    /// Whether the pass moved anything (`setChanged`).
    pub changed: bool,
}

/// `BrewingStandBlockEntity` (`BrewingStandBlockEntity.java:28-46`).
#[derive(Debug, Clone, PartialEq)]
pub struct BrewingStandBlockEntity {
    base: ContainerNameAndLock,
    items: Vec<RuntimeItemStack>,
    brew_time: i32,
    /// `BrewingStandBlockEntity.lastPotionCount` (`:43`), the bottle bits the
    /// block state was last written from. It is not saved: a record read from
    /// disk starts at `null` and the first tick writes the state (`:125-138`).
    last_potion_count: Option<[bool; 3]>,
    /// `BrewingStandBlockEntity.ingredient` (`:44`): the item the running brew
    /// started from, which cancels the brew if the slot changes (`:113`).
    ingredient: Option<i32>,
    fuel: i32,
}

impl Default for BrewingStandBlockEntity {
    /// `new BrewingStandBlockEntity(pos, state)` (`:73-75`): five empty slots
    /// and the default brew/fuel zeros (`:38-39`).
    fn default() -> Self {
        Self {
            base: ContainerNameAndLock::default(),
            items: vec![RuntimeItemStack::empty(); BREWING_STAND_CONTAINER_SIZE],
            brew_time: 0,
            last_potion_count: None,
            ingredient: None,
            fuel: 0,
        }
    }
}

impl BrewingStandBlockEntity {
    pub fn custom_name(&self) -> Option<&NbtSummaryValue> {
        self.base.custom_name()
    }

    pub fn can_open(
        &self,
        is_spectator: bool,
        key: &RuntimeItemStack,
        registries: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> bbb_server_types::Result<bool> {
        Ok(self.base.can_unlock(is_spectator, key, registries))
    }

    /// `BaseContainerBlockEntity.collectImplicitComponents`
    /// (`BaseContainerBlockEntity.java:157-165`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        self.base.collect(components);
        components.set_container(&self.items);
    }

    /// `BaseContainerBlockEntity.applyImplicitComponents` (`BaseContainerBlockEntity.java:149-154`).
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        self.base.apply_components(components)?;
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

    /// `AbstractContainerMenu.getRedstoneSignalFromBlockEntity`
    /// (`AbstractContainerMenu.java:747-749`) over the five slots.
    pub fn redstone_signal(&self) -> bbb_server_types::Result<i32> {
        crate::inventory::redstone_signal_from_container(&self.items)
    }

    #[cfg(test)]
    pub fn items_mut(&mut self) -> &mut [RuntimeItemStack] {
        &mut self.items
    }

    #[must_use]
    pub const fn brew_time(&self) -> i32 {
        self.brew_time
    }

    #[must_use]
    pub const fn fuel(&self) -> i32 {
        self.fuel
    }

    /// The whole five-slot write the menu reaches. `BaseContainerBlockEntity`
    /// has no `setItem` override of its own, so a slot write is a plain store
    /// and the brew cancels on the next pass through `isBrewable` instead of
    /// here.
    pub fn set_items(&mut self, items: &[RuntimeItemStack]) {
        if items.len() != BREWING_STAND_CONTAINER_SIZE {
            return;
        }
        self.items = items.to_vec();
    }

    /// `BrewingStandBlockEntity.getPotionBits` (`:141-151`).
    #[must_use]
    pub fn potion_bits(&self) -> [bool; 3] {
        let mut bits = [false; 3];
        for (index, bit) in bits.iter_mut().enumerate() {
            *bit = !self.items[index].is_empty();
        }
        bits
    }

    /// One `BrewingStandBlockEntity.serverTick` pass (`:97-139`), down to the
    /// state the payload owns: the fuel top-up, the brew countdown, the
    /// cancellation, the ingredient it started from and the bottle bits.
    pub fn server_tick(
        &mut self,
        facts: &BrewingStandTickFacts<'_>,
    ) -> bbb_server_types::Result<BrewingStandTick> {
        let mut tick = BrewingStandTick::default();
        if self.fuel <= 0 && facts.fuel_is_brewing_fuel {
            self.fuel = FUEL_USES;
            shrink(&mut self.items[SLOT_FUEL]);
            tick.changed = true;
        }
        let brewable = is_brewable(facts.brewing, &self.items)?;
        let is_brewing = self.brew_time > 0;
        if is_brewing {
            self.brew_time -= 1;
            if self.brew_time == 0 && brewable {
                self.do_brew(facts, &mut tick)?;
            } else if !brewable || !self.ingredient_matches() {
                self.brew_time = 0;
            }
            tick.changed = true;
        } else if brewable && self.fuel > 0 {
            self.fuel -= 1;
            self.brew_time = BREW_TIME;
            self.ingredient = self.items[SLOT_INGREDIENT].item_id();
            tick.changed = true;
        }
        let bits = self.potion_bits();
        if self.last_potion_count != Some(bits) {
            self.last_potion_count = Some(bits);
            tick.potion_bits = Some(bits);
        }
        Ok(tick)
    }

    /// `BrewingStandBlockEntity.isBrewable` (`:153-171`) for the ingredient
    /// this payload holds.
    fn ingredient_matches(&self) -> bool {
        self.ingredient.is_some() && self.ingredient == self.items[SLOT_INGREDIENT].item_id()
    }

    /// `BrewingStandBlockEntity.doBrew` (`:173-193`).
    fn do_brew(
        &mut self,
        facts: &BrewingStandTickFacts<'_>,
        tick: &mut BrewingStandTick,
    ) -> bbb_server_types::Result<()> {
        for dest in 0..3 {
            if self.items[dest].is_empty() {
                continue;
            }
            let source = self.items[dest].summary().clone();
            if source.component_patch.potion_contents.potion_id.is_none() {
                continue;
            }
            let ingredient = self.items[SLOT_INGREDIENT].summary().clone();
            if facts.brewing.has_container_mix(&source, &ingredient)
                || facts.brewing.has_potion_mix(&source, &ingredient)
            {
                let mixed = facts.brewing.mix(&ingredient, &source);
                self.items[dest] = RuntimeItemStack::from_summary(mixed);
            }
        }
        shrink(&mut self.items[SLOT_INGREDIENT]);
        // BrewingStandBlockEntity.java:181-182: getItem reads AIR after the last shrink.
        if !self.items[SLOT_INGREDIENT].is_empty() {
            tick.dropped_remainder = facts.ingredient_remainder.clone();
        }
        tick.brewed = true;
        Ok(())
    }

    /// `BrewingStandBlockEntity.saveAdditional` (`:209-214`).
    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        self.base.save(&mut entries);
        entries.extend([
            entry(TAG_BREW_TIME, NbtSummaryValue::Short(self.brew_time as i16)),
            entry(TAG_ITEMS, saved_items(&self.items, save)?),
            entry(TAG_FUEL, NbtSummaryValue::Byte(self.fuel as i8)),
        ]);
        Ok(entries)
    }

    /// `BrewingStandBlockEntity.loadAdditional` (`:196-206`).
    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        self.base = ContainerNameAndLock::load(record);
        self.items = vec![RuntimeItemStack::empty(); BREWING_STAND_CONTAINER_SIZE];
        load_items(unique_field(record, TAG_ITEMS), &mut self.items, load)?;
        self.brew_time = short_or(record, TAG_BREW_TIME, 0);
        // `:201-203`: a record that was mid-brew remembers the item it started
        // from, and `lastPotionCount` starts unset either way.
        self.ingredient = if self.brew_time > 0 {
            self.items[SLOT_INGREDIENT].item_id()
        } else {
            None
        };
        self.last_potion_count = None;
        self.fuel = i32::from(byte_or(record, TAG_FUEL, 0));
        Ok(())
    }
}

/// `BrewingStandBlockEntity.isBrewable` (`:153-171`): an ingredient the table
/// knows, and one bottle slot it can mix.
fn is_brewable(
    brewing: &PotionBrewing,
    items: &[RuntimeItemStack],
) -> bbb_server_types::Result<bool> {
    let ingredient = &items[SLOT_INGREDIENT];
    if ingredient.is_empty() {
        return Ok(false);
    }
    let ingredient = ingredient.summary();
    if !brewing.is_ingredient(ingredient) {
        return Ok(false);
    }
    for source in &items[..3] {
        if !source.is_empty() && brewing.has_mix(source.summary(), ingredient) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `ItemStack.shrink(1)` (`ItemStack.java`).
fn shrink(stack: &mut RuntimeItemStack) {
    stack.split(1);
}

/// `Containers.dropContents` over the five brewing-stand slots.
#[must_use]
pub fn drop_contents(
    stand: &BrewingStandBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    stand
        .original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::block_entity::BlockEntityRemovalDrops;
    use bbb_protocol::NbtSummaryValue;

    use bbb_protocol::block_types::VANILLA_BLOCK_BREWING_STAND_ID;
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID;
    use bbb_protocol::item_types::{VANILLA_ITEM_BLAZE_POWDER_ID, VANILLA_ITEM_GLASS_BOTTLE_ID};
    use bbb_protocol::packets::{
        BlockPos, DataComponentPatchSummary, ItemStackSummary, PotionContentsSummary,
    };

    use super::super::unique_field;
    use super::*;
    use crate::block_entity::new_block_entity;

    fn default_state() -> i32 {
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            VANILLA_BLOCK_BREWING_STAND_ID,
        )
        .expect("minecraft:brewing_stand has a default state")
    }

    fn pos() -> BlockPos {
        BlockPos { x: 4, y: 5, z: 6 }
    }

    fn stack(item_id: i32, count: i32) -> RuntimeItemStack {
        RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        })
    }

    fn water_potion() -> RuntimeItemStack {
        RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_POTION_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                potion_contents: PotionContentsSummary {
                    potion_id: Some(
                        bbb_protocol::potion_types::vanilla_potion_for_resource_id(
                            "minecraft:water",
                        )
                        .expect("water is a built-in potion")
                        .id,
                    ),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        })
    }

    fn facts(remainder: Option<RuntimeItemStack>) -> BrewingStandTickFacts<'static> {
        BrewingStandTickFacts {
            brewing: PotionBrewing::vanilla(),
            fuel_is_brewing_fuel: true,
            ingredient_remainder: remainder,
        }
    }

    /// `BrewingStandBlockEntity.serverTick` (`BrewingStandBlockEntity.java:97-139`):
    /// the fuel top-up, the 400-tick brew, the potion mix and the bottle bits.
    #[test]
    fn a_brewing_stand_brews_a_water_potion_into_an_awkward_one() {
        use bbb_protocol::item_types::{VANILLA_ITEM_BLAZE_POWDER_ID, VANILLA_ITEM_NETHER_WART_ID};

        let mut stand = BrewingStandBlockEntity::default();
        stand.items[0] = water_potion();
        stand.items[3] = stack(VANILLA_ITEM_NETHER_WART_ID, 1);
        stand.items[4] = stack(VANILLA_ITEM_BLAZE_POWDER_ID, 2);
        let facts = facts(None);

        // `:99-103`: the fuel is topped up from the fuel slot.
        let first = stand.server_tick(&facts).expect("fixture tick");
        assert!(first.changed);
        assert_eq!(stand.fuel(), FUEL_USES - 1, "the brew spends one use");
        assert_eq!(stand.items()[4], stack(VANILLA_ITEM_BLAZE_POWDER_ID, 1));
        assert_eq!(stand.brew_time(), 400);
        assert_eq!(
            first.potion_bits,
            Some([true, false, false]),
            "`getPotionBits` (`:141-151`) and the state write (`:125-138`)"
        );

        // `:109-112`: 400 decrements after the start, so the brew lands on the
        // 401st pass.
        for _ in 0..400 {
            stand.server_tick(&facts).expect("fixture tick");
        }
        assert_eq!(stand.brew_time(), 0);
        assert!(stand.items()[3].item_id().is_none(), "the wart is spent");
        assert_eq!(
            stand.items()[0]
                .summary()
                .component_patch
                .potion_contents
                .potion_id,
            Some(
                bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:awkward")
                    .expect("awkward is a built-in potion")
                    .id
            )
        );
        assert_eq!(
            stand.items()[0].item_id(),
            Some(bbb_protocol::item_types::VANILLA_ITEM_POTION_ID)
        );
        let done = stand.server_tick(&facts).expect("fixture tick");
        assert!(!done.brewed, "the brew ran on its own tick");
    }

    /// `:113-115`: a brew whose ingredient slot changed before it finished is
    /// cancelled, and `:118-123` only starts one when the table has a mix.
    #[test]
    fn a_changed_ingredient_cancels_the_brew() {
        use bbb_protocol::item_types::{
            VANILLA_ITEM_BLAZE_POWDER_ID, VANILLA_ITEM_NETHER_WART_ID, VANILLA_ITEM_STONE_ID,
        };

        let mut stand = BrewingStandBlockEntity::default();
        stand.items[0] = water_potion();
        stand.items[3] = stack(VANILLA_ITEM_NETHER_WART_ID, 2);
        stand.items[4] = stack(VANILLA_ITEM_BLAZE_POWDER_ID, 1);
        let facts = facts(None);
        stand.server_tick(&facts).expect("fixture tick");
        assert_eq!(stand.brew_time(), 400);

        stand.items[3] = stack(VANILLA_ITEM_STONE_ID, 1);
        stand.server_tick(&facts).expect("fixture tick");
        assert_eq!(
            stand.brew_time(),
            0,
            "the ingredient is not the one it started from"
        );
        // Stone is an ingredient (`addStartMix(Items.STONE, ...)`) but the
        // bottle holds a water potion, which stone has no mix for.
        assert_eq!(stand.brew_time(), 0);
    }

    #[test]
    fn the_ingredient_remainder_is_read_after_shrinking_the_original() {
        use bbb_protocol::item_types::{
            VANILLA_ITEM_BLAZE_POWDER_ID, VANILLA_ITEM_DRAGON_BREATH_ID,
            VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_SPLASH_POTION_ID,
        };

        let remainder = stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1);
        let facts = facts(Some(remainder.clone()));
        for count in [1, 2] {
            let mut stand = BrewingStandBlockEntity::default();
            let mut potion = water_potion().summary().clone();
            potion.item_id = Some(VANILLA_ITEM_SPLASH_POTION_ID);
            stand.items[0] = bbb_server_types::item_stack::RuntimeItemStack::from_summary(potion);
            stand.items[3] = stack(VANILLA_ITEM_DRAGON_BREATH_ID, count);
            stand.items[4] = stack(VANILLA_ITEM_BLAZE_POWDER_ID, 1);
            let mut dropped = None;
            for _ in 0..401 {
                let tick = stand.server_tick(&facts).expect("fixture tick");
                if tick.dropped_remainder.is_some() {
                    dropped = tick.dropped_remainder;
                }
            }
            assert_eq!(stand.items[3].count(), count - 1);
            assert_eq!(dropped, (count == 2).then(|| remainder.clone()));
        }
    }

    /// `:196-206`: a record read mid-brew remembers the item it started from,
    /// so the next pass does not cancel the brew.
    #[test]
    fn a_restored_brew_keeps_its_ingredient() {
        use bbb_protocol::item_types::{VANILLA_ITEM_BLAZE_POWDER_ID, VANILLA_ITEM_NETHER_WART_ID};

        let state = default_state();
        let mut built = new_block_entity(pos(), state).expect("a brewing stand");
        {
            let stand = built.brewing_stand_mut().expect("brewing-stand payload");
            stand.items[0] = water_potion();
            stand.items[3] = stack(VANILLA_ITEM_NETHER_WART_ID, 1);
            stand.items[4] = stack(VANILLA_ITEM_BLAZE_POWDER_ID, 1);
            stand.brew_time = 400;
        }
        let record = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture save");
        let mut loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture load")
            .expect("the record reads back");
        let stand = loaded.brewing_stand_mut().expect("brewing-stand payload");
        assert_eq!(stand.brew_time(), 400);
        stand.server_tick(&facts(None)).expect("fixture tick");
        assert_eq!(stand.brew_time(), 399, "the ingredient still matches");
    }

    /// `BrewingStandBlockEntity.saveAdditional` / `loadAdditional`
    /// (`:209-214`, `:196-206`).
    #[test]
    fn brewing_stand_save_round_trips_items_brew_time_and_fuel() {
        let state = default_state();
        let mut built = new_block_entity(pos(), state).expect("a brewing stand");
        {
            let stand = built.brewing_stand_mut().expect("brewing-stand payload");
            stand.items[0] = stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1);
            stand.items[2] = stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1);
            stand.items[4] = stack(VANILLA_ITEM_BLAZE_POWDER_ID, 3);
            stand.brew_time = 400;
            stand.fuel = 20;
        }

        let record = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture save");
        assert_eq!(
            unique_field(&record, TAG_BREW_TIME),
            Some(&NbtSummaryValue::Short(400))
        );
        assert_eq!(
            unique_field(&record, TAG_FUEL),
            Some(&NbtSummaryValue::Byte(20))
        );
        let Some(NbtSummaryValue::List(items)) = unique_field(&record, TAG_ITEMS) else {
            panic!("Items is a list: {record:?}");
        };
        assert_eq!(items.len(), 3, "empty slots are skipped");

        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture load")
            .expect("the record names a brewing stand standing on a brewing stand");
        let stand = loaded.brewing_stand().expect("brewing-stand payload");
        assert_eq!(
            stand.items()[0].summary(),
            stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1).summary()
        );
        assert!(stand.items()[1].item_id().is_none());
        assert_eq!(
            stand.items()[2].summary(),
            stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1).summary()
        );
        assert!(stand.items()[3].item_id().is_none());
        assert_eq!(
            stand.items()[4].summary(),
            stack(VANILLA_ITEM_BLAZE_POWDER_ID, 3).summary()
        );
        assert_eq!(stand.brew_time(), 400);
        assert_eq!(stand.fuel(), 20);
        assert_eq!(
            loaded
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver(),)
                .expect("fixture save"),
            record
        );
        // Slot-aligned: the two empty slots stay because vanilla draws a drop
        // position for every slot (`Containers.java:25-30`).
        let drops = loaded.pre_remove_side_effects(loaded.block_state());
        let BlockEntityRemovalDrops::ContainerSlots(slots) = drops else {
            panic!("a brewing stand is a container");
        };
        assert_eq!(slots.len(), 5);
        assert_eq!(
            slots[0].summary(),
            stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1).summary()
        );
        assert!(slots[1].item_id().is_none());
        assert_eq!(
            slots[4].summary(),
            stack(VANILLA_ITEM_BLAZE_POWDER_ID, 3).summary()
        );
    }
}
