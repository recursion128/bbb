//! `SmithingMenu` over the level's `smithing_transform` and `smithing_trim`
//! recipes.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
};
use bbb_protocol::packets::DataComponentPatchSummary;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::recipe::ingredient::test_optional_ingredient;
use bbb_server_datapack::recipe::RecipeKind;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::{PlayerInventoryMenu, UtilityMenuEvent};

const TEMPLATE: usize = 0;
const BASE: usize = 1;
const ADDITION: usize = 2;
const RESULT: usize = 3;
/// The `hasRecipeError` data slot.
const RECIPE_ERROR: usize = 0;

impl PlayerInventoryMenu {
    /// The three input slots' `RecipePropertySet` tests
    /// (`SmithingMenu.java:53-63`).
    pub(super) fn smithing_may_place(&self, slot: i16, item: &RuntimeItemStack) -> bool {
        let Ok(input) = usize::try_from(slot) else {
            return false;
        };
        self.smithing_property_set(input, item)
    }

    fn smithing_property_set(&self, input: usize, item: &RuntimeItemStack) -> bool {
        let Some(smithing) = &self.crafting.smithing else {
            return false;
        };
        if item.is_empty() {
            return false;
        }
        item.item_id()
            .is_some_and(|item_id| smithing.inputs[input.min(ADDITION)].contains(&item_id))
    }

    /// `SmithingMenu.canMoveIntoInputSlots` (`SmithingMenu.java:134-140`).
    pub(super) fn smithing_can_move_in(&self, item: &RuntimeItemStack) -> bool {
        [TEMPLATE, BASE, ADDITION].into_iter().any(|input| {
            self.smithing_property_set(input, item) && self.container[input].is_empty()
        })
    }

    /// `ItemCombinerMenu.slotsChanged` into `SmithingMenu.createResult`
    /// (`SmithingMenu.java:109-125`), then `SmithingMenu.slotsChanged`'s
    /// recipe-error flag (`:97-106`).
    pub(super) fn smithing_slots_changed(&mut self) -> Result<()> {
        let found = self.smithing_recipe()?;
        let (recipe, result) = found.unwrap_or_default();
        self.utility_mut().recipe_used = recipe;
        self.container[RESULT] = result;
        let error = [TEMPLATE, BASE, ADDITION]
            .iter()
            .all(|&slot| !self.container[slot].is_empty())
            && self.container[RESULT].is_empty();
        self.utility_mut().data[RECIPE_ERROR] = i32::from(error);
        Ok(())
    }

    /// `RecipeManager.getRecipeFor(RecipeType.SMITHING, input)` and the found
    /// recipe's `assemble`.
    fn smithing_recipe(&self) -> Result<Option<(Option<String>, RuntimeItemStack)>> {
        let (Some(recipes), Some(smithing), Some(facts)) = (
            &self.crafting.recipes,
            &self.crafting.smithing,
            self.facts(),
        ) else {
            return Ok(None);
        };
        let [template, base, addition] =
            [TEMPLATE, BASE, ADDITION].map(|slot| self.container[slot].summary());
        if [template, base, addition]
            .iter()
            .all(|stack| stack.item_id.is_none() || stack.count <= 0)
        {
            return Ok(None);
        }
        let tags = &self.crafting.tags;
        for &index in &smithing.recipe_indices {
            let element = &recipes.recipes[index];
            let result = match &element.recipe.kind {
                // `SmithingRecipe.matches` (`SmithingRecipe.java:15-19`).
                RecipeKind::SmithingTransform(recipe)
                    if test_optional_ingredient(recipe.template.as_ref(), template, tags)
                        && recipe.base.test(base, tags)
                        && test_optional_ingredient(recipe.addition.as_ref(), addition, tags) =>
                {
                    // `SmithingTransformRecipe.assemble` (`SmithingTransformRecipe.java:60-62`).
                    recipe
                        .result
                        .apply(recipe.result.count, &base.component_patch)
                        .map(RuntimeItemStack::from_summary)
                        .unwrap_or_default()
                }
                RecipeKind::SmithingTrim(recipe)
                    if recipe.template.test(template, tags)
                        && recipe.base.test(base, tags)
                        && recipe.addition.test(addition, tags) =>
                {
                    apply_trim(
                        &self.container[BASE],
                        &self.container[ADDITION],
                        &recipe.pattern,
                        &facts.resolver,
                    )?
                }
                _ => continue,
            };
            return Ok(Some((Some(element.id.clone()), result)));
        }
        Ok(None)
    }

    /// `SmithingMenu.onTake` (`SmithingMenu.java:71-82`).
    pub(super) fn smithing_on_take(&mut self, taken: &RuntimeItemStack) -> Result<()> {
        if let Some(item_id) = taken.item_id() {
            *self.crafting.crafted_counts.entry(item_id).or_insert(0) += taken.count();
        }
        let recipe = self.utility().recipe_used.clone();
        if let Some(recipe_id) = &recipe {
            self.crafting
                .pending_awards
                .push(crate::inventory::crafting::CraftedRecipe {
                    recipe_id: recipe_id.clone(),
                    ingredients: self.container[..RESULT].to_vec(),
                    unlock: false,
                });
        }
        for slot in [TEMPLATE, BASE, ADDITION] {
            if !self.container[slot].is_empty() {
                self.container[slot].shrink(1);
            }
        }
        self.smithing_slots_changed()?;
        self.utility_mut()
            .events
            .push(UtilityMenuEvent::Smithed { recipe });
        Ok(())
    }
}

/// `SmithingTrimRecipe.applyTrim` (`SmithingTrimRecipe.java:65-80`).
fn apply_trim(
    base: &RuntimeItemStack,
    material: &RuntimeItemStack,
    pattern: &str,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<RuntimeItemStack> {
    let Some(provides) = material.effective_component(
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        resolver,
    )?
    else {
        return Ok(RuntimeItemStack::empty());
    };
    let Some(pattern_id) = resolver.holder_id("minecraft:trim_pattern", pattern) else {
        return Ok(RuntimeItemStack::empty());
    };
    let trim = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_TRIM_TYPE_ID],
        armor_trim_material_id: provides.provides_trim_material_id,
        armor_trim_material_direct: provides.provides_trim_material_direct.clone(),
        armor_trim_pattern_id: Some(pattern_id),
        ..Default::default()
    };
    if let Some(existing) =
        base.effective_component(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID, resolver)?
    {
        if existing.armor_trim_material_id == trim.armor_trim_material_id
            && existing.armor_trim_material_direct == trim.armor_trim_material_direct
            && existing.armor_trim_pattern_id == trim.armor_trim_pattern_id
            && existing.armor_trim_pattern_direct.is_none()
        {
            return Ok(RuntimeItemStack::empty());
        }
    }
    base.with_count(1).with_component_patch(&trim, resolver)
}
