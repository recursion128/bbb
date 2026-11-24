//! `ShapelessRecipe` (`ShapelessRecipe.java:60-67`).

use super::ingredient::{Ingredient, ItemStackTemplate, ItemTagIndex};
use super::input::CraftingInput;
use super::RecipeDecodeError;
use bbb_registry_codec::{
    registry_codec::RegistryReferenceLookup, PersistentItemStackHolderResolver,
    RegistryElementValueRef,
};

fn decode_error(message: impl Into<String>) -> RecipeDecodeError {
    RecipeDecodeError {
        message: message.into(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapelessRecipe {
    pub ingredients: Vec<Ingredient>,
    pub result: ItemStackTemplate,
}

impl ShapelessRecipe {
    pub(super) fn parse_source(
        root: RegistryElementValueRef<'_>,
        lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
    ) -> Result<Self, RecipeDecodeError> {
        use super::source::{list, required, template};
        let raw = list(required(root, "ingredients")?, "ingredients")?;
        if raw.len() == 0 || raw.len() > 9 {
            return Err(decode_error("shapeless ingredients must be 1..=9"));
        }
        let content_registry = lookup.content_registry();
        Ok(Self {
            ingredients: raw
                .map(|value| {
                    Ingredient::parse_source_with_content_registry(value, content_registry)
                })
                .collect::<Result<_, _>>()?,
            result: template(required(root, "result")?, lookup)?,
        })
    }

    /// `ShapelessRecipe.matches` (`ShapelessRecipe.java:60-67`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        if input.ingredient_count() != i32::try_from(self.ingredients.len()).unwrap_or(0) {
            return false;
        }
        if input.size() == 1 && self.ingredients.len() == 1 {
            return self.ingredients[0].test(input.get_item(0), tags);
        }
        can_craft(&self.ingredients, &input.occupied(), tags)
    }
}

/// Compact bipartite assign for `StackedItemContents.canCraft` (≤9 slots).
fn can_craft(
    ingredients: &[Ingredient],
    stacks: &[&bbb_protocol::packets::ItemStackSummary],
    tags: &ItemTagIndex,
) -> bool {
    if ingredients.len() != stacks.len() {
        return false;
    }
    let mut used = vec![false; stacks.len()];
    assign(0, ingredients, stacks, tags, &mut used)
}

fn assign(
    index: usize,
    ingredients: &[Ingredient],
    stacks: &[&bbb_protocol::packets::ItemStackSummary],
    tags: &ItemTagIndex,
    used: &mut [bool],
) -> bool {
    if index == ingredients.len() {
        return true;
    }
    for (slot, stack) in stacks.iter().enumerate() {
        if used[slot] || !ingredients[index].test(stack, tags) {
            continue;
        }
        used[slot] = true;
        if assign(index + 1, ingredients, stacks, tags, used) {
            return true;
        }
        used[slot] = false;
    }
    false
}
