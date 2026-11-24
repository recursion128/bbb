//!
//! It is a live recipe question, not a table: `findColorMixInRecipes`
//! (`:129-156`) asks the item registry which items carry `DataComponents.DYE`
//! for each colour, crafts one of each in a 2x1 grid against the level's own
//! `RecipeType.CRAFTING`, and reads the `DYE` component back off the result.
//! Only when no pair crafts anything does it flip a coin on the level's random
//! (`:125`).

use crate::world::simulation_level::LevelCtx;
use std::collections::BTreeMap;
use std::sync::OnceLock;

use bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_VALUES;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DYE_TYPE_ID;
use bbb_protocol::packets::{DataComponentPatchSummary, ItemStackSummary};
use bbb_protocol::vanilla_item_prototype_component_catalog;
use bbb_server_datapack::recipe::CraftingInput;

///
/// Built from the generated prototype components rather than from item names,
/// so an item that stopped carrying the component drops out of it. 26.1 gives
/// each colour exactly one carrier, which is why the nested loop at `:141-142`
/// has one pair to try.
fn dye_items() -> &'static BTreeMap<i32, Vec<i32>> {
    static INDEX: OnceLock<BTreeMap<i32, Vec<i32>>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
        let Ok(catalog) = vanilla_item_prototype_component_catalog() else {
            return index;
        };
        for (item_type_id, components) in catalog
            .component_codec_values_by_item_id()
            .iter()
            .enumerate()
        {
            let Ok(item_type_id) = i32::try_from(item_type_id) else {
                continue;
            };
            if let Some(color) = dye_component_color(components) {
                index.entry(color).or_default().push(item_type_id);
            }
        }
        index
    })
}

/// The `DyeColor` a prototype's `minecraft:dye` component names; the codec
/// writes the colour's serialized name.
fn dye_component_color(components: &BTreeMap<i32, serde_json::Value>) -> Option<i32> {
    let name = components
        .get(&VANILLA_DATA_COMPONENT_DYE_TYPE_ID)?
        .as_str()?;
    VANILLA_DYE_COLOR_VALUES
        .iter()
        .find(|entry| entry.serialized_name == name)
        .map(|entry| entry.id)
}

///
/// The coin flip at `:125` is reached only when no pair of carriers crafts a
/// dyed result, so a level that answers the recipe question spends no draw.
pub(crate) fn mixed_color(level: &mut LevelCtx<'_, '_>, first: i32, second: i32) -> i32 {
    if let Some(mixed) = find_color_mix_in_recipes(level, first, second) {
        return mixed;
    }
    if level.level_random().next_boolean() {
        first
    } else {
        second
    }
}

fn find_color_mix_in_recipes(level: &LevelCtx<'_, '_>, first: i32, second: i32) -> Option<i32> {
    let facts = level.crafting_recipes()?;
    let resolver = level.item_stack_resolver();
    let index = dye_items();
    let first_items = index.get(&first)?;
    let second_items = index.get(&second)?;
    for first_item in first_items {
        for second_item in second_items {
            let input = CraftingInput::of(2, 1, vec![one_of(*first_item), one_of(*second_item)]);
            let Some(element) = facts.recipe_index.get_recipe_for_crafting(
                facts.recipes,
                resolver,
                &input,
                None,
                facts.item_tags,
                None,
            ) else {
                continue;
            };
            let result = element.recipe.assemble_crafting(&input, facts.item_tags);
            if let Some(color) = result_dye_color(&result) {
                return Some(color);
            }
        }
    }
    None
}

fn one_of(item_type_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_type_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn result_dye_color(result: &ItemStackSummary) -> Option<i32> {
    let catalog = vanilla_item_prototype_component_catalog().ok()?;
    dye_component_color(catalog.component_codec_values(result.item_id?)?)
}

/// The carrier item one colour resolves to, for the tests that pin the index.
#[cfg(test)]
pub(crate) fn dye_items_of(color: i32) -> Vec<i32> {
    dye_items().get(&color).cloned().unwrap_or_default()
}

///
/// `Err(())` is a patch that names `DYE` in a shape this module cannot read,
/// which fails the reading arm closed rather than answering the prototype's
/// colour for a stack that overrode it.
pub(crate) fn stack_dye_color(stack: &ItemStackSummary) -> Result<Option<i32>, ()> {
    let Some(item_id) = stack.item_id else {
        return Ok(None);
    };
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_DYE_TYPE_ID)
    {
        return Ok(None);
    }
    if let Some(color) = stack.component_patch.dye_color_id {
        return VANILLA_DYE_COLOR_VALUES
            .iter()
            .find(|entry| entry.id == color)
            .map(|entry| Some(entry.id))
            .ok_or(());
    }
    if let Some(bbb_protocol::NbtSummaryValue::String(name)) = stack
        .component_patch
        .codec_backed_nbt
        .get(&VANILLA_DATA_COMPONENT_DYE_TYPE_ID)
    {
        return name
            .with_utf8(color_for_serialized_name)
            .flatten()
            .map(Some)
            .ok_or(());
    }
    if stack
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_DYE_TYPE_ID)
        || stack
            .component_patch
            .codec_backed_nbt
            .contains_key(&VANILLA_DATA_COMPONENT_DYE_TYPE_ID)
    {
        return Err(());
    }
    let Ok(catalog) = vanilla_item_prototype_component_catalog() else {
        return Err(());
    };
    Ok(catalog
        .component_codec_values(item_id)
        .and_then(dye_component_color))
}

fn color_for_serialized_name(name: &str) -> Option<i32> {
    VANILLA_DYE_COLOR_VALUES
        .iter()
        .find(|entry| entry.serialized_name == name)
        .map(|entry| entry.id)
}
