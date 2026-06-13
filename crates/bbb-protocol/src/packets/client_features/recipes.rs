use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

const MAX_RECIPE_PROPERTY_SET_ITEMS: usize = 65_536;
const MAX_RECIPE_PROPERTY_SETS: usize = 4096;
const MAX_RECIPE_DISPLAY_BODY: usize = 2 * 1024 * 1024;
const MAX_RECIPE_BOOK_ENTRIES: usize = 65_536;
const MAX_RECIPE_BOOK_NESTED_LIST: usize = 4096;
const MAX_STONECUTTER_RECIPE_ENTRIES: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceGhostRecipe {
    pub container_id: i32,
    pub recipe_display_type: RecipeDisplayType,
    pub recipe_display_body: Vec<u8>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngredientSummary {
    pub tag: Option<String>,
    pub item_ids: Vec<i32>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecipeDisplayType {
    CraftingShapeless,
    CraftingShaped,
    Furnace,
    Stonecutter,
    Smithing,
}

impl RecipeDisplayType {
    pub fn id(self) -> i32 {
        match self {
            Self::CraftingShapeless => 0,
            Self::CraftingShaped => 1,
            Self::Furnace => 2,
            Self::Stonecutter => 3,
            Self::Smithing => 4,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::CraftingShapeless => "crafting_shapeless",
            Self::CraftingShaped => "crafting_shaped",
            Self::Furnace => "furnace",
            Self::Stonecutter => "stonecutter",
            Self::Smithing => "smithing",
        }
    }

    fn from_id(id: i32) -> Result<Self> {
        Ok(match id {
            0 => Self::CraftingShapeless,
            1 => Self::CraftingShaped,
            2 => Self::Furnace,
            3 => Self::Stonecutter,
            4 => Self::Smithing,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid recipe display type id {other}"
                )))
            }
        })
    }
}

pub(crate) fn decode_place_ghost_recipe(decoder: &mut Decoder<'_>) -> Result<PlaceGhostRecipe> {
    let container_id = decoder.read_var_i32()?;
    let recipe_display_type = RecipeDisplayType::from_id(decoder.read_var_i32()?)?;
    let body_len = decoder.remaining_len();
    if body_len > MAX_RECIPE_DISPLAY_BODY {
        return Err(ProtocolError::PacketTooLarge(
            body_len,
            MAX_RECIPE_DISPLAY_BODY,
        ));
    }

    Ok(PlaceGhostRecipe {
        container_id,
        recipe_display_type,
        recipe_display_body: decoder
            .read_exact(body_len, "recipe display body")?
            .to_vec(),
    })
}

pub(crate) fn decode_recipe_book_add(decoder: &mut Decoder<'_>) -> Result<RecipeBookAdd> {
    let entry_count = read_bounded_len(decoder, MAX_RECIPE_BOOK_ENTRIES, "recipe book entries")?;
    let mut entries = Vec::with_capacity(entry_count);
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
    Ok(RecipeBookAdd {
        entries,
        replace: decoder.read_bool()?,
    })
}

pub(crate) fn decode_recipe_book_remove(decoder: &mut Decoder<'_>) -> Result<RecipeBookRemove> {
    let count = read_bounded_len(decoder, MAX_RECIPE_BOOK_ENTRIES, "recipe book remove ids")?;
    let mut recipe_ids = Vec::with_capacity(count);
    for _ in 0..count {
        recipe_ids.push(decode_recipe_display_id(decoder)?);
    }
    Ok(RecipeBookRemove { recipe_ids })
}

pub(crate) fn decode_recipe_book_settings(decoder: &mut Decoder<'_>) -> Result<RecipeBookSettings> {
    Ok(RecipeBookSettings {
        crafting: decode_recipe_book_type_settings(decoder)?,
        furnace: decode_recipe_book_type_settings(decoder)?,
        blast_furnace: decode_recipe_book_type_settings(decoder)?,
        smoker: decode_recipe_book_type_settings(decoder)?,
    })
}

pub(crate) fn decode_update_recipes(decoder: &mut Decoder<'_>) -> Result<UpdateRecipes> {
    let property_set_count =
        read_bounded_len(decoder, MAX_RECIPE_PROPERTY_SETS, "recipe property set map")?;
    let mut property_sets = Vec::with_capacity(property_set_count);
    for _ in 0..property_set_count {
        let key = decoder.read_string(32767)?;
        let item_count = read_bounded_len(
            decoder,
            MAX_RECIPE_PROPERTY_SET_ITEMS,
            "recipe property set items",
        )?;
        let mut item_ids = Vec::with_capacity(item_count);
        for _ in 0..item_count {
            item_ids.push(decoder.read_var_i32()?);
        }
        property_sets.push(RecipePropertySetSummary { key, item_ids });
    }

    let stonecutter_count = read_bounded_len(
        decoder,
        MAX_STONECUTTER_RECIPE_ENTRIES,
        "stonecutter selectable recipes",
    )?;
    let mut stonecutter_recipes = Vec::with_capacity(stonecutter_count);
    for _ in 0..stonecutter_count {
        stonecutter_recipes.push(StonecutterSelectableRecipeSummary {
            input: decode_ingredient_summary(decoder)?,
            option_display: decode_slot_display_summary(decoder)?,
        });
    }

    Ok(UpdateRecipes {
        property_sets,
        stonecutter_recipes,
    })
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
    let display_type = RecipeDisplayType::from_id(decoder.read_var_i32()?)?;

    match display_type {
        RecipeDisplayType::CraftingShapeless => {
            skip_slot_display_list(decoder)?;
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
        }
        RecipeDisplayType::CraftingShaped => {
            decoder.read_var_i32()?;
            decoder.read_var_i32()?;
            skip_slot_display_list(decoder)?;
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
        }
        RecipeDisplayType::Furnace => {
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
            decoder.read_var_i32()?;
            decoder.read_f32()?;
        }
        RecipeDisplayType::Stonecutter => {
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
        }
        RecipeDisplayType::Smithing => {
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
        }
    }

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
    let mut ingredients = Vec::with_capacity(count);
    for _ in 0..count {
        ingredients.push(decode_ingredient_summary(decoder)?);
    }
    Ok(Some(ingredients))
}

fn decode_ingredient_summary(decoder: &mut Decoder<'_>) -> Result<IngredientSummary> {
    let encoded = decoder.read_var_i32()?;
    if encoded == 0 {
        return Ok(IngredientSummary {
            tag: Some(decoder.read_string(32767)?),
            item_ids: Vec::new(),
        });
    }
    if encoded < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid holder set size marker {encoded}"
        )));
    }

    let count = (encoded - 1) as usize;
    if count > MAX_RECIPE_BOOK_NESTED_LIST {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_RECIPE_BOOK_NESTED_LIST,
        ));
    }
    let mut item_ids = Vec::with_capacity(count);
    for _ in 0..count {
        item_ids.push(decoder.read_var_i32()?);
    }
    Ok(IngredientSummary {
        tag: None,
        item_ids,
    })
}

fn decode_slot_display_summary(decoder: &mut Decoder<'_>) -> Result<SlotDisplaySummary> {
    let display_start = decoder.remaining().to_vec();
    let before_len = decoder.remaining_len();
    let display_type_id = decoder.read_var_i32()?;
    skip_slot_display_body(decoder, display_type_id)?;
    let consumed = before_len - decoder.remaining_len();
    Ok(SlotDisplaySummary {
        display_type_id,
        raw_payload: display_start[..consumed].to_vec(),
    })
}

fn skip_slot_display_list(decoder: &mut Decoder<'_>) -> Result<()> {
    let count = read_bounded_len(
        decoder,
        MAX_RECIPE_BOOK_NESTED_LIST,
        "recipe book slot display list",
    )?;
    for _ in 0..count {
        skip_slot_display(decoder)?;
    }
    Ok(())
}

fn skip_slot_display(decoder: &mut Decoder<'_>) -> Result<()> {
    let display_type_id = decoder.read_var_i32()?;
    skip_slot_display_body(decoder, display_type_id)
}

fn skip_slot_display_body(decoder: &mut Decoder<'_>, display_type_id: i32) -> Result<()> {
    match display_type_id {
        0 | 1 => Ok(()),
        2 => skip_slot_display(decoder),
        3 => {
            skip_slot_display(decoder)?;
            decoder.read_var_i32()?;
            Ok(())
        }
        4 => {
            decoder.read_var_i32()?;
            Ok(())
        }
        5 => {
            skip_item_stack_template(decoder)?;
            Ok(())
        }
        6 => {
            decoder.read_string(32767)?;
            Ok(())
        }
        7 => {
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)
        }
        8 => {
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)?;
            skip_trim_pattern_holder(decoder)
        }
        9 => {
            skip_slot_display(decoder)?;
            skip_slot_display(decoder)
        }
        10 => skip_slot_display_list(decoder),
        other => Err(ProtocolError::InvalidData(format!(
            "invalid slot display type id {other}"
        ))),
    }
}

fn skip_item_stack_template(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decoder.read_var_i32()?;
    super::super::inventory::decode_data_component_patch_summary(decoder)?;
    Ok(())
}

fn skip_trim_pattern_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    let holder_id = decoder.read_var_i32()?;
    if holder_id == 0 {
        return Err(ProtocolError::InvalidData(
            "unsupported direct trim pattern in slot display".to_string(),
        ));
    }
    Ok(())
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
