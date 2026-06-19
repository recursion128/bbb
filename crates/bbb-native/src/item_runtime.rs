use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{Context, Result};
use bbb_pack::{
    AtlasImage, AtlasLayout, AtlasPacker, AtlasSprite, FreezeImmuneWearableCatalog,
    FurnaceFuelCatalog, ItemAttackRange as PackItemAttackRange, ItemCuboidModel,
    ItemCuboidModelCatalog, ItemCuboidModelSet, ItemCuboidTextureImageCatalog,
    ItemEquipmentSlot as PackItemEquipmentSlot, ItemMiningProfile as PackItemMiningProfile,
    ItemMiningRule as PackItemMiningRule, ItemModelCatalog, ItemModelDefinition,
    ItemRegistryCatalog, ItemTintSource, ItemUseEffects as PackItemUseEffects, LanguageCatalog,
    PackRoots, SpriteImage, TerrainColorMaps, DEFAULT_LANGUAGE_CODE,
};
use bbb_protocol::packets::{
    DataComponentPatchSummary, ItemRaritySummary, ItemStackSummary, ItemStackTemplateSummary,
};
use bbb_world::{
    ItemAttackRange as WorldItemAttackRange, ItemEquipmentSlot as WorldItemEquipmentSlot,
    ItemUseEffects as WorldItemUseEffects, WorldItemMiningProfile, WorldItemMiningRule,
};

mod icon_model;

use icon_model::{
    contains_runtime_condition, item_icon_model_ref_for_definition, ItemIconModel, ItemIconModelRef,
};

const ITEM_ATLAS_MAX_WIDTH: u32 = 4096;
const ITEM_GENERATED_MAX_LAYERS: usize = 5;
const ITEM_ICON_RECURSION_LIMIT: usize = 16;
const MISSING_TEXTURE_ID: &str = "minecraft:missingno";
const ITEM_TINT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TOOLTIP_TEXT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TOOLTIP_TEXT_YELLOW: [f32; 4] = [1.0, 1.0, 85.0 / 255.0, 1.0];
const TOOLTIP_TEXT_AQUA: [f32; 4] = [85.0 / 255.0, 1.0, 1.0, 1.0];
const TOOLTIP_TEXT_LIGHT_PURPLE: [f32; 4] = [1.0, 85.0 / 255.0, 1.0, 1.0];
const TOOLTIP_TEXT_DARK_PURPLE: [f32; 4] = [170.0 / 255.0, 0.0, 170.0 / 255.0, 1.0];
const TOOLTIP_TEXT_GRAY: [f32; 4] = [170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0, 1.0];
const TOOLTIP_TEXT_BLUE: [f32; 4] = [85.0 / 255.0, 85.0 / 255.0, 1.0, 1.0];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NativeItemTooltipLine {
    pub(crate) text: String,
    pub(crate) tint: [f32; 4],
}

#[derive(Debug, Clone)]
pub(crate) struct NativeItemRuntime {
    item_definition_count: usize,
    item_registry_count: usize,
    resolved_model_count: usize,
    missing_model_ids: BTreeSet<String>,
    missing_texture_ids: BTreeSet<String>,
    furnace_fuel_item_ids: BTreeSet<i32>,
    freeze_immune_wearable_item_ids: BTreeSet<i32>,
    powder_snow_walkable_foot_item_ids: BTreeSet<i32>,
    item_icon_models: HashMap<String, ItemIconModel>,
    registry: Option<ItemRegistryCatalog>,
    language: LanguageCatalog,
    textures: ItemTextureState,
}

impl NativeItemRuntime {
    pub(crate) fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_with_locale(roots, DEFAULT_LANGUAGE_CODE)
    }

    pub(crate) fn load_with_locale(roots: &PackRoots, language_code: &str) -> Result<Self> {
        let item_models = roots
            .load_item_model_catalog()
            .context("load item model catalog")?;
        let cuboid_models = roots
            .load_item_cuboid_model_catalog()
            .context("load item cuboid model catalog")?;
        let texture_images = ItemCuboidTextureImageCatalog::load(roots)
            .context("load item cuboid texture images")?;
        let registry = roots
            .load_item_registry_catalog()
            .context("load item registry catalog")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native item registry catalog");
                err
            })
            .ok();
        let furnace_fuel_item_ids = registry
            .as_ref()
            .and_then(|registry| {
                FurnaceFuelCatalog::load(roots, registry)
                    .map(|catalog| catalog.protocol_ids(registry))
                    .map_err(|err| {
                        tracing::warn!(?err, "continuing without native furnace fuel catalog");
                        err
                    })
                    .ok()
            })
            .unwrap_or_default();
        let freeze_immune_wearable_item_ids = registry
            .as_ref()
            .and_then(|registry| {
                FreezeImmuneWearableCatalog::load(roots, registry)
                    .map(|catalog| catalog.protocol_ids(registry))
                    .map_err(|err| {
                        tracing::warn!(
                            ?err,
                            "continuing without native freeze immune wearable catalog"
                        );
                        err
                    })
                    .ok()
            })
            .unwrap_or_default();
        let powder_snow_walkable_foot_item_ids = registry
            .as_ref()
            .and_then(|registry| registry.protocol_id("minecraft:leather_boots"))
            .into_iter()
            .collect();
        let colormaps = roots
            .load_terrain_colormaps()
            .context("load terrain colormaps for item tints")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native item tint colormaps");
                err
            })
            .ok();
        let language = roots
            .load_client_language_catalog(language_code)
            .context("load item tooltip language catalog")?;
        Self::from_loaded(
            item_models,
            cuboid_models,
            texture_images,
            registry,
            colormaps,
            furnace_fuel_item_ids,
            freeze_immune_wearable_item_ids,
            powder_snow_walkable_foot_item_ids,
            language,
        )
    }

    fn from_loaded(
        item_models: ItemModelCatalog,
        cuboid_models: ItemCuboidModelCatalog,
        texture_images: ItemCuboidTextureImageCatalog,
        registry: Option<ItemRegistryCatalog>,
        colormaps: Option<TerrainColorMaps>,
        furnace_fuel_item_ids: BTreeSet<i32>,
        freeze_immune_wearable_item_ids: BTreeSet<i32>,
        powder_snow_walkable_foot_item_ids: BTreeSet<i32>,
        language: LanguageCatalog,
    ) -> Result<Self> {
        let mut texture_ids = BTreeSet::new();
        let mut item_icon_model_refs = HashMap::new();
        let mut missing_model_ids = BTreeSet::new();
        let mut missing_texture_ids = BTreeSet::new();
        let mut resolved_model_count = 0usize;

        for (item_id, definition) in item_models.definitions() {
            let models = cuboid_models.models_for_definition(definition);
            resolved_model_count += models.models.len();
            texture_ids.extend(models.texture_ids());
            let model_tints = model_tints_for_definition(&definition.model);
            let icon_model = if contains_runtime_condition(&definition.model) {
                item_icon_model_ref_for_definition(
                    &definition.model,
                    &cuboid_models,
                    &model_tints,
                    colormaps.as_ref(),
                )
            } else {
                ItemIconModelRef::Layers(item_icon_texture_layers(
                    &models,
                    &model_tints,
                    colormaps.as_ref(),
                ))
            };
            if !icon_model.is_empty() {
                item_icon_model_refs.insert(item_id.clone(), icon_model);
            }
            missing_model_ids.extend(models.missing_model_ids);
        }

        let mut images = Vec::new();
        if let Some(image) = texture_images.image(MISSING_TEXTURE_ID) {
            images.push(image.clone());
        } else {
            missing_texture_ids.insert(MISSING_TEXTURE_ID.to_string());
        }
        for texture_id in texture_ids {
            if texture_id == MISSING_TEXTURE_ID {
                continue;
            }
            match texture_images.image(&texture_id) {
                Some(image) => images.push(image.clone()),
                None => {
                    missing_texture_ids.insert(texture_id);
                }
            }
        }

        let textures = ItemTextureState::from_images(images)?;
        let item_icon_models = item_icon_model_refs
            .into_iter()
            .map(|(item_id, model)| (item_id, model.into_indexed(&textures)))
            .collect();

        Ok(Self {
            item_definition_count: item_models.len(),
            item_registry_count: registry.as_ref().map_or(0, ItemRegistryCatalog::len),
            resolved_model_count,
            missing_model_ids,
            missing_texture_ids,
            furnace_fuel_item_ids,
            freeze_immune_wearable_item_ids,
            powder_snow_walkable_foot_item_ids,
            item_icon_models,
            registry,
            language,
            textures,
        })
    }

    pub(crate) fn item_definition_count(&self) -> usize {
        self.item_definition_count
    }

    pub(crate) fn item_registry_count(&self) -> usize {
        self.item_registry_count
    }

    pub(crate) fn item_max_stack_sizes_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        let mut sizes = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return sizes;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(size) = registry.max_stack_size(resource_id) else {
                continue;
            };
            sizes.insert(protocol_id as i32, size);
        }
        sizes
    }

    pub(crate) fn item_equipment_slots_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, WorldItemEquipmentSlot> {
        let mut slots = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return slots;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(slot) = registry.equipment_slot(resource_id) else {
                continue;
            };
            slots.insert(protocol_id as i32, world_item_equipment_slot(slot));
        }
        slots
    }

    pub(crate) fn item_equipment_slot_count(&self) -> usize {
        self.item_equipment_slots_by_protocol_id().len()
    }

    pub(crate) fn default_piercing_weapon_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::default_piercing_weapon_protocol_ids)
            .unwrap_or_default()
    }

    pub(crate) fn default_piercing_weapon_item_count(&self) -> usize {
        self.default_piercing_weapon_item_ids_by_protocol_id().len()
    }

    pub(crate) fn item_attack_ranges_by_protocol_id(&self) -> BTreeMap<i32, WorldItemAttackRange> {
        let mut ranges = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return ranges;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(range) = registry.default_attack_range(resource_id) else {
                continue;
            };
            ranges.insert(protocol_id as i32, world_item_attack_range(range));
        }
        ranges
    }

    pub(crate) fn item_attack_range_count(&self) -> usize {
        self.item_attack_ranges_by_protocol_id().len()
    }

    pub(crate) fn item_use_effects_by_protocol_id(&self) -> BTreeMap<i32, WorldItemUseEffects> {
        let mut use_effects = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return use_effects;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(effects) = registry.default_use_effects(resource_id) else {
                continue;
            };
            use_effects.insert(protocol_id as i32, world_item_use_effects(effects));
        }
        use_effects
    }

    pub(crate) fn item_use_effect_count(&self) -> usize {
        self.item_use_effects_by_protocol_id().len()
    }

    pub(crate) fn item_mining_profiles_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, WorldItemMiningProfile> {
        let mut profiles = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return profiles;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(profile) = registry.mining_profile(resource_id) else {
                continue;
            };
            profiles.insert(protocol_id as i32, world_item_mining_profile(profile));
        }
        profiles
    }

    pub(crate) fn item_mining_profile_count(&self) -> usize {
        self.item_mining_profiles_by_protocol_id().len()
    }

    pub(crate) fn furnace_fuel_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.furnace_fuel_item_ids.clone()
    }

    pub(crate) fn furnace_fuel_item_count(&self) -> usize {
        self.furnace_fuel_item_ids.len()
    }

    pub(crate) fn freeze_immune_wearable_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.freeze_immune_wearable_item_ids.clone()
    }

    pub(crate) fn freeze_immune_wearable_item_count(&self) -> usize {
        self.freeze_immune_wearable_item_ids.len()
    }

    pub(crate) fn powder_snow_walkable_foot_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.powder_snow_walkable_foot_item_ids.clone()
    }

    pub(crate) fn powder_snow_walkable_foot_item_count(&self) -> usize {
        self.powder_snow_walkable_foot_item_ids.len()
    }

    pub(crate) fn resolved_model_count(&self) -> usize {
        self.resolved_model_count
    }

    pub(crate) fn missing_model_count(&self) -> usize {
        self.missing_model_ids.len()
    }

    pub(crate) fn missing_texture_count(&self) -> usize {
        self.missing_texture_ids.len()
    }

    pub(crate) fn texture_count(&self) -> usize {
        self.textures.texture_count()
    }

    pub(crate) fn icon_texture_count(&self) -> usize {
        self.item_icon_models.len()
    }

    pub(crate) fn atlas_size(&self) -> (u32, u32) {
        self.textures.atlas_size()
    }

    pub(crate) fn atlas_rgba(&self) -> &[u8] {
        self.textures.atlas_rgba()
    }

    pub(crate) fn texture_index(&self, texture_id: &str) -> u32 {
        self.textures.texture_index(texture_id)
    }

    #[cfg(test)]
    pub(crate) fn icon_texture_index_for_protocol_id(&self, protocol_id: i32) -> Option<u32> {
        let item_id = self.registry.as_ref()?.resource_id(protocol_id)?;
        Some(
            self.item_icon_models
                .get(item_id)
                .and_then(|model| {
                    model
                        .icon_layers(None, None, None, false)
                        .into_iter()
                        .next()
                })
                .map(|layer| layer.texture_index)
                .unwrap_or(self.textures.fallback_index()),
        )
    }

    pub(crate) fn item_resource_id_for_protocol_id(&self, protocol_id: i32) -> Option<&str> {
        self.registry.as_ref()?.resource_id(protocol_id)
    }

    pub(crate) fn tooltip_lines_for_stack(
        &self,
        stack: &ItemStackSummary,
    ) -> Option<Vec<NativeItemTooltipLine>> {
        if item_stack_is_empty(stack) {
            return None;
        }
        let item_id = self.registry.as_ref()?.resource_id(stack.item_id?)?;
        let mut lines = vec![NativeItemTooltipLine {
            text: hover_name_for_stack(&self.language, item_id, stack),
            tint: item_rarity_tint(item_rarity_for_stack(&stack.component_patch)),
        }];
        if let Some(book) = &stack.component_patch.written_book {
            push_written_book_tooltip_lines(&self.language, book, &mut lines);
        }
        lines.extend(stack.component_patch.lore.iter().cloned().map(|text| {
            NativeItemTooltipLine {
                text,
                tint: TOOLTIP_TEXT_DARK_PURPLE,
            }
        }));
        if stack.component_patch.unbreakable {
            lines.push(NativeItemTooltipLine {
                text: self.language.get_or_key("item.unbreakable").to_string(),
                tint: TOOLTIP_TEXT_BLUE,
            });
        }
        Some(lines)
    }

    #[cfg(test)]
    pub(crate) fn icon_uv_for_protocol_id(&self, protocol_id: i32) -> Option<ItemAtlasUvRect> {
        self.icon_for_protocol_id(protocol_id)
            .and_then(|icon| icon.layers.first().map(|layer| layer.uv))
    }

    pub(crate) fn icon_for_stack(&self, stack: &ItemStackSummary) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_bundle_selected_item(stack, None)
    }

    pub(crate) fn icon_for_stack_with_bundle_selected_item(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_bundle_selected_item_and_using_item(
            stack,
            bundle_selected_item_index,
            false,
        )
    }

    pub(crate) fn icon_for_stack_with_bundle_selected_item_and_using_item(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
    ) -> Option<ItemAtlasIcon> {
        let item_id = self.registry.as_ref()?.resource_id(stack.item_id?)?;
        self.icon_for_resource_id(
            item_id,
            Some(&stack.component_patch),
            bundle_selected_item_index,
            using_item,
        )
    }

    #[cfg(test)]
    pub(crate) fn icon_for_protocol_id(&self, protocol_id: i32) -> Option<ItemAtlasIcon> {
        let item_id = self.registry.as_ref()?.resource_id(protocol_id)?;
        self.icon_for_resource_id(item_id, None, None, false)
    }

    fn icon_for_resource_id(
        &self,
        item_id: &str,
        component_patch: Option<&DataComponentPatchSummary>,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
    ) -> Option<ItemAtlasIcon> {
        let default_max_damage = self
            .registry
            .as_ref()
            .and_then(|registry| registry.max_damage(item_id));
        let layers = self
            .item_icon_models
            .get(item_id)
            .map(|model| {
                self.icon_layers_for_model(
                    model,
                    component_patch,
                    default_max_damage,
                    bundle_selected_item_index,
                    using_item,
                    0,
                )
            })
            .unwrap_or_else(|| self.fallback_icon_texture_layers());
        let layers = layers
            .into_iter()
            .filter_map(|layer| {
                self.textures
                    .texture_uv_rect(layer.texture_index)
                    .map(|uv| ItemAtlasIconLayer {
                        uv,
                        tint: item_icon_tint_color(&layer.tint, component_patch),
                    })
            })
            .collect::<Vec<_>>();
        (!layers.is_empty()).then_some(ItemAtlasIcon { layers })
    }

    fn icon_layers_for_model(
        &self,
        model: &ItemIconModel,
        component_patch: Option<&DataComponentPatchSummary>,
        default_max_damage: Option<i32>,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        depth: usize,
    ) -> Vec<ItemIconTextureLayer> {
        if depth >= ITEM_ICON_RECURSION_LIMIT {
            return Vec::new();
        }
        let mut resolve_bundle_selected_item = || {
            self.bundle_selected_item_layers(component_patch, bundle_selected_item_index, depth + 1)
        };
        model.icon_layers_with_bundle_resolver(
            component_patch,
            default_max_damage,
            bundle_selected_item_index,
            using_item,
            &mut resolve_bundle_selected_item,
        )
    }

    fn bundle_selected_item_layers(
        &self,
        component_patch: Option<&DataComponentPatchSummary>,
        bundle_selected_item_index: Option<i32>,
        depth: usize,
    ) -> Vec<ItemIconTextureLayer> {
        let Some(selected_item_index) = bundle_selected_item_index.filter(|index| *index >= 0)
        else {
            return Vec::new();
        };
        let Ok(selected_item_index) = usize::try_from(selected_item_index) else {
            return Vec::new();
        };
        let Some(template) =
            component_patch.and_then(|patch| patch.bundle_contents_items.get(selected_item_index))
        else {
            return Vec::new();
        };
        self.item_template_layers(template, depth)
    }

    fn item_template_layers(
        &self,
        template: &ItemStackTemplateSummary,
        depth: usize,
    ) -> Vec<ItemIconTextureLayer> {
        let Some(item_id) = self
            .registry
            .as_ref()
            .and_then(|registry| registry.resource_id(template.item_id))
        else {
            return Vec::new();
        };
        let default_max_damage = self
            .registry
            .as_ref()
            .and_then(|registry| registry.max_damage(item_id));
        let layers = self
            .item_icon_models
            .get(item_id)
            .map(|model| {
                self.icon_layers_for_model(
                    model,
                    Some(&template.component_patch),
                    default_max_damage,
                    None,
                    false,
                    depth,
                )
            })
            .unwrap_or_else(|| self.fallback_icon_texture_layers());
        resolve_item_icon_texture_layer_tints(layers, Some(&template.component_patch))
    }

    fn fallback_icon_texture_layers(&self) -> Vec<ItemIconTextureLayer> {
        vec![ItemIconTextureLayer {
            texture_index: self.textures.fallback_index(),
            tint: ItemIconTint::Static(ITEM_TINT_WHITE),
        }]
    }
}

fn localized_item_name(language: &LanguageCatalog, resource_id: &str) -> String {
    let item_key = description_key("item", resource_id);
    if let Some(name) = language.get(&item_key) {
        return name.to_string();
    }

    let block_key = description_key("block", resource_id);
    language.get(&block_key).unwrap_or(&item_key).to_string()
}

fn hover_name_for_stack(
    language: &LanguageCatalog,
    resource_id: &str,
    stack: &ItemStackSummary,
) -> String {
    if let Some(name) = &stack.component_patch.custom_name {
        return name.clone();
    }
    if let Some(title) = stack
        .component_patch
        .written_book
        .as_ref()
        .map(|book| book.title.as_str())
        .filter(|title| !title.trim().is_empty())
    {
        return title.to_string();
    }
    if let Some(name) = &stack.component_patch.item_name {
        return name.clone();
    }
    localized_item_name(language, resource_id)
}

fn push_written_book_tooltip_lines(
    language: &LanguageCatalog,
    book: &bbb_protocol::packets::WrittenBookContentSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !book.author.trim().is_empty() {
        lines.push(NativeItemTooltipLine {
            text: translate_with_first_arg(language, "book.byAuthor", &book.author),
            tint: TOOLTIP_TEXT_GRAY,
        });
    }
    lines.push(NativeItemTooltipLine {
        text: language
            .get_or_key(&format!("book.generation.{}", book.generation))
            .to_string(),
        tint: TOOLTIP_TEXT_GRAY,
    });
}

fn translate_with_first_arg(language: &LanguageCatalog, key: &str, arg: &str) -> String {
    let template = language.get_or_key(key);
    if template.contains("%1$s") {
        template.replace("%1$s", arg)
    } else {
        template.replacen("%s", arg, 1)
    }
}

fn item_rarity_for_stack(component_patch: &DataComponentPatchSummary) -> ItemRaritySummary {
    let base = component_patch.rarity.unwrap_or(ItemRaritySummary::Common);
    if component_patch.enchantments.is_empty() {
        return base;
    }
    match base {
        ItemRaritySummary::Common | ItemRaritySummary::Uncommon => ItemRaritySummary::Rare,
        ItemRaritySummary::Rare => ItemRaritySummary::Epic,
        ItemRaritySummary::Epic => ItemRaritySummary::Epic,
    }
}

fn item_rarity_tint(rarity: ItemRaritySummary) -> [f32; 4] {
    match rarity {
        ItemRaritySummary::Common => TOOLTIP_TEXT_WHITE,
        ItemRaritySummary::Uncommon => TOOLTIP_TEXT_YELLOW,
        ItemRaritySummary::Rare => TOOLTIP_TEXT_AQUA,
        ItemRaritySummary::Epic => TOOLTIP_TEXT_LIGHT_PURPLE,
    }
}

fn description_key(prefix: &str, resource_id: &str) -> String {
    let (namespace, path) = resource_id
        .split_once(':')
        .unwrap_or(("minecraft", resource_id));
    format!("{prefix}.{namespace}.{}", path.replace('/', "."))
}

fn item_stack_is_empty(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

fn world_item_equipment_slot(slot: PackItemEquipmentSlot) -> WorldItemEquipmentSlot {
    match slot {
        PackItemEquipmentSlot::MainHand => WorldItemEquipmentSlot::MainHand,
        PackItemEquipmentSlot::OffHand => WorldItemEquipmentSlot::OffHand,
        PackItemEquipmentSlot::Feet => WorldItemEquipmentSlot::Feet,
        PackItemEquipmentSlot::Legs => WorldItemEquipmentSlot::Legs,
        PackItemEquipmentSlot::Chest => WorldItemEquipmentSlot::Chest,
        PackItemEquipmentSlot::Head => WorldItemEquipmentSlot::Head,
        PackItemEquipmentSlot::Body => WorldItemEquipmentSlot::Body,
        PackItemEquipmentSlot::Saddle => WorldItemEquipmentSlot::Saddle,
    }
}

fn world_item_attack_range(range: PackItemAttackRange) -> WorldItemAttackRange {
    WorldItemAttackRange {
        min_reach: range.min_reach,
        max_reach: range.max_reach,
        min_creative_reach: range.min_creative_reach,
        max_creative_reach: range.max_creative_reach,
        hitbox_margin: range.hitbox_margin,
        mob_factor: range.mob_factor,
    }
}

fn world_item_use_effects(effects: PackItemUseEffects) -> WorldItemUseEffects {
    WorldItemUseEffects {
        can_sprint: effects.can_sprint,
        interact_vibrations: effects.interact_vibrations,
        speed_multiplier: effects.speed_multiplier,
    }
}

fn world_item_mining_profile(profile: &PackItemMiningProfile) -> WorldItemMiningProfile {
    WorldItemMiningProfile {
        default_mining_speed_thousandths: profile.default_mining_speed_thousandths,
        rules: profile.rules.iter().map(world_item_mining_rule).collect(),
    }
}

fn world_item_mining_rule(rule: &PackItemMiningRule) -> WorldItemMiningRule {
    WorldItemMiningRule {
        block_names: rule.block_names.clone(),
        mining_speed_thousandths: rule.mining_speed_thousandths,
        correct_for_drops: rule.correct_for_drops,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ItemAtlasIcon {
    pub(crate) layers: Vec<ItemAtlasIconLayer>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ItemAtlasIconLayer {
    pub(crate) uv: ItemAtlasUvRect,
    pub(crate) tint: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ItemAtlasUvRect {
    pub(crate) min: [f32; 2],
    pub(crate) max: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
struct ItemIconTextureLayer {
    texture_index: u32,
    tint: ItemIconTint,
}

#[derive(Debug, Clone, PartialEq)]
struct ItemIconTextureRef {
    texture_id: String,
    tint: ItemIconTint,
}

#[derive(Debug, Clone, PartialEq)]
enum ItemIconTint {
    Static([f32; 4]),
    Source(ItemTintSource),
}

#[derive(Debug, Clone)]
struct ItemTextureState {
    atlas: AtlasImage,
    texture_indices: HashMap<String, u32>,
    fallback_index: u32,
}

impl ItemTextureState {
    fn from_images(images: Vec<SpriteImage>) -> Result<Self> {
        let packer = AtlasPacker::new(ITEM_ATLAS_MAX_WIDTH, 1)?;
        let atlas = packer.stitch(&images)?;
        let mut texture_indices = HashMap::new();
        for (index, sprite) in atlas.layout.sprites.iter().enumerate() {
            texture_indices.insert(sprite.id.clone(), index as u32);
        }
        let fallback_index = texture_indices
            .get(MISSING_TEXTURE_ID)
            .copied()
            .unwrap_or(0);
        Ok(Self {
            atlas,
            texture_indices,
            fallback_index,
        })
    }

    fn texture_count(&self) -> usize {
        self.atlas.layout.sprites.len()
    }

    fn atlas_size(&self) -> (u32, u32) {
        (self.atlas.layout.width, self.atlas.layout.height)
    }

    fn atlas_rgba(&self) -> &[u8] {
        &self.atlas.rgba
    }

    fn fallback_index(&self) -> u32 {
        self.fallback_index
    }

    fn texture_index(&self, texture_id: &str) -> u32 {
        self.texture_indices
            .get(texture_id)
            .copied()
            .unwrap_or(self.fallback_index)
    }

    fn texture_uv_rect(&self, texture_index: u32) -> Option<ItemAtlasUvRect> {
        let sprite = self.atlas.layout.sprites.get(texture_index as usize)?;
        Some(item_uv_rect(&self.atlas.layout, sprite))
    }
}

fn item_icon_texture_layers(
    models: &ItemCuboidModelSet,
    model_tints: &HashMap<String, Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
) -> Vec<ItemIconTextureRef> {
    models
        .models
        .iter()
        .find_map(|model| generated_layer_texture_refs(model, model_tints, colormaps))
        .or_else(|| {
            models
                .models
                .iter()
                .find_map(first_texture_id)
                .map(|texture_id| {
                    vec![ItemIconTextureRef {
                        texture_id,
                        tint: ItemIconTint::Static(ITEM_TINT_WHITE),
                    }]
                })
        })
        .unwrap_or_default()
}

fn generated_layer_texture_refs(
    model: &ItemCuboidModel,
    model_tints: &HashMap<String, Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
) -> Option<Vec<ItemIconTextureRef>> {
    let tints = model_tints.get(&model.id);
    let mut layers = Vec::new();
    for layer_index in 0..ITEM_GENERATED_MAX_LAYERS {
        let Some(texture) = model.texture_slots.get(&format!("layer{layer_index}")) else {
            break;
        };
        layers.push(ItemIconTextureRef {
            texture_id: texture.id.clone(),
            tint: tints
                .and_then(|tints| tints.get(layer_index))
                .map(|tint| item_tint_source(tint, colormaps))
                .unwrap_or(ItemIconTint::Static(ITEM_TINT_WHITE)),
        });
    }
    (!layers.is_empty()).then_some(layers)
}

fn first_texture_id(model: &ItemCuboidModel) -> Option<String> {
    model
        .texture_slots
        .values()
        .next()
        .map(|texture| texture.id.clone())
        .or_else(|| {
            model
                .face_textures
                .as_ref()
                .map(|textures| textures.textures[0].clone())
        })
}

fn model_tints_for_definition(model: &ItemModelDefinition) -> HashMap<String, Vec<ItemTintSource>> {
    let mut tints = HashMap::new();
    collect_model_tints(model, &mut tints);
    tints
}

fn collect_model_tints(
    model: &ItemModelDefinition,
    tints_by_model: &mut HashMap<String, Vec<ItemTintSource>>,
) {
    match model {
        ItemModelDefinition::Empty | ItemModelDefinition::BundleSelectedItem => {}
        ItemModelDefinition::Model { model, tints, .. } => {
            tints_by_model
                .entry(model.clone())
                .or_insert_with(|| tints.clone());
        }
        ItemModelDefinition::Condition {
            on_true, on_false, ..
        } => {
            collect_model_tints(on_true, tints_by_model);
            collect_model_tints(on_false, tints_by_model);
        }
        ItemModelDefinition::RangeDispatch {
            entries, fallback, ..
        } => {
            for entry in entries {
                collect_model_tints(&entry.model, tints_by_model);
            }
            if let Some(fallback) = fallback {
                collect_model_tints(fallback, tints_by_model);
            }
        }
        ItemModelDefinition::Select {
            cases, fallback, ..
        } => {
            for case in cases {
                collect_model_tints(&case.model, tints_by_model);
            }
            if let Some(fallback) = fallback {
                collect_model_tints(fallback, tints_by_model);
            }
        }
        ItemModelDefinition::Composite { models, .. } => {
            for model in models {
                collect_model_tints(model, tints_by_model);
            }
        }
        ItemModelDefinition::Special { base, .. } => {
            tints_by_model.entry(base.clone()).or_default();
        }
    }
}

fn item_tint_source_default_color(
    tint: &ItemTintSource,
    colormaps: Option<&TerrainColorMaps>,
) -> [f32; 4] {
    match tint {
        ItemTintSource::CustomModelData { default_color, .. }
        | ItemTintSource::Dye { default_color }
        | ItemTintSource::Firework { default_color }
        | ItemTintSource::Potion { default_color }
        | ItemTintSource::MapColor { default_color }
        | ItemTintSource::Team { default_color } => rgb_i32_tint(*default_color),
        ItemTintSource::Constant { value } => rgb_i32_tint(*value),
        ItemTintSource::Grass {
            temperature,
            downfall,
        } => colormaps
            .map(|colormaps| {
                rgb_u8_tint(
                    colormaps
                        .grass
                        .sample_temperature_downfall(*temperature, *downfall),
                )
            })
            .unwrap_or_else(|| rgb_u8_tint([0x91, 0xbd, 0x59])),
    }
}

fn item_tint_source(tint: &ItemTintSource, colormaps: Option<&TerrainColorMaps>) -> ItemIconTint {
    match tint {
        ItemTintSource::Constant { .. } | ItemTintSource::Grass { .. } => {
            ItemIconTint::Static(item_tint_source_default_color(tint, colormaps))
        }
        ItemTintSource::CustomModelData { .. }
        | ItemTintSource::Dye { .. }
        | ItemTintSource::Firework { .. }
        | ItemTintSource::Potion { .. }
        | ItemTintSource::MapColor { .. }
        | ItemTintSource::Team { .. } => ItemIconTint::Source(tint.clone()),
    }
}

fn item_icon_tint_color(
    tint: &ItemIconTint,
    component_patch: Option<&DataComponentPatchSummary>,
) -> [f32; 4] {
    match tint {
        ItemIconTint::Static(color) => *color,
        ItemIconTint::Source(source) => item_tint_source_color(source, component_patch),
    }
}

fn resolve_item_icon_texture_layer_tints(
    layers: Vec<ItemIconTextureLayer>,
    component_patch: Option<&DataComponentPatchSummary>,
) -> Vec<ItemIconTextureLayer> {
    layers
        .into_iter()
        .map(|layer| ItemIconTextureLayer {
            texture_index: layer.texture_index,
            tint: ItemIconTint::Static(item_icon_tint_color(&layer.tint, component_patch)),
        })
        .collect()
}

fn item_tint_source_color(
    tint: &ItemTintSource,
    component_patch: Option<&DataComponentPatchSummary>,
) -> [f32; 4] {
    match tint {
        ItemTintSource::CustomModelData {
            index,
            default_color,
        } => {
            let color = component_patch
                .and_then(|patch| patch.custom_model_data_colors.get(*index as usize))
                .copied()
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::Dye { default_color } => {
            let color = component_patch
                .and_then(|patch| patch.dyed_color)
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::MapColor { default_color } => {
            let color = component_patch
                .and_then(|patch| patch.map_color)
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::Potion { default_color } => {
            let color = component_patch
                .and_then(|patch| patch.potion_custom_color)
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::Firework { default_color } => {
            let color = component_patch
                .and_then(|patch| firework_explosion_tint_color(&patch.firework_explosion_colors))
                .unwrap_or(*default_color);
            rgb_i32_tint(color)
        }
        ItemTintSource::Constant { value } => rgb_i32_tint(*value),
        ItemTintSource::Grass { .. } | ItemTintSource::Team { .. } => {
            item_tint_source_default_color(tint, None)
        }
    }
}

fn firework_explosion_tint_color(colors: &[i32]) -> Option<i32> {
    if colors.is_empty() {
        return None;
    }
    if colors.len() == 1 {
        return Some(colors[0]);
    }

    let mut red = 0u32;
    let mut green = 0u32;
    let mut blue = 0u32;
    for color in colors {
        let color = *color as u32;
        red += (color >> 16) & 0xff;
        green += (color >> 8) & 0xff;
        blue += color & 0xff;
    }
    let len = colors.len() as u32;
    Some(((red / len) << 16 | (green / len) << 8 | (blue / len)) as i32)
}

fn rgb_i32_tint(value: i32) -> [f32; 4] {
    let rgb = value as u32;
    rgb_u8_tint([
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    ])
}

fn rgb_u8_tint(rgb: [u8; 3]) -> [f32; 4] {
    [
        f32::from(rgb[0]) / 255.0,
        f32::from(rgb[1]) / 255.0,
        f32::from(rgb[2]) / 255.0,
        1.0,
    ]
}

fn item_uv_rect(layout: &AtlasLayout, sprite: &AtlasSprite) -> ItemAtlasUvRect {
    let width = layout.width as f32;
    let height = layout.height as f32;
    let x0 = sprite.content.x as f32;
    let y0 = sprite.content.y as f32;
    let x1 = (sprite.content.x + sprite.content.width) as f32;
    let y1 = (sprite.content.y + sprite.content.height) as f32;
    ItemAtlasUvRect {
        min: [(x0 + 0.5) / width, (y0 + 0.5) / height],
        max: [(x1 - 0.5) / width, (y1 - 0.5) / height],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    fn tooltip_line(text: &str, tint: [f32; 4]) -> NativeItemTooltipLine {
        NativeItemTooltipLine {
            text: text.to_string(),
            tint,
        }
    }

    #[test]
    fn item_texture_state_indexes_textures_and_uses_missing_fallback() {
        let missing = SpriteImage::new("minecraft:missingno", 1, 1, vec![0, 0, 0, 255]).unwrap();
        let apple = SpriteImage::new("minecraft:item/apple", 1, 1, vec![255, 0, 0, 255]).unwrap();

        let state = ItemTextureState::from_images(vec![missing, apple]).unwrap();

        assert_eq!(state.texture_count(), 2);
        assert_ne!(
            state.texture_index("minecraft:item/apple"),
            state.texture_index("minecraft:missingno")
        );
        assert_eq!(
            state.texture_index("custom:item/missing"),
            state.texture_index(MISSING_TEXTURE_ID)
        );
    }

    #[test]
    fn item_tint_sources_use_stack_component_colors_when_available() {
        let patch = DataComponentPatchSummary {
            custom_model_data_colors: vec![0x01_02_03, 0x04_05_06],
            dyed_color: Some(0x07_08_09),
            map_color: Some(0x0a_0b_0c),
            potion_custom_color: Some(0x0d_0e_0f),
            firework_explosion_colors: vec![0x10_20_30, 0x20_40_60],
            ..DataComponentPatchSummary::default()
        };

        assert_eq!(
            item_tint_source_color(
                &ItemTintSource::CustomModelData {
                    index: 1,
                    default_color: 0xff_00_ff,
                },
                Some(&patch),
            ),
            rgb_i32_tint(0x04_05_06)
        );
        assert_eq!(
            item_tint_source_color(
                &ItemTintSource::Dye {
                    default_color: 0xff_00_ff,
                },
                Some(&patch),
            ),
            rgb_i32_tint(0x07_08_09)
        );
        assert_eq!(
            item_tint_source_color(
                &ItemTintSource::MapColor {
                    default_color: 0xff_00_ff,
                },
                Some(&patch),
            ),
            rgb_i32_tint(0x0a_0b_0c)
        );
        assert_eq!(
            item_tint_source_color(
                &ItemTintSource::Potion {
                    default_color: 0xff_00_ff,
                },
                Some(&patch),
            ),
            rgb_i32_tint(0x0d_0e_0f)
        );
        assert_eq!(
            item_tint_source_color(
                &ItemTintSource::Firework {
                    default_color: 0xff_00_ff,
                },
                Some(&patch),
            ),
            rgb_i32_tint(0x18_30_48)
        );
        assert_eq!(
            item_tint_source_color(
                &ItemTintSource::CustomModelData {
                    index: 2,
                    default_color: 0xff_00_ff,
                },
                Some(&patch),
            ),
            rgb_i32_tint(0xff_00_ff)
        );
        assert_eq!(
            item_tint_source_color(
                &ItemTintSource::Firework {
                    default_color: 0xff_00_ff,
                },
                Some(&DataComponentPatchSummary::default()),
            ),
            rgb_i32_tint(0xff_00_ff)
        );
    }

    #[test]
    fn localized_item_name_prefers_item_key_then_block_key() {
        let language = LanguageCatalog::from_json_bytes(
            br#"{
                "item.minecraft.redstone": "Redstone Dust",
                "block.minecraft.stone": "Stone"
            }"#,
        )
        .unwrap();

        assert_eq!(
            localized_item_name(&language, "minecraft:redstone"),
            "Redstone Dust"
        );
        assert_eq!(localized_item_name(&language, "minecraft:stone"), "Stone");
        assert_eq!(
            localized_item_name(&language, "minecraft:missing_item"),
            "item.minecraft.missing_item"
        );
        assert_eq!(
            description_key("item", "custom:tools/hammer"),
            "item.custom.tools.hammer"
        );
    }

    #[test]
    fn native_item_runtime_loads_fixture_and_keeps_missingno_fallback() {
        let root = unique_temp_dir("item-runtime");
        let assets = assets_dir(&root);
        write_item_atlases(&assets);
        write_item_registry_sources(&root);
        write_json(
            &assets.join("lang").join("en_us.json"),
            r#"{
                "item.minecraft.test_combo": "Test Combo",
                "item.unbreakable": "Unbreakable",
                "book.byAuthor": "by %1$s",
                "book.generation.0": "Original",
                "book.generation.2": "Copy of a copy"
            }"#,
        );
        write_json(
            &assets.join("items").join("test_combo.json"),
            r#"{
                "model": {
                    "type": "minecraft:composite",
                    "models": [
                        {
                            "type": "minecraft:model",
                            "model": "minecraft:item/test_sword",
                            "tints": [
                                { "type": "minecraft:constant", "value": 3368601 },
                                { "type": "minecraft:custom_model_data", "index": 1, "default": 16711935 }
                            ]
                        },
                        {
                            "type": "minecraft:model",
                            "model": "minecraft:item/missing_model"
                        }
                    ]
                }
            }"#,
        );
        write_json(
            &assets.join("models").join("item").join("test_sword.json"),
            r##"{
                "textures": {
                    "layer0": "minecraft:item/test_sword",
                    "layer1": {
                        "sprite": "custom:item/missing_overlay",
                        "force_translucent": true
                    }
                }
            }"##,
        );
        write_test_rgba_png(
            &assets.join("textures").join("item").join("test_sword.png"),
            1,
            1,
            &[255, 0, 0, 255],
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();

        assert_eq!(runtime.item_definition_count(), 1);
        assert_eq!(runtime.item_registry_count(), 1);
        assert_eq!(runtime.item_equipment_slot_count(), 1);
        assert_eq!(runtime.item_mining_profile_count(), 0);
        assert_eq!(
            runtime.item_equipment_slots_by_protocol_id(),
            BTreeMap::from([(0, WorldItemEquipmentSlot::Chest)])
        );
        assert_eq!(runtime.resolved_model_count(), 1);
        assert_eq!(runtime.missing_model_count(), 1);
        assert_eq!(runtime.missing_texture_count(), 1);
        assert_eq!(runtime.texture_count(), 2);
        assert_eq!(runtime.icon_texture_count(), 1);
        assert_ne!(
            runtime.texture_index("minecraft:item/test_sword"),
            runtime.texture_index(MISSING_TEXTURE_ID)
        );
        assert_eq!(
            runtime.texture_index("custom:item/missing_overlay"),
            runtime.texture_index(MISSING_TEXTURE_ID)
        );
        assert_eq!(
            runtime.texture_index("unknown:item/texture"),
            runtime.texture_index(MISSING_TEXTURE_ID)
        );
        assert_eq!(
            runtime.icon_texture_index_for_protocol_id(0),
            Some(runtime.texture_index("minecraft:item/test_sword"))
        );
        assert_eq!(runtime.icon_texture_index_for_protocol_id(1), None);
        let icon = runtime.icon_for_protocol_id(0).unwrap();
        assert_eq!(icon.layers.len(), 2);
        assert_eq!(icon.layers[0].tint, rgb_i32_tint(0x33_66_99));
        assert_eq!(icon.layers[1].tint, rgb_i32_tint(0xff_00_ff));
        assert_eq!(
            icon.layers[1].uv,
            runtime
                .textures
                .texture_uv_rect(runtime.texture_index(MISSING_TEXTURE_ID))
                .unwrap()
        );
        assert_eq!(runtime.icon_uv_for_protocol_id(0), Some(icon.layers[0].uv));
        assert_eq!(
            runtime.tooltip_lines_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }),
            Some(vec![tooltip_line("Test Combo", TOOLTIP_TEXT_WHITE)])
        );
        assert_eq!(
            runtime.tooltip_lines_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 0,
                component_patch: DataComponentPatchSummary::default(),
            }),
            None
        );
        assert_eq!(
            runtime.tooltip_lines_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    custom_name: Some("Custom Pick".to_string()),
                    item_name: Some("Renamed Item Name".to_string()),
                    lore: vec!["First lore".to_string(), "Second lore".to_string()],
                    ..DataComponentPatchSummary::default()
                },
            }),
            Some(vec![
                tooltip_line("Custom Pick", TOOLTIP_TEXT_WHITE),
                tooltip_line("First lore", TOOLTIP_TEXT_DARK_PURPLE),
                tooltip_line("Second lore", TOOLTIP_TEXT_DARK_PURPLE),
            ])
        );
        assert_eq!(
            runtime.tooltip_lines_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    written_book: Some(bbb_protocol::packets::WrittenBookContentSummary {
                        title: "Book Title".to_string(),
                        author: "Alex".to_string(),
                        generation: 0,
                        pages: Vec::new(),
                        resolved: true,
                    }),
                    item_name: Some("Ignored Item Name".to_string()),
                    lore: vec!["Book lore".to_string()],
                    ..DataComponentPatchSummary::default()
                },
            }),
            Some(vec![
                tooltip_line("Book Title", TOOLTIP_TEXT_WHITE),
                tooltip_line("by Alex", TOOLTIP_TEXT_GRAY),
                tooltip_line("Original", TOOLTIP_TEXT_GRAY),
                tooltip_line("Book lore", TOOLTIP_TEXT_DARK_PURPLE),
            ])
        );
        assert_eq!(
            runtime.tooltip_lines_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    written_book: Some(bbb_protocol::packets::WrittenBookContentSummary {
                        title: "Copy".to_string(),
                        author: "   ".to_string(),
                        generation: 2,
                        pages: Vec::new(),
                        resolved: true,
                    }),
                    ..DataComponentPatchSummary::default()
                },
            }),
            Some(vec![
                tooltip_line("Copy", TOOLTIP_TEXT_WHITE),
                tooltip_line("Copy of a copy", TOOLTIP_TEXT_GRAY),
            ])
        );
        assert_eq!(
            runtime.tooltip_lines_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    item_name: Some("Component Item Name".to_string()),
                    rarity: Some(ItemRaritySummary::Uncommon),
                    ..DataComponentPatchSummary::default()
                },
            }),
            Some(vec![tooltip_line(
                "Component Item Name",
                TOOLTIP_TEXT_YELLOW
            )])
        );
        assert_eq!(
            runtime.tooltip_lines_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    item_name: Some("Durable Item".to_string()),
                    unbreakable: true,
                    ..DataComponentPatchSummary::default()
                },
            }),
            Some(vec![
                tooltip_line("Durable Item", TOOLTIP_TEXT_WHITE),
                tooltip_line("Unbreakable", TOOLTIP_TEXT_BLUE),
            ])
        );
        assert_eq!(
            runtime.tooltip_lines_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    item_name: Some("Enchanted Item".to_string()),
                    enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 7,
                        level: 1,
                    }],
                    ..DataComponentPatchSummary::default()
                },
            }),
            Some(vec![tooltip_line("Enchanted Item", TOOLTIP_TEXT_AQUA)])
        );
        assert_eq!(
            runtime.tooltip_lines_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    item_name: Some("Rare Enchanted Item".to_string()),
                    rarity: Some(ItemRaritySummary::Rare),
                    enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 7,
                        level: 1,
                    }],
                    ..DataComponentPatchSummary::default()
                },
            }),
            Some(vec![tooltip_line(
                "Rare Enchanted Item",
                TOOLTIP_TEXT_LIGHT_PURPLE
            )])
        );

        let stack_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    custom_model_data_colors: vec![0x00_00_00, 0x12_34_56],
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(stack_icon.layers[0].tint, rgb_i32_tint(0x33_66_99));
        assert_eq!(stack_icon.layers[1].tint, rgb_i32_tint(0x12_34_56));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_converts_mining_profiles_for_world() {
        let profile = PackItemMiningProfile {
            default_mining_speed_thousandths: 1_000,
            rules: vec![
                PackItemMiningRule {
                    block_names: vec!["minecraft:obsidian".to_string()],
                    mining_speed_thousandths: None,
                    correct_for_drops: Some(false),
                },
                PackItemMiningRule {
                    block_names: vec!["minecraft:stone".to_string()],
                    mining_speed_thousandths: Some(4_000),
                    correct_for_drops: Some(true),
                },
            ],
        };

        let converted = world_item_mining_profile(&profile);

        assert_eq!(converted.default_mining_speed_thousandths, 1_000);
        assert_eq!(converted.rules.len(), 2);
        assert_eq!(
            converted.rules[0].block_names,
            vec!["minecraft:obsidian".to_string()]
        );
        assert_eq!(converted.rules[0].correct_for_drops, Some(false));
        assert_eq!(
            converted.rules[1].block_names,
            vec!["minecraft:stone".to_string()]
        );
        assert_eq!(converted.rules[1].mining_speed_thousandths, Some(4_000));
        assert_eq!(converted.rules[1].correct_for_drops, Some(true));
    }

    #[test]
    fn native_item_runtime_selects_broken_condition_icon_from_stack_damage() {
        let root = unique_temp_dir("item-runtime-broken");
        write_elytra_damage_condition_fixture(
            &root,
            "minecraft:broken",
            "elytra_broken",
            &[120, 80, 40, 255],
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        assert_eq!(
            runtime
                .registry
                .as_ref()
                .and_then(|registry| registry.max_damage("minecraft:elytra")),
            Some(432)
        );
        let normal_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/elytra"))
            .unwrap();
        let broken_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/elytra_broken"))
            .unwrap();

        assert_eq!(runtime.icon_texture_count(), 1);
        assert_eq!(
            runtime.icon_texture_index_for_protocol_id(0),
            Some(runtime.texture_index("minecraft:item/elytra"))
        );

        let normal_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    damage: Some(430),
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(normal_icon.layers[0].uv, normal_uv);

        let broken_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    damage: Some(431),
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(broken_icon.layers[0].uv, broken_uv);

        let unbreakable_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    damage: Some(431),
                    unbreakable: true,
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(unbreakable_icon.layers[0].uv, normal_uv);

        let removed_max_damage_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    damage: Some(431),
                    removed_type_ids: vec![2],
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(removed_max_damage_icon.layers[0].uv, normal_uv);

        let removed_damage_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    removed_type_ids: vec![3],
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(removed_damage_icon.layers[0].uv, normal_uv);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_selects_damaged_condition_icon_from_stack_damage() {
        let root = unique_temp_dir("item-runtime-damaged");
        write_elytra_damage_condition_fixture(
            &root,
            "minecraft:damaged",
            "elytra_damaged",
            &[80, 120, 40, 255],
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let normal_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/elytra"))
            .unwrap();
        let damaged_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/elytra_damaged"))
            .unwrap();

        assert_eq!(
            runtime.icon_texture_index_for_protocol_id(0),
            Some(runtime.texture_index("minecraft:item/elytra"))
        );

        let pristine_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            })
            .unwrap();
        assert_eq!(pristine_icon.layers[0].uv, normal_uv);

        let damaged_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    damage: Some(1),
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(damaged_icon.layers[0].uv, damaged_uv);

        let unbreakable_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    damage: Some(1),
                    unbreakable: true,
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(unbreakable_icon.layers[0].uv, normal_uv);

        let removed_damage_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    damage: Some(1),
                    removed_type_ids: vec![3],
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(removed_damage_icon.layers[0].uv, normal_uv);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_selects_has_component_icon_from_dyed_color() {
        let root = unique_temp_dir("item-runtime-has-dyed-color");
        write_wolf_armor_has_component_fixture(&root);

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let normal_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/wolf_armor"))
            .unwrap();
        let dyed_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/wolf_armor_dyed"))
            .unwrap();

        assert_eq!(
            runtime.icon_texture_index_for_protocol_id(0),
            Some(runtime.texture_index("minecraft:item/wolf_armor"))
        );

        let default_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            })
            .unwrap();
        assert_eq!(default_icon.layers[0].uv, normal_uv);
        assert_eq!(default_icon.layers[0].tint, ITEM_TINT_WHITE);

        let dyed_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![44],
                    dyed_color: Some(0x33_66_99),
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(dyed_icon.layers[0].uv, dyed_uv);
        assert_eq!(dyed_icon.layers[0].tint, rgb_i32_tint(0x33_66_99));

        let removed_dye_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![44],
                    removed_type_ids: vec![44],
                    dyed_color: Some(0x33_66_99),
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(removed_dye_icon.layers[0].uv, normal_uv);
        assert_eq!(removed_dye_icon.layers[0].tint, ITEM_TINT_WHITE);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_selects_has_component_icon_from_lodestone_tracker() {
        let root = unique_temp_dir("item-runtime-has-lodestone");
        write_compass_has_component_fixture(&root);

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let normal_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/compass"))
            .unwrap();
        let lodestone_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/compass_lodestone"))
            .unwrap();

        assert_eq!(
            runtime.icon_texture_index_for_protocol_id(0),
            Some(runtime.texture_index("minecraft:item/compass"))
        );

        let normal_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            })
            .unwrap();
        assert_eq!(normal_icon.layers[0].uv, normal_uv);

        let lodestone_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![67],
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(lodestone_icon.layers[0].uv, lodestone_uv);

        let removed_lodestone_icon = runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![67],
                    removed_type_ids: vec![67],
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap();
        assert_eq!(removed_lodestone_icon.layers[0].uv, normal_uv);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_selects_bundle_icon_from_local_selected_item() {
        let root = unique_temp_dir("item-runtime-bundle-selected");
        write_bundle_selected_item_fixture(&root);

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let normal_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/bundle"))
            .unwrap();
        let open_back_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/bundle_open_back"))
            .unwrap();
        let open_front_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/bundle_open_front"))
            .unwrap();
        let apple_uv = runtime
            .textures
            .texture_uv_rect(runtime.texture_index("minecraft:item/apple"))
            .unwrap();
        let bundle_stack = ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 1,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                bundle_contents_item_count: Some(1),
                ..DataComponentPatchSummary::default()
            },
        };

        assert_eq!(
            runtime.icon_texture_index_for_protocol_id(0),
            Some(runtime.texture_index("minecraft:item/bundle"))
        );

        let default_icon = runtime.icon_for_stack(&bundle_stack).unwrap();
        assert_eq!(default_icon.layers[0].uv, normal_uv);

        let unselected_icon = runtime
            .icon_for_stack_with_bundle_selected_item(&bundle_stack, Some(-1))
            .unwrap();
        assert_eq!(unselected_icon.layers[0].uv, normal_uv);

        let selected_icon = runtime
            .icon_for_stack_with_bundle_selected_item(&bundle_stack, Some(0))
            .unwrap();
        assert_eq!(
            selected_icon
                .layers
                .iter()
                .map(|layer| layer.uv)
                .collect::<Vec<_>>(),
            vec![open_back_uv, apple_uv, open_front_uv]
        );

        let out_of_bounds_icon = runtime
            .icon_for_stack_with_bundle_selected_item(&bundle_stack, Some(1))
            .unwrap();
        assert_eq!(out_of_bounds_icon.layers[0].uv, normal_uv);

        let no_contents_icon = runtime
            .icon_for_stack_with_bundle_selected_item(
                &ItemStackSummary {
                    item_id: Some(0),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
                Some(0),
            )
            .unwrap();
        assert_eq!(no_contents_icon.layers[0].uv, normal_uv);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_loads_assets_when_registry_source_is_missing() {
        let root = unique_temp_dir("item-runtime-no-registry");
        let assets = assets_dir(&root);
        write_item_atlases(&assets);
        write_json(
            &assets.join("items").join("test_combo.json"),
            r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/test_sword"
                }
            }"#,
        );
        write_json(
            &assets.join("models").join("item").join("test_sword.json"),
            r#"{
                "textures": {
                    "layer0": "minecraft:item/test_sword"
                }
            }"#,
        );
        write_test_rgba_png(
            &assets.join("textures").join("item").join("test_sword.png"),
            1,
            1,
            &[255, 0, 0, 255],
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();

        assert_eq!(runtime.item_definition_count(), 1);
        assert_eq!(runtime.item_registry_count(), 0);
        assert_eq!(runtime.item_equipment_slot_count(), 0);
        assert_eq!(runtime.item_mining_profile_count(), 0);
        assert!(runtime.item_equipment_slots_by_protocol_id().is_empty());
        assert!(runtime.item_mining_profiles_by_protocol_id().is_empty());
        assert_eq!(runtime.texture_count(), 2);
        assert_eq!(runtime.icon_texture_count(), 1);
        assert!(!runtime.atlas_rgba().is_empty());
        assert_eq!(runtime.icon_texture_index_for_protocol_id(0), None);
        assert_eq!(runtime.icon_uv_for_protocol_id(0), None);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_item_runtime_assets() {
        let runtime = NativeItemRuntime::load(&PackRoots::discover().unwrap()).unwrap();

        assert_eq!(runtime.item_definition_count(), 1506);
        assert_eq!(runtime.item_registry_count(), 1506);
        assert_eq!(runtime.texture_count(), 1576);
        assert_eq!(runtime.icon_texture_count(), 1506);
        assert_eq!(runtime.missing_model_count(), 0);
        assert_eq!(runtime.missing_texture_count(), 0);
        assert!(runtime.icon_uv_for_protocol_id(1).is_some());
        assert!(runtime.resolved_model_count() > runtime.item_definition_count());
        assert!(runtime.atlas_size().0 > 0);
        assert!(runtime.atlas_size().1 > 0);
    }

    fn assets_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("assets")
            .join("minecraft")
    }

    fn write_item_atlases(assets: &Path) {
        write_json(
            &assets.join("atlases").join("items.json"),
            r#"{
                "sources": [
                    {
                        "type": "minecraft:directory",
                        "prefix": "item/",
                        "source": "item"
                    }
                ]
            }"#,
        );
        write_json(
            &assets.join("atlases").join("blocks.json"),
            r#"{
                "sources": [
                    {
                        "type": "minecraft:directory",
                        "prefix": "block/",
                        "source": "block"
                    }
                ]
            }"#,
        );
    }

    fn write_elytra_damage_condition_fixture(
        root: &Path,
        property: &str,
        true_model: &str,
        true_color: &[u8],
    ) {
        let assets = assets_dir(root);
        write_item_atlases(&assets);
        write_json(
            &root
                .join("sources")
                .join(bbb_pack::MC_VERSION)
                .join("net")
                .join("minecraft")
                .join("world")
                .join("item")
                .join("Items.java"),
            r#"public class Items {
                public static final Item ELYTRA = registerItem(
                    "elytra",
                    Item::new,
                    new Item.Properties().durability(432)
                );
            }"#,
        );
        write_json(
            &assets.join("items").join("elytra.json"),
            &format!(
                r#"{{
                "model": {{
                    "type": "minecraft:condition",
                    "property": "{property}",
                    "on_false": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/elytra"
                    }},
                    "on_true": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/{true_model}"
                    }}
                }}
            }}"#
            ),
        );
        write_json(
            &assets.join("models").join("item").join("elytra.json"),
            r#"{
                "textures": {
                    "layer0": "minecraft:item/elytra"
                }
            }"#,
        );
        write_json(
            &assets
                .join("models")
                .join("item")
                .join(format!("{true_model}.json")),
            &format!(
                r#"{{
                "textures": {{
                    "layer0": "minecraft:item/{true_model}"
                }}
            }}"#
            ),
        );
        write_test_rgba_png(
            &assets.join("textures").join("item").join("elytra.png"),
            1,
            1,
            &[40, 80, 120, 255],
        );
        write_test_rgba_png(
            &assets
                .join("textures")
                .join("item")
                .join(format!("{true_model}.png")),
            1,
            1,
            true_color,
        );
    }

    fn write_wolf_armor_has_component_fixture(root: &Path) {
        let assets = assets_dir(root);
        write_item_atlases(&assets);
        write_single_item_registry_source(root, "wolf_armor");
        write_json(
            &assets.join("items").join("wolf_armor.json"),
            r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:has_component",
                    "component": "minecraft:dyed_color",
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/wolf_armor"
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/wolf_armor_dyed",
                        "tints": [
                            { "type": "minecraft:dye", "default": 0 }
                        ]
                    }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "wolf_armor", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "wolf_armor_dyed", &[120, 80, 40, 255]);
    }

    fn write_compass_has_component_fixture(root: &Path) {
        let assets = assets_dir(root);
        write_item_atlases(&assets);
        write_single_item_registry_source(root, "compass");
        write_json(
            &assets.join("items").join("compass.json"),
            r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:has_component",
                    "component": "minecraft:lodestone_tracker",
                    "on_true": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:compass",
                        "target": "lodestone",
                        "scale": 32.0,
                        "entries": [
                            {
                                "threshold": 0.0,
                                "model": {
                                    "type": "minecraft:model",
                                    "model": "minecraft:item/compass_lodestone"
                                }
                            }
                        ]
                    },
                    "on_false": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:compass",
                        "target": "spawn",
                        "scale": 32.0,
                        "entries": [
                            {
                                "threshold": 0.0,
                                "model": {
                                    "type": "minecraft:model",
                                    "model": "minecraft:item/compass"
                                }
                            }
                        ]
                    }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "compass", &[40, 120, 80, 255]);
        write_flat_item_model_and_texture(&assets, "compass_lodestone", &[120, 40, 80, 255]);
    }

    fn write_bundle_selected_item_fixture(root: &Path) {
        let assets = assets_dir(root);
        write_item_atlases(&assets);
        write_item_registry_source(root, &["bundle", "apple"]);
        write_json(
            &assets.join("items").join("bundle.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:display_context",
                    "cases": [
                        {
                            "when": "gui",
                            "model": {
                                "type": "minecraft:condition",
                                "property": "minecraft:bundle/has_selected_item",
                                "on_false": {
                                    "type": "minecraft:model",
                                    "model": "minecraft:item/bundle"
                                },
                                "on_true": {
                                    "type": "minecraft:composite",
                                    "models": [
                                        {
                                            "type": "minecraft:model",
                                            "model": "minecraft:item/bundle_open_back"
                                        },
                                        {
                                            "type": "minecraft:bundle/selected_item"
                                        },
                                        {
                                            "type": "minecraft:model",
                                            "model": "minecraft:item/bundle_open_front"
                                        }
                                    ]
                                }
                            }
                        }
                    ],
                    "fallback": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/bundle"
                    }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "bundle", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "bundle_open_back", &[120, 80, 40, 255]);
        write_flat_item_model_and_texture(&assets, "bundle_open_front", &[80, 120, 40, 255]);
        write_flat_item_model_and_texture(&assets, "apple", &[200, 40, 40, 255]);
    }

    fn write_single_item_registry_source(root: &Path, item_id: &str) {
        write_item_registry_source(root, &[item_id]);
    }

    fn write_item_registry_source(root: &Path, item_ids: &[&str]) {
        let declarations = item_ids
            .iter()
            .map(|item_id| {
                let constant = item_id.to_ascii_uppercase();
                format!("public static final Item {constant} = registerItem(\"{item_id}\");")
            })
            .collect::<Vec<_>>()
            .join("\n");
        write_json(
            &root
                .join("sources")
                .join(bbb_pack::MC_VERSION)
                .join("net")
                .join("minecraft")
                .join("world")
                .join("item")
                .join("Items.java"),
            &format!(
                r#"public class Items {{
                {declarations}
            }}"#,
            ),
        );
    }

    fn write_flat_item_model_and_texture(assets: &Path, model_id: &str, color: &[u8]) {
        write_json(
            &assets
                .join("models")
                .join("item")
                .join(format!("{model_id}.json")),
            &format!(
                r#"{{
                "textures": {{
                    "layer0": "minecraft:item/{model_id}"
                }}
            }}"#
            ),
        );
        write_test_rgba_png(
            &assets
                .join("textures")
                .join("item")
                .join(format!("{model_id}.png")),
            1,
            1,
            color,
        );
    }

    fn write_item_registry_sources(root: &Path) {
        write_json(
            &root
                .join("sources")
                .join(bbb_pack::MC_VERSION)
                .join("net")
                .join("minecraft")
                .join("world")
                .join("item")
                .join("Items.java"),
            r#"public class Items {
                public static final Item TEST_COMBO = registerItem("test_combo", new Item.Properties().equippable(EquipmentSlot.CHEST));
            }"#,
        );
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_test_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::save_buffer(path, rgba, width, height, image::ColorType::Rgba8).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("bbb-native-{label}-{nanos}-{id}"))
    }
}
