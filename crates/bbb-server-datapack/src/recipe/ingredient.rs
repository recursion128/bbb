//! Holder-set ingredients and result templates (`Ingredient.java`, `ItemStackTemplate`).

use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::builtin_enum_component_values::builtin_enum_component_value_for_name;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DYE_TYPE_ID;
use bbb_protocol::item_types::{
    vanilla_item_type, vanilla_item_type_for_resource_id, VanillaBlockItemKind,
    VANILLA_ITEM_CRAFTING_REMAINDERS,
};
use bbb_protocol::packets::ItemStackSummary;
use bbb_registry_codec::RegistryElementValueRef;

use crate::item_stack_template::prototype_has_component;
pub use crate::item_stack_template::ItemStackTemplate;
use crate::tag_loader::LoadedRegistryTags;

use super::RecipeDecodeError;

/// One HolderSet member: a concrete item or a `#tag` key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngredientTerm {
    Item(i32),
    Tag(String),
}

/// `Ingredient` over a non-empty item HolderSet (`Ingredient.java:34-48`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ingredient {
    pub terms: Vec<IngredientTerm>,
}

impl Ingredient {
    pub(super) fn parse_source_with_content_registry(
        value: RegistryElementValueRef<'_>,
        content_registry: &ContentRegistry,
    ) -> Result<Self, RecipeDecodeError> {
        use super::source::{error, list};
        let terms = if value.is_string() {
            vec![source_holder_term(value, content_registry)?]
        } else if value.list_len().is_some() {
            let values = list(value, "ingredient")?;
            let terms = values
                .map(|entry| {
                    if entry.is_string() {
                        source_holder_term(entry, content_registry)
                    } else {
                        source_holder_object(entry, content_registry)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            if terms
                .iter()
                .any(|term| matches!(term, IngredientTerm::Tag(_)))
            {
                return Err(error("ingredient direct holder list can't contain tags"));
            }
            terms
        } else {
            vec![source_holder_object(value, content_registry)?]
        };
        if terms.is_empty() {
            return Err(error("ingredients can't be empty"));
        }
        Ok(Self { terms })
    }

    /// `Ingredient.isEmpty` (`Ingredient.java:59-61`): a direct holder list is
    /// non-empty by construction, so only a tag that resolved to nothing is.
    pub fn is_empty(&self, tags: &ItemTagIndex) -> bool {
        self.terms.iter().all(|term| match term {
            IngredientTerm::Item(_) => false,
            IngredientTerm::Tag(tag) => tags.members(tag).next().is_none(),
        })
    }

    /// `Ingredient.test` (`Ingredient.java:63-65`): empty stacks never match.
    pub fn test(&self, stack: &ItemStackSummary, tags: &ItemTagIndex) -> bool {
        let Some(item_id) = stack.item_id.filter(|_| stack.count > 0) else {
            return false;
        };
        self.terms.iter().any(|term| match term {
            IngredientTerm::Item(expected) => *expected == item_id,
            IngredientTerm::Tag(tag) => tags.contains(tag, item_id),
        })
    }
}

fn source_holder_object(
    value: RegistryElementValueRef<'_>,
    content_registry: &ContentRegistry,
) -> Result<IngredientTerm, RecipeDecodeError> {
    use super::source::{error, resource};
    match (value.field("id"), value.field("tag")) {
        (Some(id), None) => source_holder_term(id, content_registry),
        (None, Some(tag)) => Ok(IngredientTerm::Tag(resource(tag, "tag")?)),
        (None, None) => Err(error("holder set entry lacks `id` or `tag`")),
        _ => Err(error("holder set entry mixes `id` and `tag`")),
    }
}

fn source_holder_term(
    value: RegistryElementValueRef<'_>,
    content_registry: &ContentRegistry,
) -> Result<IngredientTerm, RecipeDecodeError> {
    use super::source::{error, string};
    let raw = string(value, "ingredient")?;
    let raw = String::from_utf16(raw.units())
        .map_err(|_| error("ingredient is not a resource identifier"))?;
    if let Some(tag) = raw.strip_prefix('#') {
        return Ok(IngredientTerm::Tag(
            bbb_protocol::normalize_resource_location(tag.to_string())
                .map_err(|err| error(err.to_string()))?,
        ));
    }
    let id =
        bbb_protocol::normalize_resource_location(raw).map_err(|err| error(err.to_string()))?;
    if let Some(item) = vanilla_item_type_for_resource_id(&id) {
        if item.id == 0 {
            return Err(error("ingredient can't contain air"));
        }
        return Ok(IngredientTerm::Item(item.id));
    }
    if let Some(mod_item) = content_registry.item_by_resource_id(&id) {
        return Ok(IngredientTerm::Item(mod_item.id));
    }
    Err(error(format!("unknown item {id}")))
}

/// `Ingredient.testOptionalIngredient` (`Ingredient.java:50-52`).
pub fn test_optional_ingredient(
    ingredient: Option<&Ingredient>,
    stack: &ItemStackSummary,
    tags: &ItemTagIndex,
) -> bool {
    match ingredient {
        Some(ingredient) => ingredient.test(stack, tags),
        None => stack.item_id.is_none() || stack.count <= 0,
    }
}

pub fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    if count <= 0 {
        return ItemStackSummary::empty();
    }
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

pub fn stack_is_empty(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

/// `ItemStack.copyWithCount` (`ItemStack.java:590-598`): a whole copy, the
/// component patch included, with the count replaced.
pub fn copy_with_count(stack: &ItemStackSummary, count: i32) -> ItemStackSummary {
    if stack_is_empty(stack) || count <= 0 {
        return ItemStackSummary::empty();
    }
    ItemStackSummary {
        count,
        ..stack.clone()
    }
}

/// `DataComponentHolder.has` (`DataComponentHolder.java:23-25`) as
/// `DataComponentMap.has` (`DataComponentMap.java:86-88`) over
/// `PatchedDataComponentMap.get` (`PatchedDataComponentMap.java:61-63`): the
/// stack's patch first, the item's registered prototype behind it.
pub fn stack_has_component(stack: &ItemStackSummary, type_id: i32) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    if stack.component_patch.removed_type_ids.contains(&type_id) {
        return false;
    }
    if stack.component_patch.added_type_ids.contains(&type_id) {
        return true;
    }
    prototype_has_component(item_id, type_id)
}

/// `Item.getCraftingRemainder` for the 26.1 `Items` registrations that set one.
pub fn crafting_remainder(item_id: i32) -> Option<ItemStackSummary> {
    let remainder = VANILLA_ITEM_CRAFTING_REMAINDERS
        .iter()
        .find_map(|&(item, remainder)| (item == item_id).then_some(remainder))?;
    Some(item_stack(remainder, 1))
}

/// One tag's members, kept in the order the tag loader bound them because
/// `HolderSet.Named` iteration order is what `FuelValues.Builder.add(TagKey)`
/// (`FuelValues.java:131-138`) inserts its rows in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct TagMembers {
    order: Vec<i32>,
    lookup: BTreeSet<i32>,
}

/// Resolved `minecraft:item` tags for ingredient tests, plus the
/// `#minecraft:curse` enchantment holder ids `RepairItemRecipe` keeps.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemTagIndex {
    tags: BTreeMap<String, TagMembers>,
    curse_enchantments: BTreeSet<i32>,
}

impl ItemTagIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, tag: impl Into<String>, items: impl IntoIterator<Item = i32>) {
        let members = self.tags.entry(tag.into()).or_default();
        for item_id in items {
            if members.lookup.insert(item_id) {
                members.order.push(item_id);
            }
        }
    }

    pub fn from_loaded(tags: &LoadedRegistryTags) -> Self {
        Self::from_loaded_with_content_registry(tags, ContentRegistry::vanilla())
    }

    pub fn from_loaded_with_content_registry(
        tags: &LoadedRegistryTags,
        content_registry: &ContentRegistry,
    ) -> Self {
        let mut index = Self::new();
        for (tag_id, elements) in &tags.tags {
            let ids = elements.iter().filter_map(|name| {
                vanilla_item_type_for_resource_id(name)
                    .map(|entry| entry.id)
                    .or_else(|| {
                        content_registry
                            .item_by_resource_id(name)
                            .map(|spec| spec.id)
                    })
            });
            index.insert(tag_id.clone(), ids);
        }
        index
    }

    /// Adds the generation's `#minecraft:curse` members by enchantment holder id.
    pub fn with_enchantment_curses(mut self, startup: &crate::startup::DatapackStartup) -> Self {
        const ENCHANTMENT: &str = "minecraft:enchantment";
        let (Some(registry), Some(curses)) = (
            startup.registries.registry(ENCHANTMENT),
            startup
                .tags
                .registry(ENCHANTMENT)
                .and_then(|tags| tags.tag("minecraft:curse")),
        ) else {
            return self;
        };
        self.curse_enchantments = registry
            .elements
            .iter()
            .enumerate()
            .filter(|(_, element)| curses.contains(&element.id))
            .map(|(holder, _)| holder as i32)
            .collect();
        self
    }

    pub fn is_curse_enchantment(&self, holder_id: i32) -> bool {
        self.curse_enchantments.contains(&holder_id)
    }

    /// How many distinct tags this index holds, for a caller that describes
    /// the surface rather than queries it.
    pub fn len(&self) -> usize {
        self.tags.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    pub fn contains(&self, tag: &str, item_id: i32) -> bool {
        self.tags
            .get(tag)
            .is_some_and(|members| members.lookup.contains(&item_id))
    }

    pub fn members(&self, tag: &str) -> impl Iterator<Item = i32> + '_ {
        self.tags
            .get(tag)
            .into_iter()
            .flat_map(|members| members.order.iter().copied())
    }
}

pub fn is_banner_item(item_id: i32) -> bool {
    vanilla_item_type(item_id).is_some_and(|entry| {
        entry
            .block_item_behavior
            .is_some_and(|behavior| behavior.kind == VanillaBlockItemKind::BannerItem)
    })
}

/// `BannerItem.getColor` (`BannerItem.java:15-17`) through
/// `AbstractBannerBlock.getColor`: each standing banner block registers the
/// `DyeColor` its own id names (`Blocks.java:3389-3580`).
pub fn banner_item_color(item_id: i32) -> Option<i32> {
    if !is_banner_item(item_id) {
        return None;
    }
    let name = item_resource_id(item_id)?
        .strip_prefix("minecraft:")?
        .strip_suffix("_banner")?;
    builtin_enum_component_value_for_name(VANILLA_DATA_COMPONENT_DYE_TYPE_ID, name)
        .map(|entry| entry.id)
}

pub fn item_resource_id(item_id: i32) -> Option<&'static str> {
    vanilla_item_type(item_id).map(|entry| entry.resource_id)
}
