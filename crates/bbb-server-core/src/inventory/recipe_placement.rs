//! `ServerPlaceRecipe` (`ServerPlaceRecipe.java:18-249`): the recipe book's
//! "place ingredients" request against the open 2x2, crafting table or
//! furnace-family menu.

use crate::entity::item_components::enchantment_count;
use crate::world::maps::ServerMapStore;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use std::collections::BTreeMap;

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID;
use bbb_server_datapack::recipe::ingredient::item_stack;
use bbb_server_datapack::recipe::{Ingredient, ItemTagIndex, RecipeKind};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::item_rules::{has_component, is_damaged};
use super::{DroppedMenuStack, MenuLayout, MenuSlot, PlayerInventoryMenu, INVENTORY_SIZE};

/// `RecipeBookMenu.PostPlaceAction` (`RecipeBookMenu.java:21-24`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostPlaceAction {
    Nothing,
    PlaceGhostRecipe,
}

/// `Inventory.isUsableForCrafting` (`Inventory.java:144-146`).
fn usable_for_crafting(item: &RuntimeItemStack) -> Result<bool> {
    Ok(!is_damaged(item.summary())?
        && enchantment_count(item) == 0
        && !has_component(item.summary(), VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID)?)
}

fn max_stack_of(item_id: i32) -> Result<i32> {
    RuntimeItemStack::from_summary(item_stack(item_id, 1)).max_stack_size()
}

/// `StackedContents.RecipePicker` (`StackedContents.java:116-407`): each
/// ingredient takes `amount` of one item kind, an item kind with `count`
/// available serves `count / amount` ingredients.
struct StackedItems<'a> {
    items: Vec<(i32, i32)>,
    accepts: Vec<Vec<usize>>,
    ingredients: &'a [&'a Ingredient],
}

impl<'a> StackedItems<'a> {
    fn new(
        available: &BTreeMap<i32, i32>,
        ingredients: &'a [&'a Ingredient],
        tags: &ItemTagIndex,
    ) -> Self {
        let items: Vec<(i32, i32)> = available
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(id, count)| (*id, *count))
            .collect();
        let accepts = ingredients
            .iter()
            .map(|ingredient| {
                (0..items.len())
                    .filter(|&index| ingredient.test(&item_stack(items[index].0, 1), tags))
                    .collect()
            })
            .collect();
        Self {
            items,
            accepts,
            ingredients,
        }
    }

    /// `RecipePicker.tryPick` (`:145-195`): the item chosen per ingredient.
    fn pick(&self, amount: i32) -> Option<Vec<i32>> {
        if amount <= 0 {
            return Some(Vec::new());
        }
        let capacity: Vec<usize> = self
            .items
            .iter()
            .map(|(_, count)| usize::try_from(count / amount).unwrap_or(0))
            .collect();
        let mut owners: Vec<Vec<usize>> = vec![Vec::new(); self.items.len()];
        let mut assigned = vec![0; self.ingredients.len()];
        for ingredient in 0..self.ingredients.len() {
            let mut visited = vec![false; self.items.len()];
            if !self.augment(
                ingredient,
                &capacity,
                &mut owners,
                &mut assigned,
                &mut visited,
            ) {
                return None;
            }
        }
        Some(assigned.iter().map(|&item| self.items[item].0).collect())
    }

    fn augment(
        &self,
        ingredient: usize,
        capacity: &[usize],
        owners: &mut [Vec<usize>],
        assigned: &mut [usize],
        visited: &mut [bool],
    ) -> bool {
        for &item in &self.accepts[ingredient] {
            if std::mem::replace(&mut visited[item], true) {
                continue;
            }
            if owners[item].len() < capacity[item] {
                owners[item].push(ingredient);
                assigned[ingredient] = item;
                return true;
            }
            for owner in 0..owners[item].len() {
                let other = owners[item][owner];
                if self.augment(other, capacity, owners, assigned, visited) {
                    owners[item][owner] = ingredient;
                    assigned[ingredient] = item;
                    return true;
                }
            }
        }
        false
    }

    /// `RecipePicker.tryPickAll` (`:386-406`) bounded by
    /// `getResultUpperBound` (`:76-104`).
    fn biggest_craftable(&self) -> i32 {
        let upper = self
            .accepts
            .iter()
            .map(|items| {
                items
                    .iter()
                    .map(|&item| self.items[item].1)
                    .max()
                    .unwrap_or(0)
            })
            .min()
            .unwrap_or(0);
        let (mut min, mut max) = (0, upper + 1);
        loop {
            let mid = (min + max) / 2;
            if self.pick(mid).is_some() {
                if max - min <= 1 {
                    return mid;
                }
                min = mid;
            } else {
                max = mid;
            }
        }
    }
}

impl PlayerInventoryMenu {
    /// `RecipeBookMenu.handlePlacement` of the open menu
    /// (`AbstractCraftingMenu.java:38-66`, `AbstractFurnaceMenu.java:173-194`)
    /// through `ServerPlaceRecipe.placeRecipe` (`ServerPlaceRecipe.java:28-48`).
    /// Stacks the cleared grid could not put back come back for `Player.drop`.
    pub(crate) fn place_recipe(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        recipe_id: &str,
        use_max_items: bool,
        allow_dropping_items_to_clear: bool,
    ) -> Result<(PostPlaceAction, Vec<DroppedMenuStack>)> {
        let nothing = Ok((PostPlaceAction::Nothing, Vec::new()));
        let Some(recipes) = self.crafting.recipes.clone() else {
            return nothing;
        };
        let Some(element) = recipes.recipe(recipe_id) else {
            return nothing;
        };
        let recipe = &element.recipe;
        let cooking = match &recipe.kind {
            RecipeKind::Cooking(cooking) => Some(cooking),
            _ => None,
        };
        let crafting = recipe.is_crafting();
        let (width, inputs, clear, accounted): (i32, Vec<i16>, Vec<i16>, Vec<i16>) = match self
            .menu
            .layout
        {
            MenuLayout::PlayerInventory if crafting => {
                (2, (1..5).collect(), (1..5).collect(), (1..5).collect())
            }
            MenuLayout::CraftingTable if crafting => {
                (3, (1..10).collect(), (1..10).collect(), (1..10).collect())
            }
            MenuLayout::Furnace { .. } if cooking.is_some() => (1, vec![0], vec![0, 2], vec![0, 2]),
            _ => return nothing,
        };
        let tags = &self.crafting.tags;
        let Ok(placement) = recipe.grid_placement(width, width, tags) else {
            return nothing;
        };
        // `ServerPlaceRecipe.java:199` / `AbstractFurnaceMenu.java:193`: test only input slots.
        if !allow_dropping_items_to_clear && !self.test_clear_grid(resolver, &inputs)? {
            return nothing;
        }
        // `Inventory.fillStackedContents` (`Inventory.java:514-518`) and the
        // menu's own slots: a crafting grid accounts like the inventory, a
        // furnace's block entity accounts every slot as it is.
        let mut available = BTreeMap::new();
        for item in &self.inventory[..INVENTORY_SIZE] {
            if !item.is_empty() && usable_for_crafting(item)? {
                *available
                    .entry(item.item_id().expect("non-empty"))
                    .or_insert(0) += item.count().min(item.max_stack_size()?);
            }
        }
        for &slot in &accounted {
            let item = self.slot_item(slot);
            if item.is_empty() || (crafting && !usable_for_crafting(&item)?) {
                continue;
            }
            *available
                .entry(item.item_id().expect("non-empty"))
                .or_insert(0) += item.count().min(item.max_stack_size()?);
        }
        let stacked = StackedItems::new(&available, placement.ingredients(), tags);
        if stacked.pick(1).is_none() {
            let dropped =
                self.clear_recipe_grid(resolver, maps.as_deref_mut(), &clear, crafting)?;
            return Ok((PostPlaceAction::PlaceGhostRecipe, dropped));
        }

        // `ServerPlaceRecipe.placeRecipe` (`:90-135`).
        let matches_placed = match cooking {
            Some(cooking) => cooking.matches(self.slot_item(0).summary(), tags),
            None => {
                let input = self.crafting.input(width, width, self.craft_grid())?.input;
                recipe.matches_crafting(
                    resolver,
                    &input,
                    tags,
                    maps.as_deref_mut()
                        .and_then(|maps| maps.crafting_map_data(&input)),
                )
            }
        };
        let biggest = stacked.biggest_craftable();
        let placed: Vec<RuntimeItemStack> = inputs
            .iter()
            .map(|&slot| self.slot_item(slot).into_owned())
            .filter(|item| !item.is_empty())
            .collect();
        if matches_placed {
            for item in &placed {
                if biggest.min(item.max_stack_size()?) < item.count() + 1 {
                    return nothing;
                }
            }
        }
        // `calculateAmountToCraft` (`:145-168`).
        let amount = if use_max_items {
            biggest
        } else if matches_placed {
            placed
                .iter()
                .map(RuntimeItemStack::count)
                .min()
                .map_or(i32::MAX, |n| n + 1)
        } else {
            1
        };
        let Some(mut picks) = stacked.pick(amount) else {
            return nothing;
        };
        // `clampToMaxStackSize` (`:137-143`).
        let mut adjusted = amount;
        for &item in &picks {
            adjusted = adjusted.min(max_stack_of(item)?);
        }
        if adjusted != amount {
            match stacked.pick(adjusted) {
                Some(repicked) => picks = repicked,
                None => return nothing,
            }
        }
        let slots = placement.slots().to_vec();
        let dropped = self.clear_recipe_grid(resolver, maps.as_deref_mut(), &clear, false)?;
        for (grid_index, &slot) in inputs.iter().enumerate() {
            let Some(&item) = slots
                .get(grid_index)
                .and_then(|&index| usize::try_from(index).ok())
                .and_then(|index| picks.get(index))
            else {
                continue;
            };
            let mut remaining = adjusted;
            while remaining > 0 {
                match self.move_item_to_grid(resolver, slot, item, remaining)? {
                    Some(left) => remaining = left,
                    None => break,
                }
            }
        }
        if crafting {
            // `CraftingMenu.finishPlacingRecipe` (`CraftingMenu.java:92-95`).
            self.open_crafting_slots_changed(resolver, maps)?;
        }
        Ok((PostPlaceAction::Nothing, dropped))
    }

    fn placement_slot_mut(&mut self, slot: i16) -> Option<&mut RuntimeItemStack> {
        match self.menu.layout.slot(slot)? {
            MenuSlot::CraftingGrid(index) => self.craft_grid_mut().get_mut(index),
            MenuSlot::CraftingResult => Some(self.craft_result_mut()),
            MenuSlot::Container(index) => self.container.get_mut(index),
            _ => None,
        }
    }

    /// `ServerPlaceRecipe.clearGrid` (`:80-88`): each slot goes back into
    /// the inventory, then `clearCraftingContent` empties the slots.
    fn clear_recipe_grid(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        slots: &[i16],
        rematch: bool,
    ) -> Result<Vec<DroppedMenuStack>> {
        let mut dropped = Vec::new();
        for &slot in slots {
            let mut item = self.slot_item(slot).into_owned();
            self.place_item_back_in_inventory(resolver, &mut item, false)?;
            if !item.is_empty() {
                dropped.push(DroppedMenuStack {
                    stack: item,
                    thrown_from_hand: false,
                });
            }
            if let Some(target) = self.placement_slot_mut(slot) {
                *target = RuntimeItemStack::empty();
            }
        }
        if rematch {
            self.open_crafting_slots_changed(resolver, maps)?;
        }
        Ok(dropped)
    }

    /// `ServerPlaceRecipe.moveItemToGrid` (`:170-193`), `None` when no
    /// inventory slot holds the item any more.
    fn move_item_to_grid(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        slot: i16,
        item_id: i32,
        count: i32,
    ) -> Result<Option<i32>> {
        let target = self.slot_item(slot).into_owned();
        // `Inventory.findSlotMatchingCraftingIngredient` (`Inventory.java:148-160`).
        let mut found = None;
        for (index, stack) in self.inventory[..INVENTORY_SIZE].iter().enumerate() {
            if stack.item_id() == Some(item_id)
                && usable_for_crafting(stack)?
                && (target.is_empty()
                    || target.same_item_same_effective_components(stack, resolver)?)
            {
                found = Some(index);
                break;
            }
        }
        let Some(index) = found else {
            return Ok(None);
        };
        let mut source = self.inventory[index].clone();
        let taken = source.split(count);
        self.set_inventory_slot(index, source);
        let taken_count = taken.count();
        if let Some(grid) = self.placement_slot_mut(slot) {
            if grid.is_empty() {
                *grid = taken;
            } else {
                grid.grow(taken_count);
            }
        }
        Ok(Some(count - taken_count))
    }

    /// `ServerPlaceRecipe.testClearGrid` (`:195-228`).
    fn test_clear_grid(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        inputs: &[i16],
    ) -> Result<bool> {
        let free_in_inventory = self.inventory[..INVENTORY_SIZE]
            .iter()
            .filter(|item| item.is_empty())
            .count();
        let mut free_slots: Vec<RuntimeItemStack> = Vec::new();
        for &slot in inputs {
            let mut item = self.slot_item(slot).into_owned();
            if item.is_empty() || self.slot_with_remaining_space(resolver, &item)?.is_some() {
                continue;
            }
            if free_slots.len() > free_in_inventory {
                return Ok(false);
            }
            for listed in &mut free_slots {
                let max = listed.max_stack_size()?;
                if listed.item_id() == item.item_id()
                    && listed.count() != max
                    && listed.count() + item.count() <= max
                {
                    listed.grow(item.count());
                    item = RuntimeItemStack::empty();
                    break;
                }
            }
            if !item.is_empty() {
                if free_slots.len() >= free_in_inventory {
                    return Ok(false);
                }
                free_slots.push(item);
            }
        }
        Ok(true)
    }
}
