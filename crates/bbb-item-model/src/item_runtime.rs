use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap},
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use bbb_pack::{
    AtlasImage, AtlasLayout, AtlasPacker, AtlasSprite, BlockModelDisplayContext,
    BlockModelDisplayTransform, BlockModelDisplayTransforms, ClientItemProperties,
    EquipmentAssetCatalog, EquipmentLayerType, FreezeImmuneWearableCatalog, FurnaceFuelCatalog,
    ItemAttackRange as PackItemAttackRange, ItemConsumable as PackItemConsumable, ItemCuboidModel,
    ItemCuboidModelCatalog, ItemCuboidTextureImageCatalog,
    ItemDefaultAttributeModifier as PackItemDefaultAttributeModifier,
    ItemEquipmentSlot as PackItemEquipmentSlot, ItemFoodProperties as PackItemFoodProperties,
    ItemKineticWeaponDefaults as PackItemKineticWeaponDefaults,
    ItemMiningProfile as PackItemMiningProfile, ItemMiningRule as PackItemMiningRule,
    ItemModelCatalog, ItemModelDefinition, ItemMountBodyArmorKind as PackItemMountBodyArmorKind,
    ItemRegistryCatalog, ItemSpecialModel, ItemTintSource,
    ItemUseAnimation as PackItemUseAnimation, ItemUseEffects as PackItemUseEffects,
    JukeboxSongRegistry, LanguageCatalog, PackResourceStack, PackRoots, ResourceLocation,
    SpriteImage, TagCatalog, TerrainColorMaps, DEFAULT_LANGUAGE_CODE,
};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
};
use bbb_protocol::item_types;
use bbb_protocol::packets::{
    AttributeModifierSummary, BannerPatternLayerSummary, ConsumableSummary,
    DataComponentPatchSummary, FireworkExplosionShapeSummary, FireworkExplosionSummary,
    ItemEnchantmentSummary, ItemRaritySummary, ItemStackSummary, ItemStackTemplateSummary,
    ItemUseAnimationSummary, JukeboxSongSummary, MapPostProcessingSummary,
    ResolvableProfileKindSummary, ResolvableProfileSummary, ResourceTextureSummary,
    TrimMaterialSummary, TrimPatternSummary,
};
// These summary types are referenced only by this crate's tests; keep them out
// of the non-test import set so the standalone library build stays warning-free.
#[cfg(test)]
use bbb_protocol::packets::{
    LodestoneTargetSummary, MobEffectDetailsSummary, MobEffectInstanceSummary, NbtSummaryEntry,
    NbtSummaryValue, SoundEventSummary, SuspiciousStewEffectSummary, WrittenBookContentSummary,
};
use bbb_render_types::{
    DynamicPlayerSkinImage, DynamicPlayerTextureImage, EntityCustomHeadSkull,
    EntityDefaultPlayerSkin, EntityDynamicPlayerSkin, EntityDynamicPlayerSkinStatus,
    EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind, EntityEquipmentAssetId,
    EntityEquipmentLayerTexture, EntityModelTextureRef, EntityPlayerSkin, EntityPlayerSkinModel,
    FirstPersonMapBackgroundKind, FirstPersonMapBackgroundTexture, HudComponentTextRun,
    HudFontGlyphBook, HudFontGlyphMap, HudStyledTextRun, HudTextStyle,
    ItemFrameMapDecorationTexture, ItemSpriteRect, SpriteAlphaMask,
};
// Referenced only by test builds and the `test-support` constructors; gate it so
// the plain library build stays clean.
#[cfg(any(test, feature = "test-support"))]
use bbb_render_types::HudUvRect;
use bbb_world::{
    ArmorMaterialKind as WorldArmorMaterialKind,
    HappyGhastHarnessColor as WorldHappyGhastHarnessColor, ItemAttackRange as WorldItemAttackRange,
    ItemEquipmentSlot as WorldItemEquipmentSlot, ItemFoodProperties as WorldItemFoodProperties,
    ItemUseEffects as WorldItemUseEffects, LlamaBodyDecorColor as WorldLlamaBodyDecorColor,
    MountArmorSlotKind as WorldMountArmorSlotKind, WorldItemMiningProfile, WorldItemMiningRule,
};

use crate::{
    profile_resolver::{AsyncProfileResolutionRuntime, HttpGameProfileFetcher},
    skin_runtime::{
        AsyncDynamicTextureRuntime, DynamicPlayerTextureKind, DynamicTextureCacheObservation,
        HttpSkinPngFetcher,
    },
};

mod icon_model;
mod item_membership_catalog;
mod profile_skin;

use icon_model::{
    default_item_name_translation_key, item_icon_model_ref_for_definition, CompassTarget,
    CrossbowChargeType, IconResolveContext, ItemIconModel, ResolvedItemIconModelLayer, TimeSource,
};
use profile_skin::ProfileSkinCache;
pub use profile_skin::{default_player_skin_for_profile, default_player_skin_for_profile_id};
use profile_skin::{entity_player_skin_model, profile_texture_handle};

use crate::font::load_hud_font_atlas;

mod icon;
mod profiles;
mod tables;
mod tooltip;

use icon::*;
use profiles::*;
pub use profiles::{NativeDynamicPlayerSkinDownload, NativeDynamicPlayerTextureDownload};
use tables::*;
#[cfg(test)]
use tooltip::*;
pub use tooltip::{NativeItemMapTooltipData, NativeItemTooltipLine, NativeItemTooltipOptions};

const VANILLA_LONG_USE_DURATION_TICKS: i32 = 72_000;

const VANILLA_BRUSH_USE_DURATION_TICKS: i32 = 200;

const VANILLA_SPYGLASS_USE_DURATION_TICKS: i32 = 1_200;

const VANILLA_ENDER_EYE_USE_DURATION_TICKS: i32 = 0;

const VANILLA_CROSSBOW_CHARGE_DURATION_TICKS: i32 = 25;

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

fn load_map_background_textures(roots: &PackRoots) -> Result<Vec<FirstPersonMapBackgroundTexture>> {
    let resources = roots.resource_stack();
    [
        FirstPersonMapBackgroundKind::Plain,
        FirstPersonMapBackgroundKind::Checkerboard,
    ]
    .into_iter()
    .map(|kind| {
        let location = ResourceLocation::parse(kind.vanilla_path())?;
        let resource = resources
            .get_resource(&location)
            .with_context(|| format!("missing map background texture {}", kind.vanilla_path()))?;
        let image = SpriteImage::from_png_file_with_resource_stack(
            kind.vanilla_path(),
            resource.path,
            &resources,
        )?;
        Ok(FirstPersonMapBackgroundTexture {
            kind,
            width: image.width,
            height: image.height,
            rgba: image.rgba,
        })
    })
    .collect()
}

fn load_map_text_fonts(roots: &PackRoots) -> Result<HudFontGlyphBook> {
    Ok(load_hud_font_atlas(roots)?.fonts)
}

#[cfg(any(test, feature = "test-support"))]
fn test_map_text_fonts() -> HudFontGlyphBook {
    let mut glyphs = HudFontGlyphMap::new();
    for byte in b' '..=b'~' {
        glyphs.insert_first_wins(
            char::from(byte),
            bbb_render_types::HudAsciiGlyph {
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                width: 6,
                height: 8,
                advance: if byte == b' ' { 4 } else { 6 },
                ascent: 7,
            },
        );
    }
    HudFontGlyphBook::from_fonts(
        crate::font::DEFAULT_FONT_ID,
        BTreeMap::from([(crate::font::DEFAULT_FONT_ID.to_string(), glyphs)]),
    )
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

const TOOLTIP_TEXT_DARK_GRAY: [f32; 4] = [85.0 / 255.0, 85.0 / 255.0, 85.0 / 255.0, 1.0];

const TOOLTIP_TEXT_DARK_GREEN: [f32; 4] = [0.0, 170.0 / 255.0, 0.0, 1.0];

const TOOLTIP_TEXT_BLUE: [f32; 4] = [85.0 / 255.0, 85.0 / 255.0, 1.0, 1.0];

const TOOLTIP_TEXT_RED: [f32; 4] = [1.0, 85.0 / 255.0, 85.0 / 255.0, 1.0];

use item_membership_catalog::{
    BREWING_INGREDIENT_ITEM_IDS, BREWING_POTION_ITEM_IDS, CARTOGRAPHY_ADDITIONAL_ITEM_IDS,
    ENCHANTMENT_LAPIS_LAZULI_ITEM_IDS,
};

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
    item_icon_models: HashMap<String, ItemIconModel>,
    item_display_transforms: HashMap<String, BlockModelDisplayTransforms>,
    client_item_properties: HashMap<String, ClientItemProperties>,
    registry: Option<ItemRegistryCatalog>,
    item_tags: Option<TagCatalog>,
    enchantment_tags: Option<TagCatalog>,
    trim_material_tags: Option<TagCatalog>,
    trim_pattern_tags: Option<TagCatalog>,
    jukebox_song_tags: Option<TagCatalog>,
    jukebox_songs: JukeboxSongRegistry,
    potion_tags: Option<TagCatalog>,
    attribute_tags: Option<TagCatalog>,
    villager_type_tags: Option<TagCatalog>,
    equipment_assets: EquipmentAssetCatalog,
    language: LanguageCatalog,
    map_background_textures: Vec<FirstPersonMapBackgroundTexture>,
    map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    map_text_fonts: Option<HudFontGlyphBook>,
    textures: ItemTextureState,
    profile_resolutions: RefCell<Option<AsyncProfileResolutionRuntime>>,
    dynamic_textures: RefCell<Option<Box<AsyncDynamicTextureRuntime>>>,
    resource_generation: u64,
    dynamic_texture_session: Cell<Option<(u64, bool)>>,
    dynamic_texture_session_generation: Cell<u64>,
    profile_presentation_revision: Cell<u64>,
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
    let image = SpriteImage::from_png_file_with_resource_stack(
        source_id.to_string(),
        resource.path,
        resources,
    )
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

/// One composed `Transformation` carried by a baked item-model leaf.
///
/// Columns use the same column-vector convention as JOML and `glam::Mat4`.
/// Keeping the value backend-neutral lets native compose it with display and
/// placement transforms without giving the item runtime any renderer handles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemModelLeafTransformation {
    pub columns: [[f32; 4]; 4],
}

impl ItemModelLeafTransformation {
    pub const IDENTITY: Self = Self {
        columns: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };
}

/// Ordered leaf selected from one client-item model tree.
///
/// `Ordinary` retains the resolved sprite layers for the existing generated
/// geometry path. `Special` deliberately carries no base-model sprite:
/// Vanilla uses that model only for render properties and submits the special
/// renderer's geometry instead.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedItemModelLayer {
    Ordinary {
        icon: ItemAtlasIcon,
        transformation: ItemModelLeafTransformation,
        /// Display transform selected from this leaf's resolved cuboid model.
        /// `None` denotes the missing-model compatibility fallback.
        display_transform: Option<BlockModelDisplayTransform>,
    },
    Special {
        model: ItemSpecialModel,
        transformation: ItemModelLeafTransformation,
        /// `ModelRenderProperties` transform supplied by the special wrapper's
        /// `base` model for the caller's display context.
        display_transform: Option<BlockModelDisplayTransform>,
    },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedItemModel {
    pub layers: Vec<ResolvedItemModelLayer>,
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
    /// Frozen environment-attribute projection. `None` is the explicit
    /// built-in/offline fallback used by context-free callers.
    pub sun_angle_degrees: Option<f32>,
    pub moon_phase_index: Option<u8>,
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
    /// Exact sprite resource identity. Atlas generation plus this id decides
    /// static extrusion residency before the alpha-mask payload is consulted.
    pub asset_id: Arc<str>,
    pub mask: SpriteAlphaMask,
    pub rect: ItemSpriteRect,
    pub tint: [f32; 4],
    pub translucent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemAtlasIconLayer {
    pub texture_index: u32,
    pub uv: ItemAtlasUvRect,
    pub tint: [f32; 4],
    pub translucent: bool,
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
    /// Monotonic identity of the accepted client resource stack. Render-side
    /// derived caches include this before touching localized/model payloads so
    /// a reload cannot reuse values resolved from the previous pack stack.
    pub fn resource_generation(&self) -> u64 {
        self.resource_generation
    }

    /// Resolves a client language key from the same selected pack stack used by item tooltips.
    /// GUI consumers keep their own semantic formatting and use this only for final labels.
    pub fn localized_text<'a>(&'a self, key: &'a str) -> &'a str {
        self.language.get_or_key(key)
    }

    /// Creates an independent owner for simulation-side item queries.
    ///
    /// Immutable pack catalogs and atlas indices are copied, while renderer-only
    /// asynchronous profile downloads, upload queues, and animation wobblers
    /// start empty. The two owners therefore never share an interior-mutable
    /// cache or require a render/simulation lock.
    pub fn independent_simulation_projection(&self) -> Self {
        Self {
            item_definition_count: self.item_definition_count,
            item_registry_count: self.item_registry_count,
            resolved_model_count: self.resolved_model_count,
            missing_model_ids: self.missing_model_ids.clone(),
            missing_texture_ids: self.missing_texture_ids.clone(),
            furnace_fuel_item_ids: self.furnace_fuel_item_ids.clone(),
            freeze_immune_wearable_item_ids: self.freeze_immune_wearable_item_ids.clone(),
            powder_snow_walkable_foot_item_ids: self.powder_snow_walkable_foot_item_ids.clone(),
            item_icon_models: self.item_icon_models.clone(),
            item_display_transforms: self.item_display_transforms.clone(),
            client_item_properties: self.client_item_properties.clone(),
            registry: self.registry.clone(),
            item_tags: self.item_tags.clone(),
            enchantment_tags: self.enchantment_tags.clone(),
            trim_material_tags: self.trim_material_tags.clone(),
            trim_pattern_tags: self.trim_pattern_tags.clone(),
            jukebox_song_tags: self.jukebox_song_tags.clone(),
            jukebox_songs: self.jukebox_songs.clone(),
            potion_tags: self.potion_tags.clone(),
            attribute_tags: self.attribute_tags.clone(),
            villager_type_tags: self.villager_type_tags.clone(),
            equipment_assets: self.equipment_assets.clone(),
            language: self.language.clone(),
            map_background_textures: self.map_background_textures.clone(),
            map_decoration_textures: self.map_decoration_textures.clone(),
            map_text_fonts: self.map_text_fonts.clone(),
            textures: self.textures.clone(),
            profile_resolutions: RefCell::default(),
            dynamic_textures: RefCell::default(),
            resource_generation: self.resource_generation,
            dynamic_texture_session: Cell::new(None),
            dynamic_texture_session_generation: Cell::new(0),
            profile_presentation_revision: Cell::new(0),
            profile_texture_resources: self.profile_texture_resources.clone(),
            local_dynamic_skins: RefCell::default(),
            local_dynamic_textures: RefCell::default(),
            profile_skins: RefCell::default(),
            local_time_epoch_millis_override: Cell::new(
                self.local_time_epoch_millis_override.get(),
            ),
            time_wobblers: RefCell::default(),
            time_randoms: RefCell::default(),
            compass_wobblers: RefCell::default(),
            compass_randoms: RefCell::default(),
        }
    }

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
            .and_then(|registry| {
                item_types::vanilla_item_type(item_types::VANILLA_ITEM_LEATHER_BOOTS_ID)
                    .and_then(|entry| registry.protocol_id(entry.resource_id))
            })
            .into_iter()
            .collect();
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
        let jukebox_songs = JukeboxSongRegistry::vanilla_26_1();
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
        let map_background_textures = load_map_background_textures(roots)
            .context("load map background textures")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native map background textures");
                err
            })
            .unwrap_or_default();
        let map_text_fonts = load_map_text_fonts(roots)
            .context("load shared world-text font metrics")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native world text");
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
            equipment_assets,
            language,
            map_background_textures,
            map_decoration_textures,
            map_text_fonts,
            item_tags,
            enchantment_tags,
            trim_material_tags,
            trim_pattern_tags,
            jukebox_song_tags,
            jukebox_songs,
            potion_tags,
            attribute_tags,
            villager_type_tags,
            roots
                .client_resource_generation()
                .map_or(0, bbb_pack::ClientResourceGeneration::generation),
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
        equipment_assets: EquipmentAssetCatalog,
        language: LanguageCatalog,
        map_background_textures: Vec<FirstPersonMapBackgroundTexture>,
        map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
        map_text_fonts: Option<HudFontGlyphBook>,
        item_tags: Option<TagCatalog>,
        enchantment_tags: Option<TagCatalog>,
        trim_material_tags: Option<TagCatalog>,
        trim_pattern_tags: Option<TagCatalog>,
        jukebox_song_tags: Option<TagCatalog>,
        jukebox_songs: JukeboxSongRegistry,
        potion_tags: Option<TagCatalog>,
        attribute_tags: Option<TagCatalog>,
        villager_type_tags: Option<TagCatalog>,
        resource_generation: u64,
        profile_texture_resources: PackResourceStack,
    ) -> Result<Self> {
        let mut texture_ids = BTreeSet::new();
        let mut item_icon_model_refs = HashMap::new();
        let mut item_display_transforms = HashMap::new();
        let mut client_item_properties = HashMap::new();
        let mut missing_model_ids = BTreeSet::new();
        let mut missing_texture_ids = BTreeSet::new();
        let mut resolved_model_count = 0usize;

        for (item_id, definition) in item_models.definitions() {
            client_item_properties.insert(item_id.clone(), definition.properties);
            let models = cuboid_models.models_for_definition(definition);
            resolved_model_count += models.models.len();
            // Retain one compatibility transform for legacy block-only paths.
            // Selected resident leaves carry their own resolved base-model
            // transforms because resource-pack composites may intentionally
            // mix different `ModelRenderProperties`.
            if let Some(model) = models.models.first() {
                item_display_transforms.insert(item_id.clone(), model.display_transforms.clone());
            }
            texture_ids.extend(models.texture_ids());
            let model_tints = model_tints_for_definition(&definition.model);
            let icon_model = item_icon_model_ref_for_definition(
                &definition.model,
                &cuboid_models,
                &model_tints,
                colormaps.as_ref(),
            );
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
            item_display_transforms,
            client_item_properties,
            registry,
            item_tags,
            enchantment_tags,
            trim_material_tags,
            trim_pattern_tags,
            jukebox_song_tags,
            jukebox_songs,
            potion_tags,
            attribute_tags,
            villager_type_tags,
            equipment_assets,
            language,
            map_background_textures,
            map_decoration_textures,
            map_text_fonts,
            textures,
            profile_resolutions: RefCell::default(),
            dynamic_textures: RefCell::default(),
            resource_generation,
            dynamic_texture_session: Cell::new(None),
            dynamic_texture_session_generation: Cell::new(0),
            profile_presentation_revision: Cell::new(0),
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
            item_icon_models: HashMap::new(),
            item_display_transforms: HashMap::new(),
            client_item_properties: HashMap::new(),
            registry: None,
            item_tags: None,
            enchantment_tags: None,
            trim_material_tags: None,
            trim_pattern_tags: None,
            jukebox_song_tags: None,
            jukebox_songs: JukeboxSongRegistry::vanilla_26_1(),
            potion_tags: None,
            attribute_tags: None,
            villager_type_tags: None,
            equipment_assets: EquipmentAssetCatalog::default(),
            language: LanguageCatalog::from_json_bytes(b"{}").expect("empty test language"),
            map_background_textures: vec![
                FirstPersonMapBackgroundTexture {
                    kind: FirstPersonMapBackgroundKind::Plain,
                    width: 1,
                    height: 1,
                    rgba: vec![0x70, 0x54, 0x34, 0xff],
                },
                FirstPersonMapBackgroundTexture {
                    kind: FirstPersonMapBackgroundKind::Checkerboard,
                    width: 1,
                    height: 1,
                    rgba: vec![0x88, 0x88, 0x88, 0xff],
                },
            ],
            map_decoration_textures: Vec::new(),
            map_text_fonts: Some(test_map_text_fonts()),
            textures: ItemTextureState::from_images(vec![missing])
                .expect("test item texture atlas is valid"),
            profile_resolutions: RefCell::default(),
            dynamic_textures: RefCell::default(),
            resource_generation: 0,
            dynamic_texture_session: Cell::new(None),
            dynamic_texture_session_generation: Cell::new(0),
            profile_presentation_revision: Cell::new(0),
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
    pub fn with_vanilla_item_registry_for_test(mut self) -> Self {
        self.registry = Some(
            ItemRegistryCatalog::vanilla_26_1().expect("checked-in vanilla item registry is valid"),
        );
        self.item_registry_count = self.registry.as_ref().map_or(0, ItemRegistryCatalog::len);
        self
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn with_client_item_swap_properties_for_test(
        mut self,
        item_model_id: impl Into<String>,
        hand_animation_on_swap: bool,
        swap_animation_scale: f32,
    ) -> Self {
        self.client_item_properties.insert(
            item_model_id.into(),
            ClientItemProperties {
                hand_animation_on_swap,
                swap_animation_scale,
                ..ClientItemProperties::default()
            },
        );
        self
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

    pub fn item_resource_id_for_block(&self, block_resource_id: &str) -> Option<&str> {
        self.registry
            .as_ref()?
            .item_resource_id_for_block(block_resource_id)
    }

    pub fn primary_block_resource_id_for_item(&self, item_resource_id: &str) -> Option<&str> {
        self.registry
            .as_ref()?
            .primary_block_resource_id_for_item(item_resource_id)
    }

    pub fn localized_item_name(&self, resource_id: &str) -> String {
        tooltip::localized_item_name(&self.language, resource_id)
    }

    pub fn item_default_kinetic_weapon(
        &self,
        resource_id: &str,
    ) -> Option<PackItemKineticWeaponDefaults> {
        self.registry.as_ref()?.default_kinetic_weapon(resource_id)
    }

    /// The `minecraft:jukebox_song` resource id for a component holder id.
    /// Uses the same ordered registry that drives jukebox tooltips and audio.
    pub fn jukebox_song_resource_id(&self, registry_id: i32) -> Option<&str> {
        self.jukebox_songs.song_id(registry_id)
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
