use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use bbb_pack::{
    AtlasImage, AtlasLayout, AtlasPacker, AtlasSprite, BlockModelDisplayContext,
    BlockModelDisplayTransform, BlockModelDisplayTransforms, EquipmentAssetCatalog,
    EquipmentLayerType, FreezeImmuneWearableCatalog, FurnaceFuelCatalog,
    ItemAttackRange as PackItemAttackRange, ItemCuboidModel, ItemCuboidModelCatalog,
    ItemCuboidModelSet, ItemCuboidTextureImageCatalog,
    ItemDefaultAttributeModifier as PackItemDefaultAttributeModifier,
    ItemEquipmentSlot as PackItemEquipmentSlot, ItemMiningProfile as PackItemMiningProfile,
    ItemMiningRule as PackItemMiningRule, ItemModelCatalog, ItemModelDefinition,
    ItemMountBodyArmorKind as PackItemMountBodyArmorKind, ItemRegistryCatalog, ItemTintSource,
    ItemUseEffects as PackItemUseEffects, LanguageCatalog, PackResourceStack, PackRoots,
    ResourceLocation, SpriteImage, TagCatalog, TerrainColorMaps, DEFAULT_LANGUAGE_CODE,
};
use bbb_protocol::packets::{
    AttributeModifierSummary, ConsumableSummary, DataComponentPatchSummary, ItemRaritySummary,
    ItemStackSummary, ItemStackTemplateSummary, ResolvableProfileSummary, ResourceTextureSummary,
};
// These summary types are referenced only by this crate's tests; keep them out
// of the non-test import set so the standalone library build stays warning-free.
#[cfg(test)]
use bbb_protocol::packets::{
    FireworkExplosionShapeSummary, FireworkExplosionSummary, JukeboxSongSummary,
    LodestoneTargetSummary, MobEffectDetailsSummary, MobEffectInstanceSummary, NbtSummaryEntry,
    NbtSummaryValue, SoundEventSummary, TrimMaterialSummary, TrimPatternSummary,
    WrittenBookContentSummary,
};
use bbb_render_types::{
    DynamicPlayerSkinImage, DynamicPlayerTextureImage, EntityCustomHeadSkull,
    EntityDefaultPlayerSkin, EntityDynamicPlayerSkin, EntityDynamicPlayerSkinStatus,
    EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind, EntityEquipmentLayerTexture,
    EntityModelTextureRef, EntityPlayerSkin, EntityPlayerSkinModel, HudAsciiGlyph,
    ItemFrameMapDecorationTexture, ItemSpriteRect, SpriteAlphaMask, HUD_ASCII_GLYPH_COUNT,
};
// Referenced only by test builds and the `test-support` constructors; gate it so
// the plain library build stays clean.
#[cfg(any(test, feature = "test-support"))]
use bbb_render_types::HudUvRect;
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
    contains_runtime_condition, default_item_name_translation_key,
    item_icon_model_ref_for_definition, CompassTarget, CrossbowChargeType, IconResolveContext,
    ItemIconModel, ItemIconModelRef, TimeSource,
};
pub use profile_skin::default_player_skin_for_profile_id;
use profile_skin::ProfileSkinCache;
use profile_skin::{entity_player_skin_model, profile_default_player_skin, profile_texture_handle};

use crate::ascii_font::{hud_ascii_atlas_from_image, load_ascii_font_texture};

mod icon;
mod profiles;
mod tables;
mod tooltip;

use icon::*;
use profiles::*;
pub use profiles::{NativeDynamicPlayerSkinDownload, NativeDynamicPlayerTextureDownload};
use tables::*;
pub use tooltip::NativeItemTooltipLine;
#[cfg(test)]
use tooltip::*;

const FIREWORK_ROCKET_ITEM_ID: &str = "minecraft:firework_rocket";

const BOW_ITEM_ID: &str = "minecraft:bow";

const BRUSH_ITEM_ID: &str = "minecraft:brush";

const CROSSBOW_ITEM_ID: &str = "minecraft:crossbow";

const SPYGLASS_ITEM_ID: &str = "minecraft:spyglass";

const TRIDENT_ITEM_ID: &str = "minecraft:trident";

const ENDER_EYE_ITEM_ID: &str = "minecraft:ender_eye";

const QUICK_CHARGE_ENCHANTMENT_ID: &str = "minecraft:quick_charge";

const VANILLA_LONG_USE_DURATION_TICKS: i32 = 72_000;

const VANILLA_BRUSH_USE_DURATION_TICKS: i32 = 200;

const VANILLA_SPYGLASS_USE_DURATION_TICKS: i32 = 1_200;

const VANILLA_ENDER_EYE_USE_DURATION_TICKS: i32 = 0;

const VANILLA_CROSSBOW_CHARGE_DURATION_TICKS: i32 = 25;

const VANILLA_ITEM_MODEL_COMPONENT_ID: i32 = 10;

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

#[cfg(any(test, feature = "test-support"))]
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

#[derive(Debug)]
pub struct NativeItemRuntime {
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
    item_tags: Option<TagCatalog>,
    enchantment_tags: Option<TagCatalog>,
    trim_material_tags: Option<TagCatalog>,
    trim_pattern_tags: Option<TagCatalog>,
    jukebox_song_tags: Option<TagCatalog>,
    potion_tags: Option<TagCatalog>,
    attribute_tags: Option<TagCatalog>,
    villager_type_tags: Option<TagCatalog>,
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
    local_time_epoch_millis_override: Cell<Option<i64>>,
    time_wobblers: RefCell<HashMap<ItemTimeWobblerKey, ItemNeedleWobbler>>,
    time_randoms: RefCell<HashMap<ItemTimeRandomKey, ItemLegacyRandom>>,
    compass_wobblers: RefCell<HashMap<ItemCompassWobblerKey, ItemNeedleWobbler>>,
    compass_randoms: RefCell<HashMap<ItemCompassRandomKey, ItemLegacyRandom>>,
}

const DATA_COMPONENT_PROFILE_TYPE_ID: i32 = 70;

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

#[derive(Debug, Clone, PartialEq)]
pub struct ItemAtlasIcon {
    pub layers: Vec<ItemAtlasIconLayer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemAtlasSpriteUv {
    pub id: String,
    pub uv: ItemAtlasUvRect,
    pub has_translucent: bool,
}

/// Per-stack use-state values for vanilla item-model numeric properties. These
/// are active only for the stack that vanilla would expose as
/// `LivingEntity.getUseItem()`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ItemModelUseContext {
    pub(crate) elapsed_ticks: u32,
    pub(crate) remaining_ticks: Option<f32>,
    pub(crate) crossbow_charge_duration_ticks: Option<f32>,
}

impl ItemModelUseContext {
    pub fn inactive() -> Self {
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemModelKeybindContext {
    pub forward: bool,
    pub left: bool,
    pub backward: bool,
    pub right: bool,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
    pub attack: bool,
    pub use_item: bool,
    pub pick_item: bool,
    pub inventory: bool,
    pub swap_offhand: bool,
    pub drop: bool,
    pub chat: bool,
    pub command: bool,
    pub player_list: bool,
    pub social_interactions: bool,
    pub screenshot: bool,
    pub toggle_perspective: bool,
    pub fullscreen: bool,
    pub advancements: bool,
    pub quick_actions: bool,
    pub toggle_gui: bool,
    pub toggle_spectator_shader_effects: bool,
    pub save_toolbar_activator: bool,
    pub load_toolbar_activator: bool,
    pub spectator_hotbar: bool,
    pub hotbar: [bool; 9],
}

impl ItemModelKeybindContext {
    pub fn keybind_down(&self, keybind: &str) -> bool {
        match keybind {
            "key.forward" => self.forward,
            "key.left" => self.left,
            "key.back" => self.backward,
            "key.right" => self.right,
            "key.jump" => self.jump,
            "key.sneak" => self.sneak,
            "key.sprint" => self.sprint,
            "key.attack" => self.attack,
            "key.use" => self.use_item,
            "key.pickItem" => self.pick_item,
            "key.inventory" => self.inventory,
            "key.swapOffhand" => self.swap_offhand,
            "key.drop" => self.drop,
            "key.chat" => self.chat,
            "key.command" => self.command,
            "key.playerlist" => self.player_list,
            "key.socialInteractions" => self.social_interactions,
            "key.screenshot" => self.screenshot,
            "key.togglePerspective" => self.toggle_perspective,
            // Vanilla Options registers these key mappings with
            // InputConstants.UNKNOWN by default, so they are valid keybind names
            // but cannot be down until user rebinding exists.
            "key.smoothCamera" | "key.spectatorOutlines" => false,
            "key.fullscreen" => self.fullscreen,
            "key.advancements" => self.advancements,
            "key.quickActions" => self.quick_actions,
            "key.toggleGui" => self.toggle_gui,
            "key.toggleSpectatorShaderEffects" => self.toggle_spectator_shader_effects,
            "key.saveToolbarActivator" => self.save_toolbar_activator,
            "key.loadToolbarActivator" => self.load_toolbar_activator,
            "key.spectatorHotbar" => self.spectator_hotbar,
            "key.hotbar.1" => self.hotbar[0],
            "key.hotbar.2" => self.hotbar[1],
            "key.hotbar.3" => self.hotbar[2],
            "key.hotbar.4" => self.hotbar[3],
            "key.hotbar.5" => self.hotbar[4],
            "key.hotbar.6" => self.hotbar[5],
            "key.hotbar.7" => self.hotbar[6],
            "key.hotbar.8" => self.hotbar[7],
            "key.hotbar.9" => self.hotbar[8],
            _ => false,
        }
    }
}

/// World-clock values exposed to vanilla item-model numeric properties such as
/// `minecraft:time`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemModelTimeContext {
    pub game_time: i64,
    pub day_time: i64,
}

/// Owner and level values exposed to vanilla compass item-model numeric
/// properties.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemModelCompassContext<'a> {
    pub game_time: i64,
    pub level_dimension: &'a str,
    pub owner_position: [f64; 3],
    pub owner_y_rot_degrees: f32,
    pub spawn: Option<ItemModelCompassTarget<'a>>,
    pub recovery: Option<ItemModelCompassTarget<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemModelCompassTarget<'a> {
    pub dimension: &'a str,
    pub pos: [i32; 3],
}

/// One layer of a generated (flat) item ready for 3D extrusion: the sprite's alpha silhouette, its
/// atlas UV rect (item atlas), and the resolved layer tint.
pub struct GeneratedItemLayer {
    pub mask: SpriteAlphaMask,
    pub rect: ItemSpriteRect,
    pub tint: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemAtlasIconLayer {
    pub uv: ItemAtlasUvRect,
    pub tint: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemAtlasUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
enum ItemIconTint {
    Static([f32; 4]),
    Source(ItemTintSource),
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

impl NativeItemRuntime {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_with_locale(roots, DEFAULT_LANGUAGE_CODE)
    }

    pub fn load_with_locale(roots: &PackRoots, language_code: &str) -> Result<Self> {
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
        let item_tags = roots
            .load_tag_catalog("item")
            .context("load native item tags")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native item tag catalog");
                err
            })
            .ok();
        let enchantment_tags = roots
            .load_tag_catalog("enchantment")
            .context("load native enchantment tags")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native enchantment tag catalog");
                err
            })
            .ok();
        let trim_material_tags = roots
            .load_tag_catalog("trim_material")
            .context("load native trim material tags")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native trim material tag catalog");
                err
            })
            .ok();
        let trim_pattern_tags = roots
            .load_tag_catalog("trim_pattern")
            .context("load native trim pattern tags")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native trim pattern tag catalog");
                err
            })
            .ok();
        let jukebox_song_tags = roots
            .load_tag_catalog("jukebox_song")
            .context("load native jukebox song tags")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native jukebox song tag catalog");
                err
            })
            .ok();
        let potion_tags = roots
            .load_tag_catalog("potion")
            .context("load native potion tags")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native potion tag catalog");
                err
            })
            .ok();
        let attribute_tags = roots
            .load_tag_catalog("attribute")
            .context("load native attribute tags")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native attribute tag catalog");
                err
            })
            .ok();
        let villager_type_tags = roots
            .load_tag_catalog("villager_type")
            .context("load native villager type tags")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native villager type tag catalog");
                err
            })
            .ok();
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
            item_tags,
            enchantment_tags,
            trim_material_tags,
            trim_pattern_tags,
            jukebox_song_tags,
            potion_tags,
            attribute_tags,
            villager_type_tags,
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
        item_tags: Option<TagCatalog>,
        enchantment_tags: Option<TagCatalog>,
        trim_material_tags: Option<TagCatalog>,
        trim_pattern_tags: Option<TagCatalog>,
        jukebox_song_tags: Option<TagCatalog>,
        potion_tags: Option<TagCatalog>,
        attribute_tags: Option<TagCatalog>,
        villager_type_tags: Option<TagCatalog>,
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
            item_tags,
            enchantment_tags,
            trim_material_tags,
            trim_pattern_tags,
            jukebox_song_tags,
            potion_tags,
            attribute_tags,
            villager_type_tags,
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
            local_time_epoch_millis_override: Cell::new(None),
            time_wobblers: RefCell::default(),
            time_randoms: RefCell::default(),
            compass_wobblers: RefCell::default(),
            compass_randoms: RefCell::default(),
        })
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn empty_for_test() -> Self {
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
            item_tags: None,
            enchantment_tags: None,
            trim_material_tags: None,
            trim_pattern_tags: None,
            jukebox_song_tags: None,
            potion_tags: None,
            attribute_tags: None,
            villager_type_tags: None,
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
            local_time_epoch_millis_override: Cell::new(None),
            time_wobblers: RefCell::default(),
            time_randoms: RefCell::default(),
            compass_wobblers: RefCell::default(),
            compass_randoms: RefCell::default(),
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn for_test_with_registry_and_equipment_assets(
        registry: ItemRegistryCatalog,
        equipment_assets: EquipmentAssetCatalog,
    ) -> Self {
        let mut runtime = Self::empty_for_test();
        runtime.registry = Some(registry);
        runtime.equipment_assets = equipment_assets;
        runtime
    }

    #[cfg(test)]
    fn set_local_time_epoch_millis_for_test(&self, epoch_millis: i64) {
        self.local_time_epoch_millis_override
            .set(Some(epoch_millis));
    }

    pub fn item_definition_count(&self) -> usize {
        self.item_definition_count
    }

    pub fn item_registry_count(&self) -> usize {
        self.item_registry_count
    }

    /// The resource id (e.g. `minecraft:stone`) for an item protocol id, via the item registry. Used to
    /// map a dropped item to the block of the same id for 3D block-item rendering.
    pub fn item_resource_id(&self, protocol_id: i32) -> Option<&str> {
        self.registry.as_ref()?.resource_id(protocol_id)
    }

    /// The protocol id for an item resource id via the loaded item registry.
    pub fn item_protocol_id(&self, resource_id: &str) -> Option<i32> {
        self.registry.as_ref()?.protocol_id(resource_id)
    }

    /// The item's own model display transform for a context (vanilla `ItemTransform`), retained from the
    /// resolved item cuboid model. `None` if the item has no registry entry or no resolved model (the
    /// caller then falls back to the parent-model default). Used to place the 3D model in hand / frame /
    /// GUI exactly as vanilla's `model.applyTransform`.
    #[cfg(test)]
    pub(crate) fn item_display_transform(
        &self,
        protocol_id: i32,
        context: BlockModelDisplayContext,
    ) -> Option<BlockModelDisplayTransform> {
        let item_id = self.registry.as_ref()?.resource_id(protocol_id)?;
        Some(self.item_display_transforms.get(item_id)?.get(context))
    }
}

#[cfg(test)]
mod tests;
