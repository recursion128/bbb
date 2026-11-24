//! `StonecutterMenu` over the level's stonecutting recipes, in the order the
//! client was sent them.

use crate::world::maps::ServerMapStore;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::recipe::RecipeKind;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::{PlayerInventoryMenu, UtilityMenuEvent};
use crate::inventory::DroppedMenuStack;

const INPUT: usize = 0;
const RESULT: usize = 1;
/// The `selectedRecipeIndex` data slot.
const SELECTED: usize = 0;

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct StonecutterState {
    /// `StonecutterMenu.input`'s item, which `slotsChanged` compares against.
    input_item: Option<i32>,
    /// `recipesForInput`: the matching recipe ids.
    recipes: Vec<String>,
}

impl PlayerInventoryMenu {
    /// `StonecutterMenu.slotsChanged` (`StonecutterMenu.java:128-134`).
    pub(super) fn stonecutter_slots_changed(&mut self) -> Result<()> {
        let input = self.container[INPUT].item_id();
        if input == self.utility().stonecutter.input_item {
            return Ok(());
        }
        self.utility_mut().stonecutter.input_item = input;
        // `setupRecipeList` (`StonecutterMenu.java:136-144`).
        self.utility_mut().data[SELECTED] = -1;
        self.container[RESULT] = RuntimeItemStack::empty();
        let recipes = self.stonecutter_recipes_for(&self.container[INPUT].clone());
        self.utility_mut().stonecutter.recipes = recipes;
        Ok(())
    }

    /// `SelectableRecipe.SingleInputSet.selectByInput` (`SelectableRecipe.java:47-49`).
    fn stonecutter_recipes_for(&self, input: &RuntimeItemStack) -> Vec<String> {
        let Some(recipes) = &self.crafting.recipes else {
            return Vec::new();
        };
        if input.is_empty() {
            return Vec::new();
        }
        recipes
            .recipes
            .iter()
            .filter(|element| match &element.recipe.kind {
                RecipeKind::Stonecutting(recipe) => {
                    recipe.ingredient.test(input.summary(), &self.crafting.tags)
                }
                _ => false,
            })
            .map(|element| element.id.clone())
            .collect()
    }

    /// `StonecutterMenu.setupResultSlot` (`StonecutterMenu.java:146-163`).
    fn stonecutter_setup_result(&mut self, index: i32) {
        let recipe = usize::try_from(index)
            .ok()
            .and_then(|index| self.utility().stonecutter.recipes.get(index).cloned());
        let result = recipe
            .as_deref()
            .and_then(|id| self.crafting.recipes.as_ref()?.recipe(id))
            .and_then(|element| match &element.recipe.kind {
                RecipeKind::Stonecutting(recipe) => {
                    Some(RuntimeItemStack::from_summary(recipe.result.create()))
                }
                _ => None,
            })
            .unwrap_or_default();
        self.utility_mut().recipe_used = recipe;
        self.container[RESULT] = result;
    }

    /// `StonecutterMenu.clickMenuButton` (`StonecutterMenu.java:110-121`).
    pub(super) fn stonecutter_button(&mut self, button_id: i32) -> Result<bool> {
        if self.utility().data[SELECTED] == button_id {
            return Ok(false);
        }
        let valid = usize::try_from(button_id)
            .is_ok_and(|index| index < self.utility().stonecutter.recipes.len());
        if valid {
            self.utility_mut().data[SELECTED] = button_id;
            self.stonecutter_setup_result(button_id);
        }
        Ok(true)
    }

    /// The result slot's `onTake` (`StonecutterMenu.java:62-82`).
    pub(super) fn stonecutter_on_take(&mut self, taken: &RuntimeItemStack) -> Result<()> {
        if let Some(item_id) = taken.item_id() {
            *self.crafting.crafted_counts.entry(item_id).or_insert(0) += taken.count();
        }
        let recipe = self.utility().recipe_used.clone();
        if let Some(recipe_id) = &recipe {
            self.crafting
                .pending_awards
                .push(crate::inventory::crafting::CraftedRecipe {
                    recipe_id: recipe_id.clone(),
                    ingredients: vec![self.container[INPUT].clone()],
                    unlock: false,
                });
        }
        let removed = self.container[INPUT].split(1);
        self.stonecutter_slots_changed()?;
        if !removed.is_empty() {
            let selected = self.utility().data[SELECTED];
            self.stonecutter_setup_result(selected);
        }
        self.utility_mut()
            .events
            .push(UtilityMenuEvent::Stonecut { recipe });
        Ok(())
    }

    /// `StonecutterMenu.quickMoveStack` (`StonecutterMenu.java:180-228`).
    pub(super) fn stonecutter_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        dropped: &mut Vec<DroppedMenuStack>,
    ) -> Result<RuntimeItemStack> {
        let clicked = self.slot_item(slot).into_owned();
        let mut stack = clicked.clone();
        let accepts = !self.stonecutter_recipes_for(&stack).is_empty();
        let moved = if slot == RESULT as i16 {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 2, 38, true)?
        } else if slot == INPUT as i16 {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 2, 38, false)?
        } else if accepts {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 0, 1, false)?
        } else if (2..29).contains(&slot) {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 29, 38, false)?
        } else if (29..38).contains(&slot) {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 2, 29, false)?
        } else {
            true
        };
        if !moved {
            return Ok(RuntimeItemStack::empty());
        }
        let remainder = stack.clone();
        let moved = self.finish_utility_move(resolver, maps, slot, &clicked, stack)?;
        if slot == RESULT as i16 && !moved.is_empty() && !remainder.is_empty() {
            dropped.push(DroppedMenuStack {
                stack: remainder,
                thrown_from_hand: false,
            });
        }
        Ok(moved)
    }
}
