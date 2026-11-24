use bbb_client_clock::UNIX_EPOCH;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use anyhow::{Context, Result};
use bbb_client_assets::{
    AtlasImage, AtlasLayout, AtlasPacker, AtlasSprite, BlockModelDisplayContext,
    BlockModelDisplayTransform, BlockModelDisplayTransforms, ClientItemProperties,
    ContentRegistryAssetProjection, EquipmentAssetCatalog, FreezeImmuneWearableCatalog,
    FurnaceFuelCatalog, ItemCuboidModel, ItemCuboidModelCatalog, ItemModelCatalog,
    ItemModelDefinition, ItemRegistryCatalog, ItemSpecialModel, ItemTintSource,
    JukeboxSongRegistry, LanguageCatalog, ModItemAssetProjection, PackResourceStack, PackRoots,
    ResourceLocation, SpriteImage, TagCatalog, TerrainColorMaps, DEFAULT_LANGUAGE_CODE,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
};
use bbb_protocol::item_types;
use bbb_protocol::packets::{
    ConsumableSummary, DataComponentPatchSummary, ItemStackSummary, ItemStackTemplateSummary,
    ResolvableProfileSummary, ResourceTextureSummary,
};
use glam::{Mat4, Vec3};
// These summary types are referenced only by this crate's tests; keep them out
// of the non-test import set so the standalone library build stays warning-free.
#[cfg(test)]
use bbb_protocol::packets::{
    AttributeModifierSummary, FireworkExplosionShapeSummary, FireworkExplosionSummary,
    ItemRaritySummary, ItemUseAnimationSummary, JukeboxSongSummary, LodestoneTargetSummary,
    MobEffectDetailsSummary, MobEffectInstanceSummary, NbtSummaryEntry, NbtSummaryValue,
    ResolvableProfileKindSummary, SoundEventSummary, TrimMaterialSummary, TrimPatternSummary,
    WrittenBookContentSummary,
};
use bbb_render_types::HudUvRect;
use bbb_render_types::{
    DynamicPlayerSkinImage, DynamicPlayerTextureImage, EntityCustomHeadSkull,
    EntityDefaultPlayerSkin, EntityDynamicPlayerSkin, EntityDynamicPlayerSkinStatus,
    EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind, EntityEquipmentAssetId,
    EntityPlayerSkin, EntityPlayerSkinModel, EntityPlayerTextures, FirstPersonMapBackgroundKind,
    FirstPersonMapBackgroundTexture, HudFontGlyphBook, HudFontGlyphMap,
    ItemFrameMapDecorationTexture, ItemSpriteRect, SpriteAlphaMask, StandaloneTextureSampling,
};

use crate::item_model::{
    profile_resolver::AsyncProfileResolutionRuntime,
    skin_runtime::{
        entity_player_skin_model, AsyncDynamicTextureRuntime, DynamicTextureCacheObservation,
        ProfileTextureAssetKind, ProfileTextureSetCache,
    },
};
use bbb_protocol::{
    profile_texture_handle, DynamicPlayerTextureKind, DynamicTextureFetchRequest,
    DynamicTextureFetchResult, ProfileResolutionKey, ProfileResolutionResult,
};

mod cuboid;
mod icon_model;
mod item_membership_catalog;
#[cfg(test)]
mod profile_skin;

pub use crate::item_model::skin_runtime::{
    default_player_skin_for_profile, default_player_skin_for_profile_id,
};
use bbb_client_assets::{
    ItemModelCompassTarget as CompassTarget, ItemModelTimeSource as TimeSource,
};
use icon_model::{
    CrossbowChargeType, IconResolveContext, ItemIconModel, ResolvedItemIconModelLayer,
};

use crate::item_model::font::{load_hud_font_atlas_with_options, providers::FontOptions};

mod icon;
mod profiles;
mod tables;

use icon::*;
use profiles::*;
pub use profiles::{NativeDynamicPlayerSkinDownload, NativeDynamicPlayerTextureDownload};

const VANILLA_LONG_USE_DURATION_TICKS: i32 = 72_000;

const VANILLA_BRUSH_USE_DURATION_TICKS: i32 = 200;

const VANILLA_SPYGLASS_USE_DURATION_TICKS: i32 = 1_200;

const VANILLA_ENDER_EYE_USE_DURATION_TICKS: i32 = 0;

const VANILLA_CROSSBOW_CHARGE_DURATION_TICKS: i32 = 25;

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
        let image = SpriteImage::from_pack_resource(kind.vanilla_path(), &resource, &resources)?;
        Ok(FirstPersonMapBackgroundTexture {
            kind,
            width: image.width,
            height: image.height,
            rgba: image.rgba,
        })
    })
    .collect()
}

fn load_map_text_fonts(
    roots: &PackRoots,
    options: FontOptions,
) -> Result<Option<HudFontGlyphBook>> {
    let default = bbb_client_assets::ResourceLocation::parse("minecraft:font/default.json")?;
    if roots.resource_stack().get_resource(&default).is_none() {
        return Ok(None);
    }
    Ok(Some(
        load_hud_font_atlas_with_options(roots, options)?.fonts,
    ))
}

fn set_map_text_direction(
    map_text_fonts: &mut Option<HudFontGlyphBook>,
    default_right_to_left: bool,
) {
    if let Some(fonts) = map_text_fonts.as_mut() {
        fonts.set_default_right_to_left(default_right_to_left);
    }
}

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
                advance: if byte == b' ' { 4.0 } else { 6.0 },
                bearing_top: 7.0,
                ..bbb_render_types::HudAsciiGlyph::default()
            },
        );
    }
    HudFontGlyphBook::from_fonts(
        crate::item_model::font::DEFAULT_FONT_ID,
        BTreeMap::from([(crate::item_model::font::DEFAULT_FONT_ID.to_string(), glyphs)]),
    )
}

const ITEM_ATLAS_MAX_WIDTH: u32 = 4096;

const ITEM_GENERATED_MAX_LAYERS: usize = 5;

const ITEM_ICON_RECURSION_LIMIT: usize = 16;

const MISSING_TEXTURE_ID: &str = "minecraft:missingno";

const ITEM_TINT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn description_key(prefix: &str, resource_id: &str) -> String {
    let (namespace, path) = resource_id
        .split_once(':')
        .unwrap_or(("minecraft", resource_id));
    format!("{prefix}.{namespace}.{}", path.replace('/', "."))
}

pub fn localized_item_name(language: &LanguageCatalog, resource_id: &str) -> String {
    let item_key = description_key("item", resource_id);
    if let Some(name) = language.get(&item_key) {
        return format_item_name_template(name.to_string());
    }

    let block_key = description_key("block", resource_id);
    format_item_name_template(language.get(&block_key).unwrap_or(&item_key).to_string())
}

fn format_item_name_template(template: String) -> String {
    if !template.contains('%') {
        return template;
    }
    let mut localized = String::new();
    bbb_client_assets::visit_localized_template(&template, 0, |part| {
        if let bbb_client_assets::LocalizedTemplatePart::Literal(literal) = part {
            localized.push_str(literal);
        }
    });
    localized
}

use item_membership_catalog::{
    BREWING_INGREDIENT_ITEM_IDS, BREWING_POTION_ITEM_IDS, CARTOGRAPHY_ADDITIONAL_ITEM_IDS,
    ENCHANTMENT_LAPIS_LAZULI_ITEM_IDS,
};

pub type BoundKeyLookupFn = fn(&[u16]) -> Option<&'static str>;
pub type LocalTimeFormatterFn = fn(i64, &str, &str, Option<&str>) -> Option<String>;

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
    content_registry: Arc<ContentRegistry>,
    content_projection: Option<Arc<ContentRegistryAssetProjection>>,
    mod_item_icon_models: HashMap<String, ItemIconModel>,
    mod_item_display_transforms: HashMap<String, BlockModelDisplayTransforms>,
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
    bound_key_lookup: Option<BoundKeyLookupFn>,
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
    profile_texture_sets: RefCell<ProfileTextureSetCache>,
    local_time_formatter: Option<LocalTimeFormatterFn>,
    local_time_epoch_millis_override: Cell<Option<i64>>,
    time_wobblers: RefCell<HashMap<ItemTimeWobblerKey, ItemNeedleWobbler>>,
    compass_wobblers: RefCell<HashMap<ItemCompassWobblerKey, ItemNeedleWobbler>>,
    presentation_random: RefCell<bbb_registry_codec::legacy_random::LegacyRandomSource>,
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
    let image = SpriteImage::from_pack_resource(source_id.to_string(), &resource, resources)
        .with_context(|| format!("load player profile resource texture {texture_path}"))?;
    let handle = profile_texture_handle(source_id);
    Ok((
        EntityDynamicPlayerTexture { handle, kind },
        DynamicPlayerTextureImage {
            handle,
            size: [image.width, image.height],
            standalone_sampling: Some(StandaloneTextureSampling::new(
                image.texture_metadata.blur,
                image.texture_metadata.clamp,
            )),
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

/// The display transform about the model center: `T(t) · Rxyz · S · T(-0.5)` (vanilla
/// `ItemTransform.apply`). `Rxyz` matches joml `rotationXYZ`. The pack's `BlockModelDisplayTransform`
/// already holds translation in world units (the model JSON's 1/16 value pre-multiplied) and per-axis
/// scale, so no further normalization is applied here. `left_hand` is vanilla's left-hand fix: it negates
/// `translation.x`, `rotation.y`, and `rotation.z` so an item mirrors correctly into the left arm.
pub fn item_display_matrix(display: &BlockModelDisplayTransform, left_hand: bool) -> Mat4 {
    let sign = if left_hand { -1.0 } else { 1.0 };
    let translation = Vec3::new(
        display.translation[0] * sign,
        display.translation[1],
        display.translation[2],
    );
    let rotation = Mat4::from_rotation_x(display.rotation[0].to_radians())
        * Mat4::from_rotation_y((display.rotation[1] * sign).to_radians())
        * Mat4::from_rotation_z((display.rotation[2] * sign).to_radians());
    Mat4::from_translation(translation)
        * rotation
        * Mat4::from_scale(Vec3::from_array(display.scale))
        * Mat4::from_translation(Vec3::splat(-0.5))
}

/// Ordered leaf selected from one client-item model tree.
///
/// `Cuboid` retains resource geometry over the items atlas.
/// `Ordinary` retains the resolved sprite layers for the generated geometry
/// path. `Special` geometry comes from the special renderer; its `base` model
/// only supplies render properties and the flat sprite used by consumers that
/// draw a 2D item (particles, billboards).
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedItemModelLayer {
    Cuboid {
        icon: ItemAtlasIcon,
        parts: Arc<[ResolvedItemCuboidPart]>,
        transformation: ItemModelLeafTransformation,
        display_transform: Option<BlockModelDisplayTransform>,
    },
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
        base_icon: Option<ItemAtlasIcon>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedItemCuboidPart {
    pub model_id: Arc<str>,
    pub part: u32,
    pub geometry: Arc<crate::item_render::ItemModelGeometry>,
    pub ground_model_extents: Option<(glam::Vec3, glam::Vec3)>,
    pub tint: [f32; 4],
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedItemModel {
    pub layers: Vec<ResolvedItemModelLayer>,
}

impl ResolvedItemModel {
    /// Every leaf flattened into one sprite stack: ordinary icons plus the
    /// special leaves' base sprites, in tree order.
    pub fn flat_icon(&self) -> Option<ItemAtlasIcon> {
        let layers = self
            .layers
            .iter()
            .flat_map(|layer| match layer {
                ResolvedItemModelLayer::Ordinary { icon, .. }
                | ResolvedItemModelLayer::Cuboid { icon, .. } => Some(icon),
                ResolvedItemModelLayer::Special { base_icon, .. } => base_icon.as_ref(),
            })
            .flat_map(|icon| icon.layers.iter().copied())
            .collect::<Vec<_>>();
        (!layers.is_empty()).then_some(ItemAtlasIcon { layers })
    }
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
    pub smooth_camera: bool,
    pub spectator_outlines: bool,
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
            // InputConstants.UNKNOWN by default; an options.txt rebinding makes
            // them live, so their state flows from the input runtime.
            "key.smoothCamera" => self.smooth_camera,
            "key.spectatorOutlines" => self.spectator_outlines,
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

/// Holder-order registry keys of the client level, indexed by network holder id.
#[derive(Debug, Clone, Copy, Default)]
pub struct ItemModelRegistryKeys<'a> {
    pub trim_materials: Option<&'a [String]>,
    pub trim_patterns: Option<&'a [String]>,
    pub enchantments: Option<&'a [String]>,
    pub attributes: Option<&'a [String]>,
}

/// Everything vanilla `ItemModelResolver.updateForTopItem` reads besides the
/// stack: display context, level, owner and local-player state. Absent values
/// mean the caller has no such owner or level, so dependent properties take
/// their vanilla fallback.
#[derive(Clone, Copy)]
pub struct ItemModelContext<'a> {
    pub display_context: BlockModelDisplayContext,
    pub registry_access: Option<(
        bbb_registry_codec::RegistryAccessRevision,
        &'a dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    )>,
    pub item_profiles: Option<bbb_render_types::ItemComponentDefaultsView<'a>>,
    pub registry_keys: ItemModelRegistryKeys<'a>,
    pub context_dimension: Option<&'a str>,
    pub time: Option<ItemModelTimeContext>,
    pub compass: Option<ItemModelCompassContext<'a>>,
    pub owner_main_hand_left: Option<bool>,
    pub context_entity_type: Option<&'a str>,
    pub owner_team_color_rgb: Option<u32>,
    pub using_item: bool,
    pub use_context: ItemModelUseContext,
    pub cooldown_progress: f32,
    pub bundle_selected_item_index: Option<i32>,
    pub selected_item: bool,
    pub carried_item: bool,
    pub view_entity: bool,
    pub shift_down: bool,
    pub keybind: ItemModelKeybindContext,
    pub fishing_rod_cast: bool,
    pub seed: i32,
}

impl ItemModelContext<'_> {
    pub fn new(display_context: BlockModelDisplayContext) -> Self {
        Self {
            display_context,
            registry_access: None,
            item_profiles: None,
            registry_keys: ItemModelRegistryKeys::default(),
            context_dimension: None,
            time: None,
            compass: None,
            owner_main_hand_left: None,
            context_entity_type: None,
            owner_team_color_rgb: None,
            using_item: false,
            use_context: ItemModelUseContext::inactive(),
            cooldown_progress: 0.0,
            bundle_selected_item_index: None,
            selected_item: false,
            carried_item: false,
            view_entity: false,
            shift_down: false,
            keybind: ItemModelKeybindContext::default(),
            fishing_rod_cast: false,
            seed: 0,
        }
    }
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
    // Vanilla `TextureAtlasSprite` exposes exact content edges (with replicated gutter padding).
    ItemAtlasUvRect {
        min: [x0 / width, y0 / height],
        max: [x1 / width, y1 / height],
    }
}

impl NativeItemRuntime {
    pub fn with_bound_key_lookup(mut self, lookup: fn(&[u16]) -> Option<&'static str>) -> Self {
        self.bound_key_lookup = Some(lookup);
        self
    }

    pub fn with_local_time_formatter(
        mut self,
        formatter: fn(i64, &str, &str, Option<&str>) -> Option<String>,
    ) -> Self {
        self.local_time_formatter = Some(formatter);
        self
    }

    pub fn with_content_registry(
        mut self,
        content_registry: Arc<ContentRegistry>,
        content_projection: Option<Arc<ContentRegistryAssetProjection>>,
    ) -> Self {
        self.sync_content_registry(content_registry, content_projection);
        self
    }

    /// Synchronizes session modded item definitions and projected pack assets into this runtime so
    /// inventory icons, held/dropped 3D models, particles, and tooltips resolve custom mod items.
    pub fn sync_content_registry(
        &mut self,
        content_registry: Arc<ContentRegistry>,
        content_projection: Option<Arc<ContentRegistryAssetProjection>>,
    ) -> bool {
        let registry_unchanged = Arc::ptr_eq(&self.content_registry, &content_registry)
            || self.content_registry.items() == content_registry.items();
        let projection_unchanged = match (&self.content_projection, &content_projection) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a.items == b.items,
            _ => false,
        };
        if registry_unchanged && projection_unchanged {
            return false;
        }
        self.content_registry = content_registry;
        self.content_projection = content_projection;
        self.mod_item_icon_models.clear();
        self.mod_item_display_transforms.clear();

        if let Some(projection) = self.content_projection.as_ref() {
            for (item_id, projected) in &projection.items {
                if self.item_icon_models.contains_key(item_id) {
                    continue;
                }
                let (mut model, display_transforms) =
                    self.synthesize_mod_item_icon_model(projected);
                if !model.is_empty() {
                    model.prepare_ground_extents(Mat4::IDENTITY);
                    self.mod_item_icon_models.insert(item_id.clone(), model);
                }
                if let Some(transforms) = display_transforms {
                    self.mod_item_display_transforms
                        .insert(item_id.clone(), transforms);
                }
            }
        }
        true
    }

    pub fn content_registry(&self) -> &Arc<ContentRegistry> {
        &self.content_registry
    }

    fn synthesize_mod_item_icon_model(
        &self,
        projected: &ModItemAssetProjection,
    ) -> (ItemIconModel, Option<BlockModelDisplayTransforms>) {
        let has_pack_texture = projected.texture_ids.iter().any(|texture_id| {
            texture_id != MISSING_TEXTURE_ID
                && self.textures.texture_indices.contains_key(texture_id)
        });
        let fallback_tint = (!projected.resolved_from_pack && !has_pack_texture).then(|| {
            let rgb = projected.fallback_tint.rgb;
            [rgb[0], rgb[1], rgb[2], 1.0]
        });

        if let Some(cuboid_model) = &projected.model {
            let mut layers = Vec::new();
            if cuboid_model.generated {
                for index in 0..ITEM_GENERATED_MAX_LAYERS {
                    let key = format!("layer{index}");
                    let Some(texture) = cuboid_model.texture_slots.get(&key) else {
                        continue;
                    };
                    let texture_index = self.textures.texture_index(&texture.id);
                    layers.push(ItemIconTextureLayer {
                        texture_index,
                        tint: ItemIconTint::Static(fallback_tint.unwrap_or(ITEM_TINT_WHITE)),
                        translucent: texture.force_translucent
                            || self.textures.texture_has_translucent(texture_index),
                    });
                }
            }
            if layers.is_empty() {
                let texture_id = projected
                    .texture_ids
                    .first()
                    .map(String::as_str)
                    .unwrap_or(MISSING_TEXTURE_ID);
                let texture_index = self.textures.texture_index(texture_id);
                layers.push(ItemIconTextureLayer {
                    texture_index,
                    tint: ItemIconTint::Static(fallback_tint.unwrap_or(ITEM_TINT_WHITE)),
                    translucent: self.textures.texture_has_translucent(texture_index),
                });
            }
            let cuboid = (!cuboid_model.generated).then(|| {
                let mut parts = cuboid::bake_model(cuboid_model, None, None, &self.textures);
                if let Some(tint) = fallback_tint {
                    for part in Arc::make_mut(&mut parts) {
                        part.tint = ItemIconTint::Static(tint);
                    }
                }
                parts
            });
            return (
                ItemIconModel::Layers {
                    layers,
                    display_transforms: cuboid_model.display_transforms.clone(),
                    cuboid,
                },
                Some(cuboid_model.display_transforms.clone()),
            );
        }

        if let Some(block_model) = &projected.fallback_block_model {
            let synthetic_model = ItemCuboidModel {
                generated: false,
                id: projected.resource_id.clone(),
                parent: None,
                use_ambient_occlusion: block_model.use_ambient_occlusion,
                gui_light: block_model.gui_light,
                display_transforms: block_model.display_transforms.clone(),
                texture_slots: BTreeMap::new(),
                face_textures: Some(block_model.face_textures.clone()),
                shape: block_model.shape.clone(),
            };
            let mut parts = cuboid::bake_model(&synthetic_model, None, None, &self.textures);
            if let Some(tint) = fallback_tint {
                for part in Arc::make_mut(&mut parts) {
                    part.tint = ItemIconTint::Static(tint);
                }
            }
            let texture_id = block_model
                .particle_texture
                .as_deref()
                .or_else(|| projected.texture_ids.first().map(String::as_str))
                .unwrap_or(MISSING_TEXTURE_ID);
            let texture_index = self.textures.texture_index(texture_id);
            let layers = vec![ItemIconTextureLayer {
                texture_index,
                tint: ItemIconTint::Static(fallback_tint.unwrap_or(ITEM_TINT_WHITE)),
                translucent: self.textures.texture_has_translucent(texture_index),
            }];
            return (
                ItemIconModel::Layers {
                    layers,
                    display_transforms: block_model.display_transforms.clone(),
                    cuboid: Some(parts),
                },
                Some(block_model.display_transforms.clone()),
            );
        }

        let texture_id = projected
            .texture_ids
            .first()
            .map(String::as_str)
            .unwrap_or(MISSING_TEXTURE_ID);
        let texture_index = self.textures.texture_index(texture_id);
        (
            ItemIconModel::Layers {
                layers: vec![ItemIconTextureLayer {
                    texture_index,
                    tint: ItemIconTint::Static(fallback_tint.unwrap_or(ITEM_TINT_WHITE)),
                    translucent: self.textures.texture_has_translucent(texture_index),
                }],
                display_transforms: BlockModelDisplayTransforms::default(),
                cuboid: None,
            },
            None,
        )
    }

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

    // ClientLanguage: fallback depends on key presence, not value equality.
    pub fn localized_text_if_present(&self, key: &str) -> Option<&str> {
        self.language.get(key)
    }

    pub fn localized_text_raw_if_present(
        &self,
        key: &bbb_protocol::JavaString,
    ) -> Option<&bbb_protocol::JavaString> {
        self.language.get_raw(key)
    }

    pub fn localized_text_raw_units_if_present(
        &self,
        key: &[u16],
    ) -> Option<&bbb_protocol::JavaString> {
        self.language.get_raw_units(key)
    }

    pub fn language_bidirectional(&self) -> bool {
        self.language.is_bidirectional()
    }

    /// Whether the accepted client pack stack declares `language_code` in
    /// `pack.mcmeta`. Vanilla's `LanguageManager.languages` uses that metadata,
    /// not the mere presence of a `lang/*.json` resource.
    pub fn has_available_language(&self, language_code: &str) -> bool {
        self.language.has_available_language(language_code)
    }

    /// Runs a presentation-only RNG query against this item runtime's random stream.
    pub fn with_presentation_random<T>(
        &self,
        f: impl FnOnce(&mut bbb_registry_codec::legacy_random::LegacyRandomSource) -> T,
    ) -> T {
        f(&mut self.presentation_random.borrow_mut())
    }

    pub fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_with_locale(roots, DEFAULT_LANGUAGE_CODE)
    }

    pub fn load_with_locale(roots: &PackRoots, language_code: &str) -> Result<Self> {
        Self::load_with_locale_and_font_options(roots, language_code, FontOptions::default())
    }

    fn load_with_locale_and_font_options(
        roots: &PackRoots,
        language_code: &str,
        font_options: FontOptions,
    ) -> Result<Self> {
        Self::load_with_locale_and_fonts(
            roots,
            language_code,
            load_map_text_fonts(roots, font_options)?,
        )
    }

    pub fn load_with_locale_and_fonts(
        roots: &PackRoots,
        language_code: &str,
        mut map_text_fonts: Option<HudFontGlyphBook>,
    ) -> Result<Self> {
        let item_models = roots
            .load_item_model_catalog()
            .context("load item model catalog")?;
        let cuboid_models = roots
            .load_item_cuboid_model_catalog()
            .context("load item cuboid model catalog")?;
        let texture_atlases = bbb_client_assets::ITEM_CUBOID_TEXTURE_ATLASES
            .iter()
            .map(|atlas_name| roots.load_atlas_texture_images_shared(atlas_name))
            .collect::<Result<Vec<_>>>()
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
        set_map_text_direction(&mut map_text_fonts, language.is_bidirectional());
        Self::from_loaded(
            item_models,
            cuboid_models,
            texture_atlases,
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
                .map_or(0, bbb_client_assets::ClientResourceGeneration::generation),
            roots.resource_stack(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_loaded(
        item_models: ItemModelCatalog,
        cuboid_models: ItemCuboidModelCatalog,
        texture_atlases: Vec<Arc<Vec<SpriteImage>>>,
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
            missing_model_ids.extend(models.missing_model_ids);
        }

        let mut image_by_id = HashMap::<&str, &SpriteImage>::new();
        for atlas_images in &texture_atlases {
            for image in atlas_images.iter() {
                image_by_id.entry(image.id.as_str()).or_insert(image);
                // Keep non-vanilla pack sprites in the item atlas so modded items synced later via
                // `sync_content_registry` can reference pack textures without an `items/*.json` entry.
                if !image.id.starts_with("minecraft:") {
                    texture_ids.insert(image.id.clone());
                }
            }
        }
        let lookup_image = |texture_id: &str| -> Option<&SpriteImage> {
            image_by_id.get(texture_id).copied().or_else(|| {
                let normalized = ResourceLocation::parse(texture_id).ok()?.id();
                image_by_id.get(normalized.as_str()).copied()
            })
        };

        let mut images = Vec::new();
        if let Some(image) = lookup_image(MISSING_TEXTURE_ID) {
            images.push(image);
        } else {
            missing_texture_ids.insert(MISSING_TEXTURE_ID.to_string());
        }
        for texture_id in texture_ids {
            if texture_id == MISSING_TEXTURE_ID {
                continue;
            }
            match lookup_image(&texture_id) {
                Some(image) => images.push(image),
                None => {
                    missing_texture_ids.insert(texture_id);
                }
            }
        }

        let textures = ItemTextureState::from_image_refs_with_stack(
            &images,
            Some(&profile_texture_resources),
        )?;
        let mut cuboid_cache = icon_model::CuboidBakeCache::new(Some(&profile_texture_resources));
        let mut item_icon_models = HashMap::with_capacity(item_models.len());
        for (item_id, definition) in item_models.definitions() {
            let model_tints = model_tints_for_definition(&definition.model);
            let model = icon_model::item_icon_model_for_definition_cached(
                &definition.model,
                &cuboid_models,
                &model_tints,
                colormaps.as_ref(),
                &textures,
                &mut cuboid_cache,
            )?;
            if !model.is_empty() {
                item_icon_models.insert(item_id.clone(), model);
            }
        }

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
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            content_projection: None,
            mod_item_icon_models: HashMap::new(),
            mod_item_display_transforms: HashMap::new(),
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
            bound_key_lookup: None,
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
            profile_texture_sets: RefCell::default(),
            local_time_formatter: None,
            local_time_epoch_millis_override: Cell::new(None),
            time_wobblers: RefCell::default(),
            compass_wobblers: RefCell::default(),
            presentation_random: RefCell::new(
                bbb_registry_codec::legacy_random::LegacyRandomSource::with_seed(0),
            ),
        })
    }

    #[doc(hidden)]
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
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            content_projection: None,
            mod_item_icon_models: HashMap::new(),
            mod_item_display_transforms: HashMap::new(),
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
            bound_key_lookup: None,
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
            profile_texture_sets: RefCell::default(),
            local_time_formatter: None,
            local_time_epoch_millis_override: Cell::new(None),
            time_wobblers: RefCell::default(),
            compass_wobblers: RefCell::default(),
            presentation_random: RefCell::new(
                bbb_registry_codec::legacy_random::LegacyRandomSource::with_seed(0),
            ),
        }
    }

    #[doc(hidden)]
    pub fn with_vanilla_item_registry_for_test(mut self) -> Self {
        self.registry = Some(
            ItemRegistryCatalog::vanilla_26_1().expect("checked-in vanilla item registry is valid"),
        );
        self.item_registry_count = self.registry.as_ref().map_or(0, ItemRegistryCatalog::len);
        self
    }

    #[doc(hidden)]
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

    #[doc(hidden)]
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
        self.item_resource_id_for_protocol_id(protocol_id)
    }

    /// The protocol id for an item resource id via the loaded item registry.
    pub fn item_protocol_id(&self, resource_id: &str) -> Option<i32> {
        self.registry
            .as_ref()
            .and_then(|registry| registry.protocol_id(resource_id))
            .or_else(|| {
                self.content_registry
                    .item_by_resource_id(resource_id)
                    .map(|spec| spec.id)
            })
    }

    pub fn item_resource_id_for_block(&self, block_resource_id: &str) -> Option<&str> {
        self.registry
            .as_ref()
            .and_then(|registry| registry.item_resource_id_for_block(block_resource_id))
            .or_else(|| {
                self.content_registry
                    .item_by_resource_id(block_resource_id)
                    .filter(|spec| spec.is_block_item)
                    .map(|spec| spec.resource_id())
            })
    }

    pub fn primary_block_resource_id_for_item(&self, item_resource_id: &str) -> Option<&str> {
        self.registry
            .as_ref()
            .and_then(|registry| registry.primary_block_resource_id_for_item(item_resource_id))
            .or_else(|| {
                let item = self
                    .content_registry
                    .item_by_resource_id(item_resource_id)?;
                item.is_block_item.then(|| {
                    self.content_registry
                        .block_by_resource_id(item_resource_id)
                        .map(|spec| spec.resource_id())
                })?
            })
    }

    pub fn localized_item_name(&self, resource_id: &str) -> String {
        localized_item_name(&self.language, resource_id)
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
        let item_id = self.item_resource_id_for_protocol_id(protocol_id)?;
        let transforms = self
            .item_display_transforms
            .get(item_id)
            .or_else(|| self.mod_item_display_transforms.get(item_id))?;
        Some(transforms.get(context))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[test]
fn content_registry_sync_resolves_mod_items_icons_and_models() {
    let mut builder = bbb_protocol::content_registry::ContentRegistryBuilder::new();
    builder.register_item(bbb_protocol::content_registry::ModItemRegistration {
        resource_id: "examplemod:ruby_gem".to_string(),
        max_stack_size: 16,
        is_block_item: false,
    });
    let registry = Arc::new(builder.freeze().unwrap());
    let mod_item = registry
        .item_by_resource_id("examplemod:ruby_gem")
        .expect("mod item registered");

    let mut projection = ContentRegistryAssetProjection::default();
    projection.items.insert(
        "examplemod:ruby_gem".to_string(),
        ModItemAssetProjection {
            item_id: mod_item.id,
            resource_id: "examplemod:ruby_gem".to_string(),
            is_block_item: false,
            kind: bbb_client_assets::ModAssetModelKind::FallbackFlat,
            generated: true,
            texture_ids: vec![MISSING_TEXTURE_ID.to_string()],
            model: None,
            fallback_block_model: None,
            fallback_tint: bbb_client_assets::ModAssetTint {
                rgb: [0.8, 0.1, 0.2],
                argb: 0xffcc_1933,
            },
            resolved_from_pack: false,
        },
    );

    let mut runtime = NativeItemRuntime::empty_for_test();
    assert!(runtime.sync_content_registry(Arc::clone(&registry), Some(Arc::new(projection))));
    assert_eq!(
        runtime.item_resource_id(mod_item.id),
        Some("examplemod:ruby_gem")
    );
    assert_eq!(
        runtime.item_protocol_id("examplemod:ruby_gem"),
        Some(mod_item.id)
    );
    let stack = ItemStackSummary {
        item_id: Some(mod_item.id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let resolved = runtime
        .resolve_item_model(
            &stack,
            &ItemModelContext::new(BlockModelDisplayContext::Gui),
        )
        .expect("mod item resolves to an icon model");
    let flat = resolved.flat_icon().expect("mod item has flat icon");
    assert_eq!(flat.layers.len(), 1);
    assert!((flat.layers[0].tint[0] - 0.8).abs() < 1e-5);
}
