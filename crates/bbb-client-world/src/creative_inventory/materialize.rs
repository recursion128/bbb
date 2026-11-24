use std::fmt;

use bbb_protocol::{
    block_state_types::vanilla_item_default_block_state_properties,
    builtin_enum_component_values::{
        VANILLA_DYE_COLOR_BLACK_ID, VANILLA_DYE_COLOR_CYAN_ID, VANILLA_DYE_COLOR_GRAY_ID,
        VANILLA_DYE_COLOR_LIGHT_GRAY_ID,
    },
    data_component_types::{
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
    },
    item_component_value_types::ItemRaritySummary,
    item_types::{vanilla_item_type_for_resource_id, VANILLA_SUSPICIOUS_STEW_INGREDIENTS},
    packets::{
        BannerPatternLayerSummary, DataComponentPatchSummary, ItemEnchantmentSummary,
        ItemStackSummary, NbtSummaryEntry, NbtSummaryValue, SuspiciousStewEffectSummary,
    },
    potion_types::VANILLA_POTIONS,
    styled_component_runs_from_nbt_value, styled_runs_summary_text,
};
use bbb_render_types::BannerPatternKind;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::{
    loader_ordered_resource_id, CreativeInventoryBackground, CreativeInventoryCatalogEntry,
    CreativeInventoryDynamicEntry, CreativeInventoryTabRow, CreativeInventoryTabType,
    CreativeInventoryTabVisibility, RegistryAccessRevision, VanillaEnchantmentIdentity, WorldStore,
    BANNER_PATTERN_REGISTRY, INSTRUMENT_REGISTRY, PAINTING_VARIANT_REGISTRY,
};

const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";
const POTION_REGISTRY: &str = "minecraft:potion";
const PLACEABLE_PAINTING_TAG: &str = "minecraft:placeable";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreativeInventoryMaterializationRevision {
    pub registry: RegistryAccessRevision,
    pub enabled_features: Vec<String>,
    pub has_permissions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreativeInventoryTabSnapshot {
    pub resource_id: String,
    pub row: CreativeInventoryTabRow,
    pub column: u8,
    pub title_key: String,
    pub icon: ItemStackSummary,
    pub tab_type: CreativeInventoryTabType,
    pub background: CreativeInventoryBackground,
    pub aligned_right: bool,
    pub show_title: bool,
    pub can_scroll: bool,
    pub items: Vec<ItemStackSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreativeInventorySnapshot {
    pub generation: u64,
    pub revision: CreativeInventoryMaterializationRevision,
    pub tabs: Vec<CreativeInventoryTabSnapshot>,
}

impl CreativeInventorySnapshot {
    pub fn tab(&self, resource_id: &str) -> Option<&CreativeInventoryTabSnapshot> {
        self.tabs.iter().find(|tab| tab.resource_id == resource_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreativeInventoryBuildError {
    pub context: String,
    pub reason: String,
}

impl CreativeInventoryBuildError {
    fn new(context: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            context: context.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for CreativeInventoryBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.context, self.reason)
    }
}

impl std::error::Error for CreativeInventoryBuildError {}

pub(super) fn materialize_creative_inventory(
    world: &WorldStore,
    generation: u64,
    has_permissions: bool,
) -> Result<CreativeInventorySnapshot, CreativeInventoryBuildError> {
    let revision = CreativeInventoryMaterializationRevision {
        registry: world.registry_access_revision(),
        enabled_features: world.enabled_feature_list(),
        has_permissions,
    };
    let items_enabled = world.is_feature_enabled("minecraft:vanilla");
    let catalog = super::vanilla_creative_inventory_tabs();
    let mut tab_items = Vec::with_capacity(catalog.len());

    for tab in catalog {
        let mut parent_items = CreativeStackSet::default();
        let mut search_items = CreativeStackSet::default();
        if tab.tab_type == CreativeInventoryTabType::Category
            && (!tab.requires_permissions || has_permissions)
            && items_enabled
        {
            for entry in tab.entries {
                let stacks = materialize_entry(world, *entry)?;
                for stack in stacks {
                    match entry.visibility() {
                        CreativeInventoryTabVisibility::ParentAndSearch => {
                            parent_items.insert_unique(stack.clone(), tab.resource_id)?;
                            search_items.insert_unique(stack, tab.resource_id)?;
                        }
                        CreativeInventoryTabVisibility::ParentOnly => {
                            parent_items.insert_unique(stack, tab.resource_id)?;
                        }
                        CreativeInventoryTabVisibility::SearchOnly => {
                            search_items.insert_unique(stack, tab.resource_id)?;
                        }
                    }
                }
            }
        }
        tab_items.push((tab, parent_items.into_vec(), search_items.into_vec()));
    }

    let mut all_search_items = CreativeStackSet::default();
    for (tab, _, search_items) in &tab_items {
        if tab.tab_type != CreativeInventoryTabType::Category {
            continue;
        }
        for stack in search_items {
            all_search_items.insert_if_absent(stack);
        }
    }
    let all_search_items = all_search_items.into_vec();

    let mut tabs = Vec::with_capacity(catalog.len());
    for (tab, parent_items, _) in tab_items {
        if tab.tab_type == CreativeInventoryTabType::Category && parent_items.is_empty() {
            continue;
        }
        let items = if tab.tab_type == CreativeInventoryTabType::Search {
            all_search_items.clone()
        } else {
            parent_items
        };
        tabs.push(CreativeInventoryTabSnapshot {
            resource_id: tab.resource_id.to_string(),
            row: tab.row,
            column: tab.column,
            title_key: tab.title_key.to_string(),
            icon: item_stack(tab.icon_item_resource_id)?,
            tab_type: tab.tab_type,
            background: tab.background,
            aligned_right: tab.aligned_right,
            show_title: tab.show_title,
            can_scroll: tab.can_scroll,
            items,
        });
    }

    Ok(CreativeInventorySnapshot {
        generation,
        revision,
        tabs,
    })
}

/// Ordered deduplicating set for `ItemStackSummary` backed by a fast hash
/// bucket index so creative-tab materialization stays O(N) even with tens of
/// thousands of custom enchantments, potions, or paintings.
#[derive(Default)]
struct CreativeStackSet {
    items: Vec<ItemStackSummary>,
    buckets: FxHashMap<u64, Vec<usize>>,
}

impl CreativeStackSet {
    fn contains(&self, hash: u64, stack: &ItemStackSummary) -> bool {
        self.buckets
            .get(&hash)
            .is_some_and(|indices| indices.iter().any(|&index| &self.items[index] == stack))
    }

    fn insert_unique(
        &mut self,
        stack: ItemStackSummary,
        tab: &str,
    ) -> Result<(), CreativeInventoryBuildError> {
        let hash = creative_stack_hash(&stack);
        if self.contains(hash, &stack) {
            return Err(CreativeInventoryBuildError::new(
                tab,
                format!(
                    "duplicate creative stack for item id {}",
                    stack.item_id.unwrap_or(-1)
                ),
            ));
        }
        let index = self.items.len();
        self.items.push(stack);
        self.buckets.entry(hash).or_default().push(index);
        Ok(())
    }

    fn insert_if_absent(&mut self, stack: &ItemStackSummary) {
        let hash = creative_stack_hash(stack);
        if self.contains(hash, stack) {
            return;
        }
        let index = self.items.len();
        self.items.push(stack.clone());
        self.buckets.entry(hash).or_default().push(index);
    }

    fn into_vec(self) -> Vec<ItemStackSummary> {
        self.items
    }
}

fn creative_stack_hash(stack: &ItemStackSummary) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    stack.item_id.hash(&mut hasher);
    stack.count.hash(&mut hasher);
    let patch = &stack.component_patch;
    patch.added_type_ids.hash(&mut hasher);
    patch.removed_type_ids.hash(&mut hasher);
    for ench in &patch.stored_enchantments {
        ench.holder_id.hash(&mut hasher);
        ench.level.hash(&mut hasher);
    }
    patch.potion_contents.potion_id.hash(&mut hasher);
    patch.instrument.registry_id.hash(&mut hasher);
    patch.painting_variant_id.hash(&mut hasher);
    patch.ominous_bottle_amplifier.hash(&mut hasher);
    for stew in &patch.suspicious_stew_effects {
        stew.effect_id.hash(&mut hasher);
        stew.duration.hash(&mut hasher);
    }
    patch.fireworks_flight_duration.hash(&mut hasher);
    for (name, value) in &patch.block_state_properties {
        name.hash(&mut hasher);
        value.hash(&mut hasher);
    }
    hasher.finish()
}

fn materialize_entry(
    world: &WorldStore,
    entry: CreativeInventoryCatalogEntry,
) -> Result<Vec<ItemStackSummary>, CreativeInventoryBuildError> {
    match entry {
        CreativeInventoryCatalogEntry::Item { resource_id, .. } => {
            Ok(vec![item_stack(resource_id)?])
        }
        CreativeInventoryCatalogEntry::Dynamic { kind, .. } => materialize_dynamic(world, kind),
    }
}

fn materialize_dynamic(
    world: &WorldStore,
    kind: CreativeInventoryDynamicEntry,
) -> Result<Vec<ItemStackSummary>, CreativeInventoryBuildError> {
    match kind {
        CreativeInventoryDynamicEntry::FireworksAllDurations => {
            (1..=3).map(materialize_firework).collect()
        }
        CreativeInventoryDynamicEntry::Instruments {
            item_resource_id,
            tag_resource_id,
        } => materialize_instruments(world, item_resource_id, tag_resource_id),
        CreativeInventoryDynamicEntry::Potions { item_resource_id } => {
            materialize_potions(world, item_resource_id)
        }
        CreativeInventoryDynamicEntry::SuspiciousStews => materialize_suspicious_stews(),
        CreativeInventoryDynamicEntry::OminousBottles => {
            (0..=4).map(materialize_ominous_bottle).collect()
        }
        CreativeInventoryDynamicEntry::EnchantmentBooksMaxLevel => {
            materialize_enchantment_books(world, false)
        }
        CreativeInventoryDynamicEntry::EnchantmentBooksAllLevels => {
            materialize_enchantment_books(world, true)
        }
        CreativeInventoryDynamicEntry::PresetPaintings { placeable } => {
            materialize_paintings(world, placeable)
        }
        CreativeInventoryDynamicEntry::OminousBanner => materialize_ominous_banner(world),
        CreativeInventoryDynamicEntry::TestBlockModes => ["start", "log", "fail", "accept"]
            .into_iter()
            .map(|mode| materialize_block_state_item("minecraft:test_block", "mode", mode))
            .collect(),
        CreativeInventoryDynamicEntry::LightLevels => (0..=15)
            .rev()
            .map(|level| {
                materialize_block_state_item("minecraft:light", "level", &level.to_string())
            })
            .collect(),
    }
}

fn materialize_firework(duration: i32) -> Result<ItemStackSummary, CreativeInventoryBuildError> {
    // Items.FIREWORK_ROCKET prototypes carry Fireworks(1, List.of()). Setting
    // that same value leaves Vanilla's PatchedDataComponentMap patch empty.
    if duration == 1 {
        return item_stack("minecraft:firework_rocket");
    }
    patched_item_stack(
        "minecraft:firework_rocket",
        vec![VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID],
        |patch| {
            patch.fireworks_flight_duration = Some(duration);
        },
    )
}

fn materialize_ominous_bottle(
    amplifier: i32,
) -> Result<ItemStackSummary, CreativeInventoryBuildError> {
    // The item prototype already carries OminousBottleAmplifier(0).
    if amplifier == 0 {
        return item_stack("minecraft:ominous_bottle");
    }
    patched_item_stack(
        "minecraft:ominous_bottle",
        vec![VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID],
        |patch| patch.ominous_bottle_amplifier = Some(amplifier),
    )
}

fn materialize_block_state_item(
    item_resource_id: &str,
    property: &str,
    value: &str,
) -> Result<ItemStackSummary, CreativeInventoryBuildError> {
    let item = vanilla_item_type_for_resource_id(item_resource_id).ok_or_else(|| {
        CreativeInventoryBuildError::new(item_resource_id, "unknown vanilla item resource id")
    })?;
    let is_prototype_value = vanilla_item_default_block_state_properties(item.id)
        .is_some_and(|defaults| defaults == [(property, value)]);
    if is_prototype_value {
        return item_stack(item_resource_id);
    }
    patched_item_stack(
        item_resource_id,
        vec![VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID],
        |patch| {
            patch
                .block_state_properties
                .insert(property.into(), value.into());
        },
    )
}

fn materialize_instruments(
    world: &WorldStore,
    item_resource_id: &str,
    tag_resource_id: &str,
) -> Result<Vec<ItemStackSummary>, CreativeInventoryBuildError> {
    let holder_ids = world
        .registry_holder_ids_for_tag(INSTRUMENT_REGISTRY, tag_resource_id)
        .ok_or_else(|| {
            CreativeInventoryBuildError::new(
                tag_resource_id,
                "synchronized instrument tag is unavailable",
            )
        })?;
    holder_ids
        .iter()
        .copied()
        .map(|holder_id| {
            let resource_id = world
                .registry_resource_id_by_holder_id(INSTRUMENT_REGISTRY, holder_id)
                .ok_or_else(|| {
                    CreativeInventoryBuildError::new(
                        tag_resource_id,
                        format!("unresolved instrument holder id {holder_id}"),
                    )
                })?;
            // Items.GOAT_HORN resolves its delayed prototype component to
            // PONDER_GOAT_HORN against this same frozen registry revision.
            if item_resource_id == "minecraft:goat_horn"
                && resource_id == "minecraft:ponder_goat_horn"
            {
                return item_stack(item_resource_id);
            }
            patched_item_stack(
                item_resource_id,
                vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID],
                |patch| patch.instrument.registry_id = Some(holder_id),
            )
        })
        .collect()
}

fn materialize_potions(
    world: &WorldStore,
    item_resource_id: &str,
) -> Result<Vec<ItemStackSummary>, CreativeInventoryBuildError> {
    let holder_ids: Vec<i32> = if let Some(content) = world.registry_content(POTION_REGISTRY) {
        (0..content.entries.len())
            .map(|holder_id| holder_id as i32)
            .collect()
    } else {
        VANILLA_POTIONS.iter().map(|potion| potion.id).collect()
    };
    holder_ids
        .into_iter()
        .map(|holder_id| {
            patched_item_stack(
                item_resource_id,
                vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                |patch| {
                    patch.potion_contents.potion_id = Some(holder_id);
                },
            )
        })
        .collect()
}

fn materialize_suspicious_stews() -> Result<Vec<ItemStackSummary>, CreativeInventoryBuildError> {
    let mut stacks = Vec::new();
    for ingredient in VANILLA_SUSPICIOUS_STEW_INGREDIENTS {
        let stack = patched_item_stack(
            "minecraft:suspicious_stew",
            vec![VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID],
            |patch| {
                patch.suspicious_stew_effects = vec![SuspiciousStewEffectSummary {
                    effect_id: ingredient.effect_type_id,
                    duration: ingredient.duration_ticks,
                }];
            },
        )?;
        if !stacks.contains(&stack) {
            stacks.push(stack);
        }
    }
    Ok(stacks)
}

fn materialize_enchantment_books(
    world: &WorldStore,
    all_levels: bool,
) -> Result<Vec<ItemStackSummary>, CreativeInventoryBuildError> {
    let mut levels = Vec::new();
    if let Some(content) = world.registry_content(ENCHANTMENT_REGISTRY) {
        let definitions = world
            .enchantment_tooltip_definitions_in_holder_order()
            .ok_or_else(|| {
                CreativeInventoryBuildError::new(
                    ENCHANTMENT_REGISTRY,
                    "synchronized enchantment definitions are unavailable",
                )
            })?;
        for (holder_id, (entry, definition)) in content.entries.iter().zip(definitions).enumerate()
        {
            let max_level = definition
                .map(|definition| definition.max_level)
                .or_else(|| {
                    (!entry.has_data)
                        .then(|| VanillaEnchantmentIdentity::for_resource_id(&entry.id))
                        .flatten()
                        .map(VanillaEnchantmentIdentity::max_level)
                })
                .ok_or_else(|| {
                    CreativeInventoryBuildError::new(
                        &entry.id,
                        "enchantment max level could not be materialized",
                    )
                })?;
            levels.push((holder_id as i32, max_level));
        }
    } else {
        // `minecraft:enchantment` is loaded from data files
        // (RegistryDataLoader.java:118) and registered in sorted `Identifier`
        // order (ResourceManagerRegistryLoadTask.java:62), so the holder ids a
        // server would hand out -- and the order the tab lists them in -- follow
        // sorted resource ids, not `Enchantments` declaration order.
        let mut holder_id = 0;
        while let Some(enchantment) = loader_ordered_resource_id(ENCHANTMENT_REGISTRY, holder_id)
            .and_then(VanillaEnchantmentIdentity::for_resource_id)
        {
            levels.push((holder_id, enchantment.max_level()));
            holder_id += 1;
        }
    }

    let mut stacks = Vec::new();
    for (holder_id, max_level) in levels {
        let level_range = if all_levels {
            1..=max_level
        } else {
            max_level..=max_level
        };
        for level in level_range {
            stacks.push(patched_item_stack(
                "minecraft:enchanted_book",
                vec![VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID],
                |patch| {
                    patch.stored_enchantments = vec![ItemEnchantmentSummary { holder_id, level }];
                },
            )?);
        }
    }
    Ok(stacks)
}

fn materialize_paintings(
    world: &WorldStore,
    placeable: bool,
) -> Result<Vec<ItemStackSummary>, CreativeInventoryBuildError> {
    let mut paintings = Vec::new();
    let content = world
        .registry_content(PAINTING_VARIANT_REGISTRY)
        .ok_or_else(|| {
            CreativeInventoryBuildError::new(
                PAINTING_VARIANT_REGISTRY,
                "synchronized painting registry is unavailable",
            )
        })?;
    let placeable_holders: FxHashSet<i32> = world
        .registry_holder_ids_for_tag(PAINTING_VARIANT_REGISTRY, PLACEABLE_PAINTING_TAG)
        .ok_or_else(|| {
            CreativeInventoryBuildError::new(
                PLACEABLE_PAINTING_TAG,
                "synchronized painting tag is unavailable",
            )
        })?
        .iter()
        .copied()
        .collect();
    for holder_id in 0..content.entries.len() as i32 {
        if placeable_holders.contains(&holder_id) != placeable {
            continue;
        }
        let presentation = world
            .painting_variant_client_projection(holder_id)
            .ok_or_else(|| {
                CreativeInventoryBuildError::new(
                    format!("{PAINTING_VARIANT_REGISTRY}[{holder_id}]"),
                    "painting projection is unavailable",
                )
            })?;
        paintings.push((holder_id, presentation.width, presentation.height));
    }
    paintings.sort_by_key(|(_, width, height)| (width * height, *width));
    paintings
        .into_iter()
        .map(|(holder_id, _, _)| {
            patched_item_stack(
                "minecraft:painting",
                vec![VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID],
                |patch| patch.painting_variant_id = Some(holder_id),
            )
        })
        .collect()
}

fn materialize_ominous_banner(
    world: &WorldStore,
) -> Result<Vec<ItemStackSummary>, CreativeInventoryBuildError> {
    let patterns = [
        ("minecraft:rhombus", VANILLA_DYE_COLOR_CYAN_ID),
        ("minecraft:stripe_bottom", VANILLA_DYE_COLOR_LIGHT_GRAY_ID),
        ("minecraft:stripe_center", VANILLA_DYE_COLOR_GRAY_ID),
        ("minecraft:border", VANILLA_DYE_COLOR_LIGHT_GRAY_ID),
        ("minecraft:stripe_middle", VANILLA_DYE_COLOR_BLACK_ID),
        ("minecraft:half_horizontal", VANILLA_DYE_COLOR_LIGHT_GRAY_ID),
        ("minecraft:circle", VANILLA_DYE_COLOR_LIGHT_GRAY_ID),
        ("minecraft:border", VANILLA_DYE_COLOR_BLACK_ID),
    ];
    let mut banner_pattern_layers = Vec::new();
    for (resource_id, color_id) in patterns {
        let registry_id = if world.registry_content(BANNER_PATTERN_REGISTRY).is_some() {
            world.registry_holder_id_by_resource_id(BANNER_PATTERN_REGISTRY, resource_id)
        } else {
            BannerPatternKind::for_resource_id(resource_id)
                .map(|pattern| pattern.vanilla_registry_index() as i32)
        };
        if let Some(registry_id) = registry_id {
            banner_pattern_layers.push(BannerPatternLayerSummary {
                registry_id: Some(registry_id),
                color_id,
                ..BannerPatternLayerSummary::default()
            });
        }
    }
    let item_name_nbt = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "translate".into(),
        value: NbtSummaryValue::string("block.minecraft.ominous_banner"),
    }]);
    let item_name_styled = styled_component_runs_from_nbt_value(&item_name_nbt);
    let item_name = styled_runs_summary_text(&item_name_styled);
    Ok(vec![patched_item_stack(
        "minecraft:white_banner",
        vec![
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
            VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
            VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
        ],
        |patch| {
            patch.banner_pattern_layers = banner_pattern_layers;
            patch.tooltip_hidden_component_type_ids =
                vec![VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID];
            patch.item_name = Some(item_name);
            patch.item_name_styled = Some(item_name_styled);
            patch.item_name_nbt = Some(item_name_nbt);
            patch.rarity = Some(ItemRaritySummary::Uncommon);
        },
    )?])
}

fn item_stack(resource_id: &str) -> Result<ItemStackSummary, CreativeInventoryBuildError> {
    let item = vanilla_item_type_for_resource_id(resource_id).ok_or_else(|| {
        CreativeInventoryBuildError::new(resource_id, "unknown vanilla item resource id")
    })?;
    Ok(ItemStackSummary {
        item_id: Some(item.id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    })
}

fn patched_item_stack(
    resource_id: &str,
    added_type_ids: Vec<i32>,
    apply: impl FnOnce(&mut DataComponentPatchSummary),
) -> Result<ItemStackSummary, CreativeInventoryBuildError> {
    let mut stack = item_stack(resource_id)?;
    stack.component_patch.added_type_ids = added_type_ids;
    apply(&mut stack.component_patch);
    bbb_protocol::packets::validate_data_component_patch_summary(&stack.component_patch).map_err(
        |error| {
            CreativeInventoryBuildError::new(
                resource_id,
                format!("invalid creative data component patch: {error}"),
            )
        },
    )?;
    Ok(stack)
}
