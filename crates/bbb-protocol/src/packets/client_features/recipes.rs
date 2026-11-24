use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, Encoder, ProtocolError, Result};
use crate::content_registry::ContentRegistry;
use crate::data_component_types::vanilla_data_component_type;
use crate::ids;
pub use crate::recipe_wire_types::{RecipeBookType, RecipeDisplayType};
use crate::recipe_wire_types::{
    VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID, VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID,
    VANILLA_SLOT_DISPLAY_TYPE_DYED_ID, VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
    VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID, VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
    VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID, VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID,
    VANILLA_SLOT_DISPLAY_TYPE_TAG_ID, VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID,
    VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
};

use super::super::{
    data_components::{
        catalog::encode_trim_pattern_holder, decode_trim_pattern_holder,
        encode_data_component_patch_summary_with_framing_and_registry, ComponentValueFraming,
    },
    inventory, normalize_resource_location, read_resource_location, ItemStackSummary,
    TrimPatternSummary,
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

impl RecipeBookAddEntry {
    pub fn new(contents: RecipeDisplayEntry, notification: bool, highlight: bool) -> Self {
        Self {
            contents,
            flags: u8::from(notification) | (u8::from(highlight) << 1),
            notification,
            highlight,
        }
    }
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

const SLOT_DISPLAY_CACHE_BUCKETS: usize = 512;
const SLOT_DISPLAY_CACHE_WAYS: usize = 2;
const MAX_CACHED_SLOT_DISPLAY_PAYLOAD_BYTES: usize = 4096;

struct SlotDisplayCacheEntry {
    hash: u64,
    display_type_id: i32,
    raw_payload: Box<[u8]>,
    value: Arc<SlotDisplayValue>,
}

struct SlotDisplayDecodeCache {
    buckets: Vec<([Option<SlotDisplayCacheEntry>; SLOT_DISPLAY_CACHE_WAYS], u8)>,
}

impl SlotDisplayDecodeCache {
    fn new() -> Self {
        let mut buckets = Vec::with_capacity(SLOT_DISPLAY_CACHE_BUCKETS);
        for _ in 0..SLOT_DISPLAY_CACHE_BUCKETS {
            buckets.push(([None, None], 0));
        }
        Self { buckets }
    }

    fn hash_key(display_type_id: i32, raw_payload: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in display_type_id.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        for &byte in raw_payload {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash
    }

    fn get(&mut self, display_type_id: i32, raw_payload: &[u8]) -> Option<Arc<SlotDisplayValue>> {
        if raw_payload.len() > MAX_CACHED_SLOT_DISPLAY_PAYLOAD_BYTES {
            return None;
        }
        let hash = Self::hash_key(display_type_id, raw_payload);
        let bucket_idx = (hash as usize) & (SLOT_DISPLAY_CACHE_BUCKETS - 1);
        let (ways, next_victim) = &mut self.buckets[bucket_idx];
        for (way_idx, slot) in ways.iter().enumerate() {
            if let Some(entry) = slot {
                if entry.hash == hash
                    && entry.display_type_id == display_type_id
                    && &*entry.raw_payload == raw_payload
                {
                    *next_victim = (way_idx ^ 1) as u8;
                    return Some(Arc::clone(&entry.value));
                }
            }
        }
        None
    }

    fn insert(
        &mut self,
        display_type_id: i32,
        raw_payload: &[u8],
        value: Arc<SlotDisplayValue>,
    ) -> Arc<SlotDisplayValue> {
        if raw_payload.len() > MAX_CACHED_SLOT_DISPLAY_PAYLOAD_BYTES {
            return value;
        }
        let hash = Self::hash_key(display_type_id, raw_payload);
        let bucket_idx = (hash as usize) & (SLOT_DISPLAY_CACHE_BUCKETS - 1);
        let (ways, next_victim) = &mut self.buckets[bucket_idx];
        for (way_idx, slot) in ways.iter().enumerate() {
            if let Some(entry) = slot {
                if entry.hash == hash
                    && entry.display_type_id == display_type_id
                    && &*entry.raw_payload == raw_payload
                {
                    *next_victim = (way_idx ^ 1) as u8;
                    return Arc::clone(&entry.value);
                }
            }
        }
        let target = if ways[0].is_none() {
            0
        } else if ways[1].is_none() {
            1
        } else {
            usize::from(*next_victim & 1)
        };
        ways[target] = Some(SlotDisplayCacheEntry {
            hash,
            display_type_id,
            raw_payload: raw_payload.into(),
            value: Arc::clone(&value),
        });
        *next_victim = (target ^ 1) as u8;
        value
    }
}

fn slot_display_decode_cache() -> &'static Mutex<SlotDisplayDecodeCache> {
    static CACHE: OnceLock<Mutex<SlotDisplayDecodeCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(SlotDisplayDecodeCache::new()))
}

impl SlotDisplaySummary {
    pub fn from_value(value: SlotDisplayValue) -> Result<Self> {
        Self::from_value_with_content_registry(value, ContentRegistry::vanilla())
    }

    pub fn from_value_with_content_registry(
        value: SlotDisplayValue,
        registry: &ContentRegistry,
    ) -> Result<Self> {
        let mut encoder = Encoder::new();
        encode_slot_display_value_with_registry(&mut encoder, &value, 0, registry)?;
        let raw_payload = encoder.into_inner();
        if raw_payload.len() > MAX_RECIPE_DISPLAY_BODY {
            return Err(ProtocolError::PacketTooLarge(
                raw_payload.len(),
                MAX_RECIPE_DISPLAY_BODY,
            ));
        }

        let display_type_id = slot_display_value_type_id(&value);
        let (item_stack, tag) = match value {
            SlotDisplayValue::Item { stack } => (
                Some(ItemStackSummary {
                    item_id: stack.item_id,
                    count: 1,
                    component_patch: Default::default(),
                }),
                None,
            ),
            SlotDisplayValue::ItemStack { mut stack } => {
                if stack.component_patch.is_empty() {
                    stack.component_patch = Default::default();
                }
                (Some(*stack), None)
            }
            SlotDisplayValue::Tag { tag } => (None, Some(normalize_resource_location(tag)?)),
            _ => (None, None),
        };
        Ok(Self {
            display_type_id,
            raw_payload,
            item_stack,
            tag,
        })
    }

    pub fn decode_value_with_content_registry(
        &self,
        registry: &ContentRegistry,
    ) -> Result<SlotDisplayValue> {
        self.decode_value_arc_with_content_registry(registry)
            .map(|value| (*value).clone())
    }

    pub fn decode_value_arc(&self) -> Result<Arc<SlotDisplayValue>> {
        self.decode_value_arc_with_content_registry(ContentRegistry::vanilla())
    }

    pub fn decode_value_arc_with_content_registry(
        &self,
        registry: &ContentRegistry,
    ) -> Result<Arc<SlotDisplayValue>> {
        if registry.is_vanilla_only() {
            if let Some(cached) = slot_display_decode_cache()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(self.display_type_id, &self.raw_payload)
            {
                return Ok(cached);
            }
        }
        let mut decoder = Decoder::new(&self.raw_payload);
        let budget = SlotDisplayDecodeBudget::new(decoder.position());
        let (display_type_id, value) =
            decode_slot_display_value_with_registry(&mut decoder, 0, &budget, registry)?;
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
        if registry.is_vanilla_only() {
            let arc_value = slot_display_decode_cache()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(self.display_type_id, &self.raw_payload, Arc::new(value));
            Ok(arc_value)
        } else {
            Ok(Arc::new(value))
        }
    }

    pub fn decode_value(&self) -> Result<SlotDisplayValue> {
        self.decode_value_arc().map(|value| (*value).clone())
    }

    pub fn stack_resolving_children(&self) -> Vec<SlotDisplayValue> {
        self.stack_resolving_children_with_content_registry(ContentRegistry::vanilla())
    }

    pub fn stack_resolving_children_with_content_registry(
        &self,
        registry: &ContentRegistry,
    ) -> Vec<SlotDisplayValue> {
        match self
            .decode_value_arc_with_content_registry(registry)
            .as_deref()
        {
            Ok(SlotDisplayValue::WithRemainder { input, .. }) => vec![(**input).clone()],
            Ok(SlotDisplayValue::Composite { contents }) => contents.clone(),
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
    // The typed branches stay boxed: inline they sum to ~53KB per summary,
    // which blows up every embedder (`PlayClientbound`, `NetEvent`, the tokio
    // mpsc block, the event-stream poll frame) far past worker stack budgets.
    pub crafting: Option<Box<CraftingRecipeDisplaySummary>>,
    pub furnace: Option<Box<FurnaceRecipeDisplaySummary>>,
    #[serde(default)]
    pub stonecutter: Option<Box<StonecutterRecipeDisplaySummary>>,
    #[serde(default)]
    pub smithing: Option<Box<SmithingRecipeDisplaySummary>>,
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
    pub fn crafting_shapeless(
        ingredients: Vec<SlotDisplaySummary>,
        result: SlotDisplaySummary,
        crafting_station: SlotDisplaySummary,
    ) -> Result<Self> {
        let mut encoder = recipe_display_encoder(RecipeDisplayType::CraftingShapeless);
        encode_slot_display_list(&mut encoder, &ingredients)?;
        encode_validated_slot_display(&mut encoder, &result)?;
        encode_validated_slot_display(&mut encoder, &crafting_station)?;
        let raw_body = finish_recipe_display_body(encoder)?;
        Ok(Self {
            display_type: RecipeDisplayType::CraftingShapeless,
            raw_body,
            crafting: Some(Box::new(CraftingRecipeDisplaySummary::Shapeless {
                ingredients,
                result,
                crafting_station,
            })),
            furnace: None,
            stonecutter: None,
            smithing: None,
        })
    }

    pub fn crafting_shaped(
        width: i32,
        height: i32,
        ingredients: Vec<SlotDisplaySummary>,
        result: SlotDisplaySummary,
        crafting_station: SlotDisplaySummary,
    ) -> Result<Self> {
        let expected = width
            .checked_mul(height)
            .and_then(|value| usize::try_from(value).ok());
        if expected != Some(ingredients.len()) {
            return Err(ProtocolError::InvalidData(format!(
                "shaped recipe display has {} ingredients for {width}x{height}",
                ingredients.len()
            )));
        }

        let mut encoder = recipe_display_encoder(RecipeDisplayType::CraftingShaped);
        encoder.write_var_i32(width);
        encoder.write_var_i32(height);
        encode_slot_display_list(&mut encoder, &ingredients)?;
        encode_validated_slot_display(&mut encoder, &result)?;
        encode_validated_slot_display(&mut encoder, &crafting_station)?;
        let raw_body = finish_recipe_display_body(encoder)?;
        Ok(Self {
            display_type: RecipeDisplayType::CraftingShaped,
            raw_body,
            crafting: Some(Box::new(CraftingRecipeDisplaySummary::Shaped {
                width,
                height,
                ingredients,
                result,
                crafting_station,
            })),
            furnace: None,
            stonecutter: None,
            smithing: None,
        })
    }

    pub fn furnace(
        ingredient: SlotDisplaySummary,
        fuel: SlotDisplaySummary,
        result: SlotDisplaySummary,
        crafting_station: SlotDisplaySummary,
        duration: i32,
        experience_bits: u32,
    ) -> Result<Self> {
        let mut encoder = recipe_display_encoder(RecipeDisplayType::Furnace);
        for display in [&ingredient, &fuel, &result, &crafting_station] {
            encode_validated_slot_display(&mut encoder, display)?;
        }
        encoder.write_var_i32(duration);
        encoder.write_f32(f32::from_bits(experience_bits));
        let raw_body = finish_recipe_display_body(encoder)?;
        Ok(Self {
            display_type: RecipeDisplayType::Furnace,
            raw_body,
            crafting: None,
            furnace: Some(Box::new(FurnaceRecipeDisplaySummary {
                ingredient,
                fuel,
                result,
                crafting_station,
                duration,
                experience_bits,
            })),
            stonecutter: None,
            smithing: None,
        })
    }

    pub fn stonecutter(
        input: SlotDisplaySummary,
        result: SlotDisplaySummary,
        crafting_station: SlotDisplaySummary,
    ) -> Result<Self> {
        let mut encoder = recipe_display_encoder(RecipeDisplayType::Stonecutter);
        for display in [&input, &result, &crafting_station] {
            encode_validated_slot_display(&mut encoder, display)?;
        }
        let raw_body = finish_recipe_display_body(encoder)?;
        Ok(Self {
            display_type: RecipeDisplayType::Stonecutter,
            raw_body,
            crafting: None,
            furnace: None,
            stonecutter: Some(Box::new(StonecutterRecipeDisplaySummary {
                input,
                result,
                crafting_station,
            })),
            smithing: None,
        })
    }

    pub fn smithing(
        template: SlotDisplaySummary,
        base: SlotDisplaySummary,
        addition: SlotDisplaySummary,
        result: SlotDisplaySummary,
        crafting_station: SlotDisplaySummary,
    ) -> Result<Self> {
        let mut encoder = recipe_display_encoder(RecipeDisplayType::Smithing);
        for display in [&template, &base, &addition, &result, &crafting_station] {
            encode_validated_slot_display(&mut encoder, display)?;
        }
        let raw_body = finish_recipe_display_body(encoder)?;
        Ok(Self {
            display_type: RecipeDisplayType::Smithing,
            raw_body,
            crafting: None,
            furnace: None,
            stonecutter: None,
            smithing: Some(Box::new(SmithingRecipeDisplaySummary {
                template,
                base,
                addition,
                result,
                crafting_station,
            })),
        })
    }

    pub fn crafting_station(&self) -> Option<&SlotDisplaySummary> {
        match self.display_type {
            RecipeDisplayType::CraftingShapeless | RecipeDisplayType::CraftingShaped => {
                match self.crafting.as_deref()? {
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
                match self.crafting.as_deref()? {
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
                match self.crafting.as_deref()? {
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

fn recipe_display_encoder(display_type: RecipeDisplayType) -> Encoder {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(display_type.id());
    encoder
}

fn finish_recipe_display_body(encoder: Encoder) -> Result<Vec<u8>> {
    let raw_body = encoder.into_inner();
    if raw_body.len() > MAX_RECIPE_DISPLAY_BODY {
        return Err(ProtocolError::PacketTooLarge(
            raw_body.len(),
            MAX_RECIPE_DISPLAY_BODY,
        ));
    }
    Ok(raw_body)
}

fn encode_slot_display_list(encoder: &mut Encoder, displays: &[SlotDisplaySummary]) -> Result<()> {
    if displays.len() > MAX_RECIPE_BOOK_NESTED_LIST {
        return Err(ProtocolError::PacketTooLarge(
            displays.len(),
            MAX_RECIPE_BOOK_NESTED_LIST,
        ));
    }
    write_var_len(encoder, displays.len(), "recipe book slot display list")?;
    for display in displays {
        encode_validated_slot_display(encoder, display)?;
    }
    Ok(())
}

fn encode_validated_slot_display(
    encoder: &mut Encoder,
    display: &SlotDisplaySummary,
) -> Result<()> {
    if display.raw_payload.is_empty() {
        return Err(ProtocolError::InvalidData(
            "slot display summary has empty raw payload".to_string(),
        ));
    }
    if display.raw_payload.len() > MAX_RECIPE_DISPLAY_BODY {
        return Err(ProtocolError::PacketTooLarge(
            display.raw_payload.len(),
            MAX_RECIPE_DISPLAY_BODY,
        ));
    }
    encoder.write_bytes(&display.raw_payload);
    Ok(())
}

const fn slot_display_value_type_id(value: &SlotDisplayValue) -> i32 {
    match value {
        SlotDisplayValue::Empty => VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
        SlotDisplayValue::AnyFuel => VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
        SlotDisplayValue::WithAnyPotion { .. } => VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID,
        SlotDisplayValue::OnlyWithComponent { .. } => {
            VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID
        }
        SlotDisplayValue::Item { .. } => VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        SlotDisplayValue::ItemStack { .. } => VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
        SlotDisplayValue::Tag { .. } => VANILLA_SLOT_DISPLAY_TYPE_TAG_ID,
        SlotDisplayValue::Dyed { .. } => VANILLA_SLOT_DISPLAY_TYPE_DYED_ID,
        SlotDisplayValue::SmithingTrim { .. } => VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID,
        SlotDisplayValue::WithRemainder { .. } => VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID,
        SlotDisplayValue::Composite { .. } => VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID,
    }
}

fn encode_slot_display_value_with_registry(
    encoder: &mut Encoder,
    value: &SlotDisplayValue,
    depth: usize,
    registry: &ContentRegistry,
) -> Result<()> {
    if depth > MAX_SLOT_DISPLAY_DEPTH {
        return Err(ProtocolError::InvalidData(
            "slot display tree exceeded max depth".to_string(),
        ));
    }

    let display_type_id = match value {
        SlotDisplayValue::Empty => VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
        SlotDisplayValue::AnyFuel => VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
        SlotDisplayValue::WithAnyPotion { display } => {
            encoder.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID);
            encode_slot_display_value_with_registry(encoder, display, depth + 1, registry)?;
            return Ok(());
        }
        SlotDisplayValue::OnlyWithComponent {
            source,
            component_type_id,
        } => {
            if vanilla_data_component_type(*component_type_id).is_none() {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid vanilla data component type id {component_type_id}"
                )));
            }
            encoder.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID);
            encode_slot_display_value_with_registry(encoder, source, depth + 1, registry)?;
            encoder.write_var_i32(*component_type_id);
            return Ok(());
        }
        SlotDisplayValue::Item { stack } => {
            let item_id = stack.item_id.ok_or_else(|| {
                ProtocolError::InvalidData("item slot display must contain an item".to_string())
            })?;
            if stack.count != 1 || !stack.component_patch.is_empty() {
                return Err(ProtocolError::InvalidData(
                    "item slot display must have count one and no component patch".to_string(),
                ));
            }
            encoder.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID);
            encoder.write_var_i32(inventory::validate_item_holder_id_with_registry(
                item_id, registry,
            )?);
            return Ok(());
        }
        SlotDisplayValue::ItemStack { stack } => {
            let item_id = stack.item_id.ok_or_else(|| {
                ProtocolError::InvalidData(
                    "item stack slot display must contain an item".to_string(),
                )
            })?;
            if stack.count <= 0 {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid item stack display count {}",
                    stack.count
                )));
            }
            encoder.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID);
            encode_non_empty_item_holder_id_with_registry(encoder, item_id, registry)?;
            encoder.write_var_i32(stack.count);
            encode_data_component_patch_summary_with_framing_and_registry(
                encoder,
                &stack.component_patch,
                ComponentValueFraming::Direct,
                registry,
            )?;
            return Ok(());
        }
        SlotDisplayValue::Tag { tag } => {
            encoder.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_TAG_ID);
            write_resource_location(encoder, tag)?;
            return Ok(());
        }
        SlotDisplayValue::Dyed { dye, target } => {
            encoder.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_DYED_ID);
            encode_slot_display_value_with_registry(encoder, dye, depth + 1, registry)?;
            encode_slot_display_value_with_registry(encoder, target, depth + 1, registry)?;
            return Ok(());
        }
        SlotDisplayValue::SmithingTrim {
            base,
            material,
            pattern,
        } => {
            encoder.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID);
            encode_slot_display_value_with_registry(encoder, base, depth + 1, registry)?;
            encode_slot_display_value_with_registry(encoder, material, depth + 1, registry)?;
            encode_trim_pattern_holder(
                encoder,
                pattern.registry_id,
                pattern.direct.as_ref(),
                registry,
            )?;
            return Ok(());
        }
        SlotDisplayValue::WithRemainder { input, remainder } => {
            encoder.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID);
            encode_slot_display_value_with_registry(encoder, input, depth + 1, registry)?;
            encode_slot_display_value_with_registry(encoder, remainder, depth + 1, registry)?;
            return Ok(());
        }
        SlotDisplayValue::Composite { contents } => {
            if contents.len() > MAX_RECIPE_BOOK_NESTED_LIST {
                return Err(ProtocolError::PacketTooLarge(
                    contents.len(),
                    MAX_RECIPE_BOOK_NESTED_LIST,
                ));
            }
            encoder.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID);
            write_var_len(encoder, contents.len(), "nested slot display list")?;
            for content in contents {
                encode_slot_display_value_with_registry(encoder, content, depth + 1, registry)?;
            }
            return Ok(());
        }
    };

    encoder.write_var_i32(display_type_id);
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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

pub(crate) fn decode_place_ghost_recipe_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<PlaceGhostRecipe> {
    let container_id = decoder.read_var_i32()?;
    let recipe_display = decode_recipe_display_summary_with_registry(decoder, registry)?;

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

pub(crate) fn decode_recipe_book_add_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<RecipeBookAdd> {
    let entry_count = read_bounded_len(decoder, MAX_RECIPE_BOOK_ENTRIES, "recipe book entries")?;
    let mut entries = Vec::with_capacity(decoder.bounded_capacity(entry_count, 6));
    for _ in 0..entry_count {
        let contents = decode_recipe_display_entry_with_registry(decoder, registry)?;
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
    let mut recipe_ids = Vec::with_capacity(decoder.bounded_capacity(count, 1));
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

pub(crate) fn decode_update_recipes_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<UpdateRecipes> {
    let property_set_count =
        read_bounded_len(decoder, MAX_RECIPE_PROPERTY_SETS, "recipe property set map")?;
    let mut property_sets = Vec::with_capacity(decoder.bounded_capacity(property_set_count, 3));
    for _ in 0..property_set_count {
        let key = read_resource_location(decoder)?;
        let item_count = read_bounded_len(
            decoder,
            MAX_RECIPE_PROPERTY_SET_ITEMS,
            "recipe property set items",
        )?;
        let mut item_ids = Vec::with_capacity(decoder.bounded_capacity(item_count, 1));
        for _ in 0..item_count {
            item_ids.push(inventory::decode_item_holder_id_with_registry(
                decoder, registry,
            )?);
        }
        property_sets.push(RecipePropertySetSummary { key, item_ids });
    }

    let stonecutter_count = read_bounded_len(
        decoder,
        MAX_STONECUTTER_RECIPE_ENTRIES,
        "stonecutter selectable recipes",
    )?;
    let mut stonecutter_recipes =
        Vec::with_capacity(decoder.bounded_capacity(stonecutter_count, 2));
    for _ in 0..stonecutter_count {
        stonecutter_recipes.push(StonecutterSelectableRecipeSummary {
            input: decode_ingredient_summary(decoder, registry)?,
            option_display: decode_slot_display_summary_with_registry(decoder, registry)?,
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

/// `ClientboundRecipeBookAddPacket.STREAM_CODEC` (`ClientboundRecipeBookAddPacket.java:12-18`).
pub fn encode_play_clientbound_recipe_book_add(packet: &RecipeBookAdd) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_recipe_book_add_with_content_registry(
        packet,
        ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_recipe_book_add_with_content_registry(
    packet: &RecipeBookAdd,
    registry: &ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    write_var_len(
        &mut encoder,
        packet.entries.len(),
        "recipe book add entries",
    )?;
    for entry in &packet.entries {
        encode_recipe_book_add_entry(&mut encoder, entry, registry)?;
    }
    encoder.write_bool(packet.replace);
    Ok((ids::play::CLIENTBOUND_RECIPE_BOOK_ADD, encoder.into_inner()))
}

/// `ClientboundPlaceGhostRecipePacket.STREAM_CODEC` (`ClientboundPlaceGhostRecipePacket.java:11-17`).
pub fn encode_play_clientbound_place_ghost_recipe(
    packet: &PlaceGhostRecipe,
) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_place_ghost_recipe_with_content_registry(
        packet,
        ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_place_ghost_recipe_with_content_registry(
    packet: &PlaceGhostRecipe,
    _registry: &ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(packet.container_id);
    encoder.write_bytes(&packet.recipe_display.raw_body);
    Ok((
        ids::play::CLIENTBOUND_PLACE_GHOST_RECIPE,
        encoder.into_inner(),
    ))
}

/// `ClientboundRecipeBookRemovePacket.STREAM_CODEC` (`ClientboundRecipeBookRemovePacket.java:12-14`).
pub fn encode_play_clientbound_recipe_book_remove(
    packet: &RecipeBookRemove,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    write_var_len(
        &mut encoder,
        packet.recipe_ids.len(),
        "recipe book remove ids",
    )?;
    for id in &packet.recipe_ids {
        encoder.write_var_i32(id.index);
    }
    Ok((
        ids::play::CLIENTBOUND_RECIPE_BOOK_REMOVE,
        encoder.into_inner(),
    ))
}

/// `ClientboundRecipeBookSettingsPacket.STREAM_CODEC` (`ClientboundRecipeBookSettingsPacket.java:10-12`)
/// plus `RecipeBookSettings.STREAM_CODEC` (`RecipeBookSettings.java:15-25`).
pub fn encode_play_clientbound_recipe_book_settings(
    packet: &RecipeBookSettings,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encode_recipe_book_type_settings(&mut encoder, packet.crafting);
    encode_recipe_book_type_settings(&mut encoder, packet.furnace);
    encode_recipe_book_type_settings(&mut encoder, packet.blast_furnace);
    encode_recipe_book_type_settings(&mut encoder, packet.smoker);
    Ok((
        ids::play::CLIENTBOUND_RECIPE_BOOK_SETTINGS,
        encoder.into_inner(),
    ))
}

/// `ClientboundUpdateRecipesPacket.STREAM_CODEC` (`ClientboundUpdateRecipesPacket.java:18-24`).
pub fn encode_play_clientbound_update_recipes(packet: &UpdateRecipes) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_update_recipes_with_content_registry(packet, ContentRegistry::vanilla())
}

pub fn encode_play_clientbound_update_recipes_with_content_registry(
    packet: &UpdateRecipes,
    registry: &ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    write_var_len(
        &mut encoder,
        packet.property_sets.len(),
        "recipe property set map",
    )?;
    for set in &packet.property_sets {
        write_resource_location(&mut encoder, &set.key)?;
        write_var_len(
            &mut encoder,
            set.item_ids.len(),
            "recipe property set items",
        )?;
        for item_id in &set.item_ids {
            encoder.write_var_i32(inventory::validate_item_holder_id_with_registry(
                *item_id, registry,
            )?);
        }
    }
    write_var_len(
        &mut encoder,
        packet.stonecutter_recipes.len(),
        "stonecutter selectable recipes",
    )?;
    for recipe in &packet.stonecutter_recipes {
        encode_ingredient_summary(&mut encoder, &recipe.input, registry)?;
        encoder.write_bytes(&recipe.option_display.raw_payload);
    }
    Ok((ids::play::CLIENTBOUND_UPDATE_RECIPES, encoder.into_inner()))
}

/// `ClientboundRecipeBookAddPacket.Entry.STREAM_CODEC` (`ClientboundRecipeBookAddPacket.java:32-38`).
fn encode_recipe_book_add_entry(
    encoder: &mut Encoder,
    entry: &RecipeBookAddEntry,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_recipe_display_entry(encoder, &entry.contents, registry)?;
    encoder.write_u8(entry.flags);
    Ok(())
}

/// `RecipeDisplayEntry.STREAM_CODEC` (`RecipeDisplayEntry.java:19-31`): id before display.
fn encode_recipe_display_entry(
    encoder: &mut Encoder,
    entry: &RecipeDisplayEntry,
    registry: &ContentRegistry,
) -> Result<()> {
    encoder.write_var_i32(entry.id.index);
    encoder.write_bytes(&entry.display.raw_body);
    encode_optional_var_i32(encoder, entry.group)?;
    encoder.write_var_i32(entry.category_id);
    encode_optional_ingredient_list(encoder, entry.crafting_requirements.as_deref(), registry)?;
    Ok(())
}

fn encode_recipe_book_type_settings(encoder: &mut Encoder, settings: RecipeBookTypeSettings) {
    encoder.write_bool(settings.open);
    encoder.write_bool(settings.filtering);
}

/// `ByteBufCodecs.OPTIONAL_VAR_INT` (`ByteBufCodecs.java:111-113`): 0 is empty, else value+1.
fn encode_optional_var_i32(encoder: &mut Encoder, value: Option<i32>) -> Result<()> {
    match value {
        None => encoder.write_var_i32(0),
        Some(value) => {
            let encoded = value.checked_add(1).ok_or_else(|| {
                ProtocolError::InvalidData("optional recipe group overflow".to_string())
            })?;
            encoder.write_var_i32(encoded);
        }
    }
    Ok(())
}

fn encode_optional_ingredient_list(
    encoder: &mut Encoder,
    ingredients: Option<&[IngredientSummary]>,
    registry: &ContentRegistry,
) -> Result<()> {
    match ingredients {
        None => encoder.write_bool(false),
        Some(ingredients) => {
            encoder.write_bool(true);
            write_var_len(encoder, ingredients.len(), "recipe book ingredient list")?;
            for ingredient in ingredients {
                encode_ingredient_summary(encoder, ingredient, registry)?;
            }
        }
    }
    Ok(())
}

/// `ByteBufCodecs.holderSet` encode (`ByteBufCodecs.java:638-649`): 0 + tag, or size+1 + holders.
fn encode_ingredient_summary(
    encoder: &mut Encoder,
    ingredient: &IngredientSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    if let Some(tag) = &ingredient.tag {
        encoder.write_var_i32(0);
        write_resource_location(encoder, tag)?;
        return Ok(());
    }
    if ingredient.item_ids.is_empty() {
        return Err(ProtocolError::InvalidData(
            "ingredient holder set must not be empty".to_string(),
        ));
    }
    let count = i32::try_from(ingredient.item_ids.len()).map_err(|_| {
        ProtocolError::InvalidData("ingredient holder set does not fit a varint".into())
    })?;
    let encoded = count.checked_add(1).ok_or_else(|| {
        ProtocolError::InvalidData("ingredient holder set size overflow".to_string())
    })?;
    encoder.write_var_i32(encoded);
    for item_id in &ingredient.item_ids {
        encode_non_empty_item_holder_id_with_registry(encoder, *item_id, registry)?;
    }
    Ok(())
}

fn encode_non_empty_item_holder_id_with_registry(
    encoder: &mut Encoder,
    item_id: i32,
    registry: &ContentRegistry,
) -> Result<()> {
    let item_id = inventory::validate_item_holder_id_with_registry(item_id, registry)?;
    if item_id == crate::item_types::VANILLA_ITEM_AIR_ID {
        return Err(ProtocolError::InvalidData(
            "item stack template holder must not be minecraft:air".to_string(),
        ));
    }
    encoder.write_var_i32(item_id);
    Ok(())
}

fn write_var_len(encoder: &mut Encoder, len: usize, what: &'static str) -> Result<()> {
    encoder.write_var_i32(
        i32::try_from(len)
            .map_err(|_| ProtocolError::InvalidData(format!("{what} do not fit a varint")))?,
    );
    Ok(())
}

fn write_resource_location(encoder: &mut Encoder, raw: &str) -> Result<()> {
    encoder.write_string(&normalize_resource_location(raw.to_string())?);
    Ok(())
}

fn decode_recipe_display_entry_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<RecipeDisplayEntry> {
    Ok(RecipeDisplayEntry {
        id: decode_recipe_display_id(decoder)?,
        display: decode_recipe_display_summary_with_registry(decoder, registry)?,
        group: decode_optional_var_i32(decoder)?,
        category_id: decoder.read_var_i32()?,
        crafting_requirements: decode_optional_ingredient_list(decoder, registry)?,
    })
}

fn decode_recipe_display_id(decoder: &mut Decoder<'_>) -> Result<RecipeDisplayId> {
    Ok(RecipeDisplayId {
        index: decoder.read_var_i32()?,
    })
}

fn decode_recipe_display_summary_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<RecipeDisplaySummary> {
    let start = decoder.position();
    let type_id = decoder.read_var_i32()?;
    let display_type = RecipeDisplayType::from_id(type_id).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid recipe display type id {type_id}"))
    })?;

    let (crafting, furnace, stonecutter, smithing) = match display_type {
        RecipeDisplayType::CraftingShapeless => {
            let ingredients = decode_slot_display_list(decoder, registry)?;
            let result = decode_slot_display_summary_with_registry(decoder, registry)?;
            let crafting_station = decode_slot_display_summary_with_registry(decoder, registry)?;
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
            let ingredients = decode_slot_display_list(decoder, registry)?;
            if i32::try_from(ingredients.len()).ok() != Some(width.wrapping_mul(height)) {
                return Err(ProtocolError::InvalidData(format!(
                    "shaped recipe display has {} ingredients for {width}x{height}",
                    ingredients.len()
                )));
            }
            let result = decode_slot_display_summary_with_registry(decoder, registry)?;
            let crafting_station = decode_slot_display_summary_with_registry(decoder, registry)?;
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
            let ingredient = decode_slot_display_summary_with_registry(decoder, registry)?;
            let fuel = decode_slot_display_summary_with_registry(decoder, registry)?;
            let result = decode_slot_display_summary_with_registry(decoder, registry)?;
            let crafting_station = decode_slot_display_summary_with_registry(decoder, registry)?;
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
            let input = decode_slot_display_summary_with_registry(decoder, registry)?;
            let result = decode_slot_display_summary_with_registry(decoder, registry)?;
            let crafting_station = decode_slot_display_summary_with_registry(decoder, registry)?;
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
            let template = decode_slot_display_summary_with_registry(decoder, registry)?;
            let base = decode_slot_display_summary_with_registry(decoder, registry)?;
            let addition = decode_slot_display_summary_with_registry(decoder, registry)?;
            let result = decode_slot_display_summary_with_registry(decoder, registry)?;
            let crafting_station = decode_slot_display_summary_with_registry(decoder, registry)?;
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

    let raw_body = decoder.bytes_from(start);
    if raw_body.len() > MAX_RECIPE_DISPLAY_BODY {
        return Err(ProtocolError::PacketTooLarge(
            raw_body.len(),
            MAX_RECIPE_DISPLAY_BODY,
        ));
    }
    Ok(RecipeDisplaySummary {
        display_type,
        raw_body: raw_body.to_vec(),
        crafting: crafting.map(Box::new),
        furnace: furnace.map(Box::new),
        stonecutter: stonecutter.map(Box::new),
        smithing: smithing.map(Box::new),
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
        let value = raw
            .checked_sub(1)
            .ok_or_else(|| ProtocolError::InvalidData("optional varint underflow".to_string()))?;
        Ok(Some(value))
    }
}

fn decode_optional_ingredient_list(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<Option<Vec<IngredientSummary>>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    let count = read_bounded_len(
        decoder,
        MAX_RECIPE_BOOK_NESTED_LIST,
        "recipe book ingredient list",
    )?;
    let mut ingredients = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    for _ in 0..count {
        ingredients.push(decode_ingredient_summary(decoder, registry)?);
    }
    Ok(Some(ingredients))
}

fn decode_ingredient_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<IngredientSummary> {
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
    let mut item_ids = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    for _ in 0..count {
        item_ids.push(inventory::decode_non_empty_item_holder_id_with_registry(
            decoder, registry,
        )?);
    }
    Ok(IngredientSummary {
        tag: None,
        item_ids,
    })
}

fn decode_slot_display_summary_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SlotDisplaySummary> {
    let start = decoder.position();
    let budget = SlotDisplayDecodeBudget::new(start);
    let (display_type_id, value) =
        decode_slot_display_value_with_registry(decoder, 0, &budget, registry)?;
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

fn decode_slot_display_list(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<Vec<SlotDisplaySummary>> {
    let count = read_bounded_len(
        decoder,
        MAX_RECIPE_BOOK_NESTED_LIST,
        "recipe book slot display list",
    )?;
    let mut displays = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    for _ in 0..count {
        displays.push(decode_slot_display_summary_with_registry(
            decoder, registry,
        )?);
    }
    Ok(displays)
}

fn decode_slot_display_value_with_registry(
    decoder: &mut Decoder<'_>,
    depth: usize,
    budget: &SlotDisplayDecodeBudget,
    registry: &ContentRegistry,
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
            display: Box::new(decode_nested_slot_display(
                decoder, depth, budget, registry,
            )?),
        },
        VANILLA_SLOT_DISPLAY_TYPE_ONLY_WITH_COMPONENT_ID => {
            let source = Box::new(decode_nested_slot_display(
                decoder, depth, budget, registry,
            )?);
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
            stack: decode_item_slot_display(decoder, registry)?,
        },
        VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID => SlotDisplayValue::ItemStack {
            stack: decode_item_stack_slot_display(decoder, registry)?,
        },
        VANILLA_SLOT_DISPLAY_TYPE_TAG_ID => SlotDisplayValue::Tag {
            tag: read_resource_location(decoder)?,
        },
        VANILLA_SLOT_DISPLAY_TYPE_DYED_ID => SlotDisplayValue::Dyed {
            dye: Box::new(decode_nested_slot_display(
                decoder, depth, budget, registry,
            )?),
            target: Box::new(decode_nested_slot_display(
                decoder, depth, budget, registry,
            )?),
        },
        VANILLA_SLOT_DISPLAY_TYPE_SMITHING_TRIM_ID => {
            let base = Box::new(decode_nested_slot_display(
                decoder, depth, budget, registry,
            )?);
            let material = Box::new(decode_nested_slot_display(
                decoder, depth, budget, registry,
            )?);
            let (registry_id, direct) = decode_trim_pattern_holder(decoder, registry)?;
            SlotDisplayValue::SmithingTrim {
                base,
                material,
                pattern: SlotDisplayTrimPatternSummary {
                    registry_id,
                    direct,
                },
            }
        }
        VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID => SlotDisplayValue::WithRemainder {
            input: Box::new(decode_nested_slot_display(
                decoder, depth, budget, registry,
            )?),
            remainder: Box::new(decode_nested_slot_display(
                decoder, depth, budget, registry,
            )?),
        },
        VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID => SlotDisplayValue::Composite {
            contents: decode_nested_slot_display_list(decoder, depth, budget, registry)?,
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

fn decode_item_slot_display(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<Box<ItemStackSummary>> {
    Ok(Box::new(ItemStackSummary {
        item_id: Some(inventory::decode_item_holder_id_with_registry(
            decoder, registry,
        )?),
        count: 1,
        component_patch: Default::default(),
    }))
}

fn decode_item_stack_slot_display(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<Box<ItemStackSummary>> {
    let item_id = inventory::decode_non_empty_item_holder_id_with_registry(decoder, registry)?;
    let count = decoder.read_var_i32()?;
    if count <= 0 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid item stack display count {count}"
        )));
    }
    Ok(Box::new(ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch:
            super::super::inventory::decode_data_component_patch_summary_with_registry(
                decoder, registry,
            )?
            .into(),
    }))
}

fn decode_nested_slot_display(
    decoder: &mut Decoder<'_>,
    parent_depth: usize,
    budget: &SlotDisplayDecodeBudget,
    registry: &ContentRegistry,
) -> Result<SlotDisplayValue> {
    decode_slot_display_value_with_registry(decoder, parent_depth + 1, budget, registry)
        .map(|(_, value)| value)
}

fn decode_nested_slot_display_list(
    decoder: &mut Decoder<'_>,
    parent_depth: usize,
    budget: &SlotDisplayDecodeBudget,
    registry: &ContentRegistry,
) -> Result<Vec<SlotDisplayValue>> {
    let count = read_bounded_len(
        decoder,
        MAX_RECIPE_BOOK_NESTED_LIST,
        "nested slot display list",
    )?;
    budget.check(decoder)?;
    let mut displays = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    for _ in 0..count {
        displays.push(decode_nested_slot_display(
            decoder,
            parent_depth,
            budget,
            registry,
        )?);
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
