use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};
use crate::data_component_types::vanilla_data_component_type;
pub use crate::recipe_wire_types::{RecipeBookType, RecipeDisplayType};
use crate::recipe_wire_types::{
    VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID, VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID,
    VANILLA_SLOT_DISPLAY_TYPE_DYED_ID, VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
    VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID, VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
    VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID, VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID,
    VANILLA_SLOT_DISPLAY_TYPE_TAG_ID, VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID,
    VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
};

use super::super::read_resource_location;
use super::super::{
    data_components::decode_trim_pattern_holder, inventory, DataComponentPatchSummary,
    ItemStackSummary, TrimPatternSummary,
};

const MAX_RECIPE_PROPERTY_SET_ITEMS: usize = 65_536;
const MAX_RECIPE_PROPERTY_SETS: usize = 4096;
pub(super) const MAX_RECIPE_DISPLAY_BODY: usize = 2 * 1024 * 1024;
const MAX_RECIPE_BOOK_ENTRIES: usize = 65_536;
pub(super) const MAX_RECIPE_BOOK_NESTED_LIST: usize = 4096;
pub(super) const MAX_SLOT_DISPLAY_DEPTH: usize = 64;
const MAX_STONECUTTER_RECIPE_ENTRIES: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceGhostRecipe {
    pub container_id: i32,
    pub recipe_display: RecipeDisplaySummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBookAdd {
    pub entries: Vec<RecipeBookAddEntry>,
    pub replace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBookAddEntry {
    pub contents: RecipeDisplayEntry,
    pub flags: u8,
    pub notification: bool,
    pub highlight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBookRemove {
    pub recipe_ids: Vec<RecipeDisplayId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateRecipes {
    pub property_sets: Vec<RecipePropertySetSummary>,
    pub stonecutter_recipes: Vec<StonecutterSelectableRecipeSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipePropertySetSummary {
    pub key: String,
    pub item_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StonecutterSelectableRecipeSummary {
    pub input: IngredientSummary,
    pub option_display: SlotDisplaySummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotDisplaySummary {
    pub display_type_id: i32,
    pub raw_payload: Vec<u8>,
    pub item_stack: Option<ItemStackSummary>,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotDisplayValue {
    Empty,
    AnyFuel,
    WithAnyPotion {
        display: Box<SlotDisplayValue>,
    },
    OnlyWithComponent {
        source: Box<SlotDisplayValue>,
        component_type_id: i32,
    },
    Item {
        stack: Box<ItemStackSummary>,
    },
    ItemStack {
        stack: Box<ItemStackSummary>,
    },
    Tag {
        tag: String,
    },
    Dyed {
        dye: Box<SlotDisplayValue>,
        target: Box<SlotDisplayValue>,
    },
    SmithingTrim {
        base: Box<SlotDisplayValue>,
        material: Box<SlotDisplayValue>,
        pattern: SlotDisplayTrimPatternSummary,
    },
    WithRemainder {
        input: Box<SlotDisplayValue>,
        remainder: Box<SlotDisplayValue>,
    },
    Composite {
        contents: Vec<SlotDisplayValue>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotDisplayTrimPatternSummary {
    pub registry_id: Option<i32>,
    pub direct: Option<TrimPatternSummary>,
}

impl SlotDisplaySummary {
    pub fn decode_value(&self) -> Result<SlotDisplayValue> {
        let mut decoder = Decoder::new(&self.raw_payload);
        let budget = SlotDisplayDecodeBudget::new(decoder.position());
        let (display_type_id, value) = decode_slot_display_value(&mut decoder, 0, &budget)?;
        if display_type_id != self.display_type_id {
            return Err(ProtocolError::InvalidData(format!(
                "slot display summary type {} does not match raw type {display_type_id}",
                self.display_type_id
            )));
        }
        if !decoder.is_empty() {
            return Err(ProtocolError::InvalidData(
                "trailing bytes after slot display value".to_string(),
            ));
        }
        Ok(value)
    }

    pub fn stack_resolving_children(&self) -> Vec<SlotDisplayValue> {
        match self.decode_value() {
            Ok(SlotDisplayValue::WithRemainder { input, .. }) => vec![*input],
            Ok(SlotDisplayValue::Composite { contents }) => contents,
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RecipeDisplayId {
    pub index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeDisplayEntry {
    pub id: RecipeDisplayId,
    pub display: RecipeDisplaySummary,
    pub group: Option<i32>,
    pub category_id: i32,
    pub crafting_requirements: Option<Vec<IngredientSummary>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeDisplaySummary {
    pub display_type: RecipeDisplayType,
    pub raw_body: Vec<u8>,
    pub crafting: Option<CraftingRecipeDisplaySummary>,
    pub furnace: Option<FurnaceRecipeDisplaySummary>,
    #[serde(default)]
    pub stonecutter: Option<StonecutterRecipeDisplaySummary>,
    #[serde(default)]
    pub smithing: Option<SmithingRecipeDisplaySummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngredientSummary {
    pub tag: Option<String>,
    pub item_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CraftingRecipeDisplaySummary {
    Shapeless {
        ingredients: Vec<SlotDisplaySummary>,
        result: SlotDisplaySummary,
        crafting_station: SlotDisplaySummary,
    },
    Shaped {
        width: i32,
        height: i32,
        ingredients: Vec<SlotDisplaySummary>,
        result: SlotDisplaySummary,
        crafting_station: SlotDisplaySummary,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FurnaceRecipeDisplaySummary {
    pub ingredient: SlotDisplaySummary,
    pub fuel: SlotDisplaySummary,
    pub result: SlotDisplaySummary,
    pub crafting_station: SlotDisplaySummary,
    pub duration: i32,
    pub experience_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StonecutterRecipeDisplaySummary {
    pub input: SlotDisplaySummary,
    pub result: SlotDisplaySummary,
    pub crafting_station: SlotDisplaySummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmithingRecipeDisplaySummary {
    pub template: SlotDisplaySummary,
    pub base: SlotDisplaySummary,
    pub addition: SlotDisplaySummary,
    pub result: SlotDisplaySummary,
    pub crafting_station: SlotDisplaySummary,
}

impl RecipeDisplaySummary {
    pub fn crafting_station(&self) -> Option<&SlotDisplaySummary> {
        match self.display_type {
            RecipeDisplayType::CraftingShapeless | RecipeDisplayType::CraftingShaped => {
                match self.crafting.as_ref()? {
                    CraftingRecipeDisplaySummary::Shapeless {
                        crafting_station, ..
                    }
                    | CraftingRecipeDisplaySummary::Shaped {
                        crafting_station, ..
                    } => Some(crafting_station),
                }
            }
            RecipeDisplayType::Furnace => Some(&self.furnace.as_ref()?.crafting_station),
            RecipeDisplayType::Stonecutter => Some(&self.stonecutter.as_ref()?.crafting_station),
            RecipeDisplayType::Smithing => Some(&self.smithing.as_ref()?.crafting_station),
        }
    }

    pub fn result(&self) -> Option<&SlotDisplaySummary> {
        match self.display_type {
            RecipeDisplayType::CraftingShapeless | RecipeDisplayType::CraftingShaped => {
                match self.crafting.as_ref()? {
                    CraftingRecipeDisplaySummary::Shapeless { result, .. }
                    | CraftingRecipeDisplaySummary::Shaped { result, .. } => Some(result),
                }
            }
            RecipeDisplayType::Furnace => Some(&self.furnace.as_ref()?.result),
            RecipeDisplayType::Stonecutter => Some(&self.stonecutter.as_ref()?.result),
            RecipeDisplayType::Smithing => Some(&self.smithing.as_ref()?.result),
        }
    }

    pub fn slot_displays(&self) -> Option<Vec<&SlotDisplaySummary>> {
        match self.display_type {
            RecipeDisplayType::CraftingShapeless | RecipeDisplayType::CraftingShaped => {
                match self.crafting.as_ref()? {
                    CraftingRecipeDisplaySummary::Shapeless {
                        ingredients,
                        result,
                        crafting_station,
                    }
                    | CraftingRecipeDisplaySummary::Shaped {
                        ingredients,
                        result,
                        crafting_station,
                        ..
                    } => {
                        let mut displays = Vec::with_capacity(ingredients.len() + 2);
                        displays.extend(ingredients);
                        displays.extend([result, crafting_station]);
                        Some(displays)
                    }
                }
            }
            RecipeDisplayType::Furnace => {
                let furnace = self.furnace.as_ref()?;
                Some(vec![
                    &furnace.ingredient,
                    &furnace.fuel,
                    &furnace.result,
                    &furnace.crafting_station,
                ])
            }
            RecipeDisplayType::Stonecutter => {
                let stonecutter = self.stonecutter.as_ref()?;
                Some(vec![
                    &stonecutter.input,
                    &stonecutter.result,
                    &stonecutter.crafting_station,
                ])
            }
            RecipeDisplayType::Smithing => {
                let smithing = self.smithing.as_ref()?;
                Some(vec![
                    &smithing.template,
                    &smithing.base,
                    &smithing.addition,
                    &smithing.result,
                    &smithing.crafting_station,
                ])
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBookSettings {
    pub crafting: RecipeBookTypeSettings,
    pub furnace: RecipeBookTypeSettings,
    pub blast_furnace: RecipeBookTypeSettings,
    pub smoker: RecipeBookTypeSettings,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBookTypeSettings {
    pub open: bool,
    pub filtering: bool,
}

impl Default for RecipeBookSettings {
    fn default() -> Self {
        Self {
            crafting: RecipeBookTypeSettings::default(),
            furnace: RecipeBookTypeSettings::default(),
            blast_furnace: RecipeBookTypeSettings::default(),
            smoker: RecipeBookTypeSettings::default(),
        }
    }
}

pub(crate) fn decode_place_ghost_recipe(decoder: &mut Decoder<'_>) -> Result<PlaceGhostRecipe> {
    let container_id = decoder.read_var_i32()?;
    let recipe_display = decode_recipe_display_summary(decoder)?;

    let packet = PlaceGhostRecipe {
        container_id,
        recipe_display,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after place ghost recipe packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_recipe_book_add(decoder: &mut Decoder<'_>) -> Result<RecipeBookAdd> {
    let entry_count = read_bounded_len(decoder, MAX_RECIPE_BOOK_ENTRIES, "recipe book entries")?;
    let mut entries = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(
        entry_count,
    ));
    for _ in 0..entry_count {
        let contents = decode_recipe_display_entry(decoder)?;
        let flags = decoder.read_u8()?;
        entries.push(RecipeBookAddEntry {
            contents,
            flags,
            notification: flags & 1 != 0,
            highlight: flags & 2 != 0,
        });
    }
    let packet = RecipeBookAdd {
        entries,
        replace: decoder.read_bool()?,
    };
    reject_recipe_book_trailing(decoder, "recipe book add packet")?;
    Ok(packet)
}

pub(crate) fn decode_recipe_book_remove(decoder: &mut Decoder<'_>) -> Result<RecipeBookRemove> {
    let count = read_bounded_len(decoder, MAX_RECIPE_BOOK_ENTRIES, "recipe book remove ids")?;
    let mut recipe_ids =
        Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
    for _ in 0..count {
        recipe_ids.push(decode_recipe_display_id(decoder)?);
    }
    let packet = RecipeBookRemove { recipe_ids };
    reject_recipe_book_trailing(decoder, "recipe book remove packet")?;
    Ok(packet)
}

pub(crate) fn decode_recipe_book_settings(decoder: &mut Decoder<'_>) -> Result<RecipeBookSettings> {
    let packet = RecipeBookSettings {
        crafting: decode_recipe_book_type_settings(decoder)?,
        furnace: decode_recipe_book_type_settings(decoder)?,
        blast_furnace: decode_recipe_book_type_settings(decoder)?,
        smoker: decode_recipe_book_type_settings(decoder)?,
    };
    reject_recipe_book_trailing(decoder, "recipe book settings packet")?;
    Ok(packet)
}

fn reject_recipe_book_trailing(decoder: &Decoder<'_>, packet_name: &'static str) -> Result<()> {
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(format!(
            "trailing bytes after {packet_name}"
        )));
    }
    Ok(())
}

pub(crate) fn decode_update_recipes(decoder: &mut Decoder<'_>) -> Result<UpdateRecipes> {
    let property_set_count =
        read_bounded_len(decoder, MAX_RECIPE_PROPERTY_SETS, "recipe property set map")?;
    let mut property_sets = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(
        property_set_count,
    ));
    for _ in 0..property_set_count {
        let key = read_resource_location(decoder)?;
        let item_count = read_bounded_len(
            decoder,
            MAX_RECIPE_PROPERTY_SET_ITEMS,
            "recipe property set items",
        )?;
        let mut item_ids = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(
            item_count,
        ));
        for _ in 0..item_count {
            item_ids.push(inventory::decode_item_holder_id(decoder)?);
        }
        property_sets.push(RecipePropertySetSummary { key, item_ids });
    }

    let stonecutter_count = read_bounded_len(
        decoder,
        MAX_STONECUTTER_RECIPE_ENTRIES,
        "stonecutter selectable recipes",
    )?;
    let mut stonecutter_recipes = Vec::with_capacity(
        crate::codec::bounded_collection_initial_capacity(stonecutter_count),
    );
    for _ in 0..stonecutter_count {
        stonecutter_recipes.push(StonecutterSelectableRecipeSummary {
            input: decode_ingredient_summary(decoder)?,
            option_display: decode_slot_display_summary(decoder)?,
        });
    }

    let packet = UpdateRecipes {
        property_sets,
        stonecutter_recipes,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after update recipes packet".to_string(),
        ));
    }
    Ok(packet)
}

fn decode_recipe_display_entry(decoder: &mut Decoder<'_>) -> Result<RecipeDisplayEntry> {
    Ok(RecipeDisplayEntry {
        id: decode_recipe_display_id(decoder)?,
        display: decode_recipe_display_summary(decoder)?,
        group: decode_optional_var_i32(decoder)?,
        category_id: decoder.read_var_i32()?,
        crafting_requirements: decode_optional_ingredient_list(decoder)?,
    })
}

fn decode_recipe_display_id(decoder: &mut Decoder<'_>) -> Result<RecipeDisplayId> {
    Ok(RecipeDisplayId {
        index: decoder.read_var_i32()?,
    })
}

fn decode_recipe_display_summary(decoder: &mut Decoder<'_>) -> Result<RecipeDisplaySummary> {
    let display_start = decoder.remaining().to_vec();
    let start_len = display_start.len();
    let type_id = decoder.read_var_i32()?;
    let display_type = RecipeDisplayType::from_id(type_id).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid recipe display type id {type_id}"))
    })?;

    let (crafting, furnace, stonecutter, smithing) = match display_type {
        RecipeDisplayType::CraftingShapeless => {
            let ingredients = decode_slot_display_list(decoder)?;
            let result = decode_slot_display_summary(decoder)?;
            let crafting_station = decode_slot_display_summary(decoder)?;
            (
                Some(CraftingRecipeDisplaySummary::Shapeless {
                    ingredients,
                    result,
                    crafting_station,
                }),
                None,
                None,
                None,
            )
        }
        RecipeDisplayType::CraftingShaped => {
            let width = decoder.read_var_i32()?;
            let height = decoder.read_var_i32()?;
            let ingredients = decode_slot_display_list(decoder)?;
            if i32::try_from(ingredients.len()).ok() != Some(width.wrapping_mul(height)) {
                return Err(ProtocolError::InvalidData(format!(
                    "shaped recipe display has {} ingredients for {width}x{height}",
                    ingredients.len()
                )));
            }
            let result = decode_slot_display_summary(decoder)?;
            let crafting_station = decode_slot_display_summary(decoder)?;
            (
                Some(CraftingRecipeDisplaySummary::Shaped {
                    width,
                    height,
                    ingredients,
                    result,
                    crafting_station,
                }),
                None,
                None,
                None,
            )
        }
        RecipeDisplayType::Furnace => {
            let ingredient = decode_slot_display_summary(decoder)?;
            let fuel = decode_slot_display_summary(decoder)?;
            let result = decode_slot_display_summary(decoder)?;
            let crafting_station = decode_slot_display_summary(decoder)?;
            let duration = decoder.read_var_i32()?;
            let experience_bits = decoder.read_f32()?.to_bits();
            (
                None,
                Some(FurnaceRecipeDisplaySummary {
                    ingredient,
                    fuel,
                    result,
                    crafting_station,
                    duration,
                    experience_bits,
                }),
                None,
                None,
            )
        }
        RecipeDisplayType::Stonecutter => {
            let input = decode_slot_display_summary(decoder)?;
            let result = decode_slot_display_summary(decoder)?;
            let crafting_station = decode_slot_display_summary(decoder)?;
            (
                None,
                None,
                Some(StonecutterRecipeDisplaySummary {
                    input,
                    result,
                    crafting_station,
                }),
                None,
            )
        }
        RecipeDisplayType::Smithing => {
            let template = decode_slot_display_summary(decoder)?;
            let base = decode_slot_display_summary(decoder)?;
            let addition = decode_slot_display_summary(decoder)?;
            let result = decode_slot_display_summary(decoder)?;
            let crafting_station = decode_slot_display_summary(decoder)?;
            (
                None,
                None,
                None,
                Some(SmithingRecipeDisplaySummary {
                    template,
                    base,
                    addition,
                    result,
                    crafting_station,
                }),
            )
        }
    };

    let consumed = start_len.saturating_sub(decoder.remaining_len());
    if consumed > MAX_RECIPE_DISPLAY_BODY {
        return Err(ProtocolError::PacketTooLarge(
            consumed,
            MAX_RECIPE_DISPLAY_BODY,
        ));
    }
    Ok(RecipeDisplaySummary {
        display_type,
        raw_body: display_start[..consumed].to_vec(),
        crafting,
        furnace,
        stonecutter,
        smithing,
    })
}

fn decode_recipe_book_type_settings(decoder: &mut Decoder<'_>) -> Result<RecipeBookTypeSettings> {
    Ok(RecipeBookTypeSettings {
        open: decoder.read_bool()?,
        filtering: decoder.read_bool()?,
    })
}

fn decode_optional_var_i32(decoder: &mut Decoder<'_>) -> Result<Option<i32>> {
    let raw = decoder.read_var_i32()?;
    if raw == 0 {
        Ok(None)
    } else {
        Ok(Some(raw - 1))
    }
}

fn decode_optional_ingredient_list(
    decoder: &mut Decoder<'_>,
) -> Result<Option<Vec<IngredientSummary>>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    let count = read_bounded_len(
        decoder,
        MAX_RECIPE_BOOK_NESTED_LIST,
        "recipe book ingredient list",
    )?;
    let mut ingredients =
        Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
    for _ in 0..count {
        ingredients.push(decode_ingredient_summary(decoder)?);
    }
    Ok(Some(ingredients))
}

fn decode_ingredient_summary(decoder: &mut Decoder<'_>) -> Result<IngredientSummary> {
    let encoded = decoder.read_var_i32()?;
    if encoded == 0 {
        return Ok(IngredientSummary {
            tag: Some(read_resource_location(decoder)?),
            item_ids: Vec::new(),
        });
    }
    if encoded < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid holder set size marker {encoded}"
        )));
    }

    let count = (encoded - 1) as usize;
    if count == 0 {
        return Err(ProtocolError::InvalidData(
            "ingredient holder set must not be empty".to_string(),
        ));
    }
    if count > MAX_RECIPE_BOOK_NESTED_LIST {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_RECIPE_BOOK_NESTED_LIST,
        ));
    }
    let mut item_ids = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
    for _ in 0..count {
        item_ids.push(inventory::decode_non_empty_item_holder_id(decoder)?);
    }
    Ok(IngredientSummary {
        tag: None,
        item_ids,
    })
}

fn decode_slot_display_summary(decoder: &mut Decoder<'_>) -> Result<SlotDisplaySummary> {
    let start = decoder.position();
    let budget = SlotDisplayDecodeBudget::new(start);
    let (display_type_id, value) = decode_slot_display_value(decoder, 0, &budget)?;
    budget.check(decoder)?;
    let raw_payload = decoder.bytes_from(start).to_vec();
    let (item_stack, tag) = match value {
        SlotDisplayValue::Item { stack } | SlotDisplayValue::ItemStack { stack } => {
            (Some(*stack), None)
        }
        SlotDisplayValue::Tag { tag } => (None, Some(tag)),
        _ => (None, None),
    };
    Ok(SlotDisplaySummary {
        display_type_id,
        raw_payload,
        item_stack,
        tag,
    })
}

fn decode_slot_display_list(decoder: &mut Decoder<'_>) -> Result<Vec<SlotDisplaySummary>> {
    let count = read_bounded_len(
        decoder,
        MAX_RECIPE_BOOK_NESTED_LIST,
        "recipe book slot display list",
    )?;
    let mut displays = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
    for _ in 0..count {
        displays.push(decode_slot_display_summary(decoder)?);
    }
    Ok(displays)
}

fn decode_slot_display_value(
    decoder: &mut Decoder<'_>,
    depth: usize,
    budget: &SlotDisplayDecodeBudget,
) -> Result<(i32, SlotDisplayValue)> {
    if depth > MAX_SLOT_DISPLAY_DEPTH {
        return Err(ProtocolError::InvalidData(
            "slot display tree exceeded max depth".to_string(),
        ));
    }
    budget.check(decoder)?;
    let display_type_id = decoder.read_var_i32()?;
    let value = match display_type_id {
        VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID | VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID => {
            if display_type_id == VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID {
                SlotDisplayValue::Empty
            } else {
                SlotDisplayValue::AnyFuel
            }
        }
        VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID => SlotDisplayValue::WithAnyPotion {
            display: Box::new(decode_nested_slot_display(decoder, depth, budget)?),
        },
        VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID => {
            let source = Box::new(decode_nested_slot_display(decoder, depth, budget)?);
            let component_type_id = decoder.read_var_i32()?;
            if vanilla_data_component_type(component_type_id).is_none() {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid vanilla data component type id {component_type_id}"
                )));
            }
            SlotDisplayValue::OnlyWithComponent {
                source,
                component_type_id,
            }
        }
        VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID => SlotDisplayValue::Item {
            stack: decode_item_slot_display(decoder)?,
        },
        VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID => SlotDisplayValue::ItemStack {
            stack: decode_item_stack_slot_display(decoder)?,
        },
        VANILLA_SLOT_DISPLAY_TYPE_TAG_ID => SlotDisplayValue::Tag {
            tag: read_resource_location(decoder)?,
        },
        VANILLA_SLOT_DISPLAY_TYPE_DYED_ID => SlotDisplayValue::Dyed {
            dye: Box::new(decode_nested_slot_display(decoder, depth, budget)?),
            target: Box::new(decode_nested_slot_display(decoder, depth, budget)?),
        },
        VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID => {
            let base = Box::new(decode_nested_slot_display(decoder, depth, budget)?);
            let material = Box::new(decode_nested_slot_display(decoder, depth, budget)?);
            let pattern = decode_trim_pattern_holder(decoder)?;
            SlotDisplayValue::SmithingTrim {
                base,
                material,
                pattern: SlotDisplayTrimPatternSummary {
                    registry_id: pattern.registry_id,
                    direct: pattern.direct,
                },
            }
        }
        VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID => SlotDisplayValue::WithRemainder {
            input: Box::new(decode_nested_slot_display(decoder, depth, budget)?),
            remainder: Box::new(decode_nested_slot_display(decoder, depth, budget)?),
        },
        VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID => SlotDisplayValue::Composite {
            contents: decode_nested_slot_display_list(decoder, depth, budget)?,
        },
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid slot display type id {other}"
            )));
        }
    };
    budget.check(decoder)?;
    Ok((display_type_id, value))
}

fn decode_item_slot_display(decoder: &mut Decoder<'_>) -> Result<Box<ItemStackSummary>> {
    Ok(Box::new(ItemStackSummary {
        item_id: Some(inventory::decode_item_holder_id(decoder)?),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    }))
}

fn decode_item_stack_slot_display(decoder: &mut Decoder<'_>) -> Result<Box<ItemStackSummary>> {
    let item_id = inventory::decode_non_empty_item_holder_id(decoder)?;
    let count = decoder.read_var_i32()?;
    if count <= 0 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid item stack display count {count}"
        )));
    }
    Ok(Box::new(ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: super::super::inventory::decode_data_component_patch_summary(decoder)?,
    }))
}

fn decode_nested_slot_display(
    decoder: &mut Decoder<'_>,
    parent_depth: usize,
    budget: &SlotDisplayDecodeBudget,
) -> Result<SlotDisplayValue> {
    decode_slot_display_value(decoder, parent_depth + 1, budget).map(|(_, value)| value)
}

fn decode_nested_slot_display_list(
    decoder: &mut Decoder<'_>,
    parent_depth: usize,
    budget: &SlotDisplayDecodeBudget,
) -> Result<Vec<SlotDisplayValue>> {
    let count = read_bounded_len(
        decoder,
        MAX_RECIPE_BOOK_NESTED_LIST,
        "nested slot display list",
    )?;
    budget.check(decoder)?;
    let mut displays = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
    for _ in 0..count {
        displays.push(decode_nested_slot_display(decoder, parent_depth, budget)?);
    }
    Ok(displays)
}

struct SlotDisplayDecodeBudget {
    start: usize,
}

impl SlotDisplayDecodeBudget {
    fn new(start: usize) -> Self {
        Self { start }
    }

    fn check(&self, decoder: &Decoder<'_>) -> Result<()> {
        let consumed = decoder.position().saturating_sub(self.start);
        if consumed > MAX_RECIPE_DISPLAY_BODY {
            return Err(ProtocolError::PacketTooLarge(
                consumed,
                MAX_RECIPE_DISPLAY_BODY,
            ));
        }
        Ok(())
    }
}

fn read_bounded_len(
    decoder: &mut Decoder<'_>,
    max_len: usize,
    what: &'static str,
) -> Result<usize> {
    let len = decoder.read_len()?;
    if len > max_len {
        return Err(ProtocolError::PacketTooLarge(len, max_len));
    }
    if len > decoder.remaining_len().saturating_add(1) && max_len == MAX_RECIPE_BOOK_ENTRIES {
        return Err(ProtocolError::InvalidData(format!(
            "{what} length {len} exceeds remaining packet budget"
        )));
    }
    Ok(len)
}
