//! `AbstractFurnaceBlockEntity` (`AbstractFurnaceBlockEntity.java:41-382`).
//!
//! Shared by furnace, smoker, and blast furnace. `serverTick` (`:138-206`)
//! needs `RecipeManager` and is not wired.

use std::collections::BTreeMap;

use bbb_protocol::item_types::VANILLA_ITEM_WET_SPONGE_ID;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::chest::{load_items, saved_items, TAG_ITEMS};
use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::container_fields::ContainerNameAndLock;
use super::value_input::{codec_int, short_or};
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};

/// `NonNullList.withSize(3, ...)` (`AbstractFurnaceBlockEntity.java:60`).
pub const FURNACE_CONTAINER_SIZE: usize = 3;
/// `AbstractFurnaceBlockEntity.SLOT_INPUT` / `SLOT_FUEL` / `SLOT_RESULT`
/// (`:44-46`).
pub const SLOT_INPUT: usize = 0;
pub const SLOT_FUEL: usize = 1;
pub const SLOT_RESULT: usize = 2;
/// `getTotalCookTime`'s answer when no recipe claims the input
/// (`AbstractFurnaceBlockEntity.java:251-254`).
pub const DEFAULT_COOKING_TIME: i32 = 200;
const TAG_COOKING_TIME_SPENT: &str = "cooking_time_spent";
const TAG_COOKING_TOTAL_TIME: &str = "cooking_total_time";
const TAG_LIT_TIME_REMAINING: &str = "lit_time_remaining";
const TAG_LIT_TOTAL_TIME: &str = "lit_total_time";
const TAG_RECIPES_USED: &str = "RecipesUsed";

/// What one `serverTick` pass (`AbstractFurnaceBlockEntity.java:138-206`) needs
/// from the level, which owns the recipe map and the fuel table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FurnaceTickFacts {
    /// `recipe.value().assemble(input)` (`:161`), or `None` when no recipe
    /// claims the input slot.
    pub recipe_result: Option<RuntimeItemStack>,
    /// `recipe.value().cookingTime()` (`:178`).
    pub recipe_cooking_time: i32,
    /// The recipe id `setRecipeUsed` (`:180`) records.
    pub recipe_id: Option<String>,
    /// `getBurnDuration(level.fuelValues(), fuel)` (`:164`).
    pub burn_duration: i32,
    /// `fuelItem.getCraftingRemainder()` (`consumeFuel`, `:208-216`).
    pub fuel_remainder: Option<RuntimeItemStack>,
}

/// What one pass changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FurnaceTick {
    /// Whether `setChanged` runs (`:203-205`).
    pub changed: bool,
    /// The `LIT` value to write when it differs from the block's own
    /// (`:197-201`).
    pub lit_changed: Option<bool>,
}

/// Transient `RecipeManager.CachedCheck` hint (`AbstractFurnaceBlockEntity.java:63`),
/// ignored in structural equality and persistence.
#[derive(Debug, Clone, Default)]
struct LastRecipeHint(Option<String>);

impl PartialEq for LastRecipeHint {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

/// `AbstractFurnaceBlockEntity` (`AbstractFurnaceBlockEntity.java:41-65`).
#[derive(Debug, Clone, PartialEq)]
pub struct AbstractFurnaceBlockEntity {
    base: ContainerNameAndLock,
    items: Vec<RuntimeItemStack>,
    cooking_timer: i32,
    cooking_total_time: i32,
    lit_time_remaining: i32,
    lit_total_time: i32,
    recipes_used: BTreeMap<String, i32>,
    last_recipe_id: LastRecipeHint,
}

impl Default for AbstractFurnaceBlockEntity {
    /// `new AbstractFurnaceBlockEntity(...)` (`:107-112`): three empty slots
    /// and the four default shorts (`:56-59`).
    fn default() -> Self {
        Self {
            base: ContainerNameAndLock::default(),
            items: vec![RuntimeItemStack::empty(); FURNACE_CONTAINER_SIZE],
            cooking_timer: 0,
            cooking_total_time: 0,
            lit_time_remaining: 0,
            lit_total_time: 0,
            recipes_used: BTreeMap::new(),
            last_recipe_id: LastRecipeHint::default(),
        }
    }
}

impl AbstractFurnaceBlockEntity {
    pub fn last_recipe_id(&self) -> Option<&str> {
        self.last_recipe_id.0.as_deref()
    }

    pub fn set_last_recipe_id(&mut self, recipe_id: Option<String>) {
        self.last_recipe_id.0 = recipe_id;
    }

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
    /// (`AbstractContainerMenu.java:747-749`) over the three slots.
    pub fn redstone_signal(&self) -> bbb_server_types::Result<i32> {
        crate::inventory::redstone_signal_from_container(&self.items)
    }

    #[cfg(test)]
    pub fn items_mut(&mut self) -> &mut [RuntimeItemStack] {
        &mut self.items
    }

    #[must_use]
    pub const fn cooking_time_spent(&self) -> i32 {
        self.cooking_timer
    }

    #[must_use]
    pub const fn cooking_total_time(&self) -> i32 {
        self.cooking_total_time
    }

    #[must_use]
    pub const fn lit_time_remaining(&self) -> i32 {
        self.lit_time_remaining
    }

    #[must_use]
    pub const fn lit_total_time(&self) -> i32 {
        self.lit_total_time
    }

    #[must_use]
    pub fn recipes_used(&self) -> &BTreeMap<String, i32> {
        &self.recipes_used
    }

    /// `AbstractFurnaceBlockEntity.awardUsedRecipesAndPopExperience`'s last
    /// statement (`:342`): the map is cleared by the take that awarded it.
    pub fn clear_recipes_used(&mut self) {
        self.recipes_used.clear();
    }

    /// `AbstractFurnaceBlockEntity.setItem(0, ...)` (`:291-299`): a new input
    /// stack resets the timer and takes the recipe's own cooking time, which is
    /// the only place `cooking_total_time` starts from zero.
    fn set_original_input(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        item: RuntimeItemStack,
        cooking_total_time: i32,
    ) -> bbb_server_types::Result<bool> {
        let same = !item.is_empty()
            && self.items[SLOT_INPUT].same_item_same_effective_components(&item, resolver)?;
        self.items[SLOT_INPUT] = item;
        if !same {
            self.cooking_total_time = cooking_total_time;
            self.cooking_timer = 0;
        }
        Ok(!same)
    }

    pub fn set_item(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        slot: usize,
        mut item: RuntimeItemStack,
        cooking_total_time: i32,
    ) -> bbb_server_types::Result<bool> {
        if slot >= self.items.len() {
            return Ok(false);
        }
        let max = crate::inventory::container_max_stack_size(&item)?;
        item.set_count(item.count().min(max));
        if slot == SLOT_INPUT {
            self.set_original_input(resolver, item, cooking_total_time)
        } else {
            self.items[slot] = item;
            Ok(false)
        }
    }

    /// The whole three-slot write the menu and automation both reach
    /// (`Container.setItem`, `AbstractFurnaceBlockEntity.java:291-299`): only a
    /// change to the input slot takes a new cooking time and resets the timer,
    /// and `cooking_total_time` is what `getTotalCookTime` answers for the new
    /// input (200 when no recipe claims it).
    pub fn set_items(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        items: &[RuntimeItemStack],
        cooking_total_time: i32,
    ) -> bbb_server_types::Result<()> {
        if items.len() != FURNACE_CONTAINER_SIZE {
            return Ok(());
        }
        for (slot, item) in items.iter().enumerate() {
            if !self.items[slot].matches(item) {
                self.set_item(resolver, slot, item.clone(), cooking_total_time)?;
            }
        }
        Ok(())
    }

    /// `AbstractFurnaceBlockEntity.serverTick` (`:138-206`) down to the state
    /// it owns: the lit countdown, the recipe gate, the cooking timer and the
    /// fuel and input consumption. The level supplies the recipe and the fuel
    /// table and carries out the `LIT` write and the change mark.
    pub fn server_tick(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        facts: &FurnaceTickFacts,
    ) -> bbb_server_types::Result<FurnaceTick> {
        let mut changed = false;
        let was_lit = self.lit_time_remaining > 0;
        let mut is_lit = was_lit;
        if was_lit {
            // `:142-149`.
            self.lit_time_remaining -= 1;
            is_lit = self.lit_time_remaining > 0;
        }
        let has_ingredient = !self.items[SLOT_INPUT].is_empty();
        let has_fuel = !self.items[SLOT_FUEL].is_empty();
        if is_lit || (has_fuel && has_ingredient) {
            if has_ingredient {
                // `:156-159`: a recipe that claims the input, and no arm for
                // the case where none does -- the timer is left alone then.
                if let Some(result) = &facts.recipe_result {
                    if !result.is_empty() && self.can_burn(result, resolver)? {
                        if !is_lit {
                            // `:163-172`.
                            self.lit_time_remaining = facts.burn_duration;
                            self.lit_total_time = facts.burn_duration;
                            if facts.burn_duration > 0 {
                                self.consume_fuel(facts.fuel_remainder.as_ref());
                                is_lit = true;
                                changed = true;
                            }
                        }
                        if is_lit {
                            // `:174-182`.
                            self.cooking_timer += 1;
                            if self.cooking_timer >= self.cooking_total_time {
                                self.cooking_timer = 0;
                                self.cooking_total_time = facts.recipe_cooking_time;
                                self.burn(result)?;
                                if let Some(id) = &facts.recipe_id {
                                    *self.recipes_used.entry(id.clone()).or_insert(0) += 1;
                                }
                                changed = true;
                            }
                        } else {
                            self.cooking_timer = 0;
                        }
                    } else {
                        self.cooking_timer = 0;
                    }
                }
            } else {
                self.cooking_timer = 0;
            }
        } else if self.cooking_timer > 0 {
            // `:193-195`.
            self.cooking_timer = (self.cooking_timer - 2).clamp(0, self.cooking_total_time);
        }
        let lit_changed = (was_lit != is_lit).then_some(is_lit);
        if lit_changed.is_some() {
            changed = true;
        }
        Ok(FurnaceTick {
            changed,
            lit_changed,
        })
    }

    /// `canBurn` (`:217-230`): the output slot must be empty or hold the same
    /// stack with room for the result.
    fn can_burn(
        &self,
        burn_result: &RuntimeItemStack,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> bbb_server_types::Result<bool> {
        let output = &self.items[SLOT_RESULT];
        if output.is_empty() {
            return Ok(true);
        }
        if !output.same_item_same_effective_components(burn_result, resolver)? {
            return Ok(false);
        }
        let ceiling = crate::inventory::container_max_stack_size(burn_result)?;
        Ok(output.count() + burn_result.count() <= ceiling)
    }

    /// `burn` (`:232-245`).
    fn burn(&mut self, result: &RuntimeItemStack) -> bbb_server_types::Result<()> {
        let output = &mut self.items[SLOT_RESULT];
        if output.is_empty() {
            *output = result.clone();
        } else {
            output.set_count(output.count() + result.count());
        }
        if self.items[SLOT_INPUT].item_id() == Some(VANILLA_ITEM_WET_SPONGE_ID)
            && !self.items[SLOT_FUEL].is_empty()
            && self.items[SLOT_FUEL].item_id()
                == Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID)
        {
            self.items[SLOT_FUEL] = RuntimeItemStack::from_summary(ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WATER_BUCKET_ID),
                count: 1,
                component_patch: Default::default(),
            });
        }
        self.items[SLOT_INPUT].split(1);
        Ok(())
    }

    /// `consumeFuel` (`:208-216`): one item off the fuel slot, replaced by its
    /// crafting remainder once the slot empties.
    fn consume_fuel(&mut self, remainder: Option<&RuntimeItemStack>) {
        self.items[SLOT_FUEL].split(1);
        if self.items[SLOT_FUEL].is_empty() {
            self.items[SLOT_FUEL] = remainder.cloned().unwrap_or_else(RuntimeItemStack::empty);
        }
    }

    /// `AbstractFurnaceBlockEntity.saveAdditional` (`:128-136`).
    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        self.base.save(&mut entries);
        entries.extend([
            entry(
                TAG_COOKING_TIME_SPENT,
                NbtSummaryValue::Short(self.cooking_timer as i16),
            ),
            entry(
                TAG_COOKING_TOTAL_TIME,
                NbtSummaryValue::Short(self.cooking_total_time as i16),
            ),
            entry(
                TAG_LIT_TIME_REMAINING,
                NbtSummaryValue::Short(self.lit_time_remaining as i16),
            ),
            entry(
                TAG_LIT_TOTAL_TIME,
                NbtSummaryValue::Short(self.lit_total_time as i16),
            ),
            entry(TAG_ITEMS, saved_items(&self.items, save)?),
            entry(TAG_RECIPES_USED, saved_recipes_used(&self.recipes_used)),
        ]);
        Ok(entries)
    }

    /// `AbstractFurnaceBlockEntity.loadAdditional` (`:115-125`).
    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        self.base = ContainerNameAndLock::load(record);
        self.items = vec![RuntimeItemStack::empty(); FURNACE_CONTAINER_SIZE];
        load_items(unique_field(record, TAG_ITEMS), &mut self.items, load)?;
        self.cooking_timer = short_field(record, TAG_COOKING_TIME_SPENT);
        self.cooking_total_time = short_field(record, TAG_COOKING_TOTAL_TIME);
        self.lit_time_remaining = short_field(record, TAG_LIT_TIME_REMAINING);
        self.lit_total_time = short_field(record, TAG_LIT_TOTAL_TIME);
        self.recipes_used = load_recipes_used(unique_field(record, TAG_RECIPES_USED));
        Ok(())
    }
}

/// `TagValueInput.getShortOr` (`TagValueInput.java:155-158`) over the four
/// cooking/lit fields, which `putShort` wrote (`:130-133`).
fn short_field(record: &[NbtSummaryEntry], name: &str) -> i32 {
    short_or(record, name, 0)
}

/// `Codec.unboundedMap(Recipe.KEY_CODEC, Codec.INT)` (`:55`).
fn saved_recipes_used(recipes: &BTreeMap<String, i32>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        recipes
            .iter()
            .map(|(id, count)| entry(id, NbtSummaryValue::Int(*count)))
            .collect(),
    )
}

/// `input.read("RecipesUsed", RECIPES_USED_CODEC).orElse(Map.of())` (`:123-124`):
/// a codec failure drops the whole map.
fn load_recipes_used(value: Option<&NbtSummaryValue>) -> BTreeMap<String, i32> {
    const MAX_RECIPES_USED_ENTRIES: usize = 4096;
    let Some(NbtSummaryValue::Compound(entries)) = value else {
        return BTreeMap::new();
    };
    if entries.len() > MAX_RECIPES_USED_ENTRIES {
        return BTreeMap::new();
    }
    let mut recipes = BTreeMap::new();
    for field in entries {
        let Some(count) = codec_int(&field.value) else {
            return BTreeMap::new();
        };
        let Ok(name) = field.name.try_to_string() else {
            return BTreeMap::new();
        };
        recipes.insert(name, count);
    }
    recipes
}

/// `Containers.dropContents` over the three furnace slots.
#[must_use]
pub fn drop_contents(
    furnace: &AbstractFurnaceBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    furnace
        .original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::block_entity::BlockEntityRemovalDrops;
    use bbb_protocol::NbtSummaryValue;

    use bbb_protocol::block_types::VANILLA_BLOCK_FURNACE_ID;
    use bbb_protocol::item_types::{
        VANILLA_ITEM_COAL_ID, VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_STONE_ID,
    };
    use bbb_protocol::packets::{BlockPos, ItemStackSummary};

    use super::super::unique_field;
    use super::*;
    use crate::block_entity::new_block_entity;

    fn default_state(block_id: i32) -> i32 {
        bbb_protocol::block_state_types::vanilla_block_default_state_id(block_id)
            .expect("a registered block has a default state")
    }

    fn pos() -> BlockPos {
        BlockPos { x: 1, y: 2, z: 3 }
    }

    fn stack(item_id: i32, count: i32) -> RuntimeItemStack {
        RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        })
    }

    fn stone_with_component(name: &str, value: NbtSummaryValue) -> RuntimeItemStack {
        RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::String("minecraft:stone".into())),
                entry("count", NbtSummaryValue::Int(1)),
                entry(
                    "components",
                    NbtSummaryValue::Compound(vec![entry(name, value)]),
                ),
            ]),
            crate::inventory::test_item_resolver(),
        )
        .expect("fixture component stack")
    }

    /// `AbstractFurnaceBlockEntity.saveAdditional` / `loadAdditional`
    /// (`:128-136`, `:115-125`).
    #[test]
    fn furnace_save_round_trips_items_shorts_and_recipes_used() {
        let state = default_state(VANILLA_BLOCK_FURNACE_ID);
        let mut built = new_block_entity(pos(), state).expect("a furnace builds an object");
        {
            let furnace = built.furnace_mut().expect("furnace payload");
            furnace.items[0] = stack(VANILLA_ITEM_STONE_ID, 8);
            furnace.items[1] = stack(VANILLA_ITEM_COAL_ID, 4);
            furnace.cooking_timer = 40;
            furnace.cooking_total_time = 200;
            furnace.lit_time_remaining = 1600;
            furnace.lit_total_time = 1600;
            furnace
                .recipes_used
                .insert("minecraft:iron_ingot".to_string(), 3);
            furnace
                .recipes_used
                .insert("minecraft:cooked_beef".to_string(), 1);
        }

        let record = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture save");
        assert_eq!(
            unique_field(&record, TAG_COOKING_TIME_SPENT),
            Some(&NbtSummaryValue::Short(40))
        );
        assert_eq!(
            unique_field(&record, TAG_COOKING_TOTAL_TIME),
            Some(&NbtSummaryValue::Short(200))
        );
        assert_eq!(
            unique_field(&record, TAG_LIT_TIME_REMAINING),
            Some(&NbtSummaryValue::Short(1600))
        );
        assert_eq!(
            unique_field(&record, TAG_LIT_TOTAL_TIME),
            Some(&NbtSummaryValue::Short(1600))
        );
        let Some(NbtSummaryValue::List(items)) = unique_field(&record, TAG_ITEMS) else {
            panic!("Items is a list: {record:?}");
        };
        assert_eq!(items.len(), 2, "empty result slot is skipped");
        let Some(NbtSummaryValue::Compound(recipes)) = unique_field(&record, TAG_RECIPES_USED)
        else {
            panic!("RecipesUsed is a compound: {record:?}");
        };
        assert_eq!(
            unique_field(recipes, "minecraft:iron_ingot"),
            Some(&NbtSummaryValue::Int(3))
        );
        assert_eq!(
            unique_field(recipes, "minecraft:cooked_beef"),
            Some(&NbtSummaryValue::Int(1))
        );

        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture load")
            .expect("the record names a furnace standing on a furnace");
        let furnace = loaded.furnace().expect("furnace payload");
        assert_eq!(
            furnace.items()[0].summary(),
            stack(VANILLA_ITEM_STONE_ID, 8).summary()
        );
        assert_eq!(
            furnace.items()[1].summary(),
            stack(VANILLA_ITEM_COAL_ID, 4).summary()
        );
        assert!(furnace.items()[2].item_id().is_none());
        assert_eq!(furnace.cooking_time_spent(), 40);
        assert_eq!(furnace.cooking_total_time(), 200);
        assert_eq!(furnace.lit_time_remaining(), 1600);
        assert_eq!(furnace.lit_total_time(), 1600);
        assert_eq!(furnace.recipes_used().get("minecraft:iron_ingot"), Some(&3));
        assert_eq!(
            furnace.recipes_used().get("minecraft:cooked_beef"),
            Some(&1)
        );
        assert_eq!(
            loaded
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver(),)
                .expect("fixture save"),
            record
        );
    }

    /// `serverTick` (`AbstractFurnaceBlockEntity.java:138-206`): fuel lights
    /// the block, the input cooks for the recipe's own time, and one craft
    /// moves the result into the output slot.
    #[test]
    fn server_tick_lights_burns_and_cooks_the_input() {
        let mut furnace = AbstractFurnaceBlockEntity::default();
        furnace
            .set_items(
                crate::inventory::test_item_resolver(),
                &[
                    stack(VANILLA_ITEM_STONE_ID, 2),
                    stack(VANILLA_ITEM_COAL_ID, 2),
                    RuntimeItemStack::empty(),
                ],
                10,
            )
            .expect("fixture slots");
        let facts = FurnaceTickFacts {
            recipe_result: Some(stack(VANILLA_ITEM_STONE_ID, 1)),
            recipe_cooking_time: 10,
            recipe_id: Some("test:smelting".to_string()),
            burn_duration: 1600,
            fuel_remainder: None,
        };

        // The first pass lights the block and spends one fuel item.
        let first = furnace
            .server_tick(crate::inventory::test_item_resolver(), &facts)
            .expect("fixture tick");
        assert_eq!(first.lit_changed, Some(true));
        assert!(first.changed);
        assert_eq!(furnace.lit_time_remaining(), 1600);
        assert_eq!(furnace.lit_total_time(), 1600);
        assert_eq!(furnace.items()[1], stack(VANILLA_ITEM_COAL_ID, 1));
        assert_eq!(furnace.cooking_time_spent(), 1);

        for _ in 0..9 {
            furnace
                .server_tick(crate::inventory::test_item_resolver(), &facts)
                .expect("fixture tick");
        }
        assert_eq!(furnace.cooking_time_spent(), 0);
        assert_eq!(furnace.items()[2], stack(VANILLA_ITEM_STONE_ID, 1));
        assert_eq!(furnace.items()[0], stack(VANILLA_ITEM_STONE_ID, 1));
        assert_eq!(furnace.recipes_used().get("test:smelting"), Some(&1));

        let common =
            stone_with_component("minecraft:rarity", NbtSummaryValue::String("common".into()));
        for (output, result) in [
            (common.clone(), stack(VANILLA_ITEM_STONE_ID, 1)),
            (stack(VANILLA_ITEM_STONE_ID, 1), common),
        ] {
            let mut furnace = AbstractFurnaceBlockEntity::default();
            furnace
                .set_items(
                    crate::inventory::test_item_resolver(),
                    &[
                        stack(VANILLA_ITEM_STONE_ID, 1),
                        stack(VANILLA_ITEM_COAL_ID, 2),
                        output,
                    ],
                    10,
                )
                .expect("fixture slots");
            let facts = FurnaceTickFacts {
                recipe_result: Some(result),
                ..facts.clone()
            };
            for _ in 0..10 {
                furnace
                    .server_tick(crate::inventory::test_item_resolver(), &facts)
                    .expect("fixture tick");
            }
            assert!(furnace.items()[SLOT_INPUT].is_empty());
            assert_eq!(furnace.items()[SLOT_FUEL].count(), 1);
            assert_eq!(
                furnace.items()[SLOT_RESULT].item_id(),
                Some(VANILLA_ITEM_STONE_ID)
            );
            assert_eq!(furnace.items()[SLOT_RESULT].count(), 2);
        }

        for (input, replacement, retains_progress) in [
            (
                stack(VANILLA_ITEM_STONE_ID, 1),
                stone_with_component("minecraft:rarity", NbtSummaryValue::String("common".into())),
                true,
            ),
            (
                stone_with_component("minecraft:rarity", NbtSummaryValue::String("common".into())),
                stack(VANILLA_ITEM_STONE_ID, 1),
                true,
            ),
            (
                stack(VANILLA_ITEM_STONE_ID, 1),
                stone_with_component(
                    "minecraft:rarity",
                    NbtSummaryValue::String("uncommon".into()),
                ),
                false,
            ),
        ] {
            let resolver = crate::inventory::test_item_resolver();
            let mut furnace = AbstractFurnaceBlockEntity::default();
            furnace
                .set_items(
                    resolver,
                    &[
                        input,
                        stack(VANILLA_ITEM_COAL_ID, 1),
                        RuntimeItemStack::empty(),
                    ],
                    10,
                )
                .expect("fixture slots");
            for _ in 0..4 {
                furnace.server_tick(resolver, &facts).expect("fixture tick");
            }
            let mut items = furnace.items();
            items[SLOT_INPUT] = replacement;
            furnace
                .set_items(resolver, &items, 10)
                .expect("replace input");
            for _ in 0..6 {
                furnace.server_tick(resolver, &facts).expect("fixture tick");
            }
            assert_eq!(
                furnace.items()[SLOT_RESULT].count(),
                i32::from(retains_progress)
            );
            assert_eq!(
                furnace.items()[SLOT_INPUT].count(),
                i32::from(!retains_progress)
            );
            if !retains_progress {
                for _ in 0..4 {
                    furnace.server_tick(resolver, &facts).expect("fixture tick");
                }
                assert_eq!(furnace.items()[SLOT_RESULT].count(), 1);
                assert!(furnace.items()[SLOT_INPUT].is_empty());
            }
        }

        // The fuel runs out and the lit state falls back with it.
        furnace
            .set_items(
                crate::inventory::test_item_resolver(),
                &[
                    stack(VANILLA_ITEM_STONE_ID, 1),
                    RuntimeItemStack::empty(),
                    RuntimeItemStack::empty(),
                ],
                10,
            )
            .expect("fixture slots");
        let mut unlit = false;
        for _ in 0..1601 {
            unlit |= furnace
                .server_tick(crate::inventory::test_item_resolver(), &facts)
                .expect("fixture tick")
                .lit_changed
                == Some(false);
        }
        assert!(unlit, "the lit state falls back when the fuel runs out");
        assert_eq!(furnace.lit_time_remaining(), 0);
    }

    /// `canBurn` (`:217-230`): an output slot holding something else stops the
    /// timer instead of consuming the input.
    #[test]
    fn server_tick_stops_when_the_output_slot_holds_another_stack() {
        let facts = FurnaceTickFacts {
            recipe_result: Some(stack(VANILLA_ITEM_STONE_ID, 1)),
            recipe_cooking_time: 10,
            recipe_id: Some("test:smelting".to_string()),
            burn_duration: 1600,
            fuel_remainder: None,
        };
        for output in [
            stack(VANILLA_ITEM_DIAMOND_ID, 1),
            stone_with_component(
                "minecraft:rarity",
                NbtSummaryValue::String("uncommon".into()),
            ),
            stone_with_component("!minecraft:rarity", NbtSummaryValue::Compound(vec![])),
        ] {
            let mut furnace = AbstractFurnaceBlockEntity::default();
            furnace
                .set_items(
                    crate::inventory::test_item_resolver(),
                    &[
                        stack(VANILLA_ITEM_STONE_ID, 1),
                        stack(VANILLA_ITEM_COAL_ID, 1),
                        output.clone(),
                    ],
                    10,
                )
                .expect("fixture slots");
            for _ in 0..20 {
                furnace
                    .server_tick(crate::inventory::test_item_resolver(), &facts)
                    .expect("fixture tick");
            }
            assert_eq!(
                furnace.items()[SLOT_INPUT].item_id(),
                Some(VANILLA_ITEM_STONE_ID)
            );
            assert_eq!(furnace.items()[SLOT_INPUT].count(), 1);
            assert_eq!(furnace.items()[SLOT_FUEL].count(), 1);
            assert_eq!(furnace.items()[SLOT_RESULT].item_id(), output.item_id());
            assert_eq!(furnace.items()[SLOT_RESULT].count(), 1);
        }
    }

    /// `:156-159`: an input no recipe claims leaves the timer alone, and a
    /// furnace that has nothing to burn cools its timer by two a tick
    /// (`:193-195`).
    #[test]
    fn server_tick_leaves_the_timer_alone_without_a_recipe_and_cools_it_unlit() {
        let mut furnace = AbstractFurnaceBlockEntity::default();
        furnace
            .set_items(
                crate::inventory::test_item_resolver(),
                &[
                    stack(VANILLA_ITEM_STONE_ID, 1),
                    RuntimeItemStack::empty(),
                    RuntimeItemStack::empty(),
                ],
                10,
            )
            .expect("fixture slots");
        let no_recipe = FurnaceTickFacts::default();
        let tick = furnace
            .server_tick(crate::inventory::test_item_resolver(), &no_recipe)
            .expect("fixture tick");
        assert_eq!(tick.lit_changed, None);
        assert_eq!(furnace.cooking_time_spent(), 0);

        furnace
            .set_items(
                crate::inventory::test_item_resolver(),
                &[
                    stack(VANILLA_ITEM_STONE_ID, 1),
                    stack(VANILLA_ITEM_COAL_ID, 1),
                    RuntimeItemStack::empty(),
                ],
                10,
            )
            .expect("fixture slots");
        let facts = FurnaceTickFacts {
            recipe_result: Some(stack(VANILLA_ITEM_STONE_ID, 1)),
            recipe_cooking_time: 10,
            recipe_id: Some("test:smelting".to_string()),
            burn_duration: 0,
            fuel_remainder: None,
        };
        // A fuel whose burn duration is zero never lights, so the timer resets.
        let tick = furnace
            .server_tick(crate::inventory::test_item_resolver(), &facts)
            .expect("fixture tick");
        assert_eq!(tick.lit_changed, None);
        assert_eq!(furnace.cooking_time_spent(), 0);
        assert_eq!(furnace.items()[1], stack(VANILLA_ITEM_COAL_ID, 1));
    }

    #[test]
    fn breaking_a_furnace_hands_its_contents_back() {
        let mut furnace = AbstractFurnaceBlockEntity::default();
        furnace.items[0] = stack(VANILLA_ITEM_STONE_ID, 8);
        furnace.items[1] = stack(VANILLA_ITEM_COAL_ID, 4);
        // Slot-aligned with the empty output slot kept: `Containers.dropContents`
        // hands every slot to `dropItemStack` (`Containers.java:25-30`).
        assert_eq!(
            drop_contents(&furnace),
            vec![
                stack(VANILLA_ITEM_STONE_ID, 8),
                stack(VANILLA_ITEM_COAL_ID, 4),
                RuntimeItemStack::empty()
            ]
        );
        // `AbstractFurnaceBlockEntity.preRemoveSideEffects`
        // (`AbstractFurnaceBlockEntity.java:376-381`) hands the inherited
        // container drop *and* the cooked recipes back, so the world can pop
        // their experience.
        furnace
            .recipes_used
            .insert("minecraft:iron_ingot_from_smelting_raw_iron".to_string(), 3);
        let mut built =
            new_block_entity(pos(), default_state(VANILLA_BLOCK_FURNACE_ID)).expect("furnace");
        *built.furnace_mut().expect("furnace payload") = furnace;
        assert_eq!(
            built.pre_remove_side_effects(built.block_state()),
            BlockEntityRemovalDrops::Furnace {
                slots: vec![
                    stack(VANILLA_ITEM_STONE_ID, 8),
                    stack(VANILLA_ITEM_COAL_ID, 4),
                    RuntimeItemStack::empty()
                ],
                recipes_used: vec![("minecraft:iron_ingot_from_smelting_raw_iron".to_string(), 3)],
            }
        );
    }
}
