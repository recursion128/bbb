use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
    path::PathBuf,
};

use anyhow::{Context, Result};
use bbb_pack::{
    AtlasImage, AtlasLayout, AtlasPacker, AtlasSprite, BlockModelDisplayContext,
    BlockModelDisplayTransform, BlockModelDisplayTransforms, EquipmentAssetCatalog,
    EquipmentLayerType, FreezeImmuneWearableCatalog, FurnaceFuelCatalog,
    ItemAttackRange as PackItemAttackRange, ItemCuboidModel, ItemCuboidModelCatalog,
    ItemCuboidModelSet, ItemCuboidTextureImageCatalog, ItemEquipmentSlot as PackItemEquipmentSlot,
    ItemMiningProfile as PackItemMiningProfile, ItemMiningRule as PackItemMiningRule,
    ItemModelCatalog, ItemModelDefinition, ItemMountBodyArmorKind as PackItemMountBodyArmorKind,
    ItemRegistryCatalog, ItemTintSource, ItemUseEffects as PackItemUseEffects, LanguageCatalog,
    PackResourceStack, PackRoots, ResourceLocation, SpriteImage, TerrainColorMaps,
    DEFAULT_LANGUAGE_CODE,
};
use bbb_protocol::packets::{
    DataComponentPatchSummary, ItemRaritySummary, ItemStackSummary, ItemStackTemplateSummary,
    ResolvableProfileSummary, ResourceTextureSummary,
};
use bbb_renderer::{
    DynamicPlayerSkinImage, DynamicPlayerTextureImage, EntityCustomHeadSkull,
    EntityDefaultPlayerSkin, EntityDynamicPlayerSkin, EntityDynamicPlayerSkinStatus,
    EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind, EntityEquipmentLayerTexture,
    EntityModelTextureRef, EntityPlayerSkin, EntityPlayerSkinModel, HudAsciiGlyph, HudUvRect,
    ItemFrameMapDecorationTexture, ItemSpriteRect, SpriteAlphaMask, HUD_ASCII_GLYPH_COUNT,
};
use bbb_world::{
    ArmorMaterialKind as WorldArmorMaterialKind, ItemAttackRange as WorldItemAttackRange,
    ItemEquipmentSlot as WorldItemEquipmentSlot, ItemUseEffects as WorldItemUseEffects,
    LlamaBodyDecorColor as WorldLlamaBodyDecorColor, MountArmorSlotKind as WorldMountArmorSlotKind,
    WorldItemMiningProfile, WorldItemMiningRule,
};

use crate::{
    profile_resolver::{AsyncProfileResolutionRuntime, HttpGameProfileFetcher},
    skin_runtime::{
        AsyncDynamicPlayerSkinRuntime, AsyncDynamicPlayerTextureRuntime, DynamicPlayerTextureKind,
        HttpSkinPngFetcher,
    },
};

mod icon_model;
mod profile_skin;

use icon_model::{
    contains_runtime_condition, item_icon_model_ref_for_definition, CrossbowChargeType,
    IconResolveContext, ItemIconModel, ItemIconModelRef,
};
pub(crate) use profile_skin::default_player_skin_for_profile_id;
use profile_skin::ProfileSkinCache;
use profile_skin::{entity_player_skin_model, profile_default_player_skin, profile_texture_handle};

use crate::ascii_font::{hud_ascii_atlas_from_image, load_ascii_font_texture};

const FIREWORK_ROCKET_ITEM_ID: &str = "minecraft:firework_rocket";
const BOW_ITEM_ID: &str = "minecraft:bow";
const BRUSH_ITEM_ID: &str = "minecraft:brush";
const CROSSBOW_ITEM_ID: &str = "minecraft:crossbow";
const SPYGLASS_ITEM_ID: &str = "minecraft:spyglass";
const TRIDENT_ITEM_ID: &str = "minecraft:trident";
const ENDER_EYE_ITEM_ID: &str = "minecraft:ender_eye";
const VANILLA_LONG_USE_DURATION_TICKS: i32 = 72_000;
const VANILLA_BRUSH_USE_DURATION_TICKS: i32 = 200;
const VANILLA_SPYGLASS_USE_DURATION_TICKS: i32 = 1_200;
const VANILLA_ENDER_EYE_USE_DURATION_TICKS: i32 = 20;
const VANILLA_CROSSBOW_CHARGE_DURATION_TICKS: i32 = 25;
const VANILLA_BLOCKS_ATTACKS_COMPONENT_ID: i32 = 37;
const VANILLA_KINETIC_WEAPON_COMPONENT_ID: i32 = 39;
const ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/equipment/wings/elytra.png",
    size: [64, 32],
};

fn load_map_decoration_textures(roots: &PackRoots) -> Result<Vec<ItemFrameMapDecorationTexture>> {
    let textures = roots
        .load_atlas_texture_images("map_decorations")?
        .into_iter()
        .filter(|image| image.id != "minecraft:missingno")
        .map(|image| ItemFrameMapDecorationTexture {
            sprite_id: image.id,
            width: image.width,
            height: image.height,
            rgba: image.rgba,
        })
        .collect();
    Ok(textures)
}

fn load_map_text_glyphs(roots: &PackRoots) -> Result<[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT]> {
    let ascii_font = load_ascii_font_texture(roots)?;
    Ok(hud_ascii_atlas_from_image(&ascii_font)?.glyphs)
}

#[cfg(test)]
fn test_map_text_glyphs() -> [HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT] {
    let mut glyphs = [HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        width: 6,
        height: 8,
        advance: 6,
    }; HUD_ASCII_GLYPH_COUNT];
    glyphs[(b' ' - b' ') as usize].advance = 4;
    glyphs
}

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
const BREWING_POTION_ITEM_IDS: &[&str] = &[
    "minecraft:potion",
    "minecraft:splash_potion",
    "minecraft:lingering_potion",
    "minecraft:glass_bottle",
];
const BREWING_INGREDIENT_ITEM_IDS: &[&str] = &[
    "minecraft:gunpowder",
    "minecraft:dragon_breath",
    "minecraft:glowstone_dust",
    "minecraft:redstone",
    "minecraft:nether_wart",
    "minecraft:breeze_rod",
    "minecraft:slime_block",
    "minecraft:stone",
    "minecraft:cobweb",
    "minecraft:golden_carrot",
    "minecraft:fermented_spider_eye",
    "minecraft:magma_cream",
    "minecraft:rabbit_foot",
    "minecraft:turtle_helmet",
    "minecraft:sugar",
    "minecraft:pufferfish",
    "minecraft:glistering_melon_slice",
    "minecraft:spider_eye",
    "minecraft:ghast_tear",
    "minecraft:blaze_powder",
    "minecraft:phantom_membrane",
];
const ENCHANTMENT_LAPIS_LAZULI_ITEM_IDS: &[&str] = &["minecraft:lapis_lazuli"];
const CARTOGRAPHY_ADDITIONAL_ITEM_IDS: &[&str] =
    &["minecraft:paper", "minecraft:map", "minecraft:glass_pane"];
const RECIPE_SPECIFIC_CRAFTING_REMAINDER_ITEM_IDS: &[&str] = &[
    "minecraft:white_banner",
    "minecraft:orange_banner",
    "minecraft:magenta_banner",
    "minecraft:light_blue_banner",
    "minecraft:yellow_banner",
    "minecraft:lime_banner",
    "minecraft:pink_banner",
    "minecraft:gray_banner",
    "minecraft:light_gray_banner",
    "minecraft:cyan_banner",
    "minecraft:purple_banner",
    "minecraft:blue_banner",
    "minecraft:brown_banner",
    "minecraft:green_banner",
    "minecraft:red_banner",
    "minecraft:black_banner",
    "minecraft:written_book",
];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NativeItemTooltipLine {
    pub(crate) text: String,
    pub(crate) tint: [f32; 4],
}

#[derive(Debug, Clone)]
pub(crate) struct NativeDynamicPlayerSkinDownload {
    pub(crate) url: String,
    pub(crate) skin: Option<DynamicPlayerSkinImage>,
}

#[derive(Debug, Default)]
struct LocalDynamicPlayerSkinCache {
    entries: HashMap<String, LocalDynamicPlayerSkinEntry>,
    pending_uploads: Vec<NativeDynamicPlayerSkinDownload>,
}

#[derive(Debug, Clone, Copy)]
enum LocalDynamicPlayerSkinEntry {
    Ready(EntityDynamicPlayerSkin),
    Failed,
}

impl LocalDynamicPlayerSkinEntry {
    const fn skin(self) -> Option<EntityPlayerSkin> {
        match self {
            Self::Ready(skin) => Some(EntityPlayerSkin::Dynamic(skin)),
            Self::Failed => None,
        }
    }
}

impl LocalDynamicPlayerSkinCache {
    fn skin_for_patch(
        &mut self,
        resources: &PackResourceStack,
        profile: &ResolvableProfileSummary,
        patch: &ResourceTextureSummary,
        fallback: EntityDefaultPlayerSkin,
    ) -> Option<EntityPlayerSkin> {
        let source_id = local_player_skin_source_id(&patch.texture_path);
        if let Some(entry) = self.entries.get(&source_id) {
            return entry.skin();
        }

        let model = profile_skin_model(profile, fallback);
        match load_local_dynamic_player_skin(resources, &patch.texture_path, &source_id) {
            Ok(image) => {
                let skin = EntityDynamicPlayerSkin {
                    handle: image.handle,
                    fallback,
                    model,
                    status: EntityDynamicPlayerSkinStatus::Ready,
                };
                self.pending_uploads.push(NativeDynamicPlayerSkinDownload {
                    url: source_id.clone(),
                    skin: Some(image),
                });
                self.entries
                    .insert(source_id, LocalDynamicPlayerSkinEntry::Ready(skin));
                Some(EntityPlayerSkin::Dynamic(skin))
            }
            Err(err) => {
                tracing::warn!(
                    ?err,
                    texture_path = patch.texture_path.as_str(),
                    "failed to load player profile body resource texture patch"
                );
                self.entries
                    .insert(source_id, LocalDynamicPlayerSkinEntry::Failed);
                None
            }
        }
    }

    fn drain_results(&mut self) -> Vec<NativeDynamicPlayerSkinDownload> {
        std::mem::take(&mut self.pending_uploads)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NativeDynamicPlayerTextureDownload {
    pub(crate) kind: DynamicPlayerTextureKind,
    pub(crate) url: String,
    pub(crate) texture: Option<DynamicPlayerTextureImage>,
}

#[derive(Debug, Default)]
struct LocalDynamicPlayerTextureCache {
    entries: HashMap<String, LocalDynamicPlayerTextureEntry>,
    pending_uploads: Vec<NativeDynamicPlayerTextureDownload>,
}

#[derive(Debug, Clone, Copy)]
enum LocalDynamicPlayerTextureEntry {
    Ready(EntityDynamicPlayerTexture),
    Failed,
}

impl LocalDynamicPlayerTextureEntry {
    const fn texture(self) -> Option<EntityDynamicPlayerTexture> {
        match self {
            Self::Ready(texture) => Some(texture),
            Self::Failed => None,
        }
    }
}

impl LocalDynamicPlayerTextureCache {
    fn texture_for_patch(
        &mut self,
        resources: &PackResourceStack,
        kind: EntityDynamicPlayerTextureKind,
        patch: &ResourceTextureSummary,
    ) -> Option<EntityDynamicPlayerTexture> {
        let source_id = local_profile_texture_source_id(kind, &patch.texture_path);
        if let Some(entry) = self.entries.get(&source_id) {
            return entry.texture();
        }

        match load_local_dynamic_player_texture(resources, kind, &patch.texture_path, &source_id) {
            Ok((texture, image)) => {
                self.pending_uploads
                    .push(NativeDynamicPlayerTextureDownload {
                        kind: dynamic_player_texture_download_kind(kind),
                        url: source_id.clone(),
                        texture: Some(image),
                    });
                self.entries
                    .insert(source_id, LocalDynamicPlayerTextureEntry::Ready(texture));
                Some(texture)
            }
            Err(err) => {
                tracing::warn!(
                    ?err,
                    texture_path = patch.texture_path.as_str(),
                    "failed to load player profile resource texture patch"
                );
                self.entries
                    .insert(source_id, LocalDynamicPlayerTextureEntry::Failed);
                None
            }
        }
    }

    fn drain_results(&mut self) -> Vec<NativeDynamicPlayerTextureDownload> {
        std::mem::take(&mut self.pending_uploads)
    }
}

#[derive(Debug)]
pub(crate) struct NativeItemRuntime {
    item_definition_count: usize,
    item_registry_count: usize,
    resolved_model_count: usize,
    missing_model_ids: BTreeSet<String>,
    missing_texture_ids: BTreeSet<String>,
    furnace_fuel_item_ids: BTreeSet<i32>,
    freeze_immune_wearable_item_ids: BTreeSet<i32>,
    powder_snow_walkable_foot_item_ids: BTreeSet<i32>,
    recipe_specific_crafting_remainder_item_ids: BTreeSet<i32>,
    item_icon_models: HashMap<String, ItemIconModel>,
    item_display_transforms: HashMap<String, BlockModelDisplayTransforms>,
    registry: Option<ItemRegistryCatalog>,
    equipment_assets: EquipmentAssetCatalog,
    language: LanguageCatalog,
    map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    map_text_glyphs: Option<[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT]>,
    textures: ItemTextureState,
    profile_resolutions: RefCell<Option<AsyncProfileResolutionRuntime>>,
    dynamic_skins: RefCell<Option<AsyncDynamicPlayerSkinRuntime>>,
    dynamic_textures: RefCell<Option<AsyncDynamicPlayerTextureRuntime>>,
    profile_texture_resources: PackResourceStack,
    local_dynamic_skins: RefCell<LocalDynamicPlayerSkinCache>,
    local_dynamic_textures: RefCell<LocalDynamicPlayerTextureCache>,
    profile_skins: RefCell<ProfileSkinCache>,
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
        let recipe_specific_crafting_remainder_item_ids = registry
            .as_ref()
            .map(recipe_specific_crafting_remainder_item_ids)
            .unwrap_or_default();
        let equipment_assets = roots
            .load_equipment_asset_catalog()
            .context("load equipment asset catalog")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native equipment asset catalog");
                err
            })
            .unwrap_or_default();
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
        let map_decoration_textures = load_map_decoration_textures(roots)
            .context("load map decoration textures")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native map decoration textures");
                err
            })
            .unwrap_or_default();
        let map_text_glyphs = load_map_text_glyphs(roots)
            .context("load map label font metrics")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native map label text");
                err
            })
            .ok();
        Self::from_loaded(
            item_models,
            cuboid_models,
            texture_images,
            registry,
            colormaps,
            furnace_fuel_item_ids,
            freeze_immune_wearable_item_ids,
            powder_snow_walkable_foot_item_ids,
            recipe_specific_crafting_remainder_item_ids,
            equipment_assets,
            language,
            map_decoration_textures,
            map_text_glyphs,
            roots.resource_stack(),
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
        recipe_specific_crafting_remainder_item_ids: BTreeSet<i32>,
        equipment_assets: EquipmentAssetCatalog,
        language: LanguageCatalog,
        map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
        map_text_glyphs: Option<[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT]>,
        profile_texture_resources: PackResourceStack,
    ) -> Result<Self> {
        let mut texture_ids = BTreeSet::new();
        let mut item_icon_model_refs = HashMap::new();
        let mut item_display_transforms = HashMap::new();
        let mut missing_model_ids = BTreeSet::new();
        let mut missing_texture_ids = BTreeSet::new();
        let mut resolved_model_count = 0usize;

        for (item_id, definition) in item_models.definitions() {
            let models = cuboid_models.models_for_definition(definition);
            resolved_model_count += models.models.len();
            // Retain the item model's display transforms (the same across its conditional variants,
            // which share a parent like `item/handheld` / `item/generated` / `block/block`) so held
            // items, frames, and the GUI can place the 3D model the way vanilla `ItemTransform` does.
            if let Some(model) = models.models.first() {
                item_display_transforms.insert(item_id.clone(), model.display_transforms.clone());
            }
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
            recipe_specific_crafting_remainder_item_ids,
            item_icon_models,
            item_display_transforms,
            registry,
            equipment_assets,
            language,
            map_decoration_textures,
            map_text_glyphs,
            textures,
            profile_resolutions: RefCell::default(),
            dynamic_skins: RefCell::default(),
            dynamic_textures: RefCell::default(),
            profile_texture_resources,
            local_dynamic_skins: RefCell::default(),
            local_dynamic_textures: RefCell::default(),
            profile_skins: RefCell::default(),
        })
    }

    #[cfg(test)]
    pub(crate) fn empty_for_test() -> Self {
        let missing = SpriteImage::new(MISSING_TEXTURE_ID, 1, 1, vec![0xff, 0x00, 0xff, 0xff])
            .expect("test missing texture image is valid");
        Self {
            item_definition_count: 0,
            item_registry_count: 0,
            resolved_model_count: 0,
            missing_model_ids: BTreeSet::new(),
            missing_texture_ids: BTreeSet::new(),
            furnace_fuel_item_ids: BTreeSet::new(),
            freeze_immune_wearable_item_ids: BTreeSet::new(),
            powder_snow_walkable_foot_item_ids: BTreeSet::new(),
            recipe_specific_crafting_remainder_item_ids: BTreeSet::new(),
            item_icon_models: HashMap::new(),
            item_display_transforms: HashMap::new(),
            registry: None,
            equipment_assets: EquipmentAssetCatalog::default(),
            language: LanguageCatalog::from_json_bytes(b"{}").expect("empty test language"),
            map_decoration_textures: Vec::new(),
            map_text_glyphs: Some(test_map_text_glyphs()),
            textures: ItemTextureState::from_images(vec![missing])
                .expect("test item texture atlas is valid"),
            profile_resolutions: RefCell::default(),
            dynamic_skins: RefCell::default(),
            dynamic_textures: RefCell::default(),
            profile_texture_resources: PackResourceStack::default(),
            local_dynamic_skins: RefCell::default(),
            local_dynamic_textures: RefCell::default(),
            profile_skins: RefCell::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test_with_registry_and_equipment_assets(
        registry: ItemRegistryCatalog,
        equipment_assets: EquipmentAssetCatalog,
    ) -> Self {
        let mut runtime = Self::empty_for_test();
        runtime.registry = Some(registry);
        runtime.equipment_assets = equipment_assets;
        runtime
    }

    pub(crate) fn item_definition_count(&self) -> usize {
        self.item_definition_count
    }

    pub(crate) fn item_registry_count(&self) -> usize {
        self.item_registry_count
    }

    pub(crate) fn map_decoration_textures(&self) -> &[ItemFrameMapDecorationTexture] {
        &self.map_decoration_textures
    }

    pub(crate) fn map_text_glyphs(&self) -> Option<&[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT]> {
        self.map_text_glyphs.as_ref()
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

    pub(crate) fn item_max_damage_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        let mut max_damage = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return max_damage;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(damage) = registry.max_damage(resource_id) else {
                continue;
            };
            max_damage.insert(protocol_id as i32, damage);
        }
        max_damage
    }

    pub(crate) fn item_max_damage_count(&self) -> usize {
        self.item_max_damage_by_protocol_id().len()
    }

    pub(crate) fn item_crafting_remainders_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::crafting_remainders_by_protocol_id)
            .unwrap_or_default()
    }

    pub(crate) fn item_crafting_remainder_count(&self) -> usize {
        self.item_crafting_remainders_by_protocol_id().len()
    }

    pub(crate) fn recipe_specific_crafting_remainder_item_ids_by_protocol_id(
        &self,
    ) -> BTreeSet<i32> {
        self.recipe_specific_crafting_remainder_item_ids.clone()
    }

    pub(crate) fn recipe_specific_crafting_remainder_item_count(&self) -> usize {
        self.recipe_specific_crafting_remainder_item_ids.len()
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

    /// Item protocol id → humanoid armor material, for the `HumanoidArmorLayer` overlay: each armor
    /// item's `bbb_pack` equipment-asset name (`humanoid_armor_asset`) parsed into a world material.
    pub(crate) fn item_armor_materials_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, WorldArmorMaterialKind> {
        let mut materials = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return materials;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(asset) = registry.humanoid_armor_asset(resource_id) else {
                continue;
            };
            let Some(material) = WorldArmorMaterialKind::from_equipment_asset(asset) else {
                continue;
            };
            materials.insert(protocol_id as i32, material);
        }
        materials
    }

    pub(crate) fn item_has_humanoid_armor_asset(&self, protocol_id: i32) -> bool {
        let Some(registry) = &self.registry else {
            return false;
        };
        let Some(resource_id) = registry.resource_id(protocol_id) else {
            return false;
        };
        registry.humanoid_armor_asset(resource_id).is_some()
    }

    pub(crate) fn item_equipment_asset_has_wings_layer(&self, protocol_id: i32) -> bool {
        self.item_equipment_asset_has_layer(protocol_id, EquipmentLayerType::Wings)
    }

    pub(crate) fn item_equipment_wings_layer(
        &self,
        protocol_id: i32,
    ) -> Option<EntityEquipmentLayerTexture> {
        let layer = self
            .item_equipment_asset_layers(protocol_id, EquipmentLayerType::Wings)?
            .first()?;
        let texture = match layer.texture_location.as_str() {
            "minecraft:textures/entity/equipment/wings/elytra.png" => {
                ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF
            }
            _ => return None,
        };
        Some(EntityEquipmentLayerTexture {
            texture,
            use_player_texture: layer.use_player_texture,
        })
    }

    pub(crate) fn item_equipment_asset_has_humanoid_layer(&self, protocol_id: i32) -> bool {
        self.item_equipment_asset_has_layer(protocol_id, EquipmentLayerType::Humanoid)
    }

    fn item_equipment_asset_has_layer(
        &self,
        protocol_id: i32,
        layer_type: EquipmentLayerType,
    ) -> bool {
        self.item_equipment_asset_layers(protocol_id, layer_type)
            .is_some_and(|layers| !layers.is_empty())
    }

    fn item_equipment_asset_layers(
        &self,
        protocol_id: i32,
        layer_type: EquipmentLayerType,
    ) -> Option<&[bbb_pack::EquipmentLayer]> {
        let registry = self.registry.as_ref()?;
        let resource_id = registry.resource_id(protocol_id)?;
        let asset_id = registry.equippable_asset(resource_id)?;
        let asset = self.equipment_assets.asset(asset_id)?;
        Some(asset.layers(layer_type))
    }

    pub(crate) fn custom_head_skull_for_stack(
        &self,
        stack: &ItemStackSummary,
    ) -> Option<EntityCustomHeadSkull> {
        let registry = self.registry.as_ref()?;
        let protocol_id = stack.item_id?;
        custom_head_skull_for_resource_id(
            registry.resource_id(protocol_id)?,
            &stack.component_patch,
            &self.profile_resolutions,
            &self.dynamic_skins,
            &self.dynamic_textures,
            &self.profile_texture_resources,
            &self.local_dynamic_skins,
            &self.profile_skins,
        )
    }

    pub(crate) fn player_skin_for_profile(
        &self,
        profile: &ResolvableProfileSummary,
    ) -> EntityPlayerSkin {
        let player_skin = player_skin_for_profile(
            profile,
            &self.profile_texture_resources,
            &self.local_dynamic_skins,
            &self.profile_skins,
        );
        queue_dynamic_profile_texture_downloads(
            profile,
            player_skin,
            &self.dynamic_skins,
            &self.dynamic_textures,
        );
        player_skin
    }

    pub(crate) fn player_profile_texture_for_profile(
        &self,
        profile: &ResolvableProfileSummary,
        kind: EntityDynamicPlayerTextureKind,
    ) -> Option<EntityDynamicPlayerTexture> {
        let player_skin = player_skin_for_profile(
            profile,
            &self.profile_texture_resources,
            &self.local_dynamic_skins,
            &self.profile_skins,
        );
        queue_dynamic_profile_texture_downloads(
            profile,
            player_skin,
            &self.dynamic_skins,
            &self.dynamic_textures,
        );
        dynamic_player_texture_for_profile(
            profile,
            kind,
            &self.profile_texture_resources,
            &self.local_dynamic_textures,
        )
    }

    pub(crate) fn enable_http_profile_resolution(&self) {
        let mut profile_resolutions = self.profile_resolutions.borrow_mut();
        if profile_resolutions.is_some() {
            return;
        }
        match HttpGameProfileFetcher::new() {
            Ok(fetcher) => {
                *profile_resolutions = Some(AsyncProfileResolutionRuntime::new(fetcher));
            }
            Err(err) => {
                tracing::warn!(?err, "continuing without async profile resolution");
            }
        }
    }

    pub(crate) fn drain_profile_resolution_results(&self) -> usize {
        self.profile_resolutions
            .borrow_mut()
            .as_mut()
            .map(AsyncProfileResolutionRuntime::drain_results)
            .unwrap_or(0)
    }

    pub(crate) fn enable_http_player_skin_downloads(&self, cache_dir: impl Into<PathBuf>) {
        let cache_dir = cache_dir.into();
        let mut dynamic_skins = self.dynamic_skins.borrow_mut();
        if dynamic_skins.is_none() {
            match HttpSkinPngFetcher::new() {
                Ok(fetcher) => {
                    *dynamic_skins = Some(AsyncDynamicPlayerSkinRuntime::new(
                        cache_dir.clone(),
                        fetcher,
                    ));
                }
                Err(err) => {
                    tracing::warn!(?err, "continuing without async player skin downloads");
                }
            }
        }
        drop(dynamic_skins);

        let mut dynamic_textures = self.dynamic_textures.borrow_mut();
        if dynamic_textures.is_none() {
            match HttpSkinPngFetcher::new() {
                Ok(fetcher) => {
                    *dynamic_textures =
                        Some(AsyncDynamicPlayerTextureRuntime::new(cache_dir, fetcher));
                }
                Err(err) => {
                    tracing::warn!(
                        ?err,
                        "continuing without async player profile texture downloads"
                    );
                }
            }
        }
    }

    pub(crate) fn drain_dynamic_player_skin_download_results(
        &self,
    ) -> Vec<NativeDynamicPlayerSkinDownload> {
        let mut local_results = self.local_dynamic_skins.borrow_mut().drain_results();
        let results = self
            .dynamic_skins
            .borrow_mut()
            .as_mut()
            .map(AsyncDynamicPlayerSkinRuntime::drain_results)
            .unwrap_or_default();
        for result in &results {
            if result.skin.is_none() {
                self.profile_skins.borrow_mut().mark_failed(&result.url);
            }
        }
        local_results.extend(
            results
                .into_iter()
                .map(|result| NativeDynamicPlayerSkinDownload {
                    url: result.url,
                    skin: result.skin,
                }),
        );
        local_results
    }

    pub(crate) fn drain_dynamic_player_texture_download_results(
        &self,
    ) -> Vec<NativeDynamicPlayerTextureDownload> {
        let mut results = self.local_dynamic_textures.borrow_mut().drain_results();
        results.extend(
            self.dynamic_textures
                .borrow_mut()
                .as_mut()
                .map(AsyncDynamicPlayerTextureRuntime::drain_results)
                .unwrap_or_default()
                .into_iter()
                .map(|result| NativeDynamicPlayerTextureDownload {
                    kind: result.kind,
                    url: result.url,
                    texture: result.texture,
                }),
        );
        results
    }

    #[cfg(test)]
    fn enable_player_skin_downloads_for_test(&self, runtime: AsyncDynamicPlayerSkinRuntime) {
        *self.dynamic_skins.borrow_mut() = Some(runtime);
    }

    #[cfg(test)]
    fn enable_player_texture_downloads_for_test(&self, runtime: AsyncDynamicPlayerTextureRuntime) {
        *self.dynamic_textures.borrow_mut() = Some(runtime);
    }

    #[cfg(test)]
    fn downloaded_player_skin_count(&self) -> usize {
        self.dynamic_skins
            .borrow()
            .as_ref()
            .map(AsyncDynamicPlayerSkinRuntime::downloaded_skin_count)
            .unwrap_or(0)
    }

    #[cfg(test)]
    fn downloaded_player_texture_count(&self) -> usize {
        self.dynamic_textures
            .borrow()
            .as_ref()
            .map(AsyncDynamicPlayerTextureRuntime::downloaded_texture_count)
            .unwrap_or(0)
    }

    pub(crate) fn mark_profile_skin_resolved(&self, url: &str, texture_handle: u64) {
        self.profile_skins
            .borrow_mut()
            .mark_resolved(url, texture_handle);
    }

    pub(crate) fn mark_profile_skin_failed(&self, url: &str) {
        self.profile_skins.borrow_mut().mark_failed(url);
    }

    pub(crate) fn mount_body_armor_kinds_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, WorldMountArmorSlotKind> {
        self.registry
            .as_ref()
            .map(|registry| {
                registry
                    .mount_body_armor_kinds_by_protocol_id()
                    .into_iter()
                    .map(|(item_id, kind)| (item_id, world_mount_armor_slot_kind(kind)))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn mount_body_armor_kind_count(&self) -> usize {
        self.mount_body_armor_kinds_by_protocol_id().len()
    }

    pub(crate) fn llama_body_decor_colors_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, WorldLlamaBodyDecorColor> {
        let mut colors = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return colors;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            if registry.mount_body_armor_kind(resource_id)
                != Some(PackItemMountBodyArmorKind::Llama)
            {
                continue;
            }
            let Some(color) = llama_body_decor_color_from_item_id(resource_id) else {
                continue;
            };
            colors.insert(protocol_id as i32, color);
        }
        colors
    }

    pub(crate) fn llama_body_decor_color_count(&self) -> usize {
        self.llama_body_decor_colors_by_protocol_id().len()
    }

    pub(crate) fn nautilus_body_armor_materials_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, WorldArmorMaterialKind> {
        let mut materials = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return materials;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            if registry.mount_body_armor_kind(resource_id)
                != Some(PackItemMountBodyArmorKind::Nautilus)
            {
                continue;
            }
            let Some(asset) = registry.mount_body_armor_asset(resource_id) else {
                continue;
            };
            let Some(material) = nautilus_body_armor_material_from_asset(asset) else {
                continue;
            };
            materials.insert(protocol_id as i32, material);
        }
        materials
    }

    pub(crate) fn nautilus_body_armor_material_count(&self) -> usize {
        self.nautilus_body_armor_materials_by_protocol_id().len()
    }

    pub(crate) fn horse_body_armor_materials_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, WorldArmorMaterialKind> {
        let mut materials = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return materials;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            if registry.mount_body_armor_kind(resource_id)
                != Some(PackItemMountBodyArmorKind::Horse)
            {
                continue;
            }
            let Some(asset) = registry.mount_body_armor_asset(resource_id) else {
                continue;
            };
            let Some(material) = horse_body_armor_material_from_asset(asset) else {
                continue;
            };
            materials.insert(protocol_id as i32, material);
        }
        materials
    }

    pub(crate) fn horse_body_armor_material_count(&self) -> usize {
        self.horse_body_armor_materials_by_protocol_id().len()
    }

    pub(crate) fn wolf_body_armor_materials_by_protocol_id(
        &self,
    ) -> BTreeMap<i32, WorldArmorMaterialKind> {
        let mut materials = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return materials;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(asset) = registry.equippable_asset(resource_id) else {
                continue;
            };
            if self
                .equipment_assets
                .asset(asset)
                .is_none_or(|asset| asset.layers(EquipmentLayerType::WolfBody).is_empty())
            {
                continue;
            }
            let Some(material) = wolf_body_armor_material_from_asset(asset) else {
                continue;
            };
            materials.insert(protocol_id as i32, material);
        }
        materials
    }

    pub(crate) fn wolf_body_armor_material_count(&self) -> usize {
        self.wolf_body_armor_materials_by_protocol_id().len()
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

    pub(crate) fn default_damageable_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(ItemRegistryCatalog::max_damage_protocol_ids)
            .unwrap_or_default()
    }

    pub(crate) fn default_damageable_item_count(&self) -> usize {
        self.default_damageable_item_ids_by_protocol_id().len()
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

    pub(crate) fn item_swing_animation_durations_by_protocol_id(&self) -> BTreeMap<i32, i32> {
        let mut durations = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return durations;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(duration) = registry.default_swing_animation_duration(resource_id) else {
                continue;
            };
            durations.insert(protocol_id as i32, duration);
        }
        durations
    }

    pub(crate) fn item_swing_animation_duration_count(&self) -> usize {
        self.item_swing_animation_durations_by_protocol_id().len()
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

    pub(crate) fn brewing_potion_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(|registry| protocol_ids_for_resource_ids(registry, BREWING_POTION_ITEM_IDS))
            .unwrap_or_default()
    }

    pub(crate) fn brewing_potion_item_count(&self) -> usize {
        self.brewing_potion_item_ids_by_protocol_id().len()
    }

    pub(crate) fn brewing_ingredient_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(|registry| protocol_ids_for_resource_ids(registry, BREWING_INGREDIENT_ITEM_IDS))
            .unwrap_or_default()
    }

    pub(crate) fn brewing_ingredient_item_count(&self) -> usize {
        self.brewing_ingredient_item_ids_by_protocol_id().len()
    }

    pub(crate) fn enchantment_lapis_lazuli_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        self.registry
            .as_ref()
            .map(|registry| {
                protocol_ids_for_resource_ids(registry, ENCHANTMENT_LAPIS_LAZULI_ITEM_IDS)
            })
            .unwrap_or_default()
    }

    pub(crate) fn enchantment_lapis_lazuli_item_count(&self) -> usize {
        self.enchantment_lapis_lazuli_item_ids_by_protocol_id()
            .len()
    }

    pub(crate) fn cartography_additional_item_ids_by_protocol_id(&self) -> BTreeSet<i32> {
        let Some(registry) = &self.registry else {
            return BTreeSet::new();
        };
        let item_ids = protocol_ids_for_resource_ids(registry, CARTOGRAPHY_ADDITIONAL_ITEM_IDS);
        if item_ids.len() == CARTOGRAPHY_ADDITIONAL_ITEM_IDS.len() {
            item_ids
        } else {
            BTreeSet::new()
        }
    }

    pub(crate) fn cartography_additional_item_count(&self) -> usize {
        self.cartography_additional_item_ids_by_protocol_id().len()
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
        let default_max_stack_size_for_item =
            |item_id| self.default_max_stack_size_for_protocol_id(item_id);
        Some(
            self.item_icon_models
                .get(item_id)
                .and_then(|model| {
                    model
                        .icon_layers(IconResolveContext {
                            component_patch: None,
                            stack_count: 1,
                            default_max_stack_size: self
                                .registry
                                .as_ref()
                                .and_then(|registry| registry.max_stack_size(item_id)),
                            default_max_damage: None,
                            bundle_selected_item_index: None,
                            using_item: false,
                            use_context: ItemModelUseContext::inactive(),
                            cooldown_progress: 0.0,
                            crossbow_charge: CrossbowChargeType::None,
                            main_hand_left: None,
                            context_dimension: None,
                            context_entity_type: None,
                            default_max_stack_size_for_item: Some(&default_max_stack_size_for_item),
                            trim_material_keys: None,
                        })
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

    /// The resource id (e.g. `minecraft:stone`) for an item protocol id, via the item registry. Used to
    /// map a dropped item to the block of the same id for 3D block-item rendering.
    pub(crate) fn item_resource_id(&self, protocol_id: i32) -> Option<&str> {
        self.registry.as_ref()?.resource_id(protocol_id)
    }

    /// The item's own model display transform for a context (vanilla `ItemTransform`), retained from the
    /// resolved item cuboid model. `None` if the item has no registry entry or no resolved model (the
    /// caller then falls back to the parent-model default). Used to place the 3D model in hand / frame /
    /// GUI exactly as vanilla's `model.applyTransform`.
    pub(crate) fn item_display_transform(
        &self,
        protocol_id: i32,
        context: BlockModelDisplayContext,
    ) -> Option<BlockModelDisplayTransform> {
        let item_id = self.registry.as_ref()?.resource_id(protocol_id)?;
        Some(self.item_display_transforms.get(item_id)?.get(context))
    }

    /// Generated item layers for a non-living stack consumer that still has a
    /// level-backed dynamic trim registry, such as dropped items (`GROUND`) and
    /// item frames (`FIXED`). Vanilla `TrimMaterialProperty.get` reads only the
    /// stack's `minecraft:trim` component and the trim material registry key.
    pub(crate) fn generated_item_layers_for_stack_with_trim_materials(
        &self,
        stack: &ItemStackSummary,
        trim_material_keys: Option<&[String]>,
    ) -> Vec<GeneratedItemLayer> {
        self.generated_item_layers_for_stack_with_context(
            stack,
            None,
            false,
            ItemModelUseContext::inactive(),
            trim_material_keys,
        )
    }

    /// Generated item layers for an entity-owned stack. Vanilla `MainHand.get`
    /// returns null without a living owner; held-item paths pass the owner's
    /// main arm so `minecraft:main_hand` select cases can resolve. Vanilla
    /// `IsUsingItem.get` is true only for the stack currently returned by
    /// `owner.getUseItem()`, so held-item paths also pass whether this hand is
    /// the active use hand.
    pub(crate) fn generated_item_layers_for_stack_with_owner_context(
        &self,
        stack: &ItemStackSummary,
        owner_main_hand_left: Option<bool>,
        using_item: bool,
        use_context: ItemModelUseContext,
    ) -> Vec<GeneratedItemLayer> {
        self.generated_item_layers_for_stack_with_context(
            stack,
            owner_main_hand_left,
            using_item,
            use_context,
            None,
        )
    }

    fn generated_item_layers_for_stack_with_context(
        &self,
        stack: &ItemStackSummary,
        owner_main_hand_left: Option<bool>,
        using_item: bool,
        use_context: ItemModelUseContext,
        trim_material_keys: Option<&[String]>,
    ) -> Vec<GeneratedItemLayer> {
        let Some(icon) = self.icon_for_stack_with_context_and_use_context(
            stack,
            None,
            using_item,
            use_context,
            0.0,
            trim_material_keys,
            owner_main_hand_left,
            None,
            None,
        ) else {
            return Vec::new();
        };
        icon.layers
            .into_iter()
            .filter_map(|layer| {
                let mask = self.textures.alpha_mask_for_uv(layer.uv)?;
                Some(GeneratedItemLayer {
                    mask,
                    rect: ItemSpriteRect {
                        min: layer.uv.min,
                        max: layer.uv.max,
                    },
                    tint: layer.tint,
                })
            })
            .collect()
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
        self.icon_for_stack_with_context(
            stack,
            bundle_selected_item_index,
            using_item,
            0.0,
            None,
            None,
            None,
            None,
        )
    }

    /// Resolves a stack's icon with GUI/HUD context: bundle selected item,
    /// local using-item state, `minecraft:trim_material` registry keys, and an
    /// optional living-owner main arm / entity type for `minecraft:main_hand`
    /// / `minecraft:context_entity_type` plus the current dimension for
    /// `minecraft:context_dimension`.
    pub(crate) fn icon_for_stack_with_context(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_context_and_use_context(
            stack,
            bundle_selected_item_index,
            using_item,
            ItemModelUseContext::inactive(),
            cooldown_progress,
            trim_material_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
        )
    }

    pub(crate) fn icon_for_stack_with_context_and_use_context(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_model_context(
            stack,
            bundle_selected_item_index,
            using_item,
            use_context,
            cooldown_progress,
            trim_material_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
        )
    }

    pub(crate) fn icon_for_stack_with_owner_main_hand(
        &self,
        stack: &ItemStackSummary,
        owner_main_hand_left: Option<bool>,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_owner_context(stack, owner_main_hand_left, false)
    }

    pub(crate) fn icon_for_stack_with_owner_context(
        &self,
        stack: &ItemStackSummary,
        owner_main_hand_left: Option<bool>,
        using_item: bool,
    ) -> Option<ItemAtlasIcon> {
        self.icon_for_stack_with_model_context(
            stack,
            None,
            using_item,
            ItemModelUseContext::inactive(),
            0.0,
            None,
            owner_main_hand_left,
            None,
            None,
        )
    }

    fn icon_for_stack_with_model_context(
        &self,
        stack: &ItemStackSummary,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
    ) -> Option<ItemAtlasIcon> {
        let item_id = self.registry.as_ref()?.resource_id(stack.item_id?)?;
        self.icon_for_resource_id(
            item_id,
            stack.count,
            Some(&stack.component_patch),
            bundle_selected_item_index,
            using_item,
            use_context,
            cooldown_progress,
            trim_material_keys,
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
        )
    }

    #[cfg(test)]
    pub(crate) fn icon_for_protocol_id(&self, protocol_id: i32) -> Option<ItemAtlasIcon> {
        let item_id = self.registry.as_ref()?.resource_id(protocol_id)?;
        self.icon_for_resource_id(
            item_id,
            1,
            None,
            None,
            false,
            ItemModelUseContext::inactive(),
            0.0,
            None,
            None,
            None,
            None,
        )
    }

    fn icon_for_resource_id(
        &self,
        item_id: &str,
        stack_count: i32,
        component_patch: Option<&DataComponentPatchSummary>,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
        use_context: ItemModelUseContext,
        cooldown_progress: f32,
        trim_material_keys: Option<&[String]>,
        owner_main_hand_left: Option<bool>,
        context_entity_type: Option<&str>,
        context_dimension: Option<&str>,
    ) -> Option<ItemAtlasIcon> {
        let default_max_damage = self
            .registry
            .as_ref()
            .and_then(|registry| registry.max_damage(item_id));
        let default_max_stack_size = self
            .registry
            .as_ref()
            .and_then(|registry| registry.max_stack_size(item_id));
        let default_max_stack_size_for_item =
            |item_id| self.default_max_stack_size_for_protocol_id(item_id);
        let context = IconResolveContext {
            component_patch,
            stack_count,
            default_max_stack_size,
            default_max_damage,
            bundle_selected_item_index,
            using_item,
            use_context,
            cooldown_progress,
            crossbow_charge: self.crossbow_charge_for(component_patch),
            main_hand_left: owner_main_hand_left,
            context_dimension,
            context_entity_type,
            default_max_stack_size_for_item: Some(&default_max_stack_size_for_item),
            trim_material_keys,
        };
        let layers = self
            .item_icon_models
            .get(item_id)
            .map(|model| self.icon_layers_for_model(model, context, 0))
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
        context: IconResolveContext<'_>,
        depth: usize,
    ) -> Vec<ItemIconTextureLayer> {
        if depth >= ITEM_ICON_RECURSION_LIMIT {
            return Vec::new();
        }
        let mut resolve_bundle_selected_item =
            || self.bundle_selected_item_layers(context, depth + 1);
        model.icon_layers_with_bundle_resolver(context, &mut resolve_bundle_selected_item)
    }

    /// Vanilla `Charge.get`: `ROCKET` when any charged projectile is a
    /// `minecraft:firework_rocket`, `ARROW` when charged with anything else,
    /// else `NONE`. Projects the stack's `charged_projectiles` component.
    fn crossbow_charge_for(
        &self,
        component_patch: Option<&DataComponentPatchSummary>,
    ) -> CrossbowChargeType {
        let Some(patch) = component_patch else {
            return CrossbowChargeType::None;
        };
        if patch.charged_projectiles_items.is_empty() {
            return CrossbowChargeType::None;
        }
        let is_rocket = patch.charged_projectiles_items.iter().any(|template| {
            self.registry
                .as_ref()
                .and_then(|registry| registry.resource_id(template.item_id))
                == Some(FIREWORK_ROCKET_ITEM_ID)
        });
        if is_rocket {
            CrossbowChargeType::Rocket
        } else {
            CrossbowChargeType::Arrow
        }
    }

    pub(crate) fn item_model_use_context_for_stack(
        &self,
        stack: &ItemStackSummary,
        elapsed_ticks: u32,
    ) -> ItemModelUseContext {
        let Some(item_id) = stack
            .item_id
            .and_then(|protocol_id| self.registry.as_ref()?.resource_id(protocol_id))
        else {
            return ItemModelUseContext::inactive();
        };
        ItemModelUseContext::active(
            elapsed_ticks,
            item_use_duration_ticks(item_id, &stack.component_patch),
            crossbow_charge_duration_ticks(item_id),
        )
    }

    fn bundle_selected_item_layers(
        &self,
        context: IconResolveContext<'_>,
        depth: usize,
    ) -> Vec<ItemIconTextureLayer> {
        let Some(selected_item_index) = context
            .bundle_selected_item_index
            .filter(|index| *index >= 0)
        else {
            return Vec::new();
        };
        let Ok(selected_item_index) = usize::try_from(selected_item_index) else {
            return Vec::new();
        };
        let Some(template) = context
            .component_patch
            .and_then(|patch| patch.bundle_contents_items.get(selected_item_index))
        else {
            return Vec::new();
        };
        self.item_template_layers(template, context, depth)
    }

    fn item_template_layers(
        &self,
        template: &ItemStackTemplateSummary,
        parent_context: IconResolveContext<'_>,
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
        let default_max_stack_size = parent_context
            .default_max_stack_size_for_item
            .map(|max_stack_size| max_stack_size(template.item_id));
        let context = IconResolveContext {
            component_patch: Some(&template.component_patch),
            stack_count: template.count,
            default_max_stack_size,
            default_max_damage,
            bundle_selected_item_index: None,
            using_item: false,
            use_context: ItemModelUseContext::inactive(),
            cooldown_progress: 0.0,
            crossbow_charge: self.crossbow_charge_for(Some(&template.component_patch)),
            main_hand_left: parent_context.main_hand_left,
            context_dimension: parent_context.context_dimension,
            context_entity_type: parent_context.context_entity_type,
            default_max_stack_size_for_item: parent_context.default_max_stack_size_for_item,
            trim_material_keys: parent_context.trim_material_keys,
        };
        let layers = self
            .item_icon_models
            .get(item_id)
            .map(|model| self.icon_layers_for_model(model, context, depth))
            .unwrap_or_else(|| self.fallback_icon_texture_layers());
        resolve_item_icon_texture_layer_tints(layers, Some(&template.component_patch))
    }

    fn default_max_stack_size_for_protocol_id(&self, protocol_id: i32) -> i32 {
        self.registry
            .as_ref()
            .and_then(|registry| {
                registry
                    .resource_id(protocol_id)
                    .and_then(|resource_id| registry.max_stack_size(resource_id))
            })
            .unwrap_or(64)
            .clamp(1, 99)
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

fn item_use_duration_ticks(item_id: &str, component_patch: &DataComponentPatchSummary) -> i32 {
    if component_patch
        .added_type_ids
        .contains(&VANILLA_BLOCKS_ATTACKS_COMPONENT_ID)
        || component_patch
            .added_type_ids
            .contains(&VANILLA_KINETIC_WEAPON_COMPONENT_ID)
    {
        return VANILLA_LONG_USE_DURATION_TICKS;
    }
    match item_id {
        BOW_ITEM_ID | CROSSBOW_ITEM_ID | TRIDENT_ITEM_ID => VANILLA_LONG_USE_DURATION_TICKS,
        BRUSH_ITEM_ID => VANILLA_BRUSH_USE_DURATION_TICKS,
        SPYGLASS_ITEM_ID => VANILLA_SPYGLASS_USE_DURATION_TICKS,
        ENDER_EYE_ITEM_ID => VANILLA_ENDER_EYE_USE_DURATION_TICKS,
        _ => 0,
    }
}

fn crossbow_charge_duration_ticks(item_id: &str) -> Option<i32> {
    (item_id == CROSSBOW_ITEM_ID).then_some(VANILLA_CROSSBOW_CHARGE_DURATION_TICKS)
}

fn protocol_ids_for_resource_ids(
    registry: &ItemRegistryCatalog,
    resource_ids: &[&str],
) -> BTreeSet<i32> {
    resource_ids
        .iter()
        .filter_map(|resource_id| registry.protocol_id(resource_id))
        .collect()
}

fn recipe_specific_crafting_remainder_item_ids(registry: &ItemRegistryCatalog) -> BTreeSet<i32> {
    protocol_ids_for_resource_ids(registry, RECIPE_SPECIFIC_CRAFTING_REMAINDER_ITEM_IDS)
}

const DATA_COMPONENT_PROFILE_TYPE_ID: i32 = 70;

fn custom_head_skull_for_resource_id(
    resource_id: &str,
    component_patch: &DataComponentPatchSummary,
    profile_resolutions: &RefCell<Option<AsyncProfileResolutionRuntime>>,
    dynamic_skins: &RefCell<Option<AsyncDynamicPlayerSkinRuntime>>,
    dynamic_textures: &RefCell<Option<AsyncDynamicPlayerTextureRuntime>>,
    profile_texture_resources: &PackResourceStack,
    local_dynamic_skins: &RefCell<LocalDynamicPlayerSkinCache>,
    profile_skins: &RefCell<ProfileSkinCache>,
) -> Option<EntityCustomHeadSkull> {
    match resource_id {
        "minecraft:skeleton_skull" => Some(EntityCustomHeadSkull::Skeleton),
        "minecraft:wither_skeleton_skull" => Some(EntityCustomHeadSkull::WitherSkeleton),
        "minecraft:player_head" => custom_head_player_skull(
            component_patch,
            profile_resolutions,
            dynamic_skins,
            dynamic_textures,
            profile_texture_resources,
            local_dynamic_skins,
            profile_skins,
        ),
        "minecraft:zombie_head" => Some(EntityCustomHeadSkull::Zombie),
        "minecraft:creeper_head" => Some(EntityCustomHeadSkull::Creeper),
        "minecraft:dragon_head" => Some(EntityCustomHeadSkull::Dragon),
        "minecraft:piglin_head" => Some(EntityCustomHeadSkull::Piglin),
        _ => None,
    }
}

fn custom_head_player_skull(
    component_patch: &DataComponentPatchSummary,
    profile_resolutions: &RefCell<Option<AsyncProfileResolutionRuntime>>,
    dynamic_skins: &RefCell<Option<AsyncDynamicPlayerSkinRuntime>>,
    dynamic_textures: &RefCell<Option<AsyncDynamicPlayerTextureRuntime>>,
    profile_texture_resources: &PackResourceStack,
    local_dynamic_skins: &RefCell<LocalDynamicPlayerSkinCache>,
    profile_skins: &RefCell<ProfileSkinCache>,
) -> Option<EntityCustomHeadSkull> {
    if !component_patch_has_profile(component_patch) {
        return Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
            EntityDefaultPlayerSkin::SlimSteve,
        )));
    }

    let profile = component_patch.profile.as_ref()?;
    let profile = profile_resolutions
        .borrow_mut()
        .as_mut()
        .map(|profile_resolutions| profile_resolutions.resolve_or_queue(profile))
        .unwrap_or_else(|| profile.clone());
    let player_skin = player_skin_for_profile(
        &profile,
        profile_texture_resources,
        local_dynamic_skins,
        profile_skins,
    );
    queue_dynamic_profile_texture_downloads(&profile, player_skin, dynamic_skins, dynamic_textures);
    Some(EntityCustomHeadSkull::Player(player_skin))
}

fn player_skin_for_profile(
    profile: &ResolvableProfileSummary,
    profile_texture_resources: &PackResourceStack,
    local_dynamic_skins: &RefCell<LocalDynamicPlayerSkinCache>,
    profile_skins: &RefCell<ProfileSkinCache>,
) -> EntityPlayerSkin {
    let fallback = profile_default_player_skin(profile);
    if let Some(body) = profile.skin_patch.body.as_ref() {
        if EntityDefaultPlayerSkin::from_texture_path(&body.texture_path).is_none() {
            if let Some(skin) = local_dynamic_skins.borrow_mut().skin_for_patch(
                profile_texture_resources,
                profile,
                body,
                fallback,
            ) {
                return skin;
            }
        }
    }
    profile_skins.borrow_mut().player_skin_for_profile(profile)
}

fn queue_dynamic_profile_texture_downloads(
    profile: &ResolvableProfileSummary,
    player_skin: EntityPlayerSkin,
    dynamic_skins: &RefCell<Option<AsyncDynamicPlayerSkinRuntime>>,
    dynamic_textures: &RefCell<Option<AsyncDynamicPlayerTextureRuntime>>,
) {
    if let EntityPlayerSkin::Dynamic(skin) = player_skin {
        if skin.status == EntityDynamicPlayerSkinStatus::Loading {
            if let Some(url) = profile
                .profile_textures
                .as_ref()
                .and_then(|textures| textures.skin.as_ref())
                .map(|skin| skin.url.as_str())
            {
                if let Some(dynamic_skins) = dynamic_skins.borrow_mut().as_mut() {
                    dynamic_skins.queue(skin.handle, url);
                }
            }
        }
    }

    let Some(textures) = profile.profile_textures.as_ref() else {
        return;
    };
    let mut dynamic_textures = dynamic_textures.borrow_mut();
    let Some(dynamic_textures) = dynamic_textures.as_mut() else {
        return;
    };
    if profile.skin_patch.cape.is_none() {
        if let Some(cape) = textures.cape.as_ref() {
            dynamic_textures.queue(
                DynamicPlayerTextureKind::Cape,
                profile_texture_handle(&cape.url),
                &cape.url,
            );
        }
    }
    if profile.skin_patch.elytra.is_none() {
        if let Some(elytra) = textures.elytra.as_ref() {
            dynamic_textures.queue(
                DynamicPlayerTextureKind::Elytra,
                profile_texture_handle(&elytra.url),
                &elytra.url,
            );
        }
    }
}

fn dynamic_player_texture_for_profile(
    profile: &ResolvableProfileSummary,
    kind: EntityDynamicPlayerTextureKind,
    profile_texture_resources: &PackResourceStack,
    local_dynamic_textures: &RefCell<LocalDynamicPlayerTextureCache>,
) -> Option<EntityDynamicPlayerTexture> {
    if let Some(patch) = match kind {
        EntityDynamicPlayerTextureKind::Cape => profile.skin_patch.cape.as_ref(),
        EntityDynamicPlayerTextureKind::Elytra => profile.skin_patch.elytra.as_ref(),
    } {
        return local_dynamic_textures.borrow_mut().texture_for_patch(
            profile_texture_resources,
            kind,
            patch,
        );
    }

    let textures = profile.profile_textures.as_ref()?;
    let url = match kind {
        EntityDynamicPlayerTextureKind::Cape => textures.cape.as_ref()?.url.as_str(),
        EntityDynamicPlayerTextureKind::Elytra => textures.elytra.as_ref()?.url.as_str(),
    };
    Some(EntityDynamicPlayerTexture {
        handle: profile_texture_handle(url),
        kind,
    })
}

fn load_local_dynamic_player_skin(
    resources: &PackResourceStack,
    texture_path: &str,
    source_id: &str,
) -> Result<DynamicPlayerSkinImage> {
    let location = ResourceLocation::parse(texture_path).with_context(|| {
        format!("parse player profile body resource texture path {texture_path}")
    })?;
    let resource = resources
        .get_resource(&location)
        .with_context(|| format!("missing player profile body resource texture {texture_path}"))?;
    let image = SpriteImage::from_png_file(source_id.to_string(), resource.path)
        .with_context(|| format!("load player profile body resource texture {texture_path}"))?;
    let [width, height] = DynamicPlayerSkinImage::SIZE;
    anyhow::ensure!(
        image.width == width && image.height == height,
        "player profile body resource texture has size {}x{}, expected {}x{}",
        image.width,
        image.height,
        width,
        height
    );
    Ok(DynamicPlayerSkinImage {
        handle: profile_texture_handle(source_id),
        rgba: image.rgba,
    })
}

fn load_local_dynamic_player_texture(
    resources: &PackResourceStack,
    kind: EntityDynamicPlayerTextureKind,
    texture_path: &str,
    source_id: &str,
) -> Result<(EntityDynamicPlayerTexture, DynamicPlayerTextureImage)> {
    let location = ResourceLocation::parse(texture_path)
        .with_context(|| format!("parse player profile resource texture path {texture_path}"))?;
    let resource = resources
        .get_resource(&location)
        .with_context(|| format!("missing player profile resource texture {texture_path}"))?;
    let image = SpriteImage::from_png_file(source_id.to_string(), resource.path)
        .with_context(|| format!("load player profile resource texture {texture_path}"))?;
    let handle = profile_texture_handle(source_id);
    Ok((
        EntityDynamicPlayerTexture { handle, kind },
        DynamicPlayerTextureImage {
            handle,
            size: [image.width, image.height],
            rgba: image.rgba,
        },
    ))
}

fn local_player_skin_source_id(texture_path: &str) -> String {
    format!("resource:body:{texture_path}")
}

fn local_profile_texture_source_id(
    kind: EntityDynamicPlayerTextureKind,
    texture_path: &str,
) -> String {
    let kind = match kind {
        EntityDynamicPlayerTextureKind::Cape => "cape",
        EntityDynamicPlayerTextureKind::Elytra => "elytra",
    };
    format!("resource:{kind}:{texture_path}")
}

fn dynamic_player_texture_download_kind(
    kind: EntityDynamicPlayerTextureKind,
) -> DynamicPlayerTextureKind {
    match kind {
        EntityDynamicPlayerTextureKind::Cape => DynamicPlayerTextureKind::Cape,
        EntityDynamicPlayerTextureKind::Elytra => DynamicPlayerTextureKind::Elytra,
    }
}

fn profile_skin_model(
    profile: &ResolvableProfileSummary,
    fallback: EntityDefaultPlayerSkin,
) -> EntityPlayerSkinModel {
    profile
        .skin_patch
        .model
        .map(entity_player_skin_model)
        .or_else(|| {
            profile
                .profile_textures
                .as_ref()
                .and_then(|textures| textures.skin.as_ref())
                .map(|skin| entity_player_skin_model(skin.model))
        })
        .unwrap_or_else(|| fallback.model())
}

fn component_patch_has_profile(component_patch: &DataComponentPatchSummary) -> bool {
    component_patch
        .added_type_ids
        .contains(&DATA_COMPONENT_PROFILE_TYPE_ID)
        && !component_patch
            .removed_type_ids
            .contains(&DATA_COMPONENT_PROFILE_TYPE_ID)
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

fn world_mount_armor_slot_kind(kind: PackItemMountBodyArmorKind) -> WorldMountArmorSlotKind {
    match kind {
        PackItemMountBodyArmorKind::Horse => WorldMountArmorSlotKind::Horse,
        PackItemMountBodyArmorKind::Llama => WorldMountArmorSlotKind::Llama,
        PackItemMountBodyArmorKind::Nautilus => WorldMountArmorSlotKind::Nautilus,
    }
}

fn llama_body_decor_color_from_item_id(resource_id: &str) -> Option<WorldLlamaBodyDecorColor> {
    let path = resource_id
        .split_once(':')
        .map_or(resource_id, |(_, path)| path);
    let color = path.strip_suffix("_carpet")?;
    Some(match color {
        "white" => WorldLlamaBodyDecorColor::White,
        "orange" => WorldLlamaBodyDecorColor::Orange,
        "magenta" => WorldLlamaBodyDecorColor::Magenta,
        "light_blue" => WorldLlamaBodyDecorColor::LightBlue,
        "yellow" => WorldLlamaBodyDecorColor::Yellow,
        "lime" => WorldLlamaBodyDecorColor::Lime,
        "pink" => WorldLlamaBodyDecorColor::Pink,
        "gray" => WorldLlamaBodyDecorColor::Gray,
        "light_gray" => WorldLlamaBodyDecorColor::LightGray,
        "cyan" => WorldLlamaBodyDecorColor::Cyan,
        "purple" => WorldLlamaBodyDecorColor::Purple,
        "blue" => WorldLlamaBodyDecorColor::Blue,
        "brown" => WorldLlamaBodyDecorColor::Brown,
        "green" => WorldLlamaBodyDecorColor::Green,
        "red" => WorldLlamaBodyDecorColor::Red,
        "black" => WorldLlamaBodyDecorColor::Black,
        _ => return None,
    })
}

fn nautilus_body_armor_material_from_asset(asset: &str) -> Option<WorldArmorMaterialKind> {
    let material = WorldArmorMaterialKind::from_equipment_asset(asset)?;
    match material {
        WorldArmorMaterialKind::Copper
        | WorldArmorMaterialKind::Iron
        | WorldArmorMaterialKind::Gold
        | WorldArmorMaterialKind::Diamond
        | WorldArmorMaterialKind::Netherite => Some(material),
        WorldArmorMaterialKind::Leather
        | WorldArmorMaterialKind::Chainmail
        | WorldArmorMaterialKind::TurtleScute
        | WorldArmorMaterialKind::ArmadilloScute => None,
    }
}

fn horse_body_armor_material_from_asset(asset: &str) -> Option<WorldArmorMaterialKind> {
    let material = WorldArmorMaterialKind::from_equipment_asset(asset)?;
    match material {
        WorldArmorMaterialKind::Leather
        | WorldArmorMaterialKind::Copper
        | WorldArmorMaterialKind::Iron
        | WorldArmorMaterialKind::Gold
        | WorldArmorMaterialKind::Diamond
        | WorldArmorMaterialKind::Netherite => Some(material),
        WorldArmorMaterialKind::Chainmail
        | WorldArmorMaterialKind::TurtleScute
        | WorldArmorMaterialKind::ArmadilloScute => None,
    }
}

fn wolf_body_armor_material_from_asset(asset: &str) -> Option<WorldArmorMaterialKind> {
    let material = WorldArmorMaterialKind::from_equipment_asset(asset)?;
    match material {
        WorldArmorMaterialKind::ArmadilloScute => Some(material),
        WorldArmorMaterialKind::Leather
        | WorldArmorMaterialKind::Copper
        | WorldArmorMaterialKind::Chainmail
        | WorldArmorMaterialKind::Iron
        | WorldArmorMaterialKind::Gold
        | WorldArmorMaterialKind::Diamond
        | WorldArmorMaterialKind::TurtleScute
        | WorldArmorMaterialKind::Netherite => None,
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

/// Per-stack use-state values for vanilla item-model numeric properties. These
/// are active only for the stack that vanilla would expose as
/// `LivingEntity.getUseItem()`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct ItemModelUseContext {
    pub(crate) elapsed_ticks: u32,
    pub(crate) remaining_ticks: Option<f32>,
    pub(crate) crossbow_charge_duration_ticks: Option<f32>,
}

impl ItemModelUseContext {
    pub(crate) fn inactive() -> Self {
        Self::default()
    }

    fn active(
        elapsed_ticks: u32,
        use_duration_ticks: i32,
        crossbow_charge_duration_ticks: Option<i32>,
    ) -> Self {
        Self {
            elapsed_ticks,
            remaining_ticks: Some((use_duration_ticks - elapsed_ticks as i32).max(0) as f32),
            crossbow_charge_duration_ticks: crossbow_charge_duration_ticks
                .map(|ticks| ticks as f32),
        }
    }
}

/// One layer of a generated (flat) item ready for 3D extrusion: the sprite's alpha silhouette, its
/// atlas UV rect (item atlas), and the resolved layer tint.
pub(crate) struct GeneratedItemLayer {
    pub(crate) mask: SpriteAlphaMask,
    pub(crate) rect: ItemSpriteRect,
    pub(crate) tint: [f32; 4],
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

    /// Builds the per-pixel alpha silhouette of the sprite a UV rect covers, for generated-item
    /// extrusion. Inverts the half-texel inset [`item_uv_rect`] applies to recover the exact content
    /// pixel bounds, then reads the stitched atlas alpha (vanilla `SpriteContents.isTransparent`: a pixel
    /// is opaque iff its alpha byte is non-zero).
    fn alpha_mask_for_uv(&self, uv: ItemAtlasUvRect) -> Option<SpriteAlphaMask> {
        let (atlas_width, atlas_height) = self.atlas_size();
        let width = atlas_width as f32;
        let height = atlas_height as f32;
        let x0 = (uv.min[0] * width - 0.5).round() as i64;
        let x1 = (uv.max[0] * width + 0.5).round() as i64;
        let y0 = (uv.min[1] * height - 0.5).round() as i64;
        let y1 = (uv.max[1] * height + 0.5).round() as i64;
        if x0 < 0 || y0 < 0 || x1 <= x0 || y1 <= y0 {
            return None;
        }
        if x1 as u32 > atlas_width || y1 as u32 > atlas_height {
            return None;
        }
        let mask_width = (x1 - x0) as u32;
        let mask_height = (y1 - y0) as u32;
        let rgba = self.atlas_rgba();
        let mut opaque = Vec::with_capacity((mask_width * mask_height) as usize);
        for py in 0..mask_height {
            for px in 0..mask_width {
                let ax = x0 as u32 + px;
                let ay = y0 as u32 + py;
                let alpha_index = ((ay * atlas_width + ax) * 4 + 3) as usize;
                opaque.push(rgba.get(alpha_index).copied().unwrap_or(0) != 0);
            }
        }
        Some(SpriteAlphaMask::new(mask_width, mask_height, opaque))
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
    use crate::skin_runtime::{
        AsyncDynamicPlayerSkinRuntime, AsyncDynamicPlayerTextureRuntime, SkinPngFetcher,
    };
    use std::{
        io::Cursor,
        path::{Path, PathBuf},
        sync::{
            atomic::{AtomicU64, AtomicUsize, Ordering},
            Arc,
        },
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
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
    fn item_display_transform_is_retained_per_item() {
        let root = unique_temp_dir("item-display-transform");
        let assets = assets_dir(&root);
        write_item_atlases(&assets);
        write_item_registry_sources(&root);
        write_json(
            &assets.join("items").join("test_combo.json"),
            r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/test_sword"
                }
            }"#,
        );
        // An `item/handheld`-style angled third-person transform on the item's own model.
        write_json(
            &assets.join("models").join("item").join("test_sword.json"),
            r##"{
                "display": {
                    "thirdperson_righthand": {
                        "rotation": [0, -90, 55],
                        "translation": [0, 4, 0.5],
                        "scale": [0.85, 0.85, 0.85]
                    }
                },
                "textures": { "layer0": "minecraft:item/test_sword" }
            }"##,
        );
        write_test_rgba_png(
            &assets.join("textures").join("item").join("test_sword.png"),
            1,
            1,
            &[255, 0, 0, 255],
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let transform = runtime
            .item_display_transform(0, BlockModelDisplayContext::ThirdPersonRightHand)
            .unwrap();
        // Vanilla pre-multiplies the JSON translation by 1/16 (and clamps); rotation stays in degrees.
        assert_eq!(transform.rotation, [0.0, -90.0, 55.0]);
        assert_eq!(transform.translation, [0.0, 4.0 / 16.0, 0.5 / 16.0]);
        assert_eq!(transform.scale, [0.85, 0.85, 0.85]);
        // A context the model does not override falls back to the identity transform.
        assert_eq!(
            runtime.item_display_transform(0, BlockModelDisplayContext::Gui),
            Some(BlockModelDisplayTransform::default())
        );
        // An unregistered protocol id has no retained transform (caller uses a parent-model default).
        assert_eq!(
            runtime.item_display_transform(999, BlockModelDisplayContext::ThirdPersonRightHand),
            None
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn humanoid_armor_asset_query_excludes_head_equippable_non_armor() {
        let root = unique_temp_dir("item-runtime-humanoid-armor-asset");
        let assets = assets_dir(&root);
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
                public static final Item DIAMOND_HELMET = registerItem(
                    "diamond_helmet",
                    new Item.Properties().humanoidArmor(ArmorMaterials.DIAMOND, ArmorType.HELMET)
                );
                public static final Item DIAMOND_CHESTPLATE = registerItem(
                    "diamond_chestplate",
                    new Item.Properties().humanoidArmor(ArmorMaterials.DIAMOND, ArmorType.CHESTPLATE)
                );
                public static final Item ELYTRA = registerItem(
                    "elytra",
                    new Item.Properties()
                       .component(
                          DataComponents.EQUIPPABLE,
                          Equippable.builder(EquipmentSlot.CHEST)
                             .setAsset(EquipmentAssets.ELYTRA)
                             .build()
                       )
                );
                public static final Item CARVED_PUMPKIN = registerBlock(
                    Blocks.CARVED_PUMPKIN,
                    p -> p.component(
                        DataComponents.EQUIPPABLE,
                        Equippable.builder(EquipmentSlot.HEAD).setSwappable(false).build()
                    )
                );
                public static final Item STONE = registerBlock(Blocks.STONE);
            }"#,
        );
        write_json(
            &assets.join("equipment").join("diamond.json"),
            r#"{
                "layers": {
                    "humanoid": [
                        { "texture": "minecraft:diamond" }
                    ],
                    "humanoid_leggings": [
                        { "texture": "minecraft:diamond" }
                    ]
                }
            }"#,
        );
        write_json(
            &assets.join("equipment").join("elytra.json"),
            r#"{
                "layers": {
                    "wings": [
                        { "texture": "minecraft:elytra", "use_player_texture": true }
                    ]
                }
            }"#,
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let registry = runtime.registry.as_ref().unwrap();
        let helmet_id = registry.protocol_id("minecraft:diamond_helmet").unwrap();
        let chestplate_id = registry
            .protocol_id("minecraft:diamond_chestplate")
            .unwrap();
        let elytra_id = registry.protocol_id("minecraft:elytra").unwrap();
        let pumpkin_id = registry.protocol_id("minecraft:carved_pumpkin").unwrap();
        let stone_id = registry.protocol_id("minecraft:stone").unwrap();

        assert!(runtime.item_has_humanoid_armor_asset(helmet_id));
        assert!(runtime.item_has_humanoid_armor_asset(chestplate_id));
        assert!(!runtime.item_has_humanoid_armor_asset(pumpkin_id));
        assert!(!runtime.item_has_humanoid_armor_asset(stone_id));
        assert!(!runtime.item_has_humanoid_armor_asset(999));
        assert!(runtime.item_equipment_asset_has_humanoid_layer(chestplate_id));
        assert!(!runtime.item_equipment_asset_has_wings_layer(chestplate_id));
        assert!(runtime.item_equipment_asset_has_wings_layer(elytra_id));
        assert_eq!(
            runtime.item_equipment_wings_layer(elytra_id),
            Some(EntityEquipmentLayerTexture {
                texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
                use_player_texture: true,
            })
        );
        assert_eq!(runtime.item_equipment_wings_layer(chestplate_id), None);
        assert!(!runtime.item_equipment_asset_has_humanoid_layer(elytra_id));
        assert!(!runtime.item_equipment_asset_has_wings_layer(pumpkin_id));
        assert!(!runtime.item_equipment_asset_has_humanoid_layer(stone_id));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn custom_head_skull_projection_resolves_static_and_profileless_player_skulls() {
        let root = unique_temp_dir("item-runtime-custom-head-skulls");
        let assets = assets_dir(&root);
        write_item_atlases(&assets);
        write_item_registry_source(
            &root,
            &[
                "skeleton_skull",
                "wither_skeleton_skull",
                "zombie_head",
                "creeper_head",
                "player_head",
                "dragon_head",
                "piglin_head",
                "carved_pumpkin",
            ],
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let registry = runtime.registry.as_ref().unwrap();

        for (resource_id, expected) in [
            (
                "minecraft:skeleton_skull",
                Some(EntityCustomHeadSkull::Skeleton),
            ),
            (
                "minecraft:wither_skeleton_skull",
                Some(EntityCustomHeadSkull::WitherSkeleton),
            ),
            ("minecraft:zombie_head", Some(EntityCustomHeadSkull::Zombie)),
            (
                "minecraft:creeper_head",
                Some(EntityCustomHeadSkull::Creeper),
            ),
            (
                "minecraft:player_head",
                Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
                    EntityDefaultPlayerSkin::SlimSteve,
                ))),
            ),
            ("minecraft:dragon_head", Some(EntityCustomHeadSkull::Dragon)),
            ("minecraft:piglin_head", Some(EntityCustomHeadSkull::Piglin)),
            ("minecraft:carved_pumpkin", None),
        ] {
            let protocol_id = registry.protocol_id(resource_id).unwrap();
            let stack = ItemStackSummary {
                item_id: Some(protocol_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            };
            assert_eq!(
                runtime.custom_head_skull_for_stack(&stack),
                expected,
                "{resource_id}"
            );
        }
        assert_eq!(
            runtime.custom_head_skull_for_stack(&ItemStackSummary {
                item_id: Some(999),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }),
            None
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn custom_head_skull_projection_resolves_profiled_player_head_default_skins() {
        let root = unique_temp_dir("item-runtime-custom-profiled-player-head");
        let assets = assets_dir(&root);
        write_item_atlases(&assets);
        write_item_registry_source(&root, &["player_head"]);

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let registry = runtime.registry.as_ref().unwrap();
        let player_head_id = registry.protocol_id("minecraft:player_head").unwrap();

        let mut profiled = ItemStackSummary {
            item_id: Some(player_head_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        profiled
            .component_patch
            .added_type_ids
            .push(DATA_COMPONENT_PROFILE_TYPE_ID);
        assert_eq!(runtime.custom_head_skull_for_stack(&profiled), None);

        profiled.component_patch.profile = Some(ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: Default::default(),
        });
        assert_eq!(
            runtime.custom_head_skull_for_stack(&profiled),
            Some(EntityCustomHeadSkull::Player(
                EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
            ))
        );

        let skin_url = "https://textures.minecraft.net/texture/profile-skin";
        let mut dynamic = profiled.clone();
        dynamic.component_patch.profile = Some(ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: Vec::new(),
            profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
                skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                    url: skin_url.to_string(),
                    model: bbb_protocol::packets::PlayerModelTypeSummary::Slim,
                }),
                cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/profile-cape".to_string(),
                }),
                elytra: None,
            }),
            skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
                body: None,
                cape: None,
                elytra: None,
                model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Wide),
            },
        });
        assert_eq!(
            runtime.custom_head_skull_for_stack(&dynamic),
            Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
                EntityDynamicPlayerSkin {
                    handle: profile_texture_handle(skin_url),
                    fallback: EntityDefaultPlayerSkin::SlimAlex,
                    model: EntityPlayerSkinModel::Wide,
                    status: EntityDynamicPlayerSkinStatus::Loading,
                }
            )))
        );

        let skin_download_calls = Arc::new(AtomicUsize::new(0));
        runtime.enable_player_skin_downloads_for_test(AsyncDynamicPlayerSkinRuntime::new(
            root.join("skin-cache"),
            TestSkinPngFetcher {
                bytes: player_skin_png_bytes(),
                calls: skin_download_calls.clone(),
            },
        ));
        assert_eq!(
            runtime.custom_head_skull_for_stack(&dynamic),
            Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
                EntityDynamicPlayerSkin {
                    handle: profile_texture_handle(skin_url),
                    fallback: EntityDefaultPlayerSkin::SlimAlex,
                    model: EntityPlayerSkinModel::Wide,
                    status: EntityDynamicPlayerSkinStatus::Loading,
                }
            )))
        );
        let downloads = drain_until_player_skin_download_result(&runtime);
        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0].url, skin_url);
        assert_eq!(
            downloads[0].skin.as_ref().unwrap().handle,
            profile_texture_handle(skin_url)
        );
        assert_eq!(runtime.downloaded_player_skin_count(), 1);
        assert_eq!(skin_download_calls.load(Ordering::Relaxed), 1);
        assert_eq!(
            runtime.custom_head_skull_for_stack(&dynamic),
            Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
                EntityDynamicPlayerSkin {
                    handle: profile_texture_handle(skin_url),
                    fallback: EntityDefaultPlayerSkin::SlimAlex,
                    model: EntityPlayerSkinModel::Wide,
                    status: EntityDynamicPlayerSkinStatus::Loading,
                }
            )))
        );
        assert_eq!(skin_download_calls.load(Ordering::Relaxed), 1);

        runtime.mark_profile_skin_resolved(skin_url, 12_345);
        assert_eq!(
            runtime.custom_head_skull_for_stack(&dynamic),
            Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
                EntityDynamicPlayerSkin {
                    handle: 12_345,
                    fallback: EntityDefaultPlayerSkin::SlimAlex,
                    model: EntityPlayerSkinModel::Wide,
                    status: EntityDynamicPlayerSkinStatus::Ready,
                }
            )))
        );

        runtime.mark_profile_skin_failed(skin_url);
        assert_eq!(
            runtime.custom_head_skull_for_stack(&dynamic),
            Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
                EntityDynamicPlayerSkin {
                    handle: profile_texture_handle(skin_url),
                    fallback: EntityDefaultPlayerSkin::SlimAlex,
                    model: EntityPlayerSkinModel::Wide,
                    status: EntityDynamicPlayerSkinStatus::Failed,
                }
            )))
        );

        let mut resource_patched = dynamic.clone();
        resource_patched
            .component_patch
            .profile
            .as_mut()
            .unwrap()
            .skin_patch
            .body = Some(bbb_protocol::packets::ResourceTextureSummary {
            asset_id: "minecraft:entity/player/custom".to_string(),
            texture_path: "minecraft:textures/entity/player/custom.png".to_string(),
        });
        assert_eq!(
            runtime.custom_head_skull_for_stack(&resource_patched),
            Some(EntityCustomHeadSkull::Player(
                EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
            ))
        );

        let mut patched = profiled.clone();
        patched.component_patch.profile = Some(ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
                body: Some(bbb_protocol::packets::ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/wide/steve".to_string(),
                    texture_path: "minecraft:textures/entity/player/wide/steve.png".to_string(),
                }),
                cape: None,
                elytra: None,
                model: None,
            },
        });
        assert_eq!(
            runtime.custom_head_skull_for_stack(&patched),
            Some(EntityCustomHeadSkull::Player(
                EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::WideSteve)
            ))
        );

        let mut profile_removed = profiled.clone();
        profile_removed
            .component_patch
            .removed_type_ids
            .push(DATA_COMPONENT_PROFILE_TYPE_ID);
        assert_eq!(
            runtime.custom_head_skull_for_stack(&profile_removed),
            Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
                EntityDefaultPlayerSkin::SlimSteve
            )))
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn player_profile_queues_dynamic_cape_and_elytra_texture_downloads() {
        let root = unique_temp_dir("item-runtime-profile-texture-downloads");
        let runtime = NativeItemRuntime::empty_for_test();
        let texture_download_calls = Arc::new(AtomicUsize::new(0));
        runtime.enable_player_texture_downloads_for_test(AsyncDynamicPlayerTextureRuntime::new(
            root.join("profile-texture-cache"),
            TestSkinPngFetcher {
                bytes: player_profile_texture_png_bytes(),
                calls: texture_download_calls.clone(),
            },
        ));

        let cape_url = "https://textures.minecraft.net/texture/profile-cape";
        let elytra_url = "https://textures.minecraft.net/texture/profile-elytra";
        let profile = ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: Vec::new(),
            profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
                skin: None,
                cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: cape_url.to_string(),
                }),
                elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: elytra_url.to_string(),
                }),
            }),
            skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary::default(),
        };

        assert_eq!(
            runtime.player_skin_for_profile(&profile),
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        );

        let downloads = drain_until_player_texture_download_results(&runtime, 2);
        assert_eq!(downloads.len(), 2);
        let cape = downloads
            .iter()
            .find(|download| download.kind == DynamicPlayerTextureKind::Cape)
            .expect("cape download result");
        assert_eq!(cape.url, cape_url);
        let cape_texture = cape.texture.as_ref().expect("cape texture");
        assert_eq!(cape_texture.handle, profile_texture_handle(cape_url));
        assert_eq!(cape_texture.size, [64, 32]);

        let elytra = downloads
            .iter()
            .find(|download| download.kind == DynamicPlayerTextureKind::Elytra)
            .expect("elytra download result");
        assert_eq!(elytra.url, elytra_url);
        let elytra_texture = elytra.texture.as_ref().expect("elytra texture");
        assert_eq!(elytra_texture.handle, profile_texture_handle(elytra_url));
        assert_eq!(elytra_texture.size, [64, 32]);

        assert_eq!(runtime.downloaded_player_texture_count(), 2);
        assert_eq!(texture_download_calls.load(Ordering::Relaxed), 2);

        runtime.player_skin_for_profile(&profile);
        assert!(runtime
            .drain_dynamic_player_texture_download_results()
            .is_empty());
        assert_eq!(texture_download_calls.load(Ordering::Relaxed), 2);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn player_profile_resource_texture_patch_suppresses_remote_cape_and_elytra_downloads() {
        let root = unique_temp_dir("item-runtime-profile-texture-patches");
        let runtime = NativeItemRuntime::empty_for_test();
        let texture_download_calls = Arc::new(AtomicUsize::new(0));
        runtime.enable_player_texture_downloads_for_test(AsyncDynamicPlayerTextureRuntime::new(
            root.join("profile-texture-cache"),
            TestSkinPngFetcher {
                bytes: player_profile_texture_png_bytes(),
                calls: texture_download_calls.clone(),
            },
        ));

        let profile = ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: Vec::new(),
            profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
                skin: None,
                cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/profile-cape".to_string(),
                }),
                elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/profile-elytra".to_string(),
                }),
            }),
            skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
                body: None,
                cape: Some(bbb_protocol::packets::ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/cape/custom".to_string(),
                    texture_path: "minecraft:textures/entity/player/cape/custom.png".to_string(),
                }),
                elytra: Some(bbb_protocol::packets::ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/elytra/custom".to_string(),
                    texture_path: "minecraft:textures/entity/player/elytra/custom.png".to_string(),
                }),
                model: None,
            },
        };

        assert_eq!(
            runtime.player_skin_for_profile(&profile),
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        );
        for _ in 0..10 {
            assert!(runtime
                .drain_dynamic_player_texture_download_results()
                .is_empty());
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(runtime.downloaded_player_texture_count(), 0);
        assert_eq!(texture_download_calls.load(Ordering::Relaxed), 0);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn player_profile_resource_texture_patch_loads_local_cape_and_elytra_textures() {
        let root = unique_temp_dir("item-runtime-profile-resource-textures");
        let assets = assets_dir(&root);
        let cape_texture_path = "minecraft:textures/entity/player/cape/custom.png";
        let elytra_texture_path = "minecraft:textures/entity/player/elytra/custom.png";
        let cape_rgba = vec![
            10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255, 50, 0, 0, 255, 60, 0, 0,
            255, 70, 0, 0, 255, 80, 0, 0, 255,
        ];
        let elytra_rgba = vec![
            0, 10, 0, 255, 0, 20, 0, 255, 0, 30, 0, 255, 0, 40, 0, 255, 0, 50, 0, 255, 0, 60, 0,
            255,
        ];
        write_test_rgba_png(
            &assets
                .join("textures")
                .join("entity")
                .join("player")
                .join("cape")
                .join("custom.png"),
            4,
            2,
            &cape_rgba,
        );
        write_test_rgba_png(
            &assets
                .join("textures")
                .join("entity")
                .join("player")
                .join("elytra")
                .join("custom.png"),
            2,
            3,
            &elytra_rgba,
        );

        let mut runtime = NativeItemRuntime::empty_for_test();
        runtime.profile_texture_resources =
            PackResourceStack::from_roots([root.join("sources").join(bbb_pack::MC_VERSION)]);
        let texture_download_calls = Arc::new(AtomicUsize::new(0));
        runtime.enable_player_texture_downloads_for_test(AsyncDynamicPlayerTextureRuntime::new(
            root.join("profile-texture-cache"),
            TestSkinPngFetcher {
                bytes: player_profile_texture_png_bytes(),
                calls: texture_download_calls.clone(),
            },
        ));

        let profile = ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: Vec::new(),
            profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
                skin: None,
                cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/remote-cape".to_string(),
                }),
                elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/remote-elytra".to_string(),
                }),
            }),
            skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
                body: None,
                cape: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/cape/custom".to_string(),
                    texture_path: cape_texture_path.to_string(),
                }),
                elytra: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/elytra/custom".to_string(),
                    texture_path: elytra_texture_path.to_string(),
                }),
                model: None,
            },
        };

        let cape_source = local_profile_texture_source_id(
            EntityDynamicPlayerTextureKind::Cape,
            cape_texture_path,
        );
        let elytra_source = local_profile_texture_source_id(
            EntityDynamicPlayerTextureKind::Elytra,
            elytra_texture_path,
        );
        let cape = runtime
            .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Cape)
            .expect("local cape texture");
        let elytra = runtime
            .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Elytra)
            .expect("local elytra texture");

        assert_eq!(cape.kind, EntityDynamicPlayerTextureKind::Cape);
        assert_eq!(cape.handle, profile_texture_handle(&cape_source));
        assert_eq!(elytra.kind, EntityDynamicPlayerTextureKind::Elytra);
        assert_eq!(elytra.handle, profile_texture_handle(&elytra_source));
        assert_eq!(texture_download_calls.load(Ordering::Relaxed), 0);

        let downloads = runtime.drain_dynamic_player_texture_download_results();
        assert_eq!(downloads.len(), 2);
        let cape_download = downloads
            .iter()
            .find(|download| download.kind == DynamicPlayerTextureKind::Cape)
            .expect("cape upload");
        assert_eq!(cape_download.url, cape_source);
        let cape_image = cape_download.texture.as_ref().expect("cape image");
        assert_eq!(cape_image.handle, cape.handle);
        assert_eq!(cape_image.size, [4, 2]);
        assert_eq!(cape_image.rgba, cape_rgba);

        let elytra_download = downloads
            .iter()
            .find(|download| download.kind == DynamicPlayerTextureKind::Elytra)
            .expect("elytra upload");
        assert_eq!(elytra_download.url, elytra_source);
        let elytra_image = elytra_download.texture.as_ref().expect("elytra image");
        assert_eq!(elytra_image.handle, elytra.handle);
        assert_eq!(elytra_image.size, [2, 3]);
        assert_eq!(elytra_image.rgba, elytra_rgba);

        assert!(runtime
            .drain_dynamic_player_texture_download_results()
            .is_empty());
        assert_eq!(texture_download_calls.load(Ordering::Relaxed), 0);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn player_profile_resource_texture_patch_failure_does_not_use_remote_or_stale_texture() {
        let root = unique_temp_dir("item-runtime-profile-resource-texture-failure");
        let assets = assets_dir(&root);
        let cape_texture_path = "minecraft:textures/entity/player/cape/missing.png";
        let elytra_texture_path = "minecraft:textures/entity/player/elytra/missing.png";
        let mut runtime = NativeItemRuntime::empty_for_test();
        runtime.profile_texture_resources =
            PackResourceStack::from_roots([root.join("sources").join(bbb_pack::MC_VERSION)]);
        let texture_download_calls = Arc::new(AtomicUsize::new(0));
        runtime.enable_player_texture_downloads_for_test(AsyncDynamicPlayerTextureRuntime::new(
            root.join("profile-texture-cache"),
            TestSkinPngFetcher {
                bytes: player_profile_texture_png_bytes(),
                calls: texture_download_calls.clone(),
            },
        ));
        let profile = ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: Vec::new(),
            profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
                skin: None,
                cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/remote-cape".to_string(),
                }),
                elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/remote-elytra".to_string(),
                }),
            }),
            skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
                body: None,
                cape: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/cape/missing".to_string(),
                    texture_path: cape_texture_path.to_string(),
                }),
                elytra: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/elytra/missing".to_string(),
                    texture_path: elytra_texture_path.to_string(),
                }),
                model: None,
            },
        };

        assert_eq!(
            runtime
                .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Cape),
            None
        );
        assert_eq!(
            runtime.player_profile_texture_for_profile(
                &profile,
                EntityDynamicPlayerTextureKind::Elytra
            ),
            None
        );
        assert!(runtime
            .drain_dynamic_player_texture_download_results()
            .is_empty());
        assert_eq!(runtime.downloaded_player_texture_count(), 0);
        assert_eq!(texture_download_calls.load(Ordering::Relaxed), 0);

        write_test_rgba_png(
            &assets
                .join("textures")
                .join("entity")
                .join("player")
                .join("cape")
                .join("missing.png"),
            1,
            1,
            &[1, 2, 3, 255],
        );
        write_test_rgba_png(
            &assets
                .join("textures")
                .join("entity")
                .join("player")
                .join("elytra")
                .join("missing.png"),
            1,
            1,
            &[4, 5, 6, 255],
        );

        assert_eq!(
            runtime
                .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Cape),
            None
        );
        assert_eq!(
            runtime.player_profile_texture_for_profile(
                &profile,
                EntityDynamicPlayerTextureKind::Elytra
            ),
            None
        );
        assert!(runtime
            .drain_dynamic_player_texture_download_results()
            .is_empty());
        assert_eq!(runtime.downloaded_player_texture_count(), 0);
        assert_eq!(texture_download_calls.load(Ordering::Relaxed), 0);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn player_profile_resource_texture_patch_loads_local_body_skin() {
        let root = unique_temp_dir("item-runtime-profile-resource-body-skin");
        let assets = assets_dir(&root);
        let body_texture_path = "minecraft:textures/entity/player/body/custom.png";
        let mut body_rgba = Vec::with_capacity((64 * 64 * 4) as usize);
        for y in 0..64 {
            for x in 0..64 {
                body_rgba.extend_from_slice(&[x as u8, y as u8, 127, 255]);
            }
        }
        write_test_rgba_png(
            &assets
                .join("textures")
                .join("entity")
                .join("player")
                .join("body")
                .join("custom.png"),
            64,
            64,
            &body_rgba,
        );

        let mut runtime = NativeItemRuntime::empty_for_test();
        runtime.profile_texture_resources =
            PackResourceStack::from_roots([root.join("sources").join(bbb_pack::MC_VERSION)]);
        let skin_download_calls = Arc::new(AtomicUsize::new(0));
        runtime.enable_player_skin_downloads_for_test(AsyncDynamicPlayerSkinRuntime::new(
            root.join("skin-cache"),
            TestSkinPngFetcher {
                bytes: player_skin_png_bytes(),
                calls: skin_download_calls.clone(),
            },
        ));

        let profile = ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: Vec::new(),
            profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
                skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                    url: "https://textures.minecraft.net/texture/remote-skin".to_string(),
                    model: bbb_protocol::packets::PlayerModelTypeSummary::Slim,
                }),
                cape: None,
                elytra: None,
            }),
            skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
                body: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/body/custom".to_string(),
                    texture_path: body_texture_path.to_string(),
                }),
                cape: None,
                elytra: None,
                model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Wide),
            },
        };
        let body_source = local_player_skin_source_id(body_texture_path);
        let expected_skin = EntityDynamicPlayerSkin {
            handle: profile_texture_handle(&body_source),
            fallback: EntityDefaultPlayerSkin::SlimAlex,
            model: EntityPlayerSkinModel::Wide,
            status: EntityDynamicPlayerSkinStatus::Ready,
        };

        assert_eq!(
            runtime.player_skin_for_profile(&profile),
            EntityPlayerSkin::Dynamic(expected_skin)
        );
        assert_eq!(skin_download_calls.load(Ordering::Relaxed), 0);

        let downloads = runtime.drain_dynamic_player_skin_download_results();
        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0].url, body_source);
        let image = downloads[0].skin.as_ref().expect("local body skin image");
        assert_eq!(image.handle, expected_skin.handle);
        assert_eq!(image.rgba, body_rgba);

        assert_eq!(
            runtime.player_skin_for_profile(&profile),
            EntityPlayerSkin::Dynamic(expected_skin)
        );
        assert!(runtime
            .drain_dynamic_player_skin_download_results()
            .is_empty());
        assert_eq!(skin_download_calls.load(Ordering::Relaxed), 0);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn player_profile_resource_body_patch_failure_does_not_use_remote_or_stale_skin() {
        let root = unique_temp_dir("item-runtime-profile-resource-body-skin-failure");
        let assets = assets_dir(&root);
        let body_texture_path = "minecraft:textures/entity/player/body/missing.png";
        let mut runtime = NativeItemRuntime::empty_for_test();
        runtime.profile_texture_resources =
            PackResourceStack::from_roots([root.join("sources").join(bbb_pack::MC_VERSION)]);
        let skin_download_calls = Arc::new(AtomicUsize::new(0));
        runtime.enable_player_skin_downloads_for_test(AsyncDynamicPlayerSkinRuntime::new(
            root.join("skin-cache"),
            TestSkinPngFetcher {
                bytes: player_skin_png_bytes(),
                calls: skin_download_calls.clone(),
            },
        ));

        let profile = ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: Vec::new(),
            profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
                skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                    url: "https://textures.minecraft.net/texture/remote-skin".to_string(),
                    model: bbb_protocol::packets::PlayerModelTypeSummary::Slim,
                }),
                cape: None,
                elytra: None,
            }),
            skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
                body: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/body/missing".to_string(),
                    texture_path: body_texture_path.to_string(),
                }),
                cape: None,
                elytra: None,
                model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Wide),
            },
        };

        assert_eq!(
            runtime.player_skin_for_profile(&profile),
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        );
        assert!(runtime
            .drain_dynamic_player_skin_download_results()
            .is_empty());
        assert_eq!(runtime.downloaded_player_skin_count(), 0);
        assert_eq!(skin_download_calls.load(Ordering::Relaxed), 0);

        let mut body_rgba = Vec::with_capacity((64 * 64 * 4) as usize);
        for y in 0..64 {
            for x in 0..64 {
                body_rgba.extend_from_slice(&[x as u8, y as u8, 191, 255]);
            }
        }
        write_test_rgba_png(
            &assets
                .join("textures")
                .join("entity")
                .join("player")
                .join("body")
                .join("missing.png"),
            64,
            64,
            &body_rgba,
        );

        assert_eq!(
            runtime.player_skin_for_profile(&profile),
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        );
        assert!(runtime
            .drain_dynamic_player_skin_download_results()
            .is_empty());
        assert_eq!(runtime.downloaded_player_skin_count(), 0);
        assert_eq!(skin_download_calls.load(Ordering::Relaxed), 0);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn native_item_runtime_projects_llama_body_decor_colors() {
        let root = unique_temp_dir("item-runtime-llama-decor");
        let assets = assets_dir(&root);
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
                public static final Item WHITE_CARPET = registerBlock(Blocks.WHITE_CARPET, p -> p.component(DataComponents.EQUIPPABLE, Equippable.llamaSwag(DyeColor.WHITE)));
                public static final Item BLACK_CARPET = registerBlock(Blocks.BLACK_CARPET, p -> p.component(DataComponents.EQUIPPABLE, Equippable.llamaSwag(DyeColor.BLACK)));
                public static final Item HORSE_ARMOR = registerItem("horse_armor", new Item.Properties().horseArmor(ArmorMaterials.DIAMOND));
            }"#,
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let registry = runtime.registry.as_ref().unwrap();
        let white = registry.protocol_id("minecraft:white_carpet").unwrap();
        let black = registry.protocol_id("minecraft:black_carpet").unwrap();
        let horse_armor = registry.protocol_id("minecraft:horse_armor").unwrap();
        let colors = runtime.llama_body_decor_colors_by_protocol_id();

        assert_eq!(runtime.llama_body_decor_color_count(), 2);
        assert_eq!(colors.get(&white), Some(&WorldLlamaBodyDecorColor::White));
        assert_eq!(colors.get(&black), Some(&WorldLlamaBodyDecorColor::Black));
        assert_eq!(colors.get(&horse_armor), None);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_projects_nautilus_body_armor_materials() {
        let root = unique_temp_dir("item-runtime-nautilus-body-armor");
        let assets = assets_dir(&root);
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
                public static final Item IRON_NAUTILUS_ARMOR = registerItem("iron_nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.IRON));
                public static final Item GOLDEN_NAUTILUS_ARMOR = registerItem("golden_nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.GOLD));
                public static final Item NETHERITE_NAUTILUS_ARMOR = registerItem("netherite_nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.NETHERITE).fireResistant());
                public static final Item CHAINMAIL_NAUTILUS_ARMOR = registerItem("chainmail_nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.CHAINMAIL));
                public static final Item HORSE_ARMOR = registerItem("horse_armor", new Item.Properties().horseArmor(ArmorMaterials.DIAMOND));
            }"#,
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let registry = runtime.registry.as_ref().unwrap();
        let iron = registry
            .protocol_id("minecraft:iron_nautilus_armor")
            .unwrap();
        let gold = registry
            .protocol_id("minecraft:golden_nautilus_armor")
            .unwrap();
        let netherite = registry
            .protocol_id("minecraft:netherite_nautilus_armor")
            .unwrap();
        let chainmail = registry
            .protocol_id("minecraft:chainmail_nautilus_armor")
            .unwrap();
        let horse_armor = registry.protocol_id("minecraft:horse_armor").unwrap();
        let materials = runtime.nautilus_body_armor_materials_by_protocol_id();

        assert_eq!(runtime.nautilus_body_armor_material_count(), 3);
        assert_eq!(materials.get(&iron), Some(&WorldArmorMaterialKind::Iron));
        assert_eq!(materials.get(&gold), Some(&WorldArmorMaterialKind::Gold));
        assert_eq!(
            materials.get(&netherite),
            Some(&WorldArmorMaterialKind::Netherite)
        );
        assert_eq!(materials.get(&chainmail), None);
        assert_eq!(materials.get(&horse_armor), None);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_projects_horse_body_armor_materials() {
        let root = unique_temp_dir("item-runtime-horse-body-armor");
        let assets = assets_dir(&root);
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
                public static final Item LEATHER_HORSE_ARMOR = registerItem("leather_horse_armor", new Item.Properties().horseArmor(ArmorMaterials.LEATHER));
                public static final Item COPPER_HORSE_ARMOR = registerItem("copper_horse_armor", new Item.Properties().horseArmor(ArmorMaterials.COPPER));
                public static final Item DIAMOND_HORSE_ARMOR = registerItem("diamond_horse_armor", new Item.Properties().horseArmor(ArmorMaterials.DIAMOND));
                public static final Item NETHERITE_HORSE_ARMOR = registerItem("netherite_horse_armor", new Item.Properties().horseArmor(ArmorMaterials.NETHERITE).fireResistant());
                public static final Item CHAINMAIL_HORSE_ARMOR = registerItem("chainmail_horse_armor", new Item.Properties().horseArmor(ArmorMaterials.CHAINMAIL));
                public static final Item IRON_NAUTILUS_ARMOR = registerItem("iron_nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.IRON));
            }"#,
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let registry = runtime.registry.as_ref().unwrap();
        let leather = registry
            .protocol_id("minecraft:leather_horse_armor")
            .unwrap();
        let copper = registry
            .protocol_id("minecraft:copper_horse_armor")
            .unwrap();
        let diamond = registry
            .protocol_id("minecraft:diamond_horse_armor")
            .unwrap();
        let netherite = registry
            .protocol_id("minecraft:netherite_horse_armor")
            .unwrap();
        let chainmail = registry
            .protocol_id("minecraft:chainmail_horse_armor")
            .unwrap();
        let nautilus = registry
            .protocol_id("minecraft:iron_nautilus_armor")
            .unwrap();
        let materials = runtime.horse_body_armor_materials_by_protocol_id();

        assert_eq!(runtime.horse_body_armor_material_count(), 4);
        assert_eq!(
            materials.get(&leather),
            Some(&WorldArmorMaterialKind::Leather)
        );
        assert_eq!(
            materials.get(&copper),
            Some(&WorldArmorMaterialKind::Copper)
        );
        assert_eq!(
            materials.get(&diamond),
            Some(&WorldArmorMaterialKind::Diamond)
        );
        assert_eq!(
            materials.get(&netherite),
            Some(&WorldArmorMaterialKind::Netherite)
        );
        assert_eq!(materials.get(&chainmail), None);
        assert_eq!(materials.get(&nautilus), None);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_projects_wolf_body_armor_materials_and_max_damage() {
        let root = unique_temp_dir("item-runtime-wolf-body-armor");
        let assets = assets_dir(&root);
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
                public static final Item WOLF_ARMOR = registerItem("wolf_armor", new Item.Properties().wolfArmor(ArmorMaterials.ARMADILLO_SCUTE));
                public static final Item HORSE_ARMOR = registerItem("horse_armor", new Item.Properties().horseArmor(ArmorMaterials.DIAMOND));
            }"#,
        );
        write_json(
            &assets.join("equipment").join("armadillo_scute.json"),
            r#"{
                "layers": {
                    "wolf_body": [
                        { "texture": "minecraft:armadillo_scute" },
                        { "texture": "minecraft:armadillo_scute_overlay", "dyeable": {} }
                    ]
                }
            }"#,
        );
        write_json(
            &assets.join("equipment").join("diamond.json"),
            r#"{
                "layers": {
                    "horse_body": [
                        { "texture": "minecraft:diamond" }
                    ]
                }
            }"#,
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let registry = runtime.registry.as_ref().unwrap();
        let wolf_armor = registry.protocol_id("minecraft:wolf_armor").unwrap();
        let horse_armor = registry.protocol_id("minecraft:horse_armor").unwrap();
        let materials = runtime.wolf_body_armor_materials_by_protocol_id();
        let max_damage = runtime.item_max_damage_by_protocol_id();

        assert_eq!(runtime.wolf_body_armor_material_count(), 1);
        assert_eq!(
            materials.get(&wolf_armor),
            Some(&WorldArmorMaterialKind::ArmadilloScute)
        );
        assert_eq!(materials.get(&horse_armor), None);
        assert_eq!(max_damage.get(&wolf_armor), Some(&64));
        assert_eq!(runtime.item_max_damage_count(), 1);

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
    fn native_item_runtime_resolves_trim_material_select() {
        let root = unique_temp_dir("item-runtime-trim-material");
        write_trim_material_select_fixture(&root);

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let uv = |model_id: &str| {
            runtime
                .textures
                .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
                .unwrap()
        };
        // Trim-material registry keys by holder id (registration order).
        let trim_keys = [
            "minecraft:quartz".to_string(),
            "minecraft:iron".to_string(),
            "minecraft:diamond".to_string(),
        ];
        let trimmed = |material_id: Option<i32>| ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                armor_trim_material_id: material_id,
                ..DataComponentPatchSummary::default()
            },
        };
        let selected = |stack: &ItemStackSummary| {
            runtime
                .icon_for_stack_with_context(
                    stack,
                    None,
                    false,
                    0.0,
                    Some(&trim_keys),
                    None,
                    None,
                    None,
                )
                .unwrap()
                .layers[0]
                .uv
        };

        // No trim component → no match → fallback (plain chestplate).
        assert_eq!(selected(&trimmed(None)), uv("iron_chestplate"));
        // Holder id 0 → "minecraft:quartz" → quartz trim model.
        assert_eq!(
            selected(&trimmed(Some(0))),
            uv("iron_chestplate_quartz_trim")
        );
        // Holder id 2 → "minecraft:diamond" → diamond trim model.
        assert_eq!(
            selected(&trimmed(Some(2))),
            uv("iron_chestplate_diamond_trim")
        );
        // Holder id 1 → "minecraft:iron" (no case) → fallback.
        assert_eq!(selected(&trimmed(Some(1))), uv("iron_chestplate"));
        // Without the registry keys (no world context) → fallback.
        assert_eq!(
            runtime.icon_for_stack(&trimmed(Some(0))).unwrap().layers[0].uv,
            uv("iron_chestplate")
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_resolves_charge_type_select() {
        let root = unique_temp_dir("item-runtime-charge-type");
        write_charge_type_select_fixture(&root);

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let uv = |model_id: &str| {
            runtime
                .textures
                .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
                .unwrap()
        };
        // A charged-projectiles template list with one entry of the given item id.
        let charged = |item_id: i32| ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                charged_projectiles_items: vec![ItemStackTemplateSummary {
                    item_id,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            },
        };
        let selected =
            |stack: &ItemStackSummary| runtime.icon_for_stack(stack).unwrap().layers[0].uv;

        // Empty crossbow → NONE → no matching case → fallback (plain crossbow).
        assert_eq!(
            selected(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }),
            uv("crossbow")
        );
        // Charged with an arrow (item 2) → ARROW → "arrow" case.
        assert_eq!(selected(&charged(2)), uv("crossbow_arrow"));
        // Charged with a firework_rocket (item 1) → ROCKET → "rocket" case.
        assert_eq!(selected(&charged(1)), uv("crossbow_firework"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_resolves_use_tick_range_dispatch_properties() {
        let root = unique_temp_dir("item-runtime-use-tick-range-dispatch");
        write_use_tick_range_dispatch_fixture(&root);

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let uv = |model_id: &str| {
            runtime
                .textures
                .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
                .unwrap()
        };
        let stack = |item_id: i32| ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let selected = |stack: &ItemStackSummary, using_item: bool, elapsed_ticks: u32| {
            let use_context = if using_item {
                runtime.item_model_use_context_for_stack(stack, elapsed_ticks)
            } else {
                ItemModelUseContext::inactive()
            };
            runtime
                .icon_for_stack_with_context_and_use_context(
                    stack,
                    None,
                    using_item,
                    use_context,
                    0.0,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap()
                .layers[0]
                .uv
        };

        let bow = stack(0);
        assert_eq!(selected(&bow, false, 20), uv("bow"));
        assert_eq!(selected(&bow, true, 0), uv("bow_pulling_0"));
        assert_eq!(selected(&bow, true, 13), uv("bow_pulling_1"));
        assert_eq!(selected(&bow, true, 18), uv("bow_pulling_2"));

        let crossbow = stack(1);
        assert_eq!(selected(&crossbow, false, 25), uv("crossbow"));
        assert_eq!(selected(&crossbow, true, 0), uv("crossbow_pulling_0"));
        assert_eq!(selected(&crossbow, true, 15), uv("crossbow_pulling_1"));
        assert_eq!(selected(&crossbow, true, 25), uv("crossbow_pulling_2"));
        let charged_crossbow = ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: DataComponentPatchSummary {
                charged_projectiles_items: vec![ItemStackTemplateSummary {
                    item_id: 3,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            },
        };
        assert_eq!(selected(&charged_crossbow, true, 25), uv("crossbow_arrow"));

        let brush = stack(4);
        assert_eq!(selected(&brush, false, 1), uv("brush"));
        assert_eq!(selected(&brush, true, 0), uv("brush"));
        assert_eq!(selected(&brush, true, 7), uv("brush_brushing_0"));
        assert_eq!(selected(&brush, true, 1), uv("brush_brushing_2"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_resolves_main_hand_select_from_owner_context() {
        let root = unique_temp_dir("item-runtime-main-hand-select");
        write_main_hand_select_fixture(&root);

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let uv = |model_id: &str| {
            runtime
                .textures
                .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
                .unwrap()
        };
        let stack = ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let selected = |owner_main_hand_left| {
            runtime
                .icon_for_stack_with_owner_main_hand(&stack, owner_main_hand_left)
                .unwrap()
                .layers[0]
                .uv
        };

        // Vanilla `MainHand.get` returns null when there is no living owner, so
        // no case matches and the fallback model is used.
        assert_eq!(selected(None), uv("hand_selector"));
        assert_eq!(selected(Some(false)), uv("hand_selector_right"));
        assert_eq!(selected(Some(true)), uv("hand_selector_left"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_resolves_stack_string_select_properties() {
        let root = unique_temp_dir("item-runtime-stack-string-select");
        write_stack_string_select_fixture(&root);

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let uv = |model_id: &str| {
            runtime
                .textures
                .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
                .unwrap()
        };
        let selected =
            |stack: &ItemStackSummary| runtime.icon_for_stack(stack).unwrap().layers[0].uv;

        let block_state_stack = |properties: BTreeMap<String, String>| ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                block_state_properties: properties,
                ..DataComponentPatchSummary::default()
            },
        };
        // Vanilla `ItemBlockState.get` returns null without the component/property,
        // so no case matches and the fallback model is used.
        assert_eq!(
            selected(&block_state_stack(BTreeMap::new())),
            uv("beehive_empty")
        );
        assert_eq!(
            selected(&block_state_stack(BTreeMap::from([(
                "honey_level".to_string(),
                "5".to_string()
            )]))),
            uv("beehive_honey")
        );
        assert_eq!(
            selected(&block_state_stack(BTreeMap::from([(
                "wrong_property".to_string(),
                "5".to_string()
            )]))),
            uv("beehive_empty")
        );

        let custom_model_data_stack = |strings: Vec<&str>| ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: DataComponentPatchSummary {
                custom_model_data_strings: strings.into_iter().map(str::to_string).collect(),
                ..DataComponentPatchSummary::default()
            },
        };
        // Vanilla `CustomModelDataProperty.get` reads strings[index], not floats.
        assert_eq!(
            selected(&custom_model_data_stack(vec!["ignored", "blue"])),
            uv("cmd_blue")
        );
        assert_eq!(
            selected(&custom_model_data_stack(vec!["ignored", "lime"])),
            uv("cmd_green")
        );
        // Out-of-range index and absent component both produce no selected value.
        assert_eq!(
            selected(&custom_model_data_stack(vec!["blue"])),
            uv("cmd_plain")
        );
        assert_eq!(
            selected(&custom_model_data_stack(Vec::new())),
            uv("cmd_plain")
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_resolves_value_aware_range_dispatch() {
        let root = unique_temp_dir("item-runtime-range-dispatch");
        write_value_aware_range_dispatch_fixture(&root);

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let uv = |model_id: &str| {
            runtime
                .textures
                .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
                .unwrap()
        };
        let damage_stack = |damage: Option<i32>, max_damage: Option<i32>| ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                damage,
                max_damage,
                ..DataComponentPatchSummary::default()
            },
        };
        let selected =
            |stack: &ItemStackSummary| runtime.icon_for_stack(stack).unwrap().layers[0].uv;

        // damage 50/100 = 0.5 lands exactly on the 0.5 threshold (vanilla
        // `lastIndexLessOrEqual` is inclusive), proving sort + boundary.
        assert_eq!(
            selected(&damage_stack(Some(50), Some(100))),
            uv("damage_half")
        );
        // damage 95/100 = 0.95 reaches the top entry.
        assert_eq!(
            selected(&damage_stack(Some(95), Some(100))),
            uv("damage_low")
        );
        // damage 40/100 = 0.4 precedes the first threshold (0.5) → fallback (-1).
        assert_eq!(
            selected(&damage_stack(Some(40), Some(100))),
            uv("damage_fallback")
        );
        // No max_damage → 0/0 = NaN → fallback.
        assert_eq!(selected(&damage_stack(None, None)), uv("damage_fallback"));

        let cmd_stack = |floats: Vec<f32>| ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: DataComponentPatchSummary {
                custom_model_data_floats: floats.into(),
                ..DataComponentPatchSummary::default()
            },
        };
        // floats[1] = 0.5, scale 2.0 → 1.0 lands on the 1.0 threshold; floats[0]
        // is ignored (index 1), proving index handling, scale, and boundary.
        assert_eq!(selected(&cmd_stack(vec![9.0, 0.5])), uv("cmd_1"));
        // floats[1] = 2.0 * 2.0 = 4.0 reaches the 3.0 entry.
        assert_eq!(selected(&cmd_stack(vec![9.0, 2.0])), uv("cmd_3"));
        // Missing index 1 → 0.0 → the 0.0 entry.
        assert_eq!(selected(&cmd_stack(vec![9.0])), uv("cmd_0"));
        // No custom_model_data at all → 0.0 → the 0.0 entry.
        assert_eq!(selected(&cmd_stack(Vec::new())), uv("cmd_0"));

        let count_stack = |count: i32, max_stack_size: Option<i32>| ItemStackSummary {
            item_id: Some(2),
            count,
            component_patch: DataComponentPatchSummary {
                max_stack_size,
                ..DataComponentPatchSummary::default()
            },
        };
        // Count.get defaults to normalized `count / maxStackSize`, using the
        // item prototype default when the component is absent.
        assert_eq!(selected(&count_stack(32, None)), uv("count_half"));
        assert_eq!(selected(&count_stack(64, None)), uv("count_full"));
        assert_eq!(selected(&count_stack(1, None)), uv("count_fallback"));
        // The max_stack_size component overrides the prototype default.
        assert_eq!(selected(&count_stack(8, Some(16))), uv("count_half"));

        let apple_template = |count: i32| ItemStackTemplateSummary {
            item_id: 4,
            count,
            component_patch: DataComponentPatchSummary::default(),
        };
        let bundle_stack = |items: Vec<ItemStackTemplateSummary>| ItemStackSummary {
            item_id: Some(3),
            count: 1,
            component_patch: DataComponentPatchSummary {
                bundle_contents_item_count: Some(items.len()),
                bundle_contents_items: items,
                ..DataComponentPatchSummary::default()
            },
        };
        // BundleFullness.get sums BundleContents weights; regular items weigh
        // `count / getMaxStackSize`.
        assert_eq!(
            selected(&bundle_stack(vec![apple_template(32)])),
            uv("bundle_half")
        );
        assert_eq!(
            selected(&bundle_stack(vec![apple_template(16)])),
            uv("bundle_fallback")
        );
        // A nested bundle weighs its contents plus the fixed 1/16 bundle item weight.
        assert_eq!(
            selected(&bundle_stack(vec![ItemStackTemplateSummary {
                item_id: 3,
                count: 1,
                component_patch: DataComponentPatchSummary {
                    bundle_contents_item_count: Some(1),
                    bundle_contents_items: vec![apple_template(32)],
                    ..DataComponentPatchSummary::default()
                },
            }])),
            uv("bundle_nested")
        );
        // A beehive-like stack with non-empty BEES component weighs as a full bundle.
        assert_eq!(
            selected(&bundle_stack(vec![ItemStackTemplateSummary {
                item_id: 5,
                count: 1,
                component_patch: DataComponentPatchSummary {
                    bees_count: 1,
                    ..DataComponentPatchSummary::default()
                },
            }])),
            uv("bundle_full")
        );

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

    fn write_trim_material_select_fixture(root: &Path) {
        let assets = assets_dir(root);
        write_item_atlases(&assets);
        write_single_item_registry_source(root, "iron_chestplate");
        write_json(
            &assets.join("items").join("iron_chestplate.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:trim_material",
                    "cases": [
                        {
                            "when": "minecraft:quartz",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate_quartz_trim" }
                        },
                        {
                            "when": "minecraft:diamond",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate_diamond_trim" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "iron_chestplate", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(
            &assets,
            "iron_chestplate_quartz_trim",
            &[200, 200, 190, 255],
        );
        write_flat_item_model_and_texture(
            &assets,
            "iron_chestplate_diamond_trim",
            &[120, 200, 210, 255],
        );
    }

    fn write_charge_type_select_fixture(root: &Path) {
        let assets = assets_dir(root);
        write_item_atlases(&assets);
        // Item 0 = crossbow, item 1 = firework_rocket, item 2 = arrow.
        write_item_registry_source(root, &["crossbow", "firework_rocket", "arrow"]);
        write_json(
            &assets.join("items").join("crossbow.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:charge_type",
                    "cases": [
                        {
                            "when": "arrow",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_arrow" }
                        },
                        {
                            "when": "rocket",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_firework" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/crossbow" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "crossbow", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow_arrow", &[80, 120, 40, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow_firework", &[120, 40, 80, 255]);
    }

    fn write_use_tick_range_dispatch_fixture(root: &Path) {
        let assets = assets_dir(root);
        write_item_atlases(&assets);
        // Item ids: 0 = bow, 1 = crossbow, 2 = firework_rocket, 3 = arrow, 4 = brush.
        write_item_registry_source(
            root,
            &["bow", "crossbow", "firework_rocket", "arrow", "brush"],
        );
        write_json(
            &assets.join("items").join("bow.json"),
            r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:using_item",
                    "on_false": { "type": "minecraft:model", "model": "minecraft:item/bow" },
                    "on_true": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:use_duration",
                        "scale": 0.05,
                        "entries": [
                            {
                                "threshold": 0.65,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_1" }
                            },
                            {
                                "threshold": 0.9,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_2" }
                            }
                        ],
                        "fallback": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_0" }
                    }
                }
            }"#,
        );
        write_json(
            &assets.join("items").join("crossbow.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:charge_type",
                    "cases": [
                        {
                            "when": "arrow",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_arrow" }
                        },
                        {
                            "when": "rocket",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_firework" }
                        }
                    ],
                    "fallback": {
                        "type": "minecraft:condition",
                        "property": "minecraft:using_item",
                        "on_false": { "type": "minecraft:model", "model": "minecraft:item/crossbow" },
                        "on_true": {
                            "type": "minecraft:range_dispatch",
                            "property": "minecraft:crossbow/pull",
                            "entries": [
                                {
                                    "threshold": 0.58,
                                    "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_1" }
                                },
                                {
                                    "threshold": 1.0,
                                    "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_2" }
                                }
                            ],
                            "fallback": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_0" }
                        }
                    }
                }
            }"#,
        );
        write_json(
            &assets.join("items").join("brush.json"),
            r#"{
                "model": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:use_cycle",
                    "period": 10.0,
                    "scale": 0.1,
                    "entries": [
                        {
                            "threshold": 0.25,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/brush_brushing_0" }
                        },
                        {
                            "threshold": 0.5,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/brush_brushing_1" }
                        },
                        {
                            "threshold": 0.75,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/brush_brushing_2" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/brush" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "bow", &[80, 120, 160, 255]);
        write_flat_item_model_and_texture(&assets, "bow_pulling_0", &[160, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "bow_pulling_1", &[120, 160, 80, 255]);
        write_flat_item_model_and_texture(&assets, "bow_pulling_2", &[160, 120, 80, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow_arrow", &[80, 120, 40, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow_firework", &[120, 40, 80, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow_pulling_0", &[70, 100, 130, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow_pulling_1", &[100, 130, 70, 255]);
        write_flat_item_model_and_texture(&assets, "crossbow_pulling_2", &[130, 70, 100, 255]);
        write_flat_item_model_and_texture(&assets, "brush", &[90, 90, 90, 255]);
        write_flat_item_model_and_texture(&assets, "brush_brushing_0", &[120, 90, 90, 255]);
        write_flat_item_model_and_texture(&assets, "brush_brushing_1", &[90, 120, 90, 255]);
        write_flat_item_model_and_texture(&assets, "brush_brushing_2", &[90, 90, 120, 255]);
    }

    fn write_main_hand_select_fixture(root: &Path) {
        let assets = assets_dir(root);
        write_item_atlases(&assets);
        write_single_item_registry_source(root, "hand_selector");
        write_json(
            &assets.join("items").join("hand_selector.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:main_hand",
                    "cases": [
                        {
                            "when": "left",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_left" }
                        },
                        {
                            "when": "right",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_right" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/hand_selector" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "hand_selector", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "hand_selector_left", &[120, 40, 80, 255]);
        write_flat_item_model_and_texture(&assets, "hand_selector_right", &[80, 120, 40, 255]);
    }

    fn write_stack_string_select_fixture(root: &Path) {
        let assets = assets_dir(root);
        write_item_atlases(&assets);
        // Item 0 = beehive, item 1 = cmd_selector.
        write_item_registry_source(root, &["beehive", "cmd_selector"]);
        write_json(
            &assets.join("items").join("beehive.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:block_state",
                    "block_state_property": "honey_level",
                    "cases": [
                        {
                            "when": "5",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/beehive_honey" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/beehive_empty" }
                }
            }"#,
        );
        write_json(
            &assets.join("items").join("cmd_selector.json"),
            r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:custom_model_data",
                    "index": 1,
                    "cases": [
                        {
                            "when": "blue",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/cmd_blue" }
                        },
                        {
                            "when": ["green", "lime"],
                            "model": { "type": "minecraft:model", "model": "minecraft:item/cmd_green" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/cmd_plain" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "beehive_empty", &[90, 70, 40, 255]);
        write_flat_item_model_and_texture(&assets, "beehive_honey", &[220, 180, 40, 255]);
        write_flat_item_model_and_texture(&assets, "cmd_plain", &[40, 40, 40, 255]);
        write_flat_item_model_and_texture(&assets, "cmd_blue", &[40, 80, 220, 255]);
        write_flat_item_model_and_texture(&assets, "cmd_green", &[40, 180, 80, 255]);
    }

    fn write_value_aware_range_dispatch_fixture(root: &Path) {
        let assets = assets_dir(root);
        write_item_atlases(&assets);
        write_item_registry_source(
            root,
            &[
                "damage_dispatch",
                "cmd_dispatch",
                "count_dispatch",
                "bundle_dispatch",
                "apple",
                "bee_nest",
            ],
        );
        // `minecraft:damage` (normalize default true). Entries listed out of
        // threshold order to prove the resolver sorts before selecting.
        write_json(
            &assets.join("items").join("damage_dispatch.json"),
            r#"{
                "model": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:damage",
                    "entries": [
                        {
                            "threshold": 0.9,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/damage_low" }
                        },
                        {
                            "threshold": 0.5,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/damage_half" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/damage_fallback" }
                }
            }"#,
        );
        // `minecraft:custom_model_data` index 1, scale 2.0 (value = floats[1] * 2).
        write_json(
            &assets.join("items").join("cmd_dispatch.json"),
            r#"{
                "model": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:custom_model_data",
                    "index": 1,
                    "scale": 2.0,
                    "entries": [
                        {
                            "threshold": 3.0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/cmd_3" }
                        },
                        {
                            "threshold": 0.0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/cmd_0" }
                        },
                        {
                            "threshold": 1.0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/cmd_1" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/cmd_fallback" }
                }
            }"#,
        );
        // `minecraft:count` normalize default true (value = count / maxStackSize).
        write_json(
            &assets.join("items").join("count_dispatch.json"),
            r#"{
                "model": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:count",
                    "entries": [
                        {
                            "threshold": 1.0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/count_full" }
                        },
                        {
                            "threshold": 0.5,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/count_half" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/count_fallback" }
                }
            }"#,
        );
        // `minecraft:bundle/fullness` uses BundleContents weight.
        write_json(
            &assets.join("items").join("bundle_dispatch.json"),
            r#"{
                "model": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:bundle/fullness",
                    "entries": [
                        {
                            "threshold": 1.0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/bundle_full" }
                        },
                        {
                            "threshold": 0.5,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/bundle_half" }
                        },
                        {
                            "threshold": 0.55,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/bundle_nested" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/bundle_fallback" }
                }
            }"#,
        );
        write_flat_item_model_and_texture(&assets, "damage_half", &[40, 80, 120, 255]);
        write_flat_item_model_and_texture(&assets, "damage_low", &[120, 80, 40, 255]);
        write_flat_item_model_and_texture(&assets, "damage_fallback", &[80, 120, 40, 255]);
        write_flat_item_model_and_texture(&assets, "cmd_0", &[10, 20, 30, 255]);
        write_flat_item_model_and_texture(&assets, "cmd_1", &[40, 50, 60, 255]);
        write_flat_item_model_and_texture(&assets, "cmd_3", &[70, 80, 90, 255]);
        write_flat_item_model_and_texture(&assets, "cmd_fallback", &[100, 110, 120, 255]);
        write_flat_item_model_and_texture(&assets, "count_half", &[20, 80, 140, 255]);
        write_flat_item_model_and_texture(&assets, "count_full", &[40, 160, 220, 255]);
        write_flat_item_model_and_texture(&assets, "count_fallback", &[100, 40, 40, 255]);
        write_flat_item_model_and_texture(&assets, "bundle_half", &[140, 80, 20, 255]);
        write_flat_item_model_and_texture(&assets, "bundle_nested", &[180, 120, 40, 255]);
        write_flat_item_model_and_texture(&assets, "bundle_full", &[220, 180, 40, 255]);
        write_flat_item_model_and_texture(&assets, "bundle_fallback", &[40, 40, 100, 255]);
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

    struct TestSkinPngFetcher {
        bytes: Vec<u8>,
        calls: Arc<AtomicUsize>,
    }

    impl SkinPngFetcher for TestSkinPngFetcher {
        fn fetch_skin_png(&mut self, _url: &str) -> Result<Vec<u8>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(self.bytes.clone())
        }
    }

    fn player_skin_png_bytes() -> Vec<u8> {
        let mut image = image::RgbaImage::new(64, 64);
        for y in 0..64 {
            for x in 0..64 {
                image.put_pixel(x, y, image::Rgba([x as u8, y as u8, 31, 255]));
            }
        }
        let mut cursor = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    fn player_profile_texture_png_bytes() -> Vec<u8> {
        let mut image = image::RgbaImage::new(64, 32);
        for y in 0..32 {
            for x in 0..64 {
                image.put_pixel(x, y, image::Rgba([x as u8, y as u8, 63, 255]));
            }
        }
        let mut cursor = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    fn drain_until_player_skin_download_result(
        runtime: &NativeItemRuntime,
    ) -> Vec<NativeDynamicPlayerSkinDownload> {
        for _ in 0..100 {
            let downloads = runtime.drain_dynamic_player_skin_download_results();
            if !downloads.is_empty() {
                return downloads;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for player skin download result");
    }

    fn drain_until_player_texture_download_results(
        runtime: &NativeItemRuntime,
        expected_len: usize,
    ) -> Vec<NativeDynamicPlayerTextureDownload> {
        let mut downloads = Vec::new();
        for _ in 0..100 {
            downloads.extend(runtime.drain_dynamic_player_texture_download_results());
            if downloads.len() >= expected_len {
                return downloads;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for player profile texture download results");
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
