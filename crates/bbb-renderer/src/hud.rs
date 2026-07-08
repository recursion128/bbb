use anyhow::{bail, Result};
use winit::dpi::PhysicalSize;

use crate::entity_models::{
    EntityModelInstance, SignModelAttachment, SignModelWood, ENTITY_FULL_BRIGHT_LIGHT_COORDS,
};
use crate::item_models::{
    GuiItemLightingEntry, HudBlockItemModel, ItemModelFoil, ItemModelMeshSet,
    ITEM_MODEL_FULL_BRIGHT_LIGHT, ITEM_MODEL_NO_OVERLAY,
};
use crate::Renderer;

mod gpu;
mod layout;

pub(super) use self::gpu::{
    create_hud_bind_group_layout, create_hud_item_glint_pipeline, create_hud_pipeline,
    create_hud_sprite_gpu, HudSpriteGpu,
};
use self::layout::{
    absolute_hud_rect, air_bubble_hud_rect, armor_hud_rect, boss_bar_hud_rect, centered_hud_rect,
    experience_bar_hud_rect, food_hud_rect, gui_item_slot_placement, hotbar_hud_rect,
    hotbar_item_hud_rect, hotbar_selection_hud_rect, hud_air_bubble_icons,
    hud_air_bubble_wobble_offsets, hud_air_bubbles_visible, hud_armor_fill, hud_boss_bar_fill_uv,
    hud_boss_bar_name_origin, hud_boss_bar_progress_width, hud_boss_bar_rows,
    hud_contextual_bar_progress_width, hud_experience_level_text_origin, hud_food_fill,
    hud_food_jitter_offsets, hud_health_row_geometry, hud_inventory_text_label_origin,
    hud_inventory_tooltip_background_hud_rect, hud_inventory_tooltip_line_origin,
    hud_inventory_tooltip_sprite_segments, hud_inventory_tooltip_text_height,
    hud_item_cooldown_rect, hud_item_count_digit_hud_rect, hud_item_durability_bar_rect,
    hud_overlay_message_text_origin, hud_player_heart_instances, hud_quad_vertices,
    hud_styled_quad_vertices, hud_subtitle_text_origin, hud_title_text_origin,
    hud_vehicle_heart_fill, hud_vehicle_max_hearts, inventory_background_hud_rect,
    inventory_slot_highlight_hud_rect, inventory_slot_item_hud_rect, vehicle_heart_hud_rect,
    HudAirBubbleIcon, HudIconFill, HudRect, HudTooltipSpriteLayer, HUD_AIR_BUBBLES_PER_ROW,
    HUD_ARMOR_ICONS_PER_ROW, HUD_BOSS_BAR_WIDTH, HUD_FOOD_ICONS_PER_ROW, HUD_INVENTORY_ITEM_SIZE,
    HUD_SINGLE_HEALTH_ROW_HEIGHT, HUD_SUBTITLE_TEXT_SCALE, HUD_TITLE_TEXT_SCALE,
    HUD_VEHICLE_HEARTS_PER_ROW,
};

pub use bbb_render_types::{
    HudAsciiGlyph, HudDigitGlyph, HudFontGlyphMap, HudObfuscatedGlyphPool, HudStyledTextRun,
    HudTextStyle, HudUvRect, HUD_FONT_BASELINE,
};

use bbb_render_types::{HudEffectRect, HudGlyphQuad, HudObfuscatedRandom};

pub const HUD_HOTBAR_SLOTS: usize = 9;

/// Nine-slice scaling metadata for a HUD sprite, mirroring vanilla `GuiSpriteScaling.NineSlice`
/// (`assets/.../gui/sprites/**.png.mcmeta` → `gui.scaling`). `sprite_width`/`sprite_height` are the
/// declared nine-slice grid size (the sprite is uploaded as its own texture, so UVs are direct
/// fractions of these), the four borders are per-edge slice widths, and `stretch_inner` selects
/// stretch (`true`) vs tile (`false`) for the four edge slices and the center.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudNineSliceScaling {
    pub sprite_width: u32,
    pub sprite_height: u32,
    pub border_left: u32,
    pub border_top: u32,
    pub border_right: u32,
    pub border_bottom: u32,
    pub stretch_inner: bool,
}

/// A HUD sprite uploaded together with its nine-slice scaling, so draw-time geometry can split it
/// into border/edge/center quads instead of stretching a single flat quad.
pub(crate) struct HudNineSliceSprite {
    pub(crate) gpu: HudSpriteGpu,
    pub(crate) scaling: HudNineSliceScaling,
}

/// One frame's action-bar overlay message state (vanilla
/// `Gui.extractOverlayMessage` inputs): the styled line, the post-tick
/// `overlayMessageTime`, the frame partial tick, and the jukebox rainbow flag
/// (`Gui.animateOverlayMessageColor`). The renderer derives fade alpha and
/// position per frame; it keeps no countdown state of its own.
#[derive(Debug, Clone, PartialEq)]
pub struct HudActionBarText {
    pub runs: Vec<HudStyledTextRun>,
    /// Vanilla `overlayMessageTime` after `Gui.tick` (starts at 60).
    pub remaining_ticks: i32,
    pub partial_tick: f32,
    pub animate_color: bool,
}

/// One frame's title/subtitle overlay state (vanilla `Gui.extractTitle`
/// inputs): styled lines, the post-tick `titleTime`, the fade windows, and
/// the frame partial tick. An empty `subtitle_runs` means no subtitle is set.
#[derive(Debug, Clone, PartialEq)]
pub struct HudTitleText {
    pub title_runs: Vec<HudStyledTextRun>,
    pub subtitle_runs: Vec<HudStyledTextRun>,
    /// Vanilla `titleTime` after `Gui.tick` (starts at fade_in+stay+fade_out).
    pub remaining_ticks: i32,
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
    pub partial_tick: f32,
}

/// One frame's food-bar effect inputs (vanilla `Gui.extractFood`, Gui.java:939-971):
/// the starvation-shake gate (`FoodData.getSaturationLevel() <= 0` plus the
/// client `tickCount` modulo) and the hunger potion swap
/// (`player.hasEffect(MobEffects.HUNGER)` → the `food_*_hunger` sprites). The
/// food level itself is projected separately (`set_hud_food`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudFoodEffect {
    /// Vanilla `FoodData.getSaturationLevel() <= 0` — arms the starvation shake.
    pub saturation_empty: bool,
    /// Vanilla `player.hasEffect(MobEffects.HUNGER)` — swaps to `food_*_hunger`.
    pub hunger_effect: bool,
    /// Client tick counter (vanilla `Gui.tickCount`) gating the shake modulo.
    pub tick_count: u64,
}

/// One frame's air-bubble row inputs (vanilla `Gui.extractAirBubbles`,
/// Gui.java:887-915): the synced air supply, the fixed max, the
/// `isEyeInFluid(FluidTags.WATER)` gate, and the client tick for the all-empty
/// wobble cadence. The renderer derives visibility and the per-bubble
/// full/popping/empty split per frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudAirSupply {
    /// Vanilla `player.getAirSupply()` (the synced `DATA_AIR_SUPPLY_ID` int).
    pub air: i32,
    /// Vanilla `player.getMaxAirSupply()` (300 for players).
    pub max_air: i32,
    /// Vanilla `player.isEyeInFluid(FluidTags.WATER)` (Gui.java:890).
    pub eye_in_water: bool,
    /// Client tick counter (vanilla `Gui.tickCount`) gating the empty-row wobble.
    pub tick_count: u64,
}

/// One frame's vehicle-health inputs (vanilla `Gui.extractVehicleHealth` /
/// `getVehicleMaxHearts`, Gui.java:725-737,974-1005): the living vehicle's
/// synced health and resolved MAX_HEALTH attribute value. `None` (no living
/// vehicle) keeps the food row; a non-zero heart count replaces it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudVehicleHealth {
    /// Vanilla `vehicle.getHealth()`; the draw ceils it (Gui.java:979).
    pub health: f32,
    /// Vanilla `vehicle.getMaxHealth()` (the MAX_HEALTH attribute value).
    pub max_health: f32,
}

/// One frame's jumpable-vehicle contextual bar state (vanilla
/// `JumpableVehicleBarRenderer`): the local player's `getJumpRidingScale`
/// plus whether the controlled mount's `getJumpCooldown()` is still positive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudJumpBar {
    pub progress: f32,
    pub cooldown: bool,
}

/// Vanilla `Gui.HeartType` (Gui.java:1333-1452): the heart sprite family a
/// container/overlay draws. `Container` is the always-drawn background;
/// `Normal`/`Poisoned`/`Withered`/`Frozen` are the mutually exclusive base
/// fills picked by `HeartType.forPlayer`; `Absorbing` overlays the extra
/// absorption hearts. Declaration order is the storage index into the
/// renderer's per-kind sprite array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudHeartKind {
    Container,
    Normal,
    Poisoned,
    Withered,
    Absorbing,
    Frozen,
}

impl HudHeartKind {
    pub const ALL: [Self; 6] = [
        Self::Container,
        Self::Normal,
        Self::Poisoned,
        Self::Withered,
        Self::Absorbing,
        Self::Frozen,
    ];

    /// Vanilla `HeartType.getSprite(isHardcore, isHalf, isBlink)` path fragment
    /// under `hud/heart/` (Gui.java:1334-1435). `Container` ignores `half`
    /// (vanilla routes both slots to the container sprite) and, like `Normal`,
    /// carries no type prefix; the other kinds prefix their name. `Container`
    /// takes the hardcore marker as a `_hardcore` suffix, while the fill kinds
    /// take it as a `hardcore_` prefix — matching vanilla's asset names.
    pub fn sprite_name(self, hardcore: bool, half: bool, blinking: bool) -> String {
        let blink = if blinking { "_blinking" } else { "" };
        if matches!(self, Self::Container) {
            let hardcore = if hardcore { "_hardcore" } else { "" };
            return format!("container{hardcore}{blink}");
        }
        let hardcore = if hardcore { "hardcore_" } else { "" };
        let shape = if half { "half" } else { "full" };
        match self {
            Self::Container => unreachable!("handled above"),
            Self::Normal => format!("{hardcore}{shape}{blink}"),
            Self::Poisoned => format!("poisoned_{hardcore}{shape}{blink}"),
            Self::Withered => format!("withered_{hardcore}{shape}{blink}"),
            Self::Absorbing => format!("absorbing_{hardcore}{shape}{blink}"),
            Self::Frozen => format!("frozen_{hardcore}{shape}{blink}"),
        }
    }
}

/// One frame's player-health inputs (vanilla `Gui.extractPlayerHealth` /
/// `extractHearts`, Gui.java:743-873): the synced health, the resolved
/// MAX_HEALTH attribute, the absorption amount, the base `HeartType`, the
/// hardcore flag, the Regeneration wave gate, and the client tick. The
/// renderer derives the container/absorption rows, `numHealthRows` /
/// `healthRowHeight`, the regen per-heart lift, and the low-health shake.
///
/// Blink (vanilla's damage/heal flash) is intentionally not modeled: it needs
/// the untracked `player.invulnerableTime` and the wall-clock `displayHealth`
/// hold, so `blinking` is always `false` here (see the ledger deferral).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudPlayerHealth {
    /// Vanilla `player.getHealth()`; the draw ceils it (Gui.java:746).
    pub health: f32,
    /// Vanilla `player.getAttributeValue(Attributes.MAX_HEALTH)` (Gui.java:768).
    pub max_health: f32,
    /// Vanilla `player.getAbsorptionAmount()` (Gui.java:769); the draw ceils it.
    pub absorption: f32,
    /// Vanilla `HeartType.forPlayer(player)` base fill (Gui.java:833): one of
    /// `Normal`/`Poisoned`/`Withered`/`Frozen`.
    pub heart_type: HudHeartKind,
    /// Vanilla `player.level().getLevelData().isHardcore()` (Gui.java:834).
    pub hardcore: bool,
    /// Vanilla `player.hasEffect(MobEffects.REGENERATION)` (Gui.java:774) —
    /// arms the per-heart lift wave.
    pub regen: bool,
    /// Client tick counter (vanilla `Gui.tickCount`) driving the regen wave
    /// index and the low-health shake seed.
    pub tick_count: u64,
}

/// Vanilla `BossEvent.BossBarColor` (BossEvent.java:90-97): selects the
/// `boss_bar/{name}_background` / `boss_bar/{name}_progress` sprite pair.
/// Declaration order is the vanilla ordinal (the sprite-array index,
/// BossHealthOverlay.java:20-37).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudBossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

impl HudBossBarColor {
    pub const ALL: [Self; 7] = [
        Self::Pink,
        Self::Blue,
        Self::Red,
        Self::Green,
        Self::Yellow,
        Self::Purple,
        Self::White,
    ];

    /// Vanilla `BossBarColor.getName()` — also the sprite-path fragment.
    pub fn name(self) -> &'static str {
        match self {
            Self::Pink => "pink",
            Self::Blue => "blue",
            Self::Red => "red",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Purple => "purple",
            Self::White => "white",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|color| color.name() == name)
    }
}

/// Vanilla `BossEvent.BossBarOverlay` (BossEvent.java:122-127): `Progress`
/// draws the plain bar; the notched variants layer a `boss_bar/notched_*`
/// sheet over both the background and the fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudBossBarOverlay {
    Progress,
    Notched6,
    Notched10,
    Notched12,
    Notched20,
}

impl HudBossBarOverlay {
    const ALL: [Self; 5] = [
        Self::Progress,
        Self::Notched6,
        Self::Notched10,
        Self::Notched12,
        Self::Notched20,
    ];
    pub const NOTCHED: [Self; 4] = [
        Self::Notched6,
        Self::Notched10,
        Self::Notched12,
        Self::Notched20,
    ];

    /// Vanilla `BossBarOverlay.getName()` — also the sprite-path fragment.
    pub fn name(self) -> &'static str {
        match self {
            Self::Progress => "progress",
            Self::Notched6 => "notched_6",
            Self::Notched10 => "notched_10",
            Self::Notched12 => "notched_12",
            Self::Notched20 => "notched_20",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|overlay| overlay.name() == name)
    }

    /// Index into the notched sprite arrays (vanilla `overlay.ordinal() - 1`,
    /// BossHealthOverlay.java:103); `Progress` has no notched sheet.
    fn notched_index(self) -> Option<usize> {
        (self != Self::Progress).then(|| self as usize - 1)
    }
}

/// One projected boss bar (the render-relevant slice of vanilla
/// `LerpingBossEvent`): the styled name line, the latest packet progress,
/// and the color/overlay style. The projection supplies the stacking order.
#[derive(Debug, Clone, PartialEq)]
pub struct HudBossBar {
    pub name_runs: Vec<HudStyledTextRun>,
    pub progress: f32,
    pub color: HudBossBarColor,
    pub overlay: HudBossBarOverlay,
}

const HUD_TINT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const HUD_TEXT_SHADOW_TINT: [f32; 4] = [0.25, 0.25, 0.25, 1.0];

/// Which food icon a draw needs, so `hud_food_variant_sprite` can pick the base
/// or the Hunger-effect variant of that shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HudFoodSprite {
    Empty,
    Half,
    Full,
}

/// Vanilla `ContextualBarRenderer.extractExperienceLevel` outline color
/// `-16777216` (0xFF000000 opaque black), drawn at the four axis offsets.
const HUD_EXPERIENCE_LEVEL_OUTLINE_TINT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
/// Vanilla `ContextualBarRenderer.extractExperienceLevel` center color
/// `-8323296` (0xFF80FF20 green), drawn last on top of the black outline.
const HUD_EXPERIENCE_LEVEL_FILL_TINT: [f32; 4] = [128.0 / 255.0, 255.0 / 255.0, 32.0 / 255.0, 1.0];
/// The five `extractExperienceLevel` passes in vanilla draw order
/// (ContextualBarRenderer.java:39-43): four black `(±1,0)/(0,±1)` copies, then
/// the green center — each `dropShadow = false`.
const HUD_EXPERIENCE_LEVEL_PASSES: [(f32, f32, [f32; 4]); 5] = [
    (1.0, 0.0, HUD_EXPERIENCE_LEVEL_OUTLINE_TINT),
    (-1.0, 0.0, HUD_EXPERIENCE_LEVEL_OUTLINE_TINT),
    (0.0, 1.0, HUD_EXPERIENCE_LEVEL_OUTLINE_TINT),
    (0.0, -1.0, HUD_EXPERIENCE_LEVEL_OUTLINE_TINT),
    (0.0, 0.0, HUD_EXPERIENCE_LEVEL_FILL_TINT),
];
const HUD_ITEM_BAR_BACKGROUND_TINT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const HUD_ITEM_BAR_BACKGROUND_WIDTH: u32 = 13;
const HUD_ITEM_BAR_BACKGROUND_HEIGHT: u32 = 2;
const HUD_ITEM_BAR_FOREGROUND_HEIGHT: u32 = 1;
const HUD_ITEM_COOLDOWN_TINT: [f32; 4] = [1.0, 1.0, 1.0, 127.0 / 255.0];
const HUD_TOOLTIP_BACKGROUND_TINT: [f32; 4] = [0.0625, 0.0, 0.0625, 0.94];
/// Codepoints the `font/default.json` bitmap pages don't cover (CJK etc. —
/// unihex/unifont is deferred) fall back to this glyph, standing in for the
/// vanilla missing-glyph box.
const HUD_FONT_REPLACEMENT_GLYPH: char = '?';
const HUD_ITEM_SPECIAL_FOIL_GUI_SCALE: f32 = 0.5;
const HUD_ITEM_SPECIAL_FOIL_TEXTURE_SCALE: f32 = 1.0 / 128.0;

#[derive(Debug, Clone, PartialEq)]
pub struct HudIconLayer {
    pub uv: HudUvRect,
    pub tint: [f32; 4],
}

impl HudIconLayer {
    pub fn new(uv: HudUvRect, tint: [f32; 4]) -> Self {
        Self { uv, tint }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudItemIcon {
    pub lighting: GuiItemLightingEntry,
    pub layers: Vec<HudIconLayer>,
    /// Vanilla item foil for flat HUD / inventory sprites. 3D block-item icons use
    /// [`HudBlockItemModel::foil`] in the GUI item pass instead.
    pub foil: HudItemFoil,
    pub count_label: Option<HudItemCountLabel>,
    pub durability_bar: Option<HudItemDurabilityBar>,
    pub cooldown_progress: Option<f32>,
}

impl HudItemIcon {
    pub fn single(uv: HudUvRect) -> Self {
        Self {
            lighting: GuiItemLightingEntry::ItemsFlat,
            layers: vec![HudIconLayer::new(uv, HUD_TINT_WHITE)],
            foil: HudItemFoil::None,
            count_label: None,
            durability_bar: None,
            cooldown_progress: None,
        }
    }
}

/// Vanilla `ItemStackRenderState.FoilType` for flat GUI item sprites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudItemFoil {
    None,
    Standard,
    /// Clocks and `ItemTags.COMPASSES` use `FoilType.SPECIAL`, projecting glint UVs through
    /// `SheetedDecalTextureGenerator` with GUI's `0.5` decal-pose scale.
    Special,
}

impl HudItemFoil {
    pub fn from_has_foil(foil: bool) -> Self {
        if foil {
            Self::Standard
        } else {
            Self::None
        }
    }

    pub fn has_foil(self) -> bool {
        self != Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudItemDurabilityBar {
    pub width: u32,
    pub color: [f32; 3],
}

impl HudItemDurabilityBar {
    pub fn new(width: u32, color: [f32; 3]) -> Self {
        Self { width, color }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudItemCountLabel {
    pub text: String,
}

impl HudItemCountLabel {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudInventoryBackgroundTexture {
    Inventory,
    GenericContainer,
    Dispenser,
    CraftingTable,
    CartographyTable,
    CartographyTableError,
    CartographyTableScaledMap,
    CartographyTableDuplicatedMap,
    CartographyTableMap,
    CartographyTableLocked,
    Loom,
    LoomBannerSlot,
    LoomDyeSlot,
    LoomPatternSlot,
    LoomScroller,
    LoomScrollerDisabled,
    LoomPatternSelected,
    LoomPatternHighlighted,
    LoomPattern,
    LoomError,
    Crafter,
    CrafterDisabledSlot,
    CrafterPoweredRedstone,
    CrafterUnpoweredRedstone,
    WidgetTextField,
    WidgetTextFieldHighlighted,
    Anvil,
    AnvilTextField,
    AnvilTextFieldDisabled,
    AnvilError,
    EnchantingTable,
    EnchantingTableLapisSlot,
    EnchantingTableEnchantmentSlotDisabled,
    EnchantingTableEnchantmentSlotHighlighted,
    EnchantingTableEnchantmentSlot,
    EnchantingTableLevel1,
    EnchantingTableLevel2,
    EnchantingTableLevel3,
    EnchantingTableLevel1Disabled,
    EnchantingTableLevel2Disabled,
    EnchantingTableLevel3Disabled,
    Beacon,
    BeaconButtonDisabled,
    BeaconButtonSelected,
    BeaconButtonHighlighted,
    BeaconButton,
    BeaconConfirm,
    BeaconCancel,
    BeaconEffectSpeed,
    BeaconEffectHaste,
    BeaconEffectResistance,
    BeaconEffectJumpBoost,
    BeaconEffectStrength,
    BeaconEffectRegeneration,
    BrewingStand,
    BrewingStandFuelLength,
    BrewingStandBrewProgress,
    BrewingStandBubbles,
    Furnace,
    FurnaceLitProgress,
    FurnaceBurnProgress,
    AdvancementsWindow,
    RecipeBook,
    RecipeBookTab,
    RecipeBookTabSelected,
    RecipeBookButton,
    RecipeBookButtonHighlighted,
    RecipeBookFilterEnabled,
    RecipeBookFilterDisabled,
    RecipeBookFilterEnabledHighlighted,
    RecipeBookFilterDisabledHighlighted,
    RecipeBookFurnaceFilterEnabled,
    RecipeBookFurnaceFilterDisabled,
    RecipeBookFurnaceFilterEnabledHighlighted,
    RecipeBookFurnaceFilterDisabledHighlighted,
    RecipeBookSlotCraftable,
    RecipeBookSlotUncraftable,
    RecipeBookSlotManyCraftable,
    RecipeBookSlotManyUncraftable,
    RecipeBookPageForward,
    RecipeBookPageForwardHighlighted,
    RecipeBookPageBackward,
    RecipeBookPageBackwardHighlighted,
    RecipeBookOverlayRecipe,
    RecipeBookCraftingOverlay,
    RecipeBookCraftingOverlayHighlighted,
    RecipeBookCraftingOverlayDisabled,
    RecipeBookCraftingOverlayDisabledHighlighted,
    RecipeBookFurnaceOverlay,
    RecipeBookFurnaceOverlayHighlighted,
    RecipeBookFurnaceOverlayDisabled,
    RecipeBookFurnaceOverlayDisabledHighlighted,
    BlastFurnace,
    BlastFurnaceLitProgress,
    BlastFurnaceBurnProgress,
    Smoker,
    SmokerLitProgress,
    SmokerBurnProgress,
    Smithing,
    SmithingError,
    Grindstone,
    GrindstoneError,
    Hopper,
    Horse,
    Nautilus,
    MountSlot,
    MountSaddleSlot,
    MountHorseArmorSlot,
    MountLlamaArmorSlot,
    MountNautilusArmorSlot,
    MountChestSlots,
    Book,
    PageBackward,
    PageForward,
    ShulkerBox,
    Stonecutter,
    StonecutterScroller,
    StonecutterScrollerDisabled,
    StonecutterRecipeSelected,
    StonecutterRecipeHighlighted,
    StonecutterRecipe,
    Villager,
    VillagerOutOfStock,
    VillagerExperienceBarBackground,
    VillagerExperienceBarCurrent,
    VillagerExperienceBarResult,
    VillagerScroller,
    VillagerScrollerDisabled,
    VillagerTradeArrow,
    VillagerTradeArrowOutOfStock,
    VillagerDiscountStrikethrough,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryBackgroundLayer {
    pub texture: HudInventoryBackgroundTexture,
    /// Layer x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Layer y position relative to the centered inventory screen origin.
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub uv: HudUvRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudInventoryFillStage {
    BeforeGhostItem,
    AfterGhostItem,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryFillLayer {
    /// Fill x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Fill y position relative to the centered inventory screen origin.
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub tint: [f32; 4],
    pub stage: HudInventoryFillStage,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventorySlot {
    /// Slot id in the currently open inventory container.
    pub slot_id: u16,
    /// Slot x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Slot y position relative to the centered inventory screen origin.
    pub y: i32,
    pub icon: Option<HudItemIcon>,
    /// The slot's 3D block-item model (vanilla 3D inventory icon), when the item is a block. Drawn in
    /// the GUI item pass at the slot's pixel rect; when present, the 2D `icon`'s flat sprite layers are
    /// suppressed (the 3D model replaces them) while its count / durability overlays still draw.
    pub block_model: Option<HudBlockItemModel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryItem {
    /// Item icon x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Item icon y position relative to the centered inventory screen origin.
    pub y: i32,
    /// Horizontal pose scale applied around the item's top-left GUI rect.
    pub scale: f32,
    /// Vertical pose scale applied around the item's top-left GUI rect.
    pub scale_y: f32,
    pub icon: HudItemIcon,
    /// Whether count, durability, and cooldown overlays should be drawn for this floating item.
    pub draw_decorations: bool,
    /// The item's 3D block-item model (vanilla 3D inventory icon), when it is a block. See
    /// [`HudInventorySlot::block_model`].
    pub block_model: Option<HudBlockItemModel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryGhostItem {
    /// Fake item x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Fake item y position relative to the centered inventory screen origin.
    pub y: i32,
    pub icon: HudItemIcon,
    pub draw_decorations: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudEntityPreviewRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl HudEntityPreviewRect {
    fn right(self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    fn bottom(self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }

    fn intersection(self, other: Self) -> Option<Self> {
        let left = i64::from(self.x.max(other.x));
        let top = i64::from(self.y.max(other.y));
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if left >= right || top >= bottom {
            return None;
        }
        Some(Self {
            x: i32::try_from(left).ok()?,
            y: i32::try_from(top).ok()?,
            width: u32::try_from(right - left).ok()?,
            height: u32::try_from(bottom - top).ok()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudEntityPreviewItemSlot {
    LeftHand,
    Head,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudEntityPreviewItemDisplayContext {
    ThirdPersonLeftHand,
    Head,
}

/// Item layer metadata for a GUI entity picture-in-picture render plan.
///
/// Vanilla `ArmorStandRenderer` registers `ItemInHandLayer` before `WingsLayer` and
/// `CustomHeadLayer`; `SmithingScreen.updateArmorStandPreview` uses
/// `ItemDisplayContext.THIRD_PERSON_LEFT_HAND` for ordinary result stacks and
/// `ItemDisplayContext.HEAD` for HEAD-slot stacks that are not rendered by
/// `HumanoidArmorLayer` / `SkullBlockRenderer`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudEntityPreviewItemLayer {
    pub slot: HudEntityPreviewItemSlot,
    pub display_context: HudEntityPreviewItemDisplayContext,
    pub item_id: i32,
    pub count: i32,
    pub foil: bool,
    pub light_coords: u32,
    pub overlay: [f32; 2],
    pub order: u32,
    pub submit_sequence: u32,
}

/// Vanilla GUI entity picture-in-picture render plan.
///
/// `GuiGraphicsExtractor.entity` submits a `GuiEntityRenderState`, forces the entity render-state light
/// to `LightCoordsUtil.FULL_BRIGHT`, and `GuiEntityRenderer` renders it through an isolated
/// color+depth PIP target under `Lighting.Entry.ENTITY_IN_UI`. The sanitizer keeps the vanilla
/// lighting, bounds, scissor, transform, and depth-isolation contract explicit; the
/// `entity_preview_pip_passes` frame step renders each sanitized preview into its persistent PIP
/// target (`entity_models/gui_preview.rs`) and `collect_hud_draws` blits it in GUI submission
/// order. `item_layers` stays render-plan metadata: hand/head item models are not GPU-drawn yet.
#[derive(Debug, Clone, PartialEq)]
pub struct HudEntityPreview {
    pub entity: EntityModelInstance,
    pub lighting: GuiItemLightingEntry,
    /// Preview bounds in GUI pixels, equivalent to vanilla `x0/y0/x1/y1`.
    pub rect: HudEntityPreviewRect,
    /// Optional GUI scissor rectangle; visible bounds are `rect ∩ scissor`.
    pub scissor: Option<HudEntityPreviewRect>,
    pub translation: [f32; 3],
    /// Quaternion as `[x, y, z, w]`, matching JOML `Quaternionf`.
    pub rotation: [f32; 4],
    /// Optional camera override quaternion as `[x, y, z, w]`.
    pub override_camera_rotation: Option<[f32; 4]>,
    pub scale: f32,
    /// Vanilla PIP renderers use a private color texture and a private depth texture, cleared per preview.
    pub depth_isolated: bool,
    /// Item layers submitted by the preview renderer around the entity model, with vanilla order metadata.
    pub item_layers: Vec<HudEntityPreviewItemLayer>,
}

impl HudEntityPreview {
    pub fn visible_bounds(&self) -> Option<HudEntityPreviewRect> {
        match self.scissor {
            Some(scissor) => self.rect.intersection(scissor),
            None => Some(self.rect),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSignEditorKind {
    Standing {
        wood: SignModelWood,
        attachment: SignModelAttachment,
    },
    Hanging {
        wood: SignModelWood,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudSignEditorScreen {
    pub kind: HudSignEditorKind,
    pub sign_preview: Option<HudEntityPreview>,
    pub title: String,
    pub lines: [String; 4],
    pub line: usize,
    pub cursor: usize,
    pub selection: usize,
    pub cursor_visible: bool,
    pub text_tint: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryTextBackground {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub tint: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryTextInputDecoration {
    pub cursor: usize,
    pub selection: usize,
    pub scroll_to: usize,
    pub max_length: usize,
    pub cursor_visible: bool,
    pub cursor_tint: [f32; 4],
    pub selection_tint: [f32; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryTextLabel {
    /// Text x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Text y position relative to the centered inventory screen origin.
    pub y: i32,
    pub width: u32,
    pub text: String,
    pub tint: [f32; 4],
    pub background: Option<HudInventoryTextBackground>,
    pub input: Option<HudInventoryTextInputDecoration>,
    pub shadow: bool,
    /// Styled draw runs; concatenated run text matches `text`. Leave empty
    /// for plain labels — sanitization synthesizes a single default-style run
    /// from `text`.
    pub runs: Vec<HudStyledTextRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryTooltipLine {
    pub text: String,
    pub tint: [f32; 4],
    /// Styled draw runs; concatenated run text matches `text`. Leave empty
    /// for plain lines — sanitization synthesizes a single default-style run
    /// from `text`.
    pub runs: Vec<HudStyledTextRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryTooltip {
    pub slot_id: u16,
    /// Tooltip anchor x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Tooltip anchor y position relative to the centered inventory screen origin.
    pub y: i32,
    pub lines: Vec<HudInventoryTooltipLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryScreen {
    pub width: u32,
    pub height: u32,
    pub background_layers: Vec<HudInventoryBackgroundLayer>,
    /// Solid GUI fills drawn around recipe-book ghost fake items.
    pub fill_layers: Vec<HudInventoryFillLayer>,
    /// Slots for the currently open inventory container.
    pub slots: Vec<HudInventorySlot>,
    /// Item icons drawn by the inventory screen that are not container slots.
    pub floating_items: Vec<HudInventoryItem>,
    /// Recipe-book ghost fake items drawn above normal slots and below the foreground slot highlight.
    pub ghost_items: Vec<HudInventoryGhostItem>,
    /// Entity previews drawn through vanilla GUI picture-in-picture renderers.
    pub entity_previews: Vec<HudEntityPreview>,
    pub text_labels: Vec<HudInventoryTextLabel>,
    pub hovered_slot_id: Option<u16>,
    pub tooltip: Option<HudInventoryTooltip>,
}

pub(super) enum HudDrawCommand<'a> {
    Sprite {
        sprite: &'a HudSpriteGpu,
        start: u32,
        end: u32,
    },
    ItemGlint {
        mask: &'a HudSpriteGpu,
        start: u32,
        end: u32,
    },
    /// Blit of an entity preview's private PIP color texture into the HUD frame (vanilla
    /// `PictureInPictureRenderer.blitTexture` → `BlitRenderState` on the current GUI layer).
    /// Indexes `Renderer::hud_entity_preview_pip_targets` (same order as the sanitized
    /// `entity_previews`), which `entity_preview_pip_passes` filled earlier in the frame.
    EntityPreviewBlit {
        target_index: usize,
        start: u32,
        end: u32,
    },
}

pub(super) struct HudDraws<'a> {
    pub(super) vertices: Vec<HudVertex>,
    pub(super) commands: Vec<HudDrawCommand<'a>>,
    pub(super) post_gui_item_start: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HudItemIconDrawStep {
    Layers,
    Glint,
    DurabilityBar,
    Cooldown,
    CountLabel,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct HudVertex {
    position: [f32; 2],
    uv: [f32; 2],
    tint: [f32; 4],
    local_uv: [f32; 2],
}

impl Renderer {
    pub fn upload_hud_crosshair(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        let crosshair = self.upload_hud_sprite(width, height, rgba)?;
        self.counters.hud_crosshair_width = width;
        self.counters.hud_crosshair_height = height;
        self.hud_crosshair = Some(crosshair);
        Ok(())
    }

    pub fn upload_hud_hotbar(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_hotbar = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_hotbar_selection(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_hotbar_selection = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_item_atlas(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_item_atlas = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_digit_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        glyphs: [HudDigitGlyph; 10],
    ) -> Result<()> {
        self.hud_digit_atlas = Some(self.upload_hud_sprite(width, height, rgba)?);
        self.hud_digit_glyphs = glyphs;
        Ok(())
    }

    pub fn upload_hud_font_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        glyphs: HudFontGlyphMap,
    ) -> Result<()> {
        self.hud_font_atlas = Some(self.upload_hud_sprite(width, height, rgba)?);
        // Rebuild the advance-grouped obfuscated pool once here, not per frame.
        self.hud_obfuscated_glyph_pool = HudObfuscatedGlyphPool::from_glyph_map(&glyphs);
        self.hud_font_glyphs = glyphs;
        Ok(())
    }

    pub fn upload_hud_inventory_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_inventory_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_tooltip_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        scaling: HudNineSliceScaling,
    ) -> Result<()> {
        self.hud_tooltip_background = Some(HudNineSliceSprite {
            gpu: self.upload_hud_sprite(width, height, rgba)?,
            scaling,
        });
        Ok(())
    }

    pub fn upload_hud_tooltip_frame(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        scaling: HudNineSliceScaling,
    ) -> Result<()> {
        self.hud_tooltip_frame = Some(HudNineSliceSprite {
            gpu: self.upload_hud_sprite(width, height, rgba)?,
            scaling,
        });
        Ok(())
    }

    pub fn upload_hud_generic_container_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_generic_container_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_dispenser_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_dispenser_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_crafting_table_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_crafting_table_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_cartography_table_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_cartography_table_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_cartography_table_error(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_cartography_table_error = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_cartography_table_scaled_map(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_cartography_table_scaled_map = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_cartography_table_duplicated_map(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_cartography_table_duplicated_map =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_cartography_table_map(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_cartography_table_map = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_cartography_table_locked(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_cartography_table_locked = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_loom_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_loom_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_loom_banner_slot(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_loom_banner_slot = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_loom_dye_slot(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_loom_dye_slot = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_loom_pattern_slot(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_loom_pattern_slot = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_loom_scroller(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_loom_scroller = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_loom_scroller_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_loom_scroller_disabled = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_loom_pattern_selected(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_loom_pattern_selected = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_loom_pattern_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_loom_pattern_highlighted = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_loom_pattern(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_loom_pattern = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_loom_error(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_loom_error = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_crafter_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_crafter_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_crafter_disabled_slot(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_crafter_disabled_slot = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_crafter_powered_redstone(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_crafter_powered_redstone = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_crafter_unpowered_redstone(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_crafter_unpowered_redstone = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_widget_text_field(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_widget_text_field = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_widget_text_field_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_widget_text_field_highlighted = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_anvil_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_anvil_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_anvil_text_field(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_anvil_text_field = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_anvil_text_field_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_anvil_text_field_disabled = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_anvil_error(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_anvil_error = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_lapis_slot(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_lapis_slot = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_enchantment_slot_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_enchantment_slot_disabled =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_enchantment_slot_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_enchantment_slot_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_enchantment_slot(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_enchantment_slot =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_level_1(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_level_1 = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_level_2(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_level_2 = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_level_3(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_level_3 = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_level_1_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_level_1_disabled =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_level_2_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_level_2_disabled =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_enchanting_table_level_3_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_enchanting_table_level_3_disabled =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_button_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_button_disabled = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_button_selected(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_button_selected = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_button_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_button_highlighted = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_button(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_beacon_button = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_confirm(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_confirm = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_cancel(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_beacon_cancel = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_effect_speed(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_effect_speed = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_effect_haste(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_effect_haste = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_effect_resistance(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_effect_resistance = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_effect_jump_boost(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_effect_jump_boost = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_effect_strength(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_effect_strength = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_beacon_effect_regeneration(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_beacon_effect_regeneration = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_brewing_stand_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_brewing_stand_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_brewing_stand_fuel_length(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_brewing_stand_fuel_length = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_brewing_stand_brew_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_brewing_stand_brew_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_brewing_stand_bubbles(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_brewing_stand_bubbles = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_furnace_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_furnace_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_furnace_lit_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_furnace_lit_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_furnace_burn_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_furnace_burn_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_advancements_window(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_advancements_window = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_tab(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_tab = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_tab_selected(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_tab_selected = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_button(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_button = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_button_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_button_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_filter_enabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_filter_enabled = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_filter_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_filter_disabled = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_filter_enabled_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_filter_enabled_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_filter_disabled_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_filter_disabled_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_furnace_filter_enabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_furnace_filter_enabled =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_furnace_filter_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_furnace_filter_disabled =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_furnace_filter_enabled_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_furnace_filter_enabled_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_furnace_filter_disabled_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_furnace_filter_disabled_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_slot_craftable(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_slot_craftable = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_slot_uncraftable(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_slot_uncraftable = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_slot_many_craftable(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_slot_many_craftable =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_slot_many_uncraftable(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_slot_many_uncraftable =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_page_forward(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_page_forward = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_page_forward_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_page_forward_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_page_backward(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_page_backward = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_page_backward_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_page_backward_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_overlay_recipe(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_overlay_recipe = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_crafting_overlay(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_crafting_overlay = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_crafting_overlay_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_crafting_overlay_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_crafting_overlay_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_crafting_overlay_disabled =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_crafting_overlay_disabled_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_crafting_overlay_disabled_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_furnace_overlay(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_furnace_overlay = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_furnace_overlay_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_furnace_overlay_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_furnace_overlay_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_furnace_overlay_disabled =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_recipe_book_furnace_overlay_disabled_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_recipe_book_furnace_overlay_disabled_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_blast_furnace_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_blast_furnace_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_blast_furnace_lit_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_blast_furnace_lit_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_blast_furnace_burn_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_blast_furnace_burn_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_smoker_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_smoker_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_smoker_lit_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_smoker_lit_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_smoker_burn_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_smoker_burn_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_smithing_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_smithing_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_smithing_error(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_smithing_error = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_grindstone_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_grindstone_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_grindstone_error(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_grindstone_error = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_hopper_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_hopper_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_horse_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_horse_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_nautilus_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_nautilus_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_mount_slot(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_mount_slot = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_mount_saddle_slot(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_mount_saddle_slot = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_mount_horse_armor_slot(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_mount_horse_armor_slot = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_mount_llama_armor_slot(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_mount_llama_armor_slot = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_mount_nautilus_armor_slot(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_mount_nautilus_armor_slot = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_mount_chest_slots(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_mount_chest_slots = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_book_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_book_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_page_backward(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_page_backward = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_page_forward(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_page_forward = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_hanging_sign_background(
        &mut self,
        wood: SignModelWood,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_hanging_sign_backgrounds[sign_model_wood_index(wood)] =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_shulker_box_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_shulker_box_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_stonecutter_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_stonecutter_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_stonecutter_scroller(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_stonecutter_scroller = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_stonecutter_scroller_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_stonecutter_scroller_disabled = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_stonecutter_recipe_selected(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_stonecutter_recipe_selected = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_stonecutter_recipe_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_stonecutter_recipe_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_stonecutter_recipe(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_stonecutter_recipe = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_villager_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_villager_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_villager_out_of_stock(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_villager_out_of_stock = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_villager_experience_bar_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_villager_experience_bar_background =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_villager_experience_bar_current(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_villager_experience_bar_current =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_villager_experience_bar_result(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_villager_experience_bar_result =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_villager_scroller(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_villager_scroller = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_villager_scroller_disabled(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_villager_scroller_disabled = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_villager_trade_arrow(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_villager_trade_arrow = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_villager_trade_arrow_out_of_stock(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_villager_trade_arrow_out_of_stock =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_villager_discount_strikethrough(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_villager_discount_strikethrough =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_slot_highlight_back(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_slot_highlight_back = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_slot_highlight_front(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_slot_highlight_front = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_experience_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_experience_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_experience_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_experience_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_jump_bar_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_jump_bar_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_jump_bar_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_jump_bar_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_jump_bar_cooldown(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_jump_bar_cooldown = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Uploads one player-heart sprite (vanilla `hud/heart/*`) into the
    /// per-kind slot keyed by `(kind, hardcore, half)`. The asset loader walks
    /// [`HudHeartKind::ALL`] × hardcore × half and resolves each PNG name with
    /// [`HudHeartKind::sprite_name`]; blink variants are not uploaded (blink is
    /// deferred). `Container`'s half is ignored (its half slot mirrors full).
    pub fn upload_hud_heart_sprite(
        &mut self,
        kind: HudHeartKind,
        hardcore: bool,
        half: bool,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        let sprite = self.upload_hud_sprite(width, height, rgba)?;
        let half = half && !matches!(kind, HudHeartKind::Container);
        let variant = usize::from(hardcore) * 2 + usize::from(half);
        self.hud_heart_sprites[kind as usize][variant] = Some(sprite);
        Ok(())
    }

    pub fn upload_hud_food_empty(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_empty = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_full(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_full = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_half(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_half = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/food_empty_hunger` — the empty icon drawn while the player
    /// has the Hunger effect (`Gui.extractFood`, Gui.java:948-951).
    pub fn upload_hud_food_empty_hunger(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_food_empty_hunger = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/food_full_hunger` — the full icon drawn under the Hunger effect.
    pub fn upload_hud_food_full_hunger(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_food_full_hunger = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/food_half_hunger` — the half icon drawn under the Hunger effect.
    pub fn upload_hud_food_half_hunger(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_food_half_hunger = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/armor_empty` — the background slot the armor bar draws for
    /// each icon beyond the armor value (`Gui.extractArmor`, Gui.java:94/814).
    pub fn upload_hud_armor_empty(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_armor_empty = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/armor_half` — the half-filled armor icon (Gui.java:95/810).
    pub fn upload_hud_armor_half(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_armor_half = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/armor_full` — the full armor icon (Gui.java:96/806).
    pub fn upload_hud_armor_full(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_armor_full = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/air` — a full air bubble (`AIR_SPRITE`, Gui.java:103/905).
    pub fn upload_hud_air_bubble(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_air_bubble = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/air_bursting` — the one-tick popping bubble frame
    /// (`AIR_POPPING_SPRITE`, Gui.java:104/907).
    pub fn upload_hud_air_bubble_bursting(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_air_bubble_bursting = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/air_empty` — the popped empty bubble shell
    /// (`AIR_EMPTY_SPRITE`, Gui.java:105/911).
    pub fn upload_hud_air_bubble_empty(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_air_bubble_empty = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/heart/vehicle_container` — a vehicle heart's background
    /// (`HEART_VEHICLE_CONTAINER_SPRITE`, Gui.java:106/991).
    pub fn upload_hud_heart_vehicle_container(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_heart_vehicle_container = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/heart/vehicle_full` — a full vehicle heart overlay
    /// (`HEART_VEHICLE_FULL_SPRITE`, Gui.java:107/993).
    pub fn upload_hud_heart_vehicle_full(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_heart_vehicle_full = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    /// Vanilla `hud/heart/vehicle_half` — a half vehicle heart overlay
    /// (`HEART_VEHICLE_HALF_SPRITE`, Gui.java:108/997).
    pub fn upload_hud_heart_vehicle_half(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_heart_vehicle_half = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_boss_bar_background(
        &mut self,
        color: HudBossBarColor,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_boss_bar_backgrounds[color as usize] =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_boss_bar_progress(
        &mut self,
        color: HudBossBarColor,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_boss_bar_progress_sprites[color as usize] =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_boss_bar_notched_background(
        &mut self,
        overlay: HudBossBarOverlay,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        let Some(index) = overlay.notched_index() else {
            bail!("the progress overlay has no notched boss-bar sprite");
        };
        self.hud_boss_bar_notched_backgrounds[index] =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_boss_bar_notched_progress(
        &mut self,
        overlay: HudBossBarOverlay,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        let Some(index) = overlay.notched_index() else {
            bail!("the progress overlay has no notched boss-bar sprite");
        };
        self.hud_boss_bar_notched_progress_sprites[index] =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn set_hud_code_of_conduct_overlay(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_code_of_conduct_overlay = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn clear_hud_code_of_conduct_overlay(&mut self) {
        self.hud_code_of_conduct_overlay = None;
    }

    /// Projects `Gui.extractPlayerHealth`'s inputs (health, max health,
    /// absorption, base heart type, hardcore, regen, tick). `collect_hud_draws`
    /// derives the container/absorption rows, `numHealthRows`, the regen wave,
    /// and the low-health shake. Non-finite health/max/absorption (malformed
    /// projection) clear the row.
    pub fn set_hud_player_health(&mut self, health: Option<HudPlayerHealth>) {
        self.hud_player_health = health.filter(|health| {
            health.health.is_finite()
                && health.max_health.is_finite()
                && health.absorption.is_finite()
        });
    }

    pub fn set_hud_food(&mut self, food: Option<i32>) {
        self.hud_food = food;
    }

    /// Projects `Gui.extractArmor`'s `player.getArmorValue()` (Gui.java:799); the
    /// draw is gated on `armor > 0` in `collect_hud_draws`, matching vanilla's
    /// visibility test (Gui.java:800).
    pub fn set_hud_armor(&mut self, armor: Option<i32>) {
        self.hud_armor = armor;
    }

    /// Projects `Gui.extractAirBubbles`'s inputs (air supply, max, eye-in-water,
    /// tick); the visibility gate (`isUnderWater || air < max`, Gui.java:891)
    /// is applied in `collect_hud_draws`. A non-positive max (malformed
    /// projection) clears the row rather than dividing by zero.
    pub fn set_hud_air(&mut self, air: Option<HudAirSupply>) {
        self.hud_air = air.filter(|air| air.max_air > 0);
    }

    /// Projects the local player's living vehicle health pair
    /// (`Gui.extractVehicleHealth` inputs); `collect_hud_draws` derives the
    /// heart count via `hud_vehicle_max_hearts` and suppresses the food row
    /// while it is non-zero (Gui.java:784-788). Non-finite values (malformed
    /// projection) clear the bar.
    pub fn set_hud_vehicle_health(&mut self, vehicle: Option<HudVehicleHealth>) {
        self.hud_vehicle_health =
            vehicle.filter(|vehicle| vehicle.health.is_finite() && vehicle.max_health.is_finite());
    }

    /// Projects `JumpableVehicleBarRenderer`'s state. A present value selects
    /// the jumpable-vehicle contextual bar over the experience bar; the
    /// experience level text remains independent, matching vanilla
    /// `Gui.extractRenderState`.
    pub fn set_hud_jump_bar(&mut self, jump_bar: Option<HudJumpBar>) {
        self.hud_jump_bar = jump_bar
            .filter(|jump_bar| jump_bar.progress.is_finite())
            .map(|jump_bar| HudJumpBar {
                progress: jump_bar.progress.clamp(0.0, 1.0),
                cooldown: jump_bar.cooldown,
            });
    }

    /// Projects this frame's food-bar effect state (starvation-shake gate and
    /// hunger potion sprite swap); the food level is set by `set_hud_food`.
    pub fn set_hud_food_effect(&mut self, effect: HudFoodEffect) {
        self.hud_food_effect = effect;
    }

    pub fn set_hud_experience_progress(&mut self, progress: Option<f32>) {
        self.hud_experience_progress_value = progress
            .filter(|progress| progress.is_finite())
            .map(|progress| progress.clamp(0.0, 1.0));
    }

    /// Projects the experience level to render as centered text above the bar.
    /// Vanilla only draws it when `experienceLevel > 0`
    /// (Gui.java:533), so non-positive levels clear the text.
    pub fn set_hud_experience_level(&mut self, level: Option<i32>) {
        self.hud_experience_level = hud_experience_level_projection(level);
    }

    pub fn set_hud_selected_slot(&mut self, slot: u8) {
        self.hud_selected_slot = slot.min((HUD_HOTBAR_SLOTS - 1) as u8);
    }

    pub fn set_hud_hotbar_item_uvs(&mut self, uvs: [Option<HudUvRect>; HUD_HOTBAR_SLOTS]) {
        self.hud_hotbar_item_icons = uvs.map(|uv| uv.and_then(sanitize_hud_hotbar_item_uv));
    }

    pub fn set_hud_hotbar_item_icons(&mut self, icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS]) {
        self.hud_hotbar_item_icons = icons.map(|icon| icon.and_then(sanitize_hud_item_icon));
    }

    pub fn set_hud_inventory_screen(&mut self, screen: Option<HudInventoryScreen>) {
        self.hud_inventory_screen = screen.map(sanitize_hud_inventory_screen);
    }

    pub fn set_hud_sign_editor_screen(&mut self, screen: Option<HudSignEditorScreen>) {
        self.hud_sign_editor_screen = screen.and_then(sanitize_hud_sign_editor_screen);
    }

    pub fn set_hud_action_bar_text(&mut self, action_bar: Option<HudActionBarText>) {
        self.hud_action_bar_text = action_bar.filter(|state| state.partial_tick.is_finite());
    }

    pub fn set_hud_title_text(&mut self, title: Option<HudTitleText>) {
        self.hud_title_text = title.filter(|state| state.partial_tick.is_finite());
    }

    /// Replaces this frame's boss bars (the world's projection of vanilla
    /// `BossHealthOverlay.events`), sanitizing each bar's progress.
    pub fn set_hud_boss_bars(&mut self, bars: Vec<HudBossBar>) {
        self.hud_boss_bars = bars.into_iter().map(sanitize_hud_boss_bar).collect();
    }

    /// Resolves one food-icon sprite, honoring the Hunger potion swap: under the
    /// effect it prefers the uploaded `food_*_hunger` variant, falling back to
    /// the base sprite when the hunger variant is not loaded (vanilla
    /// `Gui.extractFood` selects the sprite id, Gui.java:945-956).
    fn hud_food_variant_sprite(&self, which: HudFoodSprite, hunger: bool) -> Option<&HudSpriteGpu> {
        let (hunger_sprite, base_sprite) = match which {
            HudFoodSprite::Empty => (&self.hud_food_empty_hunger, &self.hud_food_empty),
            HudFoodSprite::Half => (&self.hud_food_half_hunger, &self.hud_food_half),
            HudFoodSprite::Full => (&self.hud_food_full_hunger, &self.hud_food_full),
        };
        hud_food_sprite_variant(hunger, hunger_sprite.as_ref(), base_sprite.as_ref())
    }

    /// One player-heart sprite by `(kind, hardcore, half)` (vanilla
    /// `HeartType.getSprite`, blink always false). `Container`'s half normalizes
    /// to full (its half slots mirror the full sprite).
    fn hud_heart_sprite(
        &self,
        kind: HudHeartKind,
        hardcore: bool,
        half: bool,
    ) -> Option<&HudSpriteGpu> {
        let half = half && !matches!(kind, HudHeartKind::Container);
        let variant = usize::from(hardcore) * 2 + usize::from(half);
        self.hud_heart_sprites[kind as usize][variant].as_ref()
    }

    /// The uploaded 182x5 sheet backing one bar layer (vanilla
    /// `BAR_{BACKGROUND,PROGRESS}_SPRITES` / `OVERLAY_*_SPRITES` lookups,
    /// BossHealthOverlay.java:101-103).
    fn hud_boss_bar_sheet_sprite(&self, sheet: HudBossBarSheet) -> Option<&HudSpriteGpu> {
        match sheet {
            HudBossBarSheet::ColorBackground(color) => {
                self.hud_boss_bar_backgrounds[color as usize].as_ref()
            }
            HudBossBarSheet::ColorProgress(color) => {
                self.hud_boss_bar_progress_sprites[color as usize].as_ref()
            }
            HudBossBarSheet::NotchedBackground(overlay) => overlay
                .notched_index()
                .and_then(|index| self.hud_boss_bar_notched_backgrounds[index].as_ref()),
            HudBossBarSheet::NotchedProgress(overlay) => overlay
                .notched_index()
                .and_then(|index| self.hud_boss_bar_notched_progress_sprites[index].as_ref()),
        }
    }

    pub fn clear_hud_inventory_screen(&mut self) {
        self.hud_inventory_screen = None;
    }

    pub fn clear_hud_sign_editor_screen(&mut self) {
        self.hud_sign_editor_screen = None;
    }

    fn upload_hud_sprite(&self, width: u32, height: u32, rgba: &[u8]) -> Result<HudSpriteGpu> {
        create_hud_sprite_gpu(
            &self.device,
            &self.queue,
            &self.hud_bind_group_layout,
            width,
            height,
            rgba,
        )
    }

    /// Bakes this frame's hotbar 3D block items into GUI pixel space: each slot's block quads under its
    /// slot placement (`translate(slot_center)·scale(slot_px,-slot_px,slot_px)`) composed with the item's
    /// `gui` display transform. The returned set is split into vanilla solid/translucent item phases plus
    /// matching glint buckets for the GUI item pass.
    pub(crate) fn collect_hud_block_item_mesh(&self) -> ItemModelMeshSet {
        let surface_size = self.surface_size();
        let mut meshes = ItemModelMeshSet::default();
        let mut append_model = |model: &HudBlockItemModel, placement: glam::Mat4| {
            append_hud_block_item_model_mesh(&mut meshes, model, placement);
        };
        for (slot, model) in self.hud_hotbar_block_item_models.iter().enumerate() {
            if let Some(model) = model {
                let placement = gui_item_slot_placement(hotbar_item_hud_rect(surface_size, slot));
                append_model(model, placement);
            }
        }
        // The open inventory screen's block items (container slots + the cursor / floating item) render as
        // 3D icons in the same pass, seated in their slot pixel rects.
        if let Some(screen) = &self.hud_inventory_screen {
            let mut append = |model: &HudBlockItemModel, rect: HudRect| {
                let placement = gui_item_slot_placement(rect);
                append_model(model, placement);
            };
            for slot in &screen.slots {
                if let Some(model) = &slot.block_model {
                    append(
                        model,
                        inventory_slot_item_hud_rect(
                            surface_size,
                            screen.width,
                            screen.height,
                            slot.x,
                            slot.y,
                        ),
                    );
                }
            }
            for item in &screen.floating_items {
                if let Some(model) = &item.block_model {
                    append(
                        model,
                        inventory_floating_item_hud_rect(surface_size, screen, item),
                    );
                }
            }
        }
        meshes
    }

    pub(super) fn collect_hud_draws(&self) -> HudDraws<'_> {
        let mut vertices = Vec::new();
        let mut commands = Vec::new();
        let mut post_gui_item_commands = Vec::new();
        let surface_size = self.surface_size();

        if let Some(crosshair) = &self.hud_crosshair {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                crosshair,
                surface_size,
                centered_hud_rect(surface_size, crosshair.width, crosshair.height),
            );
        }

        if let Some(hotbar) = &self.hud_hotbar {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                hotbar,
                surface_size,
                hotbar_hud_rect(surface_size, hotbar.width, hotbar.height),
            );
        }
        if let Some(selection) = &self.hud_hotbar_selection {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                selection,
                surface_size,
                hotbar_selection_hud_rect(
                    surface_size,
                    self.hud_selected_slot,
                    selection.width,
                    selection.height,
                ),
            );
        }

        if let Some(atlas) = &self.hud_item_atlas {
            for (slot, icon) in self.hud_hotbar_item_icons.iter().enumerate() {
                if let Some(icon) = icon {
                    let item_rect = hotbar_item_hud_rect(surface_size, slot);
                    let renders_as_3d_block = self
                        .hud_hotbar_block_item_models
                        .get(slot)
                        .is_some_and(Option::is_some);
                    push_hud_item_icon(
                        &mut vertices,
                        &mut commands,
                        atlas,
                        &self.hud_white_pixel,
                        self.hud_digit_atlas.as_ref(),
                        &self.hud_digit_glyphs,
                        surface_size,
                        item_rect,
                        icon,
                        !renders_as_3d_block,
                        !renders_as_3d_block,
                    );
                    if renders_as_3d_block {
                        push_hud_item_icon(
                            &mut vertices,
                            &mut post_gui_item_commands,
                            atlas,
                            &self.hud_white_pixel,
                            self.hud_digit_atlas.as_ref(),
                            &self.hud_digit_glyphs,
                            surface_size,
                            item_rect,
                            icon,
                            false,
                            true,
                        );
                    }
                }
            }
        }

        if let Some(jump_bar) = self.hud_jump_bar {
            if let Some(background) = &self.hud_jump_bar_background {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    background,
                    surface_size,
                    experience_bar_hud_rect(surface_size, background.width, background.height),
                );
            }
            if jump_bar.cooldown {
                if let Some(cooldown) = &self.hud_jump_bar_cooldown {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        cooldown,
                        surface_size,
                        experience_bar_hud_rect(surface_size, cooldown.width, cooldown.height),
                    );
                }
            } else {
                let progress_width = hud_contextual_bar_progress_width(jump_bar.progress);
                if progress_width > 0 {
                    if let Some(progress_sprite) = &self.hud_jump_bar_progress {
                        push_hud_draw_with_uv(
                            &mut vertices,
                            &mut commands,
                            progress_sprite,
                            surface_size,
                            experience_bar_hud_rect(
                                surface_size,
                                progress_width,
                                progress_sprite.height,
                            ),
                            HudUvRect {
                                min: [0.0, 0.0],
                                max: [
                                    (progress_width as f32 / progress_sprite.width.max(1) as f32)
                                        .clamp(0.0, 1.0),
                                    1.0,
                                ],
                            },
                        );
                    }
                }
            }
        } else if let (Some(progress), Some(background)) = (
            self.hud_experience_progress_value,
            &self.hud_experience_background,
        ) {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                background,
                surface_size,
                experience_bar_hud_rect(surface_size, background.width, background.height),
            );

            let progress_width = hud_contextual_bar_progress_width(progress);
            if progress_width > 0 {
                if let Some(progress_sprite) = &self.hud_experience_progress {
                    push_hud_draw_with_uv(
                        &mut vertices,
                        &mut commands,
                        progress_sprite,
                        surface_size,
                        experience_bar_hud_rect(
                            surface_size,
                            progress_width,
                            progress_sprite.height,
                        ),
                        HudUvRect {
                            min: [0.0, 0.0],
                            max: [
                                (progress_width as f32 / progress_sprite.width.max(1) as f32)
                                    .clamp(0.0, 1.0),
                                1.0,
                            ],
                        },
                    );
                }
            }
        }

        // Vanilla `Gui.extractPlayerHealth` extracts the armor row before the
        // hearts (Gui.java:779 then :781); it only draws when `armor > 0`
        // (`Gui.extractArmor`, Gui.java:800), each of the 10 icons picking
        // full/half/empty per `hud_armor_fill`. The row sits
        // `(numHealthRows - 1) * healthRowHeight + 10` above the heart baseline
        // (Gui.java:801), so multi-row health pushes it up; with no projected
        // health it falls back to a single row (the fixed 10px gap).
        if let Some(armor) = self.hud_armor.filter(|&armor| armor > 0) {
            let (num_health_rows, health_row_height) = self
                .hud_player_health
                .map(hud_health_row_geometry)
                .unwrap_or((1, HUD_SINGLE_HEALTH_ROW_HEIGHT));
            for index in 0..HUD_ARMOR_ICONS_PER_ROW {
                let sprite = match hud_armor_fill(armor, index) {
                    HudIconFill::Empty => self.hud_armor_empty.as_ref(),
                    HudIconFill::Half => self.hud_armor_half.as_ref(),
                    HudIconFill::Full => self.hud_armor_full.as_ref(),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        armor_hud_rect(
                            surface_size,
                            index,
                            num_health_rows,
                            health_row_height,
                            sprite.width,
                            sprite.height,
                        ),
                    );
                }
            }
        }

        // Vanilla `Gui.extractHearts` (Gui.java:820-873, blink deferred): the
        // ordered container / absorption / fill sprites for the player's
        // health + absorption, stacked into `numHealthRows` and carrying the
        // regen lift and low-health shake (`hud_player_heart_instances`).
        if let Some(health) = self.hud_player_health {
            for instance in hud_player_heart_instances(surface_size, health) {
                if let Some(sprite) =
                    self.hud_heart_sprite(instance.kind, health.hardcore, instance.half)
                {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        instance.rect(sprite.width, sprite.height),
                    );
                }
            }
        }

        // Vanilla `Gui.extractPlayerHealth` resolves the living vehicle's heart
        // count once (Gui.java:782-783): a non-zero count replaces the food row
        // (Gui.java:784-788) and shifts the air row (`getAirBubbleYLine`).
        let vehicle_hearts = self
            .hud_vehicle_health
            .map(|vehicle| hud_vehicle_max_hearts(vehicle.max_health))
            .unwrap_or(0);

        if let (0, Some(food)) = (vehicle_hearts, self.hud_food) {
            let effect = self.hud_food_effect;
            // Vanilla `Gui.extractFood` reseeds/advances `this.random` every
            // frame; bbb reseeds the identical LCG from the render frame counter
            // so the shake flickers deterministically (layout::hud_food_jitter_offsets).
            let jitter = hud_food_jitter_offsets(
                food,
                effect.saturation_empty,
                effect.tick_count,
                self.counters.frame_index,
            );
            if let Some(empty) =
                self.hud_food_variant_sprite(HudFoodSprite::Empty, effect.hunger_effect)
            {
                for index in 0..HUD_FOOD_ICONS_PER_ROW {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        empty,
                        surface_size,
                        food_hud_rect(
                            surface_size,
                            index,
                            empty.width,
                            empty.height,
                            jitter[index as usize],
                        ),
                    );
                }

                for index in 0..HUD_FOOD_ICONS_PER_ROW {
                    let sprite =
                        match hud_food_fill(food, index) {
                            HudIconFill::Empty => None,
                            HudIconFill::Half => self
                                .hud_food_variant_sprite(HudFoodSprite::Half, effect.hunger_effect),
                            HudIconFill::Full => self
                                .hud_food_variant_sprite(HudFoodSprite::Full, effect.hunger_effect),
                        };
                    if let Some(sprite) = sprite {
                        push_hud_draw(
                            &mut vertices,
                            &mut commands,
                            sprite,
                            surface_size,
                            food_hud_rect(
                                surface_size,
                                index,
                                sprite.width,
                                sprite.height,
                                jitter[index as usize],
                            ),
                        );
                    }
                }
            }
        }

        // Vanilla `Gui.extractAirBubbles` draws after the food row
        // (Gui.java:790-791), only while under water or below the max supply
        // (Gui.java:891): full bubbles, the one-tick popping frame, and the
        // delayed empty shells (with the all-empty drowning wobble).
        if let Some(air) = self
            .hud_air
            .filter(|air| hud_air_bubbles_visible(air.air, air.max_air, air.eye_in_water))
        {
            let icons = hud_air_bubble_icons(air.air, air.max_air, air.eye_in_water);
            let all_bubbles_empty = icons
                .iter()
                .all(|icon| *icon == Some(HudAirBubbleIcon::Empty));
            let wobble = hud_air_bubble_wobble_offsets(
                all_bubbles_empty,
                air.tick_count,
                self.counters.frame_index,
            );
            for index in 0..HUD_AIR_BUBBLES_PER_ROW {
                let (sprite, y_offset) = match icons[index as usize] {
                    Some(HudAirBubbleIcon::Full) => (self.hud_air_bubble.as_ref(), 0),
                    Some(HudAirBubbleIcon::Popping) => (self.hud_air_bubble_bursting.as_ref(), 0),
                    Some(HudAirBubbleIcon::Empty) => {
                        (self.hud_air_bubble_empty.as_ref(), wobble[index as usize])
                    }
                    None => (None, 0),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        air_bubble_hud_rect(
                            surface_size,
                            index,
                            vehicle_hearts,
                            sprite.width,
                            sprite.height,
                            y_offset,
                        ),
                    );
                }
            }
        }

        // Vanilla `Gui.extractVehicleHealth` runs after `extractPlayerHealth`
        // (Gui.java:523-526): rows of up to 10 hearts stack upward from the
        // food baseline, each heart drawing its container and then the
        // full/half overlay against `ceil(health)` (Gui.java:981-1002).
        if let Some(vehicle) = self.hud_vehicle_health.filter(|_| vehicle_hearts > 0) {
            let mut remaining_hearts = vehicle_hearts;
            let mut row = 0u32;
            while remaining_hearts > 0 {
                let row_hearts = remaining_hearts.min(HUD_VEHICLE_HEARTS_PER_ROW);
                for index in 0..row_hearts {
                    if let Some(container) = &self.hud_heart_vehicle_container {
                        push_hud_draw(
                            &mut vertices,
                            &mut commands,
                            container,
                            surface_size,
                            vehicle_heart_hud_rect(
                                surface_size,
                                row,
                                index,
                                container.width,
                                container.height,
                            ),
                        );
                    }
                    let overlay = match hud_vehicle_heart_fill(vehicle.health, row, index) {
                        HudIconFill::Empty => None,
                        HudIconFill::Half => self.hud_heart_vehicle_half.as_ref(),
                        HudIconFill::Full => self.hud_heart_vehicle_full.as_ref(),
                    };
                    if let Some(overlay) = overlay {
                        push_hud_draw(
                            &mut vertices,
                            &mut commands,
                            overlay,
                            surface_size,
                            vehicle_heart_hud_rect(
                                surface_size,
                                row,
                                index,
                                overlay.width,
                                overlay.height,
                            ),
                        );
                    }
                }
                remaining_hearts -= row_hearts;
                row += 1;
            }
        }

        // Vanilla draws the experience level number between the contextual bar
        // background and its render state (Gui.java:532-535), i.e. after the
        // status bars and before the boss overlay, gated only on
        // `experienceLevel > 0` (independent of which contextual bar — jump /
        // locator / experience — occupies the slot), so no suppression is
        // needed. It needs the font atlas.
        if let (Some(level), Some(font_atlas)) = (self.hud_experience_level, &self.hud_font_atlas) {
            push_hud_experience_level_text(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                font_atlas,
                &self.hud_font_glyphs,
                &self.hud_obfuscated_glyph_pool,
                self.counters.frame_index,
                surface_size,
                level,
            );
        }

        // Vanilla `Gui.extractRenderState` submits the boss overlay right
        // after the hotbar/status decorations and before the overlay message
        // / title strata (Gui.java:203-217). Per bar: the sprite layers, then
        // the name line — opaque white with the default drop shadow
        // (`graphics.text(..., -1)`, BossHealthOverlay.java:71-73).
        for draw in hud_boss_bar_draws(&self.hud_boss_bars, &self.hud_font_glyphs, surface_size) {
            for layer in &draw.layers {
                if let Some(sprite) = self.hud_boss_bar_sheet_sprite(layer.sheet) {
                    push_hud_draw_with_uv(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        boss_bar_hud_rect(surface_size, draw.y, layer.width),
                        hud_boss_bar_fill_uv(layer.width),
                    );
                }
            }
            if let Some(font_atlas) = &self.hud_font_atlas {
                push_hud_screen_text_draw(
                    &mut vertices,
                    &mut commands,
                    &self.hud_white_pixel,
                    font_atlas,
                    &self.hud_font_glyphs,
                    &self.hud_obfuscated_glyph_pool,
                    self.counters.frame_index,
                    surface_size,
                    &draw.name,
                );
            }
        }

        // Vanilla `Gui.extractRenderState` submits the overlay message and the
        // title/subtitle after the hotbar decorations (Gui.java:215-217); open
        // screens render in a later pass, so their draws stay above these.
        if let Some(font_atlas) = &self.hud_font_atlas {
            let mut screen_text_draws = Vec::new();
            if let Some(action_bar) = &self.hud_action_bar_text {
                screen_text_draws.extend(hud_action_bar_text_draw(
                    action_bar,
                    &self.hud_font_glyphs,
                    surface_size,
                ));
            }
            if let Some(title) = &self.hud_title_text {
                screen_text_draws.extend(hud_title_text_draws(
                    title,
                    &self.hud_font_glyphs,
                    surface_size,
                ));
            }
            for draw in &screen_text_draws {
                push_hud_screen_text_draw(
                    &mut vertices,
                    &mut commands,
                    &self.hud_white_pixel,
                    font_atlas,
                    &self.hud_font_glyphs,
                    &self.hud_obfuscated_glyph_pool,
                    self.counters.frame_index,
                    surface_size,
                    draw,
                );
            }
        }

        if let Some(screen) = &self.hud_inventory_screen {
            for layer in &screen.background_layers {
                if let Some(background) = self.hud_inventory_background_sprite(layer.texture) {
                    push_hud_draw_with_uv(
                        &mut vertices,
                        &mut commands,
                        background,
                        surface_size,
                        inventory_background_hud_rect(
                            surface_size,
                            screen.width,
                            screen.height,
                            layer.x,
                            layer.y,
                            layer.width,
                            layer.height,
                        ),
                        layer.uv,
                    );
                }
            }

            // Vanilla screens submit the entity preview right after the background blit
            // (`InventoryScreen.renderBg` / `SmithingScreen.renderBg`), and
            // `PictureInPictureRenderer.blitTexture` adds the PIP color texture to the current GUI
            // layer — above the background, below slot highlights / items / overlays. A scissored
            // preview blits only `rect ∩ scissor`, sampling the matching texture sub-rect (vanilla
            // scissors the full-rect blit; for an axis-aligned scissor the two are equivalent).
            // Under wgpu the PIP texture is already GUI-oriented (row 0 = top), so the UVs are
            // identity fractions of the rect — vanilla's `v0=1, v1=0` flip is a GL
            // framebuffer-origin artifact.
            for (target_index, preview) in screen.entity_previews.iter().enumerate() {
                if self
                    .hud_entity_preview_pip_targets
                    .get(target_index)
                    .is_none()
                {
                    continue;
                }
                let Some(visible) = preview.visible_bounds() else {
                    continue;
                };
                let uv = hud_entity_preview_blit_uv(preview.rect, visible);
                let start = vertices.len() as u32;
                vertices.extend_from_slice(&hud_quad_vertices(
                    surface_size,
                    inventory_background_hud_rect(
                        surface_size,
                        screen.width,
                        screen.height,
                        visible.x,
                        visible.y,
                        visible.width,
                        visible.height,
                    ),
                    uv,
                    [1.0, 1.0, 1.0, 1.0],
                ));
                commands.push(HudDrawCommand::EntityPreviewBlit {
                    target_index,
                    start,
                    end: vertices.len() as u32,
                });
            }

            let hovered_slot = screen
                .hovered_slot_id
                .and_then(|slot_id| screen.slots.iter().find(|slot| slot.slot_id == slot_id));

            if let (Some(slot), Some(highlight)) = (hovered_slot, &self.hud_slot_highlight_back) {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    highlight,
                    surface_size,
                    inventory_slot_highlight_hud_rect(
                        surface_size,
                        screen.width,
                        screen.height,
                        slot.x,
                        slot.y,
                    ),
                );
            }

            let item_atlas = self.hud_item_atlas.as_ref();
            if let Some(atlas) = item_atlas {
                for slot in &screen.slots {
                    if let Some(icon) = &slot.icon {
                        let item_rect = inventory_slot_item_hud_rect(
                            surface_size,
                            screen.width,
                            screen.height,
                            slot.x,
                            slot.y,
                        );
                        push_hud_item_icon(
                            &mut vertices,
                            &mut commands,
                            atlas,
                            &self.hud_white_pixel,
                            self.hud_digit_atlas.as_ref(),
                            &self.hud_digit_glyphs,
                            surface_size,
                            item_rect,
                            icon,
                            slot.block_model.is_none(),
                            slot.block_model.is_none(),
                        );
                        if slot.block_model.is_some() {
                            push_hud_item_icon(
                                &mut vertices,
                                &mut post_gui_item_commands,
                                atlas,
                                &self.hud_white_pixel,
                                self.hud_digit_atlas.as_ref(),
                                &self.hud_digit_glyphs,
                                surface_size,
                                item_rect,
                                icon,
                                false,
                                true,
                            );
                        }
                    }
                }
            }

            push_hud_inventory_fill_layers(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                surface_size,
                screen,
                HudInventoryFillStage::BeforeGhostItem,
            );
            if let Some(atlas) = item_atlas {
                for item in &screen.ghost_items {
                    let item_rect = inventory_slot_item_hud_rect(
                        surface_size,
                        screen.width,
                        screen.height,
                        item.x,
                        item.y,
                    );
                    push_hud_item_icon(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        atlas,
                        &self.hud_white_pixel,
                        self.hud_digit_atlas.as_ref(),
                        &self.hud_digit_glyphs,
                        surface_size,
                        item_rect,
                        &item.icon,
                        true,
                        false,
                    );
                }
            }
            push_hud_inventory_fill_layers(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                surface_size,
                screen,
                HudInventoryFillStage::AfterGhostItem,
            );
            if let Some(atlas) = item_atlas {
                for item in screen
                    .ghost_items
                    .iter()
                    .filter(|item| item.draw_decorations)
                {
                    let item_rect = inventory_slot_item_hud_rect(
                        surface_size,
                        screen.width,
                        screen.height,
                        item.x,
                        item.y,
                    );
                    push_hud_item_icon(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        atlas,
                        &self.hud_white_pixel,
                        self.hud_digit_atlas.as_ref(),
                        &self.hud_digit_glyphs,
                        surface_size,
                        item_rect,
                        &item.icon,
                        false,
                        true,
                    );
                }
            }

            if let Some(atlas) = item_atlas {
                for item in &screen.floating_items {
                    let item_rect = inventory_floating_item_hud_rect(surface_size, screen, item);
                    push_hud_item_icon(
                        &mut vertices,
                        &mut commands,
                        atlas,
                        &self.hud_white_pixel,
                        self.hud_digit_atlas.as_ref(),
                        &self.hud_digit_glyphs,
                        surface_size,
                        item_rect,
                        &item.icon,
                        item.block_model.is_none(),
                        item.draw_decorations && item.block_model.is_none(),
                    );
                    if item.block_model.is_some() {
                        push_hud_item_icon(
                            &mut vertices,
                            &mut post_gui_item_commands,
                            atlas,
                            &self.hud_white_pixel,
                            self.hud_digit_atlas.as_ref(),
                            &self.hud_digit_glyphs,
                            surface_size,
                            item_rect,
                            &item.icon,
                            false,
                            item.draw_decorations,
                        );
                    }
                }
            }

            push_hud_inventory_text_labels(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.hud_font_atlas.as_ref(),
                &self.hud_font_glyphs,
                &self.hud_obfuscated_glyph_pool,
                self.counters.frame_index,
                surface_size,
                screen,
            );

            if let (Some(slot), Some(highlight)) = (hovered_slot, &self.hud_slot_highlight_front) {
                push_hud_draw(
                    &mut vertices,
                    &mut post_gui_item_commands,
                    highlight,
                    surface_size,
                    inventory_slot_highlight_hud_rect(
                        surface_size,
                        screen.width,
                        screen.height,
                        slot.x,
                        slot.y,
                    ),
                );
            }

            push_hud_inventory_tooltip(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.hud_tooltip_background.as_ref(),
                self.hud_tooltip_frame.as_ref(),
                self.hud_font_atlas.as_ref(),
                &self.hud_font_glyphs,
                &self.hud_obfuscated_glyph_pool,
                self.counters.frame_index,
                surface_size,
                screen,
            );
        }

        if let Some(screen) = &self.hud_sign_editor_screen {
            push_hud_sign_editor_screen(
                &mut vertices,
                &mut commands,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.hud_font_atlas.as_ref(),
                &self.hud_font_glyphs,
                &self.hud_obfuscated_glyph_pool,
                self.counters.frame_index,
                surface_size,
                screen,
                !self.hud_entity_preview_pip_targets.is_empty(),
                &self.hud_hanging_sign_backgrounds,
            );
        }

        if let Some(overlay) = &self.hud_code_of_conduct_overlay {
            push_hud_draw(
                &mut vertices,
                &mut post_gui_item_commands,
                overlay,
                surface_size,
                centered_hud_rect(surface_size, overlay.width, overlay.height),
            );
        }

        let post_gui_item_start = commands.len();
        commands.extend(post_gui_item_commands);

        HudDraws {
            vertices,
            commands,
            post_gui_item_start,
        }
    }

    fn hud_inventory_background_sprite(
        &self,
        texture: HudInventoryBackgroundTexture,
    ) -> Option<&HudSpriteGpu> {
        match texture {
            HudInventoryBackgroundTexture::Inventory => self.hud_inventory_background.as_ref(),
            HudInventoryBackgroundTexture::GenericContainer => {
                self.hud_generic_container_background.as_ref()
            }
            HudInventoryBackgroundTexture::Dispenser => self.hud_dispenser_background.as_ref(),
            HudInventoryBackgroundTexture::CraftingTable => {
                self.hud_crafting_table_background.as_ref()
            }
            HudInventoryBackgroundTexture::CartographyTable => {
                self.hud_cartography_table_background.as_ref()
            }
            HudInventoryBackgroundTexture::CartographyTableError => {
                self.hud_cartography_table_error.as_ref()
            }
            HudInventoryBackgroundTexture::CartographyTableScaledMap => {
                self.hud_cartography_table_scaled_map.as_ref()
            }
            HudInventoryBackgroundTexture::CartographyTableDuplicatedMap => {
                self.hud_cartography_table_duplicated_map.as_ref()
            }
            HudInventoryBackgroundTexture::CartographyTableMap => {
                self.hud_cartography_table_map.as_ref()
            }
            HudInventoryBackgroundTexture::CartographyTableLocked => {
                self.hud_cartography_table_locked.as_ref()
            }
            HudInventoryBackgroundTexture::Loom => self.hud_loom_background.as_ref(),
            HudInventoryBackgroundTexture::LoomBannerSlot => self.hud_loom_banner_slot.as_ref(),
            HudInventoryBackgroundTexture::LoomDyeSlot => self.hud_loom_dye_slot.as_ref(),
            HudInventoryBackgroundTexture::LoomPatternSlot => self.hud_loom_pattern_slot.as_ref(),
            HudInventoryBackgroundTexture::LoomScroller => self.hud_loom_scroller.as_ref(),
            HudInventoryBackgroundTexture::LoomScrollerDisabled => {
                self.hud_loom_scroller_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::LoomPatternSelected => {
                self.hud_loom_pattern_selected.as_ref()
            }
            HudInventoryBackgroundTexture::LoomPatternHighlighted => {
                self.hud_loom_pattern_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::LoomPattern => self.hud_loom_pattern.as_ref(),
            HudInventoryBackgroundTexture::LoomError => self.hud_loom_error.as_ref(),
            HudInventoryBackgroundTexture::Crafter => self.hud_crafter_background.as_ref(),
            HudInventoryBackgroundTexture::CrafterDisabledSlot => {
                self.hud_crafter_disabled_slot.as_ref()
            }
            HudInventoryBackgroundTexture::CrafterPoweredRedstone => {
                self.hud_crafter_powered_redstone.as_ref()
            }
            HudInventoryBackgroundTexture::CrafterUnpoweredRedstone => {
                self.hud_crafter_unpowered_redstone.as_ref()
            }
            HudInventoryBackgroundTexture::WidgetTextField => self.hud_widget_text_field.as_ref(),
            HudInventoryBackgroundTexture::WidgetTextFieldHighlighted => {
                self.hud_widget_text_field_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::Anvil => self.hud_anvil_background.as_ref(),
            HudInventoryBackgroundTexture::AnvilTextField => self.hud_anvil_text_field.as_ref(),
            HudInventoryBackgroundTexture::AnvilTextFieldDisabled => {
                self.hud_anvil_text_field_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::AnvilError => self.hud_anvil_error.as_ref(),
            HudInventoryBackgroundTexture::EnchantingTable => {
                self.hud_enchanting_table_background.as_ref()
            }
            HudInventoryBackgroundTexture::EnchantingTableLapisSlot => {
                self.hud_enchanting_table_lapis_slot.as_ref()
            }
            HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled => {
                self.hud_enchanting_table_enchantment_slot_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotHighlighted => self
                .hud_enchanting_table_enchantment_slot_highlighted
                .as_ref(),
            HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlot => {
                self.hud_enchanting_table_enchantment_slot.as_ref()
            }
            HudInventoryBackgroundTexture::EnchantingTableLevel1 => {
                self.hud_enchanting_table_level_1.as_ref()
            }
            HudInventoryBackgroundTexture::EnchantingTableLevel2 => {
                self.hud_enchanting_table_level_2.as_ref()
            }
            HudInventoryBackgroundTexture::EnchantingTableLevel3 => {
                self.hud_enchanting_table_level_3.as_ref()
            }
            HudInventoryBackgroundTexture::EnchantingTableLevel1Disabled => {
                self.hud_enchanting_table_level_1_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::EnchantingTableLevel2Disabled => {
                self.hud_enchanting_table_level_2_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::EnchantingTableLevel3Disabled => {
                self.hud_enchanting_table_level_3_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::Beacon => self.hud_beacon_background.as_ref(),
            HudInventoryBackgroundTexture::BeaconButtonDisabled => {
                self.hud_beacon_button_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::BeaconButtonSelected => {
                self.hud_beacon_button_selected.as_ref()
            }
            HudInventoryBackgroundTexture::BeaconButtonHighlighted => {
                self.hud_beacon_button_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::BeaconButton => self.hud_beacon_button.as_ref(),
            HudInventoryBackgroundTexture::BeaconConfirm => self.hud_beacon_confirm.as_ref(),
            HudInventoryBackgroundTexture::BeaconCancel => self.hud_beacon_cancel.as_ref(),
            HudInventoryBackgroundTexture::BeaconEffectSpeed => {
                self.hud_beacon_effect_speed.as_ref()
            }
            HudInventoryBackgroundTexture::BeaconEffectHaste => {
                self.hud_beacon_effect_haste.as_ref()
            }
            HudInventoryBackgroundTexture::BeaconEffectResistance => {
                self.hud_beacon_effect_resistance.as_ref()
            }
            HudInventoryBackgroundTexture::BeaconEffectJumpBoost => {
                self.hud_beacon_effect_jump_boost.as_ref()
            }
            HudInventoryBackgroundTexture::BeaconEffectStrength => {
                self.hud_beacon_effect_strength.as_ref()
            }
            HudInventoryBackgroundTexture::BeaconEffectRegeneration => {
                self.hud_beacon_effect_regeneration.as_ref()
            }
            HudInventoryBackgroundTexture::BrewingStand => {
                self.hud_brewing_stand_background.as_ref()
            }
            HudInventoryBackgroundTexture::BrewingStandFuelLength => {
                self.hud_brewing_stand_fuel_length.as_ref()
            }
            HudInventoryBackgroundTexture::BrewingStandBrewProgress => {
                self.hud_brewing_stand_brew_progress.as_ref()
            }
            HudInventoryBackgroundTexture::BrewingStandBubbles => {
                self.hud_brewing_stand_bubbles.as_ref()
            }
            HudInventoryBackgroundTexture::Furnace => self.hud_furnace_background.as_ref(),
            HudInventoryBackgroundTexture::FurnaceLitProgress => {
                self.hud_furnace_lit_progress.as_ref()
            }
            HudInventoryBackgroundTexture::FurnaceBurnProgress => {
                self.hud_furnace_burn_progress.as_ref()
            }
            HudInventoryBackgroundTexture::AdvancementsWindow => {
                self.hud_advancements_window.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBook => self.hud_recipe_book_background.as_ref(),
            HudInventoryBackgroundTexture::RecipeBookTab => self.hud_recipe_book_tab.as_ref(),
            HudInventoryBackgroundTexture::RecipeBookTabSelected => {
                self.hud_recipe_book_tab_selected.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookButton => self.hud_recipe_book_button.as_ref(),
            HudInventoryBackgroundTexture::RecipeBookButtonHighlighted => {
                self.hud_recipe_book_button_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookFilterEnabled => {
                self.hud_recipe_book_filter_enabled.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookFilterDisabled => {
                self.hud_recipe_book_filter_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookFilterEnabledHighlighted => {
                self.hud_recipe_book_filter_enabled_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookFilterDisabledHighlighted => {
                self.hud_recipe_book_filter_disabled_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookFurnaceFilterEnabled => {
                self.hud_recipe_book_furnace_filter_enabled.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookFurnaceFilterDisabled => {
                self.hud_recipe_book_furnace_filter_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookFurnaceFilterEnabledHighlighted => self
                .hud_recipe_book_furnace_filter_enabled_highlighted
                .as_ref(),
            HudInventoryBackgroundTexture::RecipeBookFurnaceFilterDisabledHighlighted => self
                .hud_recipe_book_furnace_filter_disabled_highlighted
                .as_ref(),
            HudInventoryBackgroundTexture::RecipeBookSlotCraftable => {
                self.hud_recipe_book_slot_craftable.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookSlotUncraftable => {
                self.hud_recipe_book_slot_uncraftable.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookSlotManyCraftable => {
                self.hud_recipe_book_slot_many_craftable.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookSlotManyUncraftable => {
                self.hud_recipe_book_slot_many_uncraftable.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookPageForward => {
                self.hud_recipe_book_page_forward.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookPageForwardHighlighted => {
                self.hud_recipe_book_page_forward_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookPageBackward => {
                self.hud_recipe_book_page_backward.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookPageBackwardHighlighted => {
                self.hud_recipe_book_page_backward_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookOverlayRecipe => {
                self.hud_recipe_book_overlay_recipe.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookCraftingOverlay => {
                self.hud_recipe_book_crafting_overlay.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookCraftingOverlayHighlighted => {
                self.hud_recipe_book_crafting_overlay_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookCraftingOverlayDisabled => {
                self.hud_recipe_book_crafting_overlay_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookCraftingOverlayDisabledHighlighted => self
                .hud_recipe_book_crafting_overlay_disabled_highlighted
                .as_ref(),
            HudInventoryBackgroundTexture::RecipeBookFurnaceOverlay => {
                self.hud_recipe_book_furnace_overlay.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookFurnaceOverlayHighlighted => {
                self.hud_recipe_book_furnace_overlay_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookFurnaceOverlayDisabled => {
                self.hud_recipe_book_furnace_overlay_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::RecipeBookFurnaceOverlayDisabledHighlighted => self
                .hud_recipe_book_furnace_overlay_disabled_highlighted
                .as_ref(),
            HudInventoryBackgroundTexture::BlastFurnace => {
                self.hud_blast_furnace_background.as_ref()
            }
            HudInventoryBackgroundTexture::BlastFurnaceLitProgress => {
                self.hud_blast_furnace_lit_progress.as_ref()
            }
            HudInventoryBackgroundTexture::BlastFurnaceBurnProgress => {
                self.hud_blast_furnace_burn_progress.as_ref()
            }
            HudInventoryBackgroundTexture::Smoker => self.hud_smoker_background.as_ref(),
            HudInventoryBackgroundTexture::SmokerLitProgress => {
                self.hud_smoker_lit_progress.as_ref()
            }
            HudInventoryBackgroundTexture::SmokerBurnProgress => {
                self.hud_smoker_burn_progress.as_ref()
            }
            HudInventoryBackgroundTexture::Smithing => self.hud_smithing_background.as_ref(),
            HudInventoryBackgroundTexture::SmithingError => self.hud_smithing_error.as_ref(),
            HudInventoryBackgroundTexture::Grindstone => self.hud_grindstone_background.as_ref(),
            HudInventoryBackgroundTexture::GrindstoneError => self.hud_grindstone_error.as_ref(),
            HudInventoryBackgroundTexture::Hopper => self.hud_hopper_background.as_ref(),
            HudInventoryBackgroundTexture::Horse => self.hud_horse_background.as_ref(),
            HudInventoryBackgroundTexture::Nautilus => self.hud_nautilus_background.as_ref(),
            HudInventoryBackgroundTexture::MountSlot => self.hud_mount_slot.as_ref(),
            HudInventoryBackgroundTexture::MountSaddleSlot => self.hud_mount_saddle_slot.as_ref(),
            HudInventoryBackgroundTexture::MountHorseArmorSlot => {
                self.hud_mount_horse_armor_slot.as_ref()
            }
            HudInventoryBackgroundTexture::MountLlamaArmorSlot => {
                self.hud_mount_llama_armor_slot.as_ref()
            }
            HudInventoryBackgroundTexture::MountNautilusArmorSlot => {
                self.hud_mount_nautilus_armor_slot.as_ref()
            }
            HudInventoryBackgroundTexture::MountChestSlots => self.hud_mount_chest_slots.as_ref(),
            HudInventoryBackgroundTexture::Book => self.hud_book_background.as_ref(),
            HudInventoryBackgroundTexture::PageBackward => self.hud_page_backward.as_ref(),
            HudInventoryBackgroundTexture::PageForward => self.hud_page_forward.as_ref(),
            HudInventoryBackgroundTexture::ShulkerBox => self.hud_shulker_box_background.as_ref(),
            HudInventoryBackgroundTexture::Stonecutter => self.hud_stonecutter_background.as_ref(),
            HudInventoryBackgroundTexture::StonecutterScroller => {
                self.hud_stonecutter_scroller.as_ref()
            }
            HudInventoryBackgroundTexture::StonecutterScrollerDisabled => {
                self.hud_stonecutter_scroller_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::StonecutterRecipeSelected => {
                self.hud_stonecutter_recipe_selected.as_ref()
            }
            HudInventoryBackgroundTexture::StonecutterRecipeHighlighted => {
                self.hud_stonecutter_recipe_highlighted.as_ref()
            }
            HudInventoryBackgroundTexture::StonecutterRecipe => {
                self.hud_stonecutter_recipe.as_ref()
            }
            HudInventoryBackgroundTexture::Villager => self.hud_villager_background.as_ref(),
            HudInventoryBackgroundTexture::VillagerOutOfStock => {
                self.hud_villager_out_of_stock.as_ref()
            }
            HudInventoryBackgroundTexture::VillagerExperienceBarBackground => {
                self.hud_villager_experience_bar_background.as_ref()
            }
            HudInventoryBackgroundTexture::VillagerExperienceBarCurrent => {
                self.hud_villager_experience_bar_current.as_ref()
            }
            HudInventoryBackgroundTexture::VillagerExperienceBarResult => {
                self.hud_villager_experience_bar_result.as_ref()
            }
            HudInventoryBackgroundTexture::VillagerScroller => self.hud_villager_scroller.as_ref(),
            HudInventoryBackgroundTexture::VillagerScrollerDisabled => {
                self.hud_villager_scroller_disabled.as_ref()
            }
            HudInventoryBackgroundTexture::VillagerTradeArrow => {
                self.hud_villager_trade_arrow.as_ref()
            }
            HudInventoryBackgroundTexture::VillagerTradeArrowOutOfStock => {
                self.hud_villager_trade_arrow_out_of_stock.as_ref()
            }
            HudInventoryBackgroundTexture::VillagerDiscountStrikethrough => {
                self.hud_villager_discount_strikethrough.as_ref()
            }
        }
    }
}

fn append_hud_block_item_model_mesh(
    meshes: &mut ItemModelMeshSet,
    model: &HudBlockItemModel,
    placement: glam::Mat4,
) {
    let transform = placement * model.gui_display;
    meshes.append_quads_with_light_and_overlay_and_foil(
        &model.quads,
        transform,
        ITEM_MODEL_FULL_BRIGHT_LIGHT,
        ITEM_MODEL_NO_OVERLAY,
        ItemModelFoil::from_has_foil(model.foil),
    );
}

fn push_hud_draw<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
) {
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        sprite,
        surface_size,
        rect,
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        HUD_TINT_WHITE,
    );
}

fn push_hud_draw_with_uv<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv: HudUvRect,
) {
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        sprite,
        surface_size,
        rect,
        uv,
        HUD_TINT_WHITE,
    );
}

fn push_hud_draw_with_uv_and_tint<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv: HudUvRect,
    tint: [f32; 4],
) {
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_quad_vertices(surface_size, rect, uv, tint));
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand::Sprite { sprite, start, end });
}

fn inventory_floating_item_hud_rect(
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    item: &HudInventoryItem,
) -> HudRect {
    let width = ((HUD_INVENTORY_ITEM_SIZE as f32) * item.scale)
        .round()
        .clamp(1.0, 512.0) as u32;
    let height = ((HUD_INVENTORY_ITEM_SIZE as f32) * item.scale_y)
        .round()
        .clamp(1.0, 512.0) as u32;
    inventory_background_hud_rect(
        surface_size,
        screen.width,
        screen.height,
        item.x,
        item.y,
        width,
        height,
    )
}

fn push_hud_item_glint<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    item_atlas: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    item_rect: HudRect,
    layer: &HudIconLayer,
    foil: HudItemFoil,
) {
    let start = vertices.len() as u32;
    let mut quad_vertices = hud_quad_vertices(
        surface_size,
        item_rect,
        layer.uv,
        [1.0, 1.0, 1.0, layer.tint[3]],
    );
    if foil == HudItemFoil::Special {
        for vertex in &mut quad_vertices {
            vertex.local_uv = hud_item_special_foil_glint_uv(vertex.local_uv);
        }
    }
    vertices.extend_from_slice(&quad_vertices);
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand::ItemGlint {
        mask: item_atlas,
        start,
        end,
    });
}

fn push_hud_item_icon<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    item_atlas: &'a HudSpriteGpu,
    white_pixel: &'a HudSpriteGpu,
    digit_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &[HudDigitGlyph; 10],
    surface_size: PhysicalSize<u32>,
    item_rect: HudRect,
    icon: &HudItemIcon,
    // When the slot also renders a 3D block model (in the GUI item pass), its 2D sprite layers are the
    // flat block-texture stand-in that the 3D icon replaces. Decorations are deferred until after the
    // GUI item pass so the 3D model cannot cover count / durability / cooldown overlays.
    draw_layers: bool,
    draw_decorations: bool,
) {
    for_each_hud_item_icon_draw_step(icon, draw_layers, draw_decorations, |step| match step {
        HudItemIconDrawStep::Layers => {
            for layer in &icon.layers {
                push_hud_draw_with_uv_and_tint(
                    vertices,
                    commands,
                    item_atlas,
                    surface_size,
                    item_rect,
                    layer.uv,
                    layer.tint,
                );
            }
        }
        HudItemIconDrawStep::Glint => {
            for layer in &icon.layers {
                push_hud_item_glint(
                    vertices,
                    commands,
                    item_atlas,
                    surface_size,
                    item_rect,
                    layer,
                    icon.foil,
                );
            }
        }
        HudItemIconDrawStep::DurabilityBar => push_hud_item_durability_bar(
            vertices,
            commands,
            white_pixel,
            surface_size,
            item_rect,
            icon.durability_bar.as_ref(),
        ),
        HudItemIconDrawStep::Cooldown => push_hud_item_cooldown(
            vertices,
            commands,
            white_pixel,
            surface_size,
            item_rect,
            icon.cooldown_progress,
        ),
        HudItemIconDrawStep::CountLabel => push_hud_item_count_label(
            vertices,
            commands,
            digit_atlas,
            glyphs,
            surface_size,
            item_rect,
            icon.count_label.as_ref(),
        ),
    });
}

fn for_each_hud_item_icon_draw_step(
    icon: &HudItemIcon,
    draw_layers: bool,
    draw_decorations: bool,
    mut emit: impl FnMut(HudItemIconDrawStep),
) {
    if draw_layers && !icon.layers.is_empty() {
        emit(HudItemIconDrawStep::Layers);
        if icon.foil.has_foil() {
            emit(HudItemIconDrawStep::Glint);
        }
    }
    if !draw_decorations {
        return;
    }
    if icon.durability_bar.is_some() {
        emit(HudItemIconDrawStep::DurabilityBar);
    }
    if icon.cooldown_progress.is_some() {
        emit(HudItemIconDrawStep::Cooldown);
    }
    if icon.count_label.is_some() {
        emit(HudItemIconDrawStep::CountLabel);
    }
}

fn hud_item_special_foil_glint_uv(local_uv: [f32; 2]) -> [f32; 2] {
    let scale = HUD_ITEM_SPECIAL_FOIL_TEXTURE_SCALE / HUD_ITEM_SPECIAL_FOIL_GUI_SCALE;
    [local_uv[0] * scale, -local_uv[1] * scale]
}

fn push_hud_item_durability_bar<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    item_rect: HudRect,
    bar: Option<&HudItemDurabilityBar>,
) {
    let Some(bar) = bar else {
        return;
    };
    let width = bar.width.min(HUD_ITEM_BAR_BACKGROUND_WIDTH);
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        white_pixel,
        surface_size,
        hud_item_durability_bar_rect(
            item_rect,
            HUD_ITEM_BAR_BACKGROUND_WIDTH,
            HUD_ITEM_BAR_BACKGROUND_HEIGHT,
        ),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        HUD_ITEM_BAR_BACKGROUND_TINT,
    );
    if width == 0 {
        return;
    }
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        white_pixel,
        surface_size,
        hud_item_durability_bar_rect(item_rect, width, HUD_ITEM_BAR_FOREGROUND_HEIGHT),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        [bar.color[0], bar.color[1], bar.color[2], 1.0],
    );
}

fn push_hud_item_cooldown<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    item_rect: HudRect,
    progress: Option<f32>,
) {
    let Some(progress) = progress else {
        return;
    };
    let Some(rect) = hud_item_cooldown_rect(item_rect, progress) else {
        return;
    };
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        white_pixel,
        surface_size,
        rect,
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        HUD_ITEM_COOLDOWN_TINT,
    );
}

fn push_hud_item_count_label<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    digit_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &[HudDigitGlyph; 10],
    surface_size: PhysicalSize<u32>,
    item_rect: HudRect,
    label: Option<&HudItemCountLabel>,
) {
    let (Some(digit_atlas), Some(label)) = (digit_atlas, label) else {
        return;
    };
    let Some(text_width) = hud_digit_text_width(&label.text, glyphs) else {
        return;
    };

    for shadow_offset in [1.0, 0.0] {
        let tint = if shadow_offset > 0.0 {
            HUD_TEXT_SHADOW_TINT
        } else {
            HUD_TINT_WHITE
        };
        let mut pen_x = 0;
        for digit in label.text.bytes() {
            let glyph = glyphs[(digit - b'0') as usize];
            let rect =
                hud_item_count_digit_hud_rect(item_rect, text_width, pen_x, shadow_offset, glyph);
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                digit_atlas,
                surface_size,
                rect,
                glyph.uv,
                tint,
            );
            pen_x += glyph.advance;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_inventory_fill_layers<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
    stage: HudInventoryFillStage,
) {
    for layer in screen
        .fill_layers
        .iter()
        .filter(|layer| layer.stage == stage)
    {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            inventory_background_hud_rect(
                surface_size,
                screen.width,
                screen.height,
                layer.x,
                layer.y,
                layer.width,
                layer.height,
            ),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            layer.tint,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_inventory_text_labels<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    for label in &screen.text_labels {
        if let Some(background) = label.background {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
                surface_size,
                inventory_background_hud_rect(
                    surface_size,
                    screen.width,
                    screen.height,
                    background.x,
                    background.y,
                    background.width,
                    background.height,
                ),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                background.tint,
            );
        }
        let origin = hud_inventory_text_label_origin(
            surface_size,
            screen.width,
            screen.height,
            label.x,
            label.y,
        );
        if let Some(input) = label.input {
            push_hud_inventory_text_input_label(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                obfuscated_seed,
                surface_size,
                label,
                input,
                origin,
            );
            continue;
        }
        // Vanilla pass order: the whole line's shadow first, then the main
        // colour (the shadow geometry is the main geometry at +1,+1).
        for (shadow_offset, is_shadow) in label
            .shadow
            .then_some((1.0, true))
            .into_iter()
            .chain(std::iter::once((0.0, false)))
        {
            let geometry = hud_styled_text_pass_geometry(
                &label.runs,
                glyphs,
                obfuscated_pool,
                obfuscated_seed,
                origin,
                shadow_offset,
                is_shadow,
                label.tint,
                Some(label.width),
                1.0,
            );
            push_hud_styled_text_pass(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                surface_size,
                &geometry,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_inventory_text_input_label<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    label: &HudInventoryTextLabel,
    input: HudInventoryTextInputDecoration,
    origin: (f32, f32),
) {
    let layout = hud_inventory_text_input_layout(label, input, glyphs);
    if !layout.displayed_text.is_empty() {
        let runs = [HudStyledTextRun::plain(layout.displayed_text)];
        for (shadow_offset, is_shadow) in label
            .shadow
            .then_some((1.0, true))
            .into_iter()
            .chain(std::iter::once((0.0, false)))
        {
            let geometry = hud_styled_text_pass_geometry(
                &runs,
                glyphs,
                obfuscated_pool,
                obfuscated_seed,
                origin,
                shadow_offset,
                is_shadow,
                label.tint,
                Some(label.width),
                1.0,
            );
            push_hud_styled_text_pass(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                surface_size,
                &geometry,
            );
        }
    }

    if let Some((selection_x, selection_width)) = layout.selection_rect {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            absolute_hud_rect(origin.0 + selection_x, origin.1 - 1.0, selection_width, 11),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            input.selection_tint,
        );
    }

    if input.cursor_visible && layout.cursor_on_screen {
        if layout.insert_cursor {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
                surface_size,
                absolute_hud_rect(origin.0 + layout.cursor_x, origin.1 - 1.0, 1, 11),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                input.cursor_tint,
            );
        } else {
            push_hud_plain_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                obfuscated_seed,
                surface_size,
                "_",
                (origin.0 + layout.cursor_x, origin.1),
                input.cursor_tint,
                1.0,
                label.shadow,
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct HudInventoryTextInputLayout {
    displayed_text: String,
    cursor_x: f32,
    cursor_on_screen: bool,
    insert_cursor: bool,
    selection_rect: Option<(f32, u32)>,
}

fn hud_inventory_text_input_layout(
    label: &HudInventoryTextLabel,
    input: HudInventoryTextInputDecoration,
    glyphs: &HudFontGlyphMap,
) -> HudInventoryTextInputLayout {
    let text_len = label.text.chars().count();
    let cursor = input.cursor.min(text_len);
    let selection = input.selection.min(text_len);
    let display_start = hud_text_input_display_start(
        &label.text,
        input.scroll_to.min(text_len),
        label.width,
        glyphs,
    );
    let display_len = hud_plain_head_char_len_by_width(
        &label.text,
        display_start,
        text_len.saturating_sub(display_start),
        label.width,
        glyphs,
    );
    let display_end = display_start.saturating_add(display_len);
    let displayed_text = hud_slice_by_chars(&label.text, display_start, display_end);
    let cursor_on_screen = cursor >= display_start && cursor <= display_end;
    let cursor_x = if cursor < display_start {
        0.0
    } else if cursor > display_end {
        label.width as f32
    } else {
        let rel_cursor = cursor.saturating_sub(display_start);
        let prefix = hud_prefix_by_chars(&displayed_text, rel_cursor);
        let mut x = hud_plain_text_width(&prefix, glyphs) as f32;
        let at_append_position =
            cursor >= text_len && label.text.encode_utf16().count() < input.max_length;
        if at_append_position {
            x += 1.0;
        }
        x
    };
    let insert_cursor = cursor < text_len || label.text.encode_utf16().count() >= input.max_length;
    let selection_rect = (selection != cursor).then(|| {
        let rel_selection = selection.saturating_sub(display_start).min(display_len);
        let selection_prefix = hud_prefix_by_chars(&displayed_text, rel_selection);
        let selection_x = hud_plain_text_width(&selection_prefix, glyphs) as f32;
        let left = cursor_x.min(selection_x);
        let right = cursor_x.max(selection_x);
        let width = (right - left).ceil().max(1.0) as u32;
        (left, width)
    });
    HudInventoryTextInputLayout {
        displayed_text,
        cursor_x,
        cursor_on_screen,
        insert_cursor,
        selection_rect,
    }
}

fn hud_text_input_display_start(
    text: &str,
    scroll_to: usize,
    width: u32,
    glyphs: &HudFontGlyphMap,
) -> usize {
    let scroll_to = scroll_to.min(text.chars().count());
    if hud_plain_prefix_width_by_chars(text, scroll_to, glyphs) <= width {
        return 0;
    }

    let chars = text.chars().take(scroll_to).collect::<Vec<_>>();
    let mut start = chars.len();
    let mut used_width = 0u32;
    while start > 0 {
        let advance = hud_font_glyph(chars[start - 1], glyphs).styled_advance(Default::default());
        if used_width.saturating_add(advance) > width {
            break;
        }
        used_width = used_width.saturating_add(advance);
        start -= 1;
    }
    start
}

fn hud_plain_head_char_len_by_width(
    text: &str,
    start: usize,
    max_chars: usize,
    width: u32,
    glyphs: &HudFontGlyphMap,
) -> usize {
    let mut used_width = 0u32;
    let mut len = 0usize;
    for ch in text.chars().skip(start).take(max_chars) {
        let advance = hud_font_glyph(ch, glyphs).styled_advance(Default::default());
        if used_width.saturating_add(advance) > width {
            break;
        }
        used_width = used_width.saturating_add(advance);
        len += 1;
    }
    len
}

fn hud_plain_prefix_width_by_chars(text: &str, char_count: usize, glyphs: &HudFontGlyphMap) -> u32 {
    text.chars()
        .take(char_count)
        .map(|ch| hud_font_glyph(ch, glyphs).styled_advance(Default::default()))
        .sum()
}

fn hud_slice_by_chars(text: &str, start: usize, end: usize) -> String {
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn push_hud_sign_editor_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    post_gui_item_commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudSignEditorScreen,
    sign_pip_target_ready: bool,
    hanging_sign_backgrounds: &'a [Option<HudSpriteGpu>; 12],
) {
    match screen.kind {
        HudSignEditorKind::Standing { .. }
            if screen.sign_preview.is_some() && sign_pip_target_ready =>
        {
            let start = vertices.len() as u32;
            vertices.extend_from_slice(&hud_quad_vertices(
                surface_size,
                hud_sign_editor_standing_preview_rect(surface_size),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                HUD_TINT_WHITE,
            ));
            commands.push(HudDrawCommand::EntityPreviewBlit {
                target_index: 0,
                start,
                end: vertices.len() as u32,
            });
        }
        HudSignEditorKind::Hanging { wood } => {
            if let Some(background) = hanging_sign_backgrounds[sign_model_wood_index(wood)].as_ref()
            {
                push_hud_draw(
                    vertices,
                    commands,
                    background,
                    surface_size,
                    hud_sign_editor_hanging_background_rect(surface_size),
                );
            }
        }
        _ => {}
    }

    let Some(font_atlas) = font_atlas else {
        return;
    };
    push_hud_sign_editor_title(
        vertices,
        post_gui_item_commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        surface_size,
        &screen.title,
    );
    push_hud_sign_editor_lines(
        vertices,
        post_gui_item_commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        surface_size,
        screen,
    );
}

fn hud_sign_editor_standing_preview_rect(surface_size: PhysicalSize<u32>) -> HudRect {
    let center_x = surface_size.width.max(1) as f32 * 0.5;
    absolute_hud_rect(center_x - 48.0, 66.0, 96, 102)
}

fn hud_sign_editor_hanging_background_rect(surface_size: PhysicalSize<u32>) -> HudRect {
    let center_x = surface_size.width.max(1) as f32 * 0.5;
    absolute_hud_rect(center_x - 36.0, 30.5, 72, 72)
}

#[allow(clippy::too_many_arguments)]
fn push_hud_sign_editor_title<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    title: &str,
) {
    let width = hud_plain_text_width(title, glyphs);
    let origin = (
        surface_size.width.max(1) as f32 * 0.5 - width as f32 * 0.5,
        40.0,
    );
    push_hud_plain_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        obfuscated_seed,
        surface_size,
        title,
        origin,
        HUD_TINT_WHITE,
        1.0,
        true,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_hud_sign_editor_lines<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudSignEditorScreen,
) {
    let (offset_y, scale, line_height) = match screen.kind {
        HudSignEditorKind::Standing { .. } => (90.0, 0.976_562_8, 10.0),
        HudSignEditorKind::Hanging { .. } => (125.0, 1.0, 9.0),
    };
    let center_x = surface_size.width.max(1) as f32 * 0.5;
    let sign_midpoint = 4.0 * line_height * 0.5;

    for (line_index, line) in screen.lines.iter().enumerate() {
        let line_width = hud_plain_text_width(line, glyphs);
        if line_width == 0 {
            continue;
        }
        let line_y = line_index as f32 * line_height - sign_midpoint;
        let origin = (
            center_x - line_width as f32 * scale * 0.5,
            offset_y + line_y * scale,
        );
        push_hud_plain_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            obfuscated_seed,
            surface_size,
            line,
            origin,
            screen.text_tint,
            scale,
            false,
        );
    }

    let active_line = &screen.lines[screen.line];
    let line_width = hud_plain_text_width(active_line, glyphs);
    let selection_start = screen.cursor.min(screen.selection);
    let selection_end = screen.cursor.max(screen.selection);
    let cursor_prefix = hud_prefix_by_chars(active_line, screen.cursor);
    let cursor_position = hud_plain_text_width(&cursor_prefix, glyphs) as f32;
    let cursor_x = cursor_position - line_width as f32 * 0.5;
    let cursor_y = screen.line as f32 * line_height - sign_midpoint;

    if selection_start != selection_end {
        let start_prefix = hud_prefix_by_chars(active_line, selection_start);
        let end_prefix = hud_prefix_by_chars(active_line, selection_end);
        let start_position = hud_plain_text_width(&start_prefix, glyphs) as f32;
        let end_position = hud_plain_text_width(&end_prefix, glyphs) as f32;
        let x = center_x + (start_position - line_width as f32 * 0.5) * scale;
        let y = offset_y + cursor_y * scale;
        let width = ((end_position - start_position) * scale).ceil().max(1.0) as u32;
        let height = (line_height * scale).ceil().max(1.0) as u32;
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            absolute_hud_rect(x, y, width, height),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            [0.0, 0.0, 1.0, 1.0],
        );
    }

    if !screen.cursor_visible {
        return;
    }
    let cursor_origin = (center_x + cursor_x * scale, offset_y + cursor_y * scale);
    if screen.cursor >= active_line.chars().count() {
        push_hud_plain_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            obfuscated_seed,
            surface_size,
            "_",
            cursor_origin,
            screen.text_tint,
            scale,
            false,
        );
    } else {
        let height = ((line_height + 1.0) * scale).ceil().max(1.0) as u32;
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            absolute_hud_rect(cursor_origin.0, cursor_origin.1 - scale, 1, height),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            screen.text_tint,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_plain_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    text: &str,
    origin: (f32, f32),
    tint: [f32; 4],
    scale: f32,
    shadow: bool,
) {
    if text.is_empty() {
        return;
    }
    let runs = [HudStyledTextRun::plain(text.to_string())];
    for (shadow_offset, is_shadow) in shadow
        .then_some((1.0, true))
        .into_iter()
        .chain(std::iter::once((0.0, false)))
    {
        let geometry = hud_styled_text_pass_geometry(
            &runs,
            glyphs,
            obfuscated_pool,
            obfuscated_seed,
            origin,
            shadow_offset,
            is_shadow,
            tint,
            None,
            scale,
        );
        push_hud_styled_text_pass(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            surface_size,
            &geometry,
        );
    }
}

fn hud_plain_text_width(text: &str, glyphs: &HudFontGlyphMap) -> u32 {
    hud_font_runs_width(&[HudStyledTextRun::plain(text.to_string())], glyphs).unwrap_or_default()
}

fn hud_prefix_by_chars(text: &str, char_count: usize) -> String {
    text.chars().take(char_count).collect()
}

/// One centered HUD overlay text line resolved for this frame: the styled
/// runs, the pen origin in HUD pixels, the vanilla pose scale, and the
/// fade-resolved base tint.
#[derive(Debug, Clone, Copy, PartialEq)]
struct HudScreenTextDraw<'a> {
    runs: &'a [HudStyledTextRun],
    origin: (f32, f32),
    scale: f32,
    tint: [f32; 4],
}

/// Vanilla `Gui.extractOverlayMessage` fade (Gui.java:312-316):
/// `alpha = (int)(t * 255 / 20)` capped at 255; the draw is dropped when the
/// result is not `> 0` (:318). `t` is `overlayMessageTime - partialTick`.
fn hud_overlay_message_alpha(t: f32) -> i32 {
    let alpha = (t * 255.0 / 20.0) as i32;
    alpha.min(255)
}

/// Vanilla jukebox now-playing rainbow (Gui.java:323-324):
/// `Mth.hsvToArgb(t / 50, 0.7, 0.6, alpha)`. The hue derives from the
/// remaining display time, so the colour cycle is deterministic per frame
/// state (no wall clock). `Mth.hsvToArgb` keeps its Java quirk verbatim: `h`
/// is wrapped mod 6 but `f` is taken against the wrapped `h`, and the final
/// channels are clamped to 0..255 (Mth.java:451-497).
fn hud_overlay_message_rainbow_rgb(t: f32) -> [f32; 3] {
    const SATURATION: f32 = 0.7;
    const VALUE: f32 = 0.6;
    let hue = t / 50.0;
    let h = ((hue * 6.0) as i32) % 6;
    let f = hue * 6.0 - h as f32;
    let p = VALUE * (1.0 - SATURATION);
    let q = VALUE * (1.0 - f * SATURATION);
    let s = VALUE * (1.0 - (1.0 - f) * SATURATION);
    // `t > 0` whenever the alpha gate passes, so `h` is in 0..6 here.
    let (red, green, blue) = match h {
        1 => (q, VALUE, p),
        2 => (p, VALUE, s),
        3 => (p, q, VALUE),
        4 => (s, p, VALUE),
        5 => (VALUE, p, q),
        _ => (VALUE, s, p),
    };
    [red, green, blue].map(|channel| ((channel * 255.0) as i32).clamp(0, 255) as f32 / 255.0)
}

/// Vanilla `Gui.extractTitle` fade (Gui.java:342-353): full alpha during the
/// stay window, `(fadeIn+stay+fadeOut - t) * 255 / fadeIn` while fading in,
/// `t * 255 / fadeOut` while fading out, clamped to 0..255. `t` is
/// `titleTime - partialTick`.
fn hud_title_alpha(state: &HudTitleText) -> i32 {
    let t = state.remaining_ticks as f32 - state.partial_tick;
    let mut alpha = 255;
    if state.remaining_ticks > state.fade_out.saturating_add(state.stay) {
        let total = state
            .fade_in
            .saturating_add(state.stay)
            .saturating_add(state.fade_out);
        alpha = ((total as f32 - t) * 255.0 / state.fade_in as f32) as i32;
    }
    if state.remaining_ticks <= state.fade_out {
        alpha = (t * 255.0 / state.fade_out as f32) as i32;
    }
    alpha.clamp(0, 255)
}

/// Resolves the action-bar overlay message into a centered draw, mirroring
/// `Gui.extractOverlayMessage` (Gui.java:308-336): shown only while
/// `overlayMessageTime > 0`, alpha-gated (`alpha > 0`), centered above the
/// hotbar at `(guiWidth/2 - width/2, guiHeight - 68 - 4)`, white (or the
/// jukebox rainbow colour) at the fade alpha.
fn hud_action_bar_text_draw<'a>(
    state: &'a HudActionBarText,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> Option<HudScreenTextDraw<'a>> {
    if state.remaining_ticks <= 0 {
        return None;
    }
    let t = state.remaining_ticks as f32 - state.partial_tick;
    let alpha = hud_overlay_message_alpha(t);
    if alpha <= 0 {
        return None;
    }
    let [red, green, blue] = if state.animate_color {
        hud_overlay_message_rainbow_rgb(t)
    } else {
        [1.0, 1.0, 1.0]
    };
    let width = hud_font_runs_width(&state.runs, glyphs).unwrap_or(0);
    Some(HudScreenTextDraw {
        runs: &state.runs,
        origin: hud_overlay_message_text_origin(surface_size, width),
        scale: 1.0,
        tint: [red, green, blue, alpha as f32 / 255.0],
    })
}

/// Resolves the title (4x) and subtitle (2x) overlay lines, mirroring
/// `Gui.extractTitle` (Gui.java:338-377): shown only while `titleTime > 0`
/// and the fade alpha is `> 0`; both lines share the screen-center pose and
/// the same `ARGB.white(alpha)` tint, and the subtitle draws only while a
/// title is active (an empty title line still keeps the subtitle visible,
/// matching vanilla's non-null check).
fn hud_title_text_draws<'a>(
    state: &'a HudTitleText,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> Vec<HudScreenTextDraw<'a>> {
    let mut draws = Vec::new();
    if state.remaining_ticks <= 0 {
        return draws;
    }
    let alpha = hud_title_alpha(state);
    if alpha <= 0 {
        return draws;
    }
    let tint = [1.0, 1.0, 1.0, alpha as f32 / 255.0];
    let title_width = hud_font_runs_width(&state.title_runs, glyphs).unwrap_or(0);
    draws.push(HudScreenTextDraw {
        runs: &state.title_runs,
        origin: hud_title_text_origin(surface_size, title_width),
        scale: HUD_TITLE_TEXT_SCALE,
        tint,
    });
    if !state.subtitle_runs.is_empty() {
        let subtitle_width = hud_font_runs_width(&state.subtitle_runs, glyphs).unwrap_or(0);
        draws.push(HudScreenTextDraw {
            runs: &state.subtitle_runs,
            origin: hud_subtitle_text_origin(surface_size, subtitle_width),
            scale: HUD_SUBTITLE_TEXT_SCALE,
            tint,
        });
    }
    draws
}

/// Clamps a projected bar's progress into `0.0..=1.0` (non-finite fills
/// nothing): vanilla trusts the `ClientboundBossEventPacket` float verbatim,
/// but an out-of-range fill would sample past the 182px sheet.
fn sanitize_hud_boss_bar(bar: HudBossBar) -> HudBossBar {
    HudBossBar {
        progress: if bar.progress.is_finite() {
            bar.progress.clamp(0.0, 1.0)
        } else {
            0.0
        },
        ..bar
    }
}

/// Which 182x5 sheet one bar layer samples (vanilla
/// `BAR_{BACKGROUND,PROGRESS}_SPRITES` / `OVERLAY_*_SPRITES`,
/// BossHealthOverlay.java:20-49).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HudBossBarSheet {
    ColorBackground(HudBossBarColor),
    NotchedBackground(HudBossBarOverlay),
    ColorProgress(HudBossBarColor),
    NotchedProgress(HudBossBarOverlay),
}

/// One sprite layer of a bar: the sheet and the drawn width — 182 for
/// backgrounds, the discrete fill width for progress layers; the draw crops
/// the rect and UV to the left `width / 182` band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudBossBarLayer {
    sheet: HudBossBarSheet,
    width: u32,
}

/// One bar's resolved draw plan for this frame: the surviving stack row, the
/// sprite layers in vanilla submission order, and the centered name line.
#[derive(Debug, Clone, PartialEq)]
struct HudBossBarDraw<'a> {
    y: i32,
    layers: Vec<HudBossBarLayer>,
    name: HudScreenTextDraw<'a>,
}

/// Resolves the stacked boss bars, mirroring
/// `BossHealthOverlay.extractRenderState` (BossHealthOverlay.java:57-82):
/// bars walk top-down from y=12 stepping 10+9, dropping the remainder once
/// the accumulated offset reaches `guiHeight / 3`; each bar submits its
/// sprite layers and then its name, centered at `(guiWidth/2 - width/2,
/// y - 9)` in opaque white at scale 1.
fn hud_boss_bar_draws<'a>(
    bars: &'a [HudBossBar],
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> Vec<HudBossBarDraw<'a>> {
    hud_boss_bar_rows(surface_size, bars.len())
        .into_iter()
        .zip(bars)
        .map(|(y, bar)| {
            let name_width = hud_font_runs_width(&bar.name_runs, glyphs).unwrap_or(0);
            HudBossBarDraw {
                y,
                layers: hud_boss_bar_layers(bar),
                name: HudScreenTextDraw {
                    runs: &bar.name_runs,
                    origin: hud_boss_bar_name_origin(surface_size, y, name_width),
                    scale: 1.0,
                    tint: HUD_TINT_WHITE,
                },
            }
        })
        .collect()
}

/// Sprite layers in vanilla submission order (`BossHealthOverlay.extractBar`,
/// BossHealthOverlay.java:84-106): the full-width colored background, the
/// notched background on top, then — only when `Mth.lerpDiscrete` yields a
/// positive width — the colored and notched progress layers cropped to that
/// width.
fn hud_boss_bar_layers(bar: &HudBossBar) -> Vec<HudBossBarLayer> {
    let mut layers = vec![HudBossBarLayer {
        sheet: HudBossBarSheet::ColorBackground(bar.color),
        width: HUD_BOSS_BAR_WIDTH,
    }];
    if bar.overlay != HudBossBarOverlay::Progress {
        layers.push(HudBossBarLayer {
            sheet: HudBossBarSheet::NotchedBackground(bar.overlay),
            width: HUD_BOSS_BAR_WIDTH,
        });
    }
    let progress_width = hud_boss_bar_progress_width(bar.progress);
    if progress_width > 0 {
        layers.push(HudBossBarLayer {
            sheet: HudBossBarSheet::ColorProgress(bar.color),
            width: progress_width,
        });
        if bar.overlay != HudBossBarOverlay::Progress {
            layers.push(HudBossBarLayer {
                sheet: HudBossBarSheet::NotchedProgress(bar.overlay),
                width: progress_width,
            });
        }
    }
    layers
}

/// Vanilla `Gui.java:533` gate: the experience level number renders only when
/// `experienceLevel > 0`, so a zero (or negative) projection clears the text.
fn hud_experience_level_projection(level: Option<i32>) -> Option<i32> {
    level.filter(|&level| level > 0)
}

/// Picks a food icon's sprite honoring the Hunger potion swap (vanilla
/// `Gui.extractFood` sprite-id selection, Gui.java:945-956): under the effect
/// prefer the hunger variant, falling back to the base sprite when the variant
/// is not loaded; without the effect always use the base sprite. Generic over
/// the sprite handle so the decision is unit-testable without GPU resources.
fn hud_food_sprite_variant<T>(
    hunger_effect: bool,
    hunger: Option<T>,
    base: Option<T>,
) -> Option<T> {
    if hunger_effect {
        hunger.or(base)
    } else {
        base
    }
}

/// Draws the centered experience-level number with vanilla's 1px black outline
/// (`ContextualBarRenderer.extractExperienceLevel`,
/// ContextualBarRenderer.java:35-44): the level string `Component.translatable
/// ("gui.experience.level", level)` (the `"%s"` lang value, so just the number)
/// centered at `x = (guiWidth - width)/2`, `y = guiHeight - 24 - 9 - 2`, drawn as
/// four black axis-offset copies then the `0x80FF20` green center, all
/// `dropShadow = false`. Reuses the shared styled-text pass (`shadow_offset = 0`,
/// `is_shadow = false`) with the outline offset baked into the pen origin, so no
/// bespoke glyph loop is introduced.
#[allow(clippy::too_many_arguments)]
fn push_hud_experience_level_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    level: i32,
) {
    let runs = [HudStyledTextRun::plain(level.to_string())];
    let width = hud_font_runs_width(&runs, glyphs).unwrap_or(0);
    let (base_x, base_y) = hud_experience_level_text_origin(surface_size, width);
    for (dx, dy, tint) in HUD_EXPERIENCE_LEVEL_PASSES {
        let geometry = hud_styled_text_pass_geometry(
            &runs,
            glyphs,
            obfuscated_pool,
            frame_index,
            (base_x + dx, base_y + dy),
            0.0,
            false,
            tint,
            None,
            1.0,
        );
        push_hud_styled_text_pass(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            surface_size,
            &geometry,
        );
    }
}

/// Draws one resolved screen text line through the styled pipeline with the
/// vanilla `textWithBackdrop` pass order (GuiGraphicsExtractor.java:293-301):
/// the accessibility backdrop only draws when the text-background opacity
/// option is non-zero (default 0 — skipped here), then the whole-line shadow
/// pass, then the main colour.
#[allow(clippy::too_many_arguments)]
fn push_hud_screen_text_draw<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    draw: &HudScreenTextDraw<'_>,
) {
    for (shadow_offset, is_shadow) in [(1.0, true), (0.0, false)] {
        let geometry = hud_styled_text_pass_geometry(
            draw.runs,
            glyphs,
            obfuscated_pool,
            frame_index,
            draw.origin,
            shadow_offset,
            is_shadow,
            draw.tint,
            None,
            draw.scale,
        );
        push_hud_styled_text_pass(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            surface_size,
            &geometry,
        );
    }
}

/// Resolved main-pass colour of a styled run: the run's `Style` colour over
/// the line's base tint (vanilla `StringRenderOutput.getTextColor`), keeping
/// the base alpha.
fn hud_run_color_tint(run: &HudStyledTextRun, base_tint: [f32; 4]) -> [f32; 4] {
    match run.color {
        Some(rgb) => [
            ((rgb >> 16) & 0xFF) as f32 / 255.0,
            ((rgb >> 8) & 0xFF) as f32 / 255.0,
            (rgb & 0xFF) as f32 / 255.0,
            base_tint[3],
        ],
        None => base_tint,
    }
}

/// Vanilla `StringRenderOutput.getShadowColor` default branch:
/// `ARGB.scaleRGB(textColor, 0.25)` — the drop shadow is the text colour at a
/// quarter brightness (alpha kept).
fn hud_text_shadow_tint(tint: [f32; 4]) -> [f32; 4] {
    [tint[0] * 0.25, tint[1] * 0.25, tint[2] * 0.25, tint[3]]
}

/// One draw pass (shadow or main) of a styled text line: glyph quads in draw
/// order followed by underline/strikethrough bars, mirroring vanilla
/// `Font.StringRenderOutput.visit` (glyphs first, effects after). All
/// geometry is produced by the locked `styled_quads` / `styled_effect_rects`
/// mechanisms; tints are resolved per run (shadow passes scale the run
/// colour, vanilla `getShadowColor`).
#[derive(Debug, Clone, PartialEq)]
struct HudStyledTextPassGeometry {
    glyph_quads: Vec<(HudGlyphQuad, [f32; 4])>,
    effect_rects: Vec<(HudEffectRect, [f32; 4])>,
}

/// Computes one pass of a styled line at `origin` (line top-left in HUD
/// pixels). `width_limit` reproduces the label budget semantics: the walk
/// stops once the pen reaches the limit and a glyph/effect is only emitted
/// when its cell/advance still fits (the limit is in font pixels, pre-scale).
///
/// `scale` mirrors a vanilla pose scale around `origin`
/// (`PoseStack.scale` before the draw, as `Gui.extractTitle` does for the
/// 4x title / 2x subtitle): every font-pixel offset — pen, glyph cells,
/// shadow offset, bold offset and effect bars — is multiplied uniformly.
/// `1.0` reproduces the unscaled label path.
///
/// Italic runs are sheared through the locked `styled_quads` primitive (top
/// edge `1-0.25*up`, bottom `1-0.25*down`). Obfuscated (`§k`) non-space glyphs
/// draw a random equal-advance substitute (`FontSet.getRandomGlyph`) picked
/// from `obfuscated_pool`; `obfuscated_seed` (the render frame counter) seeds a
/// deterministic per-pass LCG so the jitter is reproducible and the shadow pass
/// picks the same substitutes as the main pass. The pen advance always follows
/// the original glyph, so obfuscation and italic never shift layout.
#[allow(clippy::too_many_arguments)]
fn hud_styled_text_pass_geometry(
    runs: &[HudStyledTextRun],
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    origin: (f32, f32),
    shadow_offset: f32,
    is_shadow: bool,
    base_tint: [f32; 4],
    width_limit: Option<u32>,
    scale: f32,
) -> HudStyledTextPassGeometry {
    let mut geometry = HudStyledTextPassGeometry {
        glyph_quads: Vec::new(),
        effect_rects: Vec::new(),
    };
    let mut pen_x = 0u32;
    let mut first_in_line = true;
    // One LCG per pass, reset from the frame seed so the shadow and main passes
    // choose identical substitutes (the shadow is the main glyph at +1,+1); it
    // advances only when an obfuscated glyph is actually substituted, matching
    // vanilla `Font.random` (touched once per `getRandomGlyph`).
    let mut obfuscated_random = HudObfuscatedRandom::with_seed(obfuscated_seed);
    'runs: for run in runs {
        let run_tint = hud_run_color_tint(run, base_tint);
        let tint = if is_shadow {
            hud_text_shadow_tint(run_tint)
        } else {
            run_tint
        };
        for ch in run.text.chars() {
            if width_limit.is_some_and(|limit| pen_x >= limit) {
                break 'runs;
            }
            let base_glyph = hud_font_glyph(ch, glyphs);
            // Layout stays on the original glyph's advance (equal for the
            // substitute, but this keeps the pen frame-stable regardless).
            let advance = base_glyph.styled_advance(run.style);
            // Vanilla `Font.getGlyph`: obfuscated non-space codepoints draw a
            // random equal-advance glyph; everything else draws its own glyph.
            let glyph = if run.style.obfuscated && ch != ' ' {
                obfuscated_pool
                    .random_glyph(base_glyph.advance, &mut obfuscated_random)
                    .unwrap_or(base_glyph)
            } else {
                base_glyph
            };
            // Glyph geometry is produced in font pixels relative to `origin`
            // and mapped through `origin + scale * offset`, exactly a vanilla
            // pose scale (the shadow offset scales with the pose too).
            let x = pen_x as f32 + shadow_offset;
            let y = shadow_offset;
            if glyph.width > 0
                && glyph.height > 0
                && width_limit.is_none_or(|limit| pen_x.saturating_add(glyph.width) <= limit)
            {
                for quad in glyph.styled_quads(x, y, run.style, false) {
                    let quad = HudGlyphQuad {
                        corners: quad
                            .corners
                            .map(|[cx, cy]| [origin.0 + cx * scale, origin.1 + cy * scale]),
                        ..quad
                    };
                    geometry.glyph_quads.push((quad, tint));
                }
            }
            // Underline/strikethrough bars follow the advance, unaffected by the
            // obfuscated bitmap swap, so they read the original glyph.
            if width_limit.is_none_or(|limit| pen_x.saturating_add(advance) <= limit) {
                for rect in base_glyph.styled_effect_rects(x, y, run.style, first_in_line) {
                    let rect = HudEffectRect {
                        x0: origin.0 + rect.x0 * scale,
                        y0: origin.1 + rect.y0 * scale,
                        x1: origin.0 + rect.x1 * scale,
                        y1: origin.1 + rect.y1 * scale,
                        ..rect
                    };
                    geometry.effect_rects.push((rect, tint));
                }
            }
            pen_x = pen_x.saturating_add(advance);
            first_in_line = false;
        }
    }
    geometry
}

fn push_hud_styled_text_pass<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    geometry: &HudStyledTextPassGeometry,
) {
    for (quad, tint) in &geometry.glyph_quads {
        let start = vertices.len() as u32;
        vertices.extend_from_slice(&hud_styled_quad_vertices(
            surface_size,
            quad.corners,
            quad.uv,
            *tint,
        ));
        let end = vertices.len() as u32;
        commands.push(HudDrawCommand::Sprite {
            sprite: font_atlas,
            start,
            end,
        });
    }
    for (rect, tint) in &geometry.effect_rects {
        let start = vertices.len() as u32;
        vertices.extend_from_slice(&hud_styled_quad_vertices(
            surface_size,
            [
                [rect.x0, rect.y0],
                [rect.x0, rect.y1],
                [rect.x1, rect.y1],
                [rect.x1, rect.y0],
            ],
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            *tint,
        ));
        let end = vertices.len() as u32;
        commands.push(HudDrawCommand::Sprite {
            sprite: white_pixel,
            start,
            end,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn push_hud_inventory_tooltip<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    tooltip_background: Option<&'a HudNineSliceSprite>,
    tooltip_frame: Option<&'a HudNineSliceSprite>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    obfuscated_seed: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
) {
    let (Some(font_atlas), Some(tooltip)) = (font_atlas, screen.tooltip.as_ref()) else {
        return;
    };
    let Some(text_height) = hud_inventory_tooltip_text_height(tooltip.lines.len()) else {
        return;
    };
    let Some(text_width) = tooltip
        .lines
        .iter()
        .filter_map(|line| hud_font_runs_width(&line.runs, glyphs))
        .max()
    else {
        return;
    };

    let background_rect = hud_inventory_tooltip_background_hud_rect(
        surface_size,
        screen.width,
        screen.height,
        tooltip.x,
        tooltip.y,
        text_width,
        text_height,
    );
    match (tooltip_background, tooltip_frame) {
        // Vanilla `TooltipRenderUtil.extractTooltipBackground`: nine-slice `tooltip/background`
        // then `tooltip/frame` over the same padded rect.
        (Some(background), Some(frame)) => {
            for segment in hud_inventory_tooltip_sprite_segments(
                background_rect,
                background.scaling,
                frame.scaling,
            ) {
                let sprite = match segment.layer {
                    HudTooltipSpriteLayer::Background => &background.gpu,
                    HudTooltipSpriteLayer::Frame => &frame.gpu,
                };
                push_hud_draw_with_uv_and_tint(
                    vertices,
                    commands,
                    sprite,
                    surface_size,
                    segment.rect,
                    segment.uv,
                    HUD_TINT_WHITE,
                );
            }
        }
        // Missing-asset fallback: the legacy flat translucent background quad.
        _ => {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
                surface_size,
                background_rect,
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                HUD_TOOLTIP_BACKGROUND_TINT,
            );
        }
    }

    // Vanilla pass order kept from the plain path: every line's shadow first,
    // then every line's main colour; per-run tints resolve inside each pass.
    for (shadow_offset, is_shadow) in [(1.0, true), (0.0, false)] {
        for (line_index, line) in tooltip.lines.iter().enumerate() {
            let origin = hud_inventory_tooltip_line_origin(
                surface_size,
                screen.width,
                screen.height,
                tooltip.x,
                tooltip.y,
                text_width,
                text_height,
                line_index,
            );
            let geometry = hud_styled_text_pass_geometry(
                &line.runs,
                glyphs,
                obfuscated_pool,
                obfuscated_seed,
                origin,
                shadow_offset,
                is_shadow,
                line.tint,
                None,
                1.0,
            );
            push_hud_styled_text_pass(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                surface_size,
                &geometry,
            );
        }
    }
}

fn hud_digit_text_width(text: &str, glyphs: &[HudDigitGlyph; 10]) -> Option<u32> {
    let mut width = 0u32;
    for digit in text.bytes() {
        if !digit.is_ascii_digit() {
            return None;
        }
        width = width.checked_add(glyphs[(digit - b'0') as usize].advance)?;
    }
    (width > 0).then_some(width)
}

/// Test-only single-style wrappers over [`hud_font_runs_width`], the one
/// width implementation.
#[cfg(test)]
fn hud_font_text_width(text: &str, glyphs: &HudFontGlyphMap) -> Option<u32> {
    hud_font_text_width_styled(text, glyphs, HudTextStyle::default())
}

#[cfg(test)]
fn hud_font_text_width_styled(
    text: &str,
    glyphs: &HudFontGlyphMap,
    style: HudTextStyle,
) -> Option<u32> {
    hud_font_runs_width(
        &[HudStyledTextRun {
            text: text.to_string(),
            style,
            color: None,
        }],
        glyphs,
    )
}

/// Vanilla `Font.width` across a line's styled runs: sum of per-glyph
/// `GlyphInfo.getAdvance(style.isBold())` — bold widens each glyph by one
/// font pixel; the unstyled default reproduces the prior plain-advance width
/// exactly.
fn hud_font_runs_width(runs: &[HudStyledTextRun], glyphs: &HudFontGlyphMap) -> Option<u32> {
    let mut width = 0u32;
    for run in runs {
        for ch in run.text.chars() {
            width = width.checked_add(hud_font_glyph(ch, glyphs).styled_advance(run.style))?;
        }
    }
    (width > 0).then_some(width)
}

/// Vanilla `FontSet.computeGlyphInfo` walks the flattened provider chain; the
/// baked `HudFontGlyphMap` already encodes that first-provider-wins order, so
/// lookup is direct, with `?` standing in for codepoints no bitmap page
/// covers (unihex/unifont deferred).
fn hud_font_glyph(ch: char, glyphs: &HudFontGlyphMap) -> HudAsciiGlyph {
    glyphs
        .get(ch)
        .or_else(|| glyphs.get(HUD_FONT_REPLACEMENT_GLYPH))
        .unwrap_or_default()
}

fn sanitize_hud_uv_rect(rect: HudUvRect) -> Option<HudUvRect> {
    let components = [rect.min[0], rect.min[1], rect.max[0], rect.max[1]];
    if !components.iter().all(|component| component.is_finite()) {
        return None;
    }

    let min_x = rect.min[0].clamp(0.0, 1.0);
    let min_y = rect.min[1].clamp(0.0, 1.0);
    let max_x = rect.max[0].clamp(0.0, 1.0);
    let max_y = rect.max[1].clamp(0.0, 1.0);
    Some(HudUvRect {
        min: [min_x.min(max_x), min_y.min(max_y)],
        max: [min_x.max(max_x), min_y.max(max_y)],
    })
}

fn sanitize_hud_hotbar_item_uv(uv: HudUvRect) -> Option<HudItemIcon> {
    sanitize_hud_item_icon(HudItemIcon::single(uv))
}

fn sanitize_hud_inventory_screen(screen: HudInventoryScreen) -> HudInventoryScreen {
    HudInventoryScreen {
        width: screen.width.clamp(1, 512),
        height: screen.height.clamp(1, 512),
        background_layers: screen
            .background_layers
            .into_iter()
            .filter_map(sanitize_hud_inventory_background_layer)
            .collect(),
        fill_layers: screen
            .fill_layers
            .into_iter()
            .filter_map(sanitize_hud_inventory_fill_layer)
            .collect(),
        slots: screen
            .slots
            .into_iter()
            .map(sanitize_hud_inventory_slot)
            .collect(),
        floating_items: screen
            .floating_items
            .into_iter()
            .filter_map(sanitize_hud_inventory_item)
            .collect(),
        ghost_items: screen
            .ghost_items
            .into_iter()
            .filter_map(sanitize_hud_inventory_ghost_item)
            .collect(),
        entity_previews: screen
            .entity_previews
            .into_iter()
            .filter_map(sanitize_hud_entity_preview)
            .collect(),
        text_labels: screen
            .text_labels
            .into_iter()
            .filter_map(sanitize_hud_inventory_text_label)
            .collect(),
        hovered_slot_id: screen.hovered_slot_id,
        tooltip: screen.tooltip.and_then(sanitize_hud_inventory_tooltip),
    }
}

fn sanitize_hud_sign_editor_screen(mut screen: HudSignEditorScreen) -> Option<HudSignEditorScreen> {
    if !screen
        .text_tint
        .iter()
        .all(|component| component.is_finite())
    {
        return None;
    }
    screen.title = sanitize_hud_text_preserving_empty(screen.title, 256);
    if screen.title.is_empty() {
        return None;
    }
    screen.lines = screen
        .lines
        .map(|line| sanitize_hud_text_preserving_empty(line, 384));
    screen.line = screen.line.min(screen.lines.len().saturating_sub(1));
    let line_len = screen.lines[screen.line].chars().count();
    screen.cursor = screen.cursor.min(line_len);
    screen.selection = screen.selection.min(line_len);
    screen.text_tint = screen.text_tint.map(|component| component.clamp(0.0, 1.0));
    screen.sign_preview = match screen.kind {
        HudSignEditorKind::Standing { .. } => {
            screen.sign_preview.and_then(sanitize_hud_entity_preview)
        }
        HudSignEditorKind::Hanging { .. } => None,
    };
    Some(screen)
}

fn sanitize_hud_inventory_background_layer(
    layer: HudInventoryBackgroundLayer,
) -> Option<HudInventoryBackgroundLayer> {
    let uv = sanitize_hud_uv_rect(layer.uv)?;
    (layer.width > 0 && layer.height > 0).then_some(HudInventoryBackgroundLayer { uv, ..layer })
}

fn sanitize_hud_inventory_fill_layer(
    layer: HudInventoryFillLayer,
) -> Option<HudInventoryFillLayer> {
    if layer.width == 0
        || layer.height == 0
        || !layer.tint.iter().all(|component| component.is_finite())
    {
        return None;
    }
    Some(HudInventoryFillLayer {
        tint: layer.tint.map(|component| component.clamp(0.0, 1.0)),
        ..layer
    })
}

fn sanitize_hud_inventory_slot(slot: HudInventorySlot) -> HudInventorySlot {
    HudInventorySlot {
        slot_id: slot.slot_id,
        x: slot.x,
        y: slot.y,
        icon: slot.icon.and_then(sanitize_hud_item_icon),
        block_model: slot.block_model.filter(hud_block_item_model_is_renderable),
    }
}

fn sanitize_hud_inventory_item(item: HudInventoryItem) -> Option<HudInventoryItem> {
    if !item.scale.is_finite()
        || item.scale <= 0.0
        || !item.scale_y.is_finite()
        || item.scale_y <= 0.0
    {
        return None;
    }
    Some(HudInventoryItem {
        x: item.x,
        y: item.y,
        scale: item.scale.clamp(0.0625, 16.0),
        scale_y: item.scale_y.clamp(0.0625, 16.0),
        icon: sanitize_hud_item_icon(item.icon)?,
        draw_decorations: item.draw_decorations,
        block_model: item.block_model.filter(hud_block_item_model_is_renderable),
    })
}

fn sanitize_hud_inventory_ghost_item(item: HudInventoryGhostItem) -> Option<HudInventoryGhostItem> {
    Some(HudInventoryGhostItem {
        x: item.x,
        y: item.y,
        icon: sanitize_hud_item_icon(item.icon)?,
        draw_decorations: item.draw_decorations,
    })
}

/// A 3D block-item icon is only worth carrying when it has geometry to draw.
fn hud_block_item_model_is_renderable(model: &HudBlockItemModel) -> bool {
    model.lighting == GuiItemLightingEntry::Items3d && !model.quads.is_empty()
}

fn sign_model_wood_index(wood: SignModelWood) -> usize {
    match wood {
        SignModelWood::Oak => 0,
        SignModelWood::Spruce => 1,
        SignModelWood::Birch => 2,
        SignModelWood::Acacia => 3,
        SignModelWood::Cherry => 4,
        SignModelWood::Jungle => 5,
        SignModelWood::DarkOak => 6,
        SignModelWood::PaleOak => 7,
        SignModelWood::Crimson => 8,
        SignModelWood::Warped => 9,
        SignModelWood::Mangrove => 10,
        SignModelWood::Bamboo => 11,
    }
}

/// The PIP-texture sub-rect (as `0..1` UV fractions of the preview rect) a scissored blit
/// samples: `visible == rect ∩ scissor` maps to the matching texture region, identity UVs when
/// no scissor applies. Vanilla scissors the full-rect blit instead; for an axis-aligned scissor
/// the two are pixel-equivalent. wgpu's row-0-top texture origin means the PIP render is already
/// GUI-oriented — vanilla's `v0=1, v1=0` vertical flip is a GL framebuffer-origin artifact and
/// has no bbb counterpart.
fn hud_entity_preview_blit_uv(
    rect: HudEntityPreviewRect,
    visible: HudEntityPreviewRect,
) -> HudUvRect {
    let width = rect.width.max(1) as f32;
    let height = rect.height.max(1) as f32;
    HudUvRect {
        min: [
            (visible.x - rect.x) as f32 / width,
            (visible.y - rect.y) as f32 / height,
        ],
        max: [
            (visible.right() - i64::from(rect.x)) as f32 / width,
            (visible.bottom() - i64::from(rect.y)) as f32 / height,
        ],
    }
}

fn sanitize_hud_entity_preview(mut preview: HudEntityPreview) -> Option<HudEntityPreview> {
    let is_gui_sign_preview = is_gui_sign_preview(&preview);
    if (!is_gui_sign_preview && preview.lighting != GuiItemLightingEntry::EntityInUi)
        || !preview.depth_isolated
    {
        return None;
    }
    preview.rect = sanitize_hud_entity_preview_rect(preview.rect)?;
    preview.scissor = match preview.scissor {
        Some(scissor) => Some(sanitize_hud_entity_preview_rect(scissor)?),
        None => None,
    };
    preview.visible_bounds()?;
    if !preview.scale.is_finite() || preview.scale <= 0.0 {
        return None;
    }
    if !preview
        .translation
        .iter()
        .all(|component| component.is_finite())
        || !preview
            .rotation
            .iter()
            .all(|component| component.is_finite())
        || !preview
            .override_camera_rotation
            .map(|rotation| rotation.iter().all(|component| component.is_finite()))
            .unwrap_or(true)
    {
        return None;
    }

    preview.entity.render_state.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
    preview.entity.render_state.outline_color = 0;
    preview.entity.render_state.appears_glowing = false;
    for layer in &mut preview.item_layers {
        if layer.item_id < 0 || layer.count <= 0 {
            return None;
        }
        layer.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
        layer.overlay = ITEM_MODEL_NO_OVERLAY;
    }
    Some(preview)
}

fn is_gui_sign_preview(preview: &HudEntityPreview) -> bool {
    preview.lighting == GuiItemLightingEntry::ItemsFlat
        && matches!(
            preview.entity.kind,
            crate::entity_models::EntityModelKind::Sign { .. }
        )
}

fn sanitize_hud_entity_preview_rect(rect: HudEntityPreviewRect) -> Option<HudEntityPreviewRect> {
    // Preview bounds live inside the (512-clamped) inventory screen; the clamp also bounds the
    // per-preview PIP color/depth texture allocation.
    (rect.width > 0 && rect.height > 0).then_some(HudEntityPreviewRect {
        width: rect.width.min(512),
        height: rect.height.min(512),
        ..rect
    })
}

fn sanitize_hud_inventory_text_label(
    label: HudInventoryTextLabel,
) -> Option<HudInventoryTextLabel> {
    if !label.tint.iter().all(|component| component.is_finite()) {
        return None;
    }
    let x = label.x;
    let y = label.y;
    let width = label.width;
    let tint = label.tint;
    let background = label
        .background
        .and_then(sanitize_hud_inventory_text_background);
    let text = if label.input.is_some() {
        sanitize_hud_text_preserving_empty(label.text, 256)
    } else {
        sanitize_hud_text_line(label.text)?
    };
    let input = label
        .input
        .and_then(|input| sanitize_hud_inventory_text_input(input, &text));
    if text.is_empty() && input.is_none() {
        return None;
    }
    let runs = sanitize_hud_styled_runs(label.runs, &text);
    (width > 0).then_some(HudInventoryTextLabel {
        x,
        y,
        width: width.min(512),
        text,
        tint: tint.map(|component| component.clamp(0.0, 1.0)),
        background,
        input,
        shadow: label.shadow,
        runs,
    })
}

fn sanitize_hud_inventory_text_input(
    input: HudInventoryTextInputDecoration,
    text: &str,
) -> Option<HudInventoryTextInputDecoration> {
    if !input
        .cursor_tint
        .iter()
        .chain(input.selection_tint.iter())
        .all(|component| component.is_finite())
    {
        return None;
    }
    let len = text.chars().count();
    Some(HudInventoryTextInputDecoration {
        cursor: input.cursor.min(len),
        selection: input.selection.min(len),
        scroll_to: input.scroll_to.min(len),
        max_length: input.max_length.min(1024),
        cursor_visible: input.cursor_visible,
        cursor_tint: input.cursor_tint.map(|component| component.clamp(0.0, 1.0)),
        selection_tint: input
            .selection_tint
            .map(|component| component.clamp(0.0, 1.0)),
    })
}

fn sanitize_hud_inventory_text_background(
    background: HudInventoryTextBackground,
) -> Option<HudInventoryTextBackground> {
    if !background
        .tint
        .iter()
        .all(|component| component.is_finite())
    {
        return None;
    }
    (background.width > 0 && background.height > 0).then_some(HudInventoryTextBackground {
        width: background.width.min(512),
        height: background.height.min(512),
        tint: background.tint.map(|component| component.clamp(0.0, 1.0)),
        ..background
    })
}

fn sanitize_hud_inventory_tooltip(tooltip: HudInventoryTooltip) -> Option<HudInventoryTooltip> {
    let lines = tooltip
        .lines
        .into_iter()
        .filter_map(sanitize_hud_inventory_tooltip_line)
        .take(16)
        .collect::<Vec<_>>();
    (!lines.is_empty()).then_some(HudInventoryTooltip { lines, ..tooltip })
}

fn sanitize_hud_inventory_tooltip_line(
    line: HudInventoryTooltipLine,
) -> Option<HudInventoryTooltipLine> {
    if !line.tint.iter().all(|component| component.is_finite()) {
        return None;
    }
    let text = sanitize_hud_text_line(line.text)?;
    let runs = sanitize_hud_styled_runs(line.runs, &text);
    Some(HudInventoryTooltipLine {
        text,
        tint: line.tint.map(|component| component.clamp(0.0, 1.0)),
        runs,
    })
}

fn sanitize_hud_text_line(line: String) -> Option<String> {
    let line = line
        .chars()
        .filter(|ch| !ch.is_control())
        .take(256)
        .collect::<String>();
    (!line.is_empty()).then_some(line)
}

fn sanitize_hud_text_preserving_empty(line: String, limit: usize) -> String {
    line.chars()
        .filter(|ch| !ch.is_control())
        .take(limit)
        .collect()
}

/// Sanitizes a line's styled runs under the same rules as
/// [`sanitize_hud_text_line`] (control characters stripped, 256-char budget
/// across the line) and clamps run colours to `0xRRGGBB`. Empty run lists —
/// plain producers — synthesize a single default-style run from the already
/// sanitized `fallback_text`, so the draw loops always consume runs.
fn sanitize_hud_styled_runs(
    runs: Vec<HudStyledTextRun>,
    fallback_text: &str,
) -> Vec<HudStyledTextRun> {
    let mut budget = 256usize;
    let mut sanitized = Vec::new();
    for run in runs {
        if budget == 0 {
            break;
        }
        let text = run
            .text
            .chars()
            .filter(|ch| !ch.is_control())
            .take(budget)
            .collect::<String>();
        if text.is_empty() {
            continue;
        }
        budget -= text.chars().count();
        sanitized.push(HudStyledTextRun {
            text,
            style: run.style,
            color: run.color.map(|color| color & 0xFF_FF_FF),
        });
    }
    if sanitized.is_empty() {
        vec![HudStyledTextRun::plain(fallback_text)]
    } else {
        sanitized
    }
}

fn sanitize_hud_item_icon(icon: HudItemIcon) -> Option<HudItemIcon> {
    if icon.lighting != GuiItemLightingEntry::ItemsFlat {
        return None;
    }
    let layers = icon
        .layers
        .into_iter()
        .filter_map(sanitize_hud_icon_layer)
        .collect::<Vec<_>>();
    (!layers.is_empty()).then_some(HudItemIcon {
        lighting: GuiItemLightingEntry::ItemsFlat,
        layers,
        foil: icon.foil,
        count_label: icon.count_label.and_then(sanitize_hud_item_count_label),
        durability_bar: icon
            .durability_bar
            .and_then(sanitize_hud_item_durability_bar),
        cooldown_progress: sanitize_hud_item_cooldown_progress(icon.cooldown_progress),
    })
}

fn sanitize_hud_icon_layer(layer: HudIconLayer) -> Option<HudIconLayer> {
    if !layer.tint.iter().all(|component| component.is_finite()) {
        return None;
    }
    Some(HudIconLayer {
        uv: sanitize_hud_uv_rect(layer.uv)?,
        tint: layer.tint.map(|component| component.clamp(0.0, 1.0)),
    })
}

fn sanitize_hud_item_count_label(label: HudItemCountLabel) -> Option<HudItemCountLabel> {
    let text = label.text;
    (!text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()))
        .then_some(HudItemCountLabel { text })
}

fn sanitize_hud_item_durability_bar(bar: HudItemDurabilityBar) -> Option<HudItemDurabilityBar> {
    if !bar.color.iter().all(|component| component.is_finite()) {
        return None;
    }
    Some(HudItemDurabilityBar {
        width: bar.width.min(HUD_ITEM_BAR_BACKGROUND_WIDTH),
        color: bar.color.map(|component| component.clamp(0.0, 1.0)),
    })
}

fn sanitize_hud_item_cooldown_progress(progress: Option<f32>) -> Option<f32> {
    let progress = progress?;
    progress.is_finite().then_some(progress.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_render_types::{HUD_FONT_BOLD_EXTRA_THICKNESS, HUD_FONT_BOLD_OFFSET};

    #[test]
    fn hud_heart_sprite_names_match_vanilla_assets() {
        // Every vanilla `hud/heart/*` sprite name (Gui.HeartType, Gui.java
        // :1334-1393), excluding the vehicle_* trio.
        let known: std::collections::HashSet<&str> = [
            "container",
            "container_blinking",
            "container_hardcore",
            "container_hardcore_blinking",
            "full",
            "full_blinking",
            "half",
            "half_blinking",
            "hardcore_full",
            "hardcore_full_blinking",
            "hardcore_half",
            "hardcore_half_blinking",
            "poisoned_full",
            "poisoned_full_blinking",
            "poisoned_half",
            "poisoned_half_blinking",
            "poisoned_hardcore_full",
            "poisoned_hardcore_full_blinking",
            "poisoned_hardcore_half",
            "poisoned_hardcore_half_blinking",
            "withered_full",
            "withered_full_blinking",
            "withered_half",
            "withered_half_blinking",
            "withered_hardcore_full",
            "withered_hardcore_full_blinking",
            "withered_hardcore_half",
            "withered_hardcore_half_blinking",
            "absorbing_full",
            "absorbing_full_blinking",
            "absorbing_half",
            "absorbing_half_blinking",
            "absorbing_hardcore_full",
            "absorbing_hardcore_full_blinking",
            "absorbing_hardcore_half",
            "absorbing_hardcore_half_blinking",
            "frozen_full",
            "frozen_full_blinking",
            "frozen_half",
            "frozen_half_blinking",
            "frozen_hardcore_full",
            "frozen_hardcore_full_blinking",
            "frozen_hardcore_half",
            "frozen_hardcore_half_blinking",
        ]
        .into_iter()
        .collect();

        // Every kind × hardcore × half × blink resolves to a real vanilla asset.
        for kind in HudHeartKind::ALL {
            for hardcore in [false, true] {
                for half in [false, true] {
                    for blink in [false, true] {
                        let name = kind.sprite_name(hardcore, half, blink);
                        assert!(known.contains(name.as_str()), "unknown heart sprite {name}");
                    }
                }
            }
        }

        // The hardcore naming asymmetry: `Normal` prefixes `hardcore_`, the
        // typed kinds embed it after their own prefix, `Container` appends
        // `_hardcore`.
        assert_eq!(
            HudHeartKind::Normal.sprite_name(true, false, false),
            "hardcore_full"
        );
        assert_eq!(
            HudHeartKind::Poisoned.sprite_name(true, false, false),
            "poisoned_hardcore_full"
        );
        assert_eq!(
            HudHeartKind::Container.sprite_name(true, false, false),
            "container_hardcore"
        );
        // `Container` ignores half; blink still applies to it.
        assert_eq!(
            HudHeartKind::Container.sprite_name(false, true, false),
            "container"
        );
        assert_eq!(
            HudHeartKind::Container.sprite_name(false, true, true),
            "container_blinking"
        );
        // Half + blinking on a typed kind.
        assert_eq!(
            HudHeartKind::Withered.sprite_name(false, true, true),
            "withered_half_blinking"
        );
        assert_eq!(
            HudHeartKind::Frozen.sprite_name(true, true, true),
            "frozen_hardcore_half_blinking"
        );
    }

    #[test]
    fn hud_block_item_mesh_splits_translucent_quads_and_matching_glint_buckets() {
        let solid = crate::item_models::ItemModelQuad {
            corners: [
                [0.0, 0.0, 8.0],
                [16.0, 0.0, 8.0],
                [16.0, 16.0, 8.0],
                [0.0, 16.0, 8.0],
            ],
            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            tint: [1.0, 1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            shade: 1.0,
            translucent: false,
        };
        let mut translucent = solid.clone();
        translucent.translucent = true;
        let model = HudBlockItemModel {
            quads: vec![solid, translucent],
            gui_display: glam::Mat4::IDENTITY,
            lighting: GuiItemLightingEntry::Items3d,
            foil: true,
        };

        let mut meshes = ItemModelMeshSet::default();
        append_hud_block_item_model_mesh(&mut meshes, &model, glam::Mat4::IDENTITY);

        assert_eq!(meshes.solid.indices.len(), 6);
        assert_eq!(meshes.translucent.indices.len(), 6);
        assert_eq!(meshes.glint.indices.len(), 6);
        assert_eq!(meshes.glint_translucent.indices.len(), 6);
        assert!(meshes
            .solid
            .vertices
            .iter()
            .chain(&meshes.translucent.vertices)
            .chain(&meshes.glint.vertices)
            .chain(&meshes.glint_translucent.vertices)
            .all(|vertex| vertex.light == ITEM_MODEL_FULL_BRIGHT_LIGHT));
    }

    /// End-to-end GPU proof of the HUD 3D block-item consumer, now through the shared offscreen
    /// whole-frame harness: a block item's quad seated at hotbar slot 4 (via the real
    /// [`gui_item_slot_placement`]) renders in `render()`'s GUI item pass under the GUI ortho
    /// camera, and the frame readback shows the slot center as the block's atlas color while a far
    /// corner stays the clear color. Skips (no assertion) when no GPU adapter is available, so it
    /// never fails the suite on adapter-less machines.
    #[test]
    fn hud_block_item_renders_visible_pixels_in_its_slot() {
        use crate::camera::ClearColor;
        use crate::item_models::{GuiItemLightingEntry, HudBlockItemModel, ItemModelQuad};
        use glam::{Mat4, Vec3, Vec4};

        const WIDTH: u32 = 320;
        const HEIGHT: u32 = 240;

        let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
            // No GPU / software adapter on this machine — skip rather than fail the suite.
            return;
        };
        // A 1x1 opaque-red blocks atlas: every UV samples red, so the block icon is unambiguously
        // visible against the blue clear color.
        renderer
            .update_terrain_texture_atlas(&[255, 0, 0, 255])
            .expect("1x1 atlas");
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        // Seat the GUI ortho camera for the item pass (production writes it whenever the camera
        // updates; a fresh renderer has not had one yet).
        renderer.update_camera();

        // One full-slot front-facing quad at hotbar slot 4, centered in the slot exactly as
        // vanilla's display transform centers the model (`gui_display = T(-0.5)`).
        let slot = 4;
        let gui_display = Mat4::from_translation(Vec3::splat(-0.5));
        let quad = ItemModelQuad {
            corners: [
                [0.0, 0.0, 8.0],
                [16.0, 0.0, 8.0],
                [16.0, 16.0, 8.0],
                [0.0, 16.0, 8.0],
            ],
            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            tint: [1.0, 1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            shade: 1.0,
            translucent: false,
        };
        let mut models: Vec<Option<HudBlockItemModel>> = vec![None; HUD_HOTBAR_SLOTS];
        models[slot] = Some(HudBlockItemModel {
            quads: vec![quad],
            gui_display,
            lighting: GuiItemLightingEntry::Items3d,
            foil: false,
        });
        renderer.set_hud_hotbar_block_item_models(models);

        // The slot-center pixel (framebuffer col,row from top-left) is where the placement seats
        // the model origin; pixel (0,0) is far from the bottom-centered hotbar, so it stays
        // background.
        let placement =
            gui_item_slot_placement(hotbar_item_hud_rect(PhysicalSize::new(WIDTH, HEIGHT), slot));
        let center = placement * gui_display * Vec4::new(0.5, 0.5, 0.5, 1.0);
        let center_px = center.x.round() as u32;
        let center_py = center.y.round() as u32;
        assert!(
            center_px < WIDTH && center_py < HEIGHT,
            "slot center in bounds"
        );

        let pixels = renderer.render_offscreen_frame().expect("offscreen frame");
        let center_pixel = pixels.pixel(center_px, center_py);
        let corner_pixel = pixels.pixel(0, 0);

        // The slot center shows the red block icon (R high, B low); the far corner stays blue
        // background.
        assert!(
            center_pixel[0] > 128 && center_pixel[2] < 128,
            "slot center should show the block item, got {center_pixel:?}"
        );
        assert!(
            corner_pixel[2] > 128 && corner_pixel[0] < 128,
            "corner should stay background, got {corner_pixel:?}"
        );
    }

    #[test]
    fn armor_bar_offscreen_frame_draws_only_when_armor_is_positive() {
        use crate::camera::ClearColor;

        const WIDTH: u32 = 320;
        const HEIGHT: u32 = 240;

        let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
            // No GPU / software adapter on this machine — skip rather than fail the suite.
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.update_camera();

        // 9x9 solid-green armor icons (the vanilla armor sprite size), unambiguously
        // visible against the blue clear color.
        let green: Vec<u8> = (0..81).flat_map(|_| [0u8, 255, 0, 255]).collect();
        renderer
            .upload_hud_armor_empty(9, 9, &green)
            .expect("armor_empty");
        renderer
            .upload_hud_armor_half(9, 9, &green)
            .expect("armor_half");
        renderer
            .upload_hud_armor_full(9, 9, &green)
            .expect("armor_full");

        // Armor icon 0's top-left is (guiWidth/2 - 91, guiHeight - 49); sample its center.
        let armor_px = WIDTH / 2 - 91 + 4;
        let armor_py = HEIGHT - 49 + 4;

        // A full 20-point armor bar paints the armor row green.
        renderer.set_hud_armor(Some(20));
        let pixels = renderer.render_offscreen_frame().expect("armor frame");
        let armor_pixel = pixels.pixel(armor_px, armor_py);
        let corner_pixel = pixels.pixel(0, 0);
        assert!(
            armor_pixel[1] > 128 && armor_pixel[0] < 128 && armor_pixel[2] < 128,
            "armor row should show the green icon, got {armor_pixel:?}"
        );
        assert!(
            corner_pixel[2] > 128 && corner_pixel[1] < 128,
            "corner should stay background, got {corner_pixel:?}"
        );

        // Armor value 0 is under vanilla's `armor > 0` gate (Gui.java:800): nothing
        // is drawn, so the armor pixel reverts to the blue background.
        renderer.set_hud_armor(Some(0));
        let pixels = renderer.render_offscreen_frame().expect("no-armor frame");
        let armor_pixel = pixels.pixel(armor_px, armor_py);
        assert!(
            armor_pixel[2] > 128 && armor_pixel[1] < 128,
            "armor row should stay background when armor is 0, got {armor_pixel:?}"
        );
    }

    #[test]
    fn air_bubbles_offscreen_frame_draw_only_underwater_or_below_max() {
        use crate::camera::ClearColor;

        const WIDTH: u32 = 320;
        const HEIGHT: u32 = 240;

        let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
            // No GPU / software adapter on this machine — skip rather than fail the suite.
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.update_camera();

        // 9x9 solid-green bubbles against the blue clear color.
        let green: Vec<u8> = (0..81).flat_map(|_| [0u8, 255, 0, 255]).collect();
        renderer.upload_hud_air_bubble(9, 9, &green).expect("air");
        renderer
            .upload_hud_air_bubble_bursting(9, 9, &green)
            .expect("air_bursting");
        renderer
            .upload_hud_air_bubble_empty(9, 9, &green)
            .expect("air_empty");

        // Bubble index 0 (the rightmost) sits at (guiWidth/2 + 91 - 9,
        // guiHeight - 49) on foot; sample its center.
        let bubble_px = WIDTH / 2 + 91 - 9 + 4;
        let bubble_py = HEIGHT - 49 + 4;

        // Underwater at the full 300-tick supply the row is visible (vanilla
        // draws it whenever the eye is in water, Gui.java:891) and all full.
        renderer.set_hud_air(Some(HudAirSupply {
            air: 300,
            max_air: 300,
            eye_in_water: true,
            tick_count: 0,
        }));
        let pixels = renderer.render_offscreen_frame().expect("underwater frame");
        let bubble_pixel = pixels.pixel(bubble_px, bubble_py);
        assert!(
            bubble_pixel[1] > 128 && bubble_pixel[0] < 128 && bubble_pixel[2] < 128,
            "air row should show the green bubble underwater, got {bubble_pixel:?}"
        );

        // On land at the full supply the `isUnderWater || air < max` gate
        // hides the row: the pixel reverts to the blue background.
        renderer.set_hud_air(Some(HudAirSupply {
            air: 300,
            max_air: 300,
            eye_in_water: false,
            tick_count: 0,
        }));
        let pixels = renderer.render_offscreen_frame().expect("surfaced frame");
        let bubble_pixel = pixels.pixel(bubble_px, bubble_py);
        assert!(
            bubble_pixel[2] > 128 && bubble_pixel[1] < 128,
            "air row should stay background at full air on land, got {bubble_pixel:?}"
        );
    }

    #[test]
    fn vehicle_hearts_offscreen_frame_replace_the_food_row() {
        use crate::camera::ClearColor;

        const WIDTH: u32 = 320;
        const HEIGHT: u32 = 240;

        let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
            // No GPU / software adapter on this machine — skip rather than fail the suite.
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.update_camera();

        // Red food icons vs green vehicle hearts, both 9x9, on the shared
        // right-anchored baseline slot (guiWidth/2 + 91 - 9, guiHeight - 39).
        let red: Vec<u8> = (0..81).flat_map(|_| [255u8, 0, 0, 255]).collect();
        let green: Vec<u8> = (0..81).flat_map(|_| [0u8, 255, 0, 255]).collect();
        renderer
            .upload_hud_food_empty(9, 9, &red)
            .expect("food_empty");
        renderer
            .upload_hud_food_full(9, 9, &red)
            .expect("food_full");
        renderer
            .upload_hud_food_half(9, 9, &red)
            .expect("food_half");
        renderer
            .upload_hud_heart_vehicle_container(9, 9, &green)
            .expect("vehicle_container");
        renderer
            .upload_hud_heart_vehicle_full(9, 9, &green)
            .expect("vehicle_full");
        renderer
            .upload_hud_heart_vehicle_half(9, 9, &green)
            .expect("vehicle_half");
        renderer.set_hud_food(Some(20));

        let slot_px = WIDTH / 2 + 91 - 9 + 4;
        let slot_py = HEIGHT - 39 + 4;

        // On foot the slot shows the red food icon.
        let pixels = renderer.render_offscreen_frame().expect("food frame");
        let slot_pixel = pixels.pixel(slot_px, slot_py);
        assert!(
            slot_pixel[0] > 128 && slot_pixel[1] < 128,
            "the baseline slot should show the red food icon on foot, got {slot_pixel:?}"
        );

        // Riding a living vehicle with hearts: the food row is suppressed
        // (vanilla `vehicleHearts == 0` gate, Gui.java:784-788) and the same
        // slot draws the green vehicle heart instead.
        renderer.set_hud_vehicle_health(Some(HudVehicleHealth {
            health: 10.0,
            max_health: 20.0,
        }));
        let pixels = renderer.render_offscreen_frame().expect("vehicle frame");
        let slot_pixel = pixels.pixel(slot_px, slot_py);
        assert!(
            slot_pixel[1] > 128 && slot_pixel[0] < 128 && slot_pixel[2] < 128,
            "the baseline slot should show the green vehicle heart while riding, got {slot_pixel:?}"
        );

        // A 0-heart vehicle (1.0 max health) keeps the food row (vanilla
        // `getVehicleMaxHearts` -> 0 -> food drawn).
        renderer.set_hud_vehicle_health(Some(HudVehicleHealth {
            health: 1.0,
            max_health: 1.0,
        }));
        let pixels = renderer
            .render_offscreen_frame()
            .expect("zero-heart vehicle frame");
        let slot_pixel = pixels.pixel(slot_px, slot_py);
        assert!(
            slot_pixel[0] > 128 && slot_pixel[1] < 128,
            "a 0-heart vehicle should keep the red food row, got {slot_pixel:?}"
        );
    }

    #[test]
    fn jump_bar_offscreen_frame_replaces_experience_bar_and_uses_cooldown_overlay() {
        use crate::camera::ClearColor;

        const WIDTH: u32 = 320;
        const HEIGHT: u32 = 240;

        let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
            // No GPU / software adapter on this machine — skip rather than fail the suite.
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        renderer.update_camera();

        let solid_bar = |rgba: [u8; 4]| -> Vec<u8> { (0..(182 * 5)).flat_map(|_| rgba).collect() };
        let red = solid_bar([255, 0, 0, 255]);
        let green = solid_bar([0, 255, 0, 255]);
        let yellow = solid_bar([255, 255, 0, 255]);
        let cyan = solid_bar([0, 255, 255, 255]);
        renderer
            .upload_hud_experience_background(182, 5, &red)
            .expect("experience background");
        renderer
            .upload_hud_experience_progress(182, 5, &red)
            .expect("experience progress");
        renderer
            .upload_hud_jump_bar_background(182, 5, &green)
            .expect("jump background");
        renderer
            .upload_hud_jump_bar_progress(182, 5, &yellow)
            .expect("jump progress");
        renderer
            .upload_hud_jump_bar_cooldown(182, 5, &cyan)
            .expect("jump cooldown");

        let left_px = WIDTH / 2 - 91 + 4;
        let right_px = WIDTH / 2 + 91 - 4;
        let bar_py = HEIGHT - 24 - 5 + 2;

        renderer.set_hud_experience_progress(Some(1.0));
        let pixels = renderer.render_offscreen_frame().expect("experience frame");
        let left = pixels.pixel(left_px, bar_py);
        assert!(
            left[0] > 128 && left[1] < 128 && left[2] < 128,
            "experience bar should draw the red sprite before a jump bar is projected, got {left:?}"
        );

        renderer.set_hud_jump_bar(Some(HudJumpBar {
            progress: 0.5,
            cooldown: false,
        }));
        let pixels = renderer.render_offscreen_frame().expect("jump frame");
        let left = pixels.pixel(left_px, bar_py);
        let right = pixels.pixel(right_px, bar_py);
        assert!(
            left[0] > 128 && left[1] > 128 && left[2] < 128,
            "jump progress should cover the left side with yellow, got {left:?}"
        );
        assert!(
            right[1] > 128 && right[0] < 128 && right[2] < 128,
            "jump background should remain visible past the clipped progress, got {right:?}"
        );

        renderer.set_hud_jump_bar(Some(HudJumpBar {
            progress: 0.5,
            cooldown: true,
        }));
        let pixels = renderer.render_offscreen_frame().expect("cooldown frame");
        let left = pixels.pixel(left_px, bar_py);
        let right = pixels.pixel(right_px, bar_py);
        assert!(
            left[1] > 128 && left[2] > 128 && left[0] < 128,
            "cooldown overlay should cover the left side with cyan, got {left:?}"
        );
        assert!(
            right[1] > 128 && right[2] > 128 && right[0] < 128,
            "cooldown overlay should cover the whole bar, got {right:?}"
        );
    }

    #[test]
    fn poison_heart_offscreen_frame_uses_the_poisoned_sprite() {
        use crate::camera::ClearColor;

        const WIDTH: u32 = 320;
        const HEIGHT: u32 = 240;

        let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
            // No GPU / software adapter on this machine — skip rather than fail.
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        renderer.update_camera();

        // A blue container under a red normal fill vs a green poisoned fill,
        // all 9x9 and opaque so the fill covers the container.
        let red: Vec<u8> = (0..81).flat_map(|_| [255u8, 0, 0, 255]).collect();
        let green: Vec<u8> = (0..81).flat_map(|_| [0u8, 255, 0, 255]).collect();
        let blue: Vec<u8> = (0..81).flat_map(|_| [0u8, 0, 255, 255]).collect();
        renderer
            .upload_hud_heart_sprite(HudHeartKind::Container, false, false, 9, 9, &blue)
            .expect("container");
        renderer
            .upload_hud_heart_sprite(HudHeartKind::Normal, false, false, 9, 9, &red)
            .expect("normal full");
        renderer
            .upload_hud_heart_sprite(HudHeartKind::Poisoned, false, false, 9, 9, &green)
            .expect("poisoned full");

        // The leftmost heart (container index 0, column 0) at xLeft = W/2 - 91,
        // yLineBase = H - 39, sampled at its center.
        let heart_px = WIDTH / 2 - 91 + 4;
        let heart_py = HEIGHT - 39 + 4;

        let full_normal = HudPlayerHealth {
            health: 20.0,
            max_health: 20.0,
            absorption: 0.0,
            heart_type: HudHeartKind::Normal,
            hardcore: false,
            regen: false,
            tick_count: 0,
        };
        renderer.set_hud_player_health(Some(full_normal));
        let pixels = renderer.render_offscreen_frame().expect("normal frame");
        let px = pixels.pixel(heart_px, heart_py);
        assert!(
            px[0] > 128 && px[1] < 128,
            "a normal heart should draw the red normal fill, got {px:?}"
        );

        // `HeartType.forPlayer` picks POISONED under the Poison effect: the same
        // slot now draws the green poisoned fill.
        renderer.set_hud_player_health(Some(HudPlayerHealth {
            heart_type: HudHeartKind::Poisoned,
            ..full_normal
        }));
        let pixels = renderer.render_offscreen_frame().expect("poison frame");
        let px = pixels.pixel(heart_px, heart_py);
        assert!(
            px[1] > 128 && px[0] < 128,
            "a poisoned heart should draw the green poisoned fill, got {px:?}"
        );
    }

    #[test]
    fn sanitize_hud_uv_rect_discards_non_finite_values() {
        let rect = HudUvRect {
            min: [0.0, f32::NAN],
            max: [1.0, 1.0],
        };
        assert_eq!(sanitize_hud_uv_rect(rect), None);

        let rect = HudUvRect {
            min: [0.0, 0.0],
            max: [f32::INFINITY, 1.0],
        };
        assert_eq!(sanitize_hud_uv_rect(rect), None);
    }

    #[test]
    fn sanitize_hud_uv_rect_clamps_and_orders_bounds() {
        let rect = HudUvRect {
            min: [1.25, 0.75],
            max: [-0.5, 0.25],
        };
        assert_eq!(
            sanitize_hud_uv_rect(rect),
            Some(HudUvRect {
                min: [0.0, 0.25],
                max: [1.0, 0.75],
            })
        );
    }

    #[test]
    fn sanitize_hud_hotbar_item_uv_wraps_legacy_api_as_single_white_layer() {
        let icon = sanitize_hud_hotbar_item_uv(HudUvRect {
            min: [1.25, 0.75],
            max: [-0.5, 0.25],
        })
        .expect("clamped legacy UV should remain");

        assert_eq!(
            icon,
            HudItemIcon {
                lighting: GuiItemLightingEntry::ItemsFlat,
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, 0.25],
                        max: [1.0, 0.75],
                    },
                    HUD_TINT_WHITE,
                )],
                foil: HudItemFoil::None,
                count_label: None,
                durability_bar: None,
                cooldown_progress: None,
            }
        );

        assert_eq!(
            sanitize_hud_hotbar_item_uv(HudUvRect {
                min: [0.0, f32::NAN],
                max: [1.0, 1.0],
            }),
            None
        );
    }

    #[test]
    fn sanitize_hud_item_icon_preserves_layer_order_and_clamps_tint() {
        let first = HudIconLayer::new(
            HudUvRect {
                min: [0.0, 0.0],
                max: [0.25, 0.25],
            },
            [-1.0, 0.25, 1.5, 1.0],
        );
        let second = HudIconLayer::new(
            HudUvRect {
                min: [0.25, 0.25],
                max: [0.5, 0.5],
            },
            [0.75, 0.5, 0.25, 0.0],
        );
        let icon = sanitize_hud_item_icon(HudItemIcon {
            lighting: GuiItemLightingEntry::ItemsFlat,
            layers: vec![first, second],
            foil: HudItemFoil::Standard,
            count_label: Some(HudItemCountLabel::new("64")),
            durability_bar: Some(HudItemDurabilityBar::new(99, [-1.0, 0.5, 1.5])),
            cooldown_progress: Some(1.5),
        })
        .expect("valid icon layers should remain");

        assert_eq!(icon.layers.len(), 2);
        assert_eq!(icon.foil, HudItemFoil::Standard);
        assert_eq!(icon.count_label, Some(HudItemCountLabel::new("64")));
        assert_eq!(
            icon.durability_bar,
            Some(HudItemDurabilityBar::new(13, [0.0, 0.5, 1.0]))
        );
        assert_eq!(icon.cooldown_progress, Some(1.0));
        assert_eq!(icon.layers[0].uv.min, [0.0, 0.0]);
        assert_eq!(icon.layers[0].uv.max, [0.25, 0.25]);
        assert_eq!(icon.layers[0].tint, [0.0, 0.25, 1.0, 1.0]);
        assert_eq!(icon.layers[1].uv.min, [0.25, 0.25]);
        assert_eq!(icon.layers[1].uv.max, [0.5, 0.5]);
        assert_eq!(icon.layers[1].tint, [0.75, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn hud_item_icon_draw_steps_match_vanilla_item_decoration_order() {
        // Vanilla `GuiGraphicsExtractor.itemDecorations` calls `itemBar`,
        // `itemCooldown`, then `itemCount`, after the item sprite itself has
        // already been submitted to the GUI item atlas.
        let icon = HudItemIcon {
            lighting: GuiItemLightingEntry::ItemsFlat,
            layers: vec![HudIconLayer::new(
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                HUD_TINT_WHITE,
            )],
            foil: HudItemFoil::Standard,
            count_label: Some(HudItemCountLabel::new("64")),
            durability_bar: Some(HudItemDurabilityBar::new(13, [1.0, 0.0, 0.0])),
            cooldown_progress: Some(0.5),
        };

        let mut steps = Vec::new();
        for_each_hud_item_icon_draw_step(&icon, true, true, |step| steps.push(step));
        assert_eq!(
            steps,
            vec![
                HudItemIconDrawStep::Layers,
                HudItemIconDrawStep::Glint,
                HudItemIconDrawStep::DurabilityBar,
                HudItemIconDrawStep::Cooldown,
                HudItemIconDrawStep::CountLabel,
            ]
        );

        let mut steps = Vec::new();
        for_each_hud_item_icon_draw_step(&icon, false, true, |step| steps.push(step));
        assert_eq!(
            steps,
            vec![
                HudItemIconDrawStep::DurabilityBar,
                HudItemIconDrawStep::Cooldown,
                HudItemIconDrawStep::CountLabel,
            ]
        );
    }

    #[test]
    fn hud_special_foil_glint_uv_uses_gui_sheeted_decal_scale() {
        // Vanilla `ItemFeatureRenderer.computeFoilDecalPose` scales GUI SPECIAL foil poses by 0.5
        // before `SheetedDecalTextureGenerator` applies its 1/128 texture scale.
        assert_eq!(hud_item_special_foil_glint_uv([0.0, 0.0]), [0.0, -0.0]);
        assert_eq!(
            hud_item_special_foil_glint_uv([1.0, 0.0]),
            [2.0 / 128.0, -0.0]
        );
        assert_eq!(
            hud_item_special_foil_glint_uv([1.0, 1.0]),
            [2.0 / 128.0, -2.0 / 128.0]
        );
    }

    #[test]
    fn sanitize_hud_item_icon_discards_invalid_layers() {
        let icon = sanitize_hud_item_icon(HudItemIcon {
            lighting: GuiItemLightingEntry::ItemsFlat,
            layers: vec![
                HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, f32::NAN],
                        max: [1.0, 1.0],
                    },
                    [1.0, 1.0, 1.0, 1.0],
                ),
                HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    [1.0, f32::INFINITY, 1.0, 1.0],
                ),
                HudIconLayer::new(
                    HudUvRect {
                        min: [0.25, 0.25],
                        max: [0.75, 0.75],
                    },
                    [1.0, 1.0, 1.0, 1.0],
                ),
            ],
            foil: HudItemFoil::Standard,
            count_label: Some(HudItemCountLabel::new("1x")),
            durability_bar: Some(HudItemDurabilityBar::new(10, [1.0, f32::NAN, 0.0])),
            cooldown_progress: Some(f32::NAN),
        })
        .expect("one valid layer should remain");

        assert_eq!(icon.layers.len(), 1);
        assert_eq!(icon.foil, HudItemFoil::Standard);
        assert_eq!(icon.count_label, None);
        assert_eq!(icon.durability_bar, None);
        assert_eq!(icon.cooldown_progress, None);
        assert_eq!(icon.layers[0].uv.min, [0.25, 0.25]);
        assert_eq!(icon.layers[0].uv.max, [0.75, 0.75]);

        assert_eq!(
            sanitize_hud_item_icon(HudItemIcon {
                lighting: GuiItemLightingEntry::EntityInUi,
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    [1.0, 1.0, 1.0, 1.0],
                )],
                foil: HudItemFoil::None,
                count_label: None,
                durability_bar: None,
                cooldown_progress: None,
            }),
            None
        );

        assert_eq!(
            sanitize_hud_item_icon(HudItemIcon {
                lighting: GuiItemLightingEntry::ItemsFlat,
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [f32::NAN, 0.0],
                        max: [1.0, 1.0],
                    },
                    [1.0, 1.0, 1.0, 1.0],
                )],
                foil: HudItemFoil::None,
                count_label: Some(HudItemCountLabel::new("64")),
                durability_bar: None,
                cooldown_progress: None,
            }),
            None
        );
    }

    #[test]
    fn hud_digit_text_width_uses_digit_advances_only() {
        let mut glyphs = [HudDigitGlyph::default(); 10];
        glyphs[4].advance = 6;
        glyphs[6].advance = 6;

        assert_eq!(hud_digit_text_width("64", &glyphs), Some(12));
        assert_eq!(hud_digit_text_width("1x", &glyphs), None);
        assert_eq!(hud_digit_text_width("", &glyphs), None);
    }

    #[test]
    fn hud_font_text_width_uses_glyph_map_with_replacement_fallback() {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, advance, ascent) in [('A', 6, 7), (' ', 4, 7), ('?', 5, 7), ('é', 7, 10)] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance,
                    ascent,
                    ..HudAsciiGlyph::default()
                },
            );
        }

        assert_eq!(hud_font_text_width("A A", &glyphs), Some(16));
        assert_eq!(hud_font_text_width("A\u{0007}", &glyphs), Some(11));
        // Mapped non-ASCII codepoints now resolve their own bitmap glyph.
        assert_eq!(hud_font_text_width("é", &glyphs), Some(7));
        assert_eq!(hud_font_glyph('é', &glyphs).ascent, 10);
        // CJK stays outside the bitmap pages (unihex deferred) and still
        // degrades to the `?` replacement glyph.
        assert_eq!(hud_font_text_width("钻", &glyphs), Some(5));
        assert_eq!(hud_font_glyph('钻', &glyphs), hud_font_glyph('?', &glyphs));
        assert_eq!(hud_font_text_width("", &glyphs), None);
    }

    #[test]
    fn hud_font_text_width_styled_adds_bold_offset_per_glyph() {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, advance) in [('a', 6), ('b', 6), ('?', 5)] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance,
                    ..HudAsciiGlyph::default()
                },
            );
        }

        // Vanilla Font.width: bold adds getBoldOffset() (1) per glyph, so bold
        // "ab" is the plain width plus one pixel per character.
        let plain = hud_font_text_width("ab", &glyphs).unwrap();
        let bold = hud_font_text_width_styled(
            "ab",
            &glyphs,
            HudTextStyle {
                bold: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(plain, 12);
        assert_eq!(bold, plain + 2);
        // The default style keeps the existing plain-advance width unchanged.
        assert_eq!(
            hud_font_text_width_styled("ab", &glyphs, HudTextStyle::default()),
            hud_font_text_width("ab", &glyphs)
        );
        // Non-bold flags leave the width alone (obfuscated is equal-advance).
        assert_eq!(
            hud_font_text_width_styled(
                "ab",
                &glyphs,
                HudTextStyle {
                    italic: true,
                    underlined: true,
                    strikethrough: true,
                    obfuscated: true,
                    ..Default::default()
                },
            ),
            Some(plain)
        );
    }

    #[test]
    fn space_provider_zero_pixel_glyphs_advance_without_a_visible_quad() {
        // The `space` provider bakes zero-size `EmptyGlyph`s (`SpaceProvider`
        // in `bbb-item-model`): the ZWNJ (U+200C) maps directly (not through
        // the `?` replacement fallback) with advance 0 and no pixel size, so
        // the draw loops' `width > 0 && height > 0` guard emits no quad for
        // it while `hud_font_text_width` still walks past it for free.
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, advance, width, height) in [
            ('a', 6, 6, 8),
            ('b', 6, 6, 8),
            ('?', 5, 6, 8),
            (' ', 4, 0, 0),
            ('\u{200c}', 0, 0, 0),
        ] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance,
                    width,
                    height,
                    ..HudAsciiGlyph::default()
                },
            );
        }

        let zwnj = hud_font_glyph('\u{200c}', &glyphs);
        assert_eq!(zwnj.advance, 0);
        // No `?` fallback: the space provider glyph resolves directly.
        assert_ne!(zwnj, hud_font_glyph('?', &glyphs));
        // Zero pixel size means the draw loops' guard never emits a quad.
        assert!(!(zwnj.width > 0 && zwnj.height > 0));

        // Inserting a ZWNJ between two glyphs must not change the total
        // advance width.
        assert_eq!(
            hud_font_text_width("a\u{200c}b", &glyphs),
            hud_font_text_width("ab", &glyphs)
        );
    }

    fn styled_test_glyphs() -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        for ch in ['a', 'b'] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance: 6,
                    width: 5,
                    height: 8,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        glyphs
    }

    #[test]
    fn styled_text_pass_geometry_plain_runs_match_the_legacy_axis_aligned_cells() {
        // Default-style runs must reproduce the old per-glyph rect path
        // exactly: one axis-aligned cell per glyph at the pen position, no
        // effects, the line's base tint untouched.
        let glyphs = styled_test_glyphs();
        let geometry = hud_styled_text_pass_geometry(
            &[HudStyledTextRun::plain("ab")],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (100.0, 50.0),
            0.0,
            false,
            [1.0, 0.5, 0.25, 1.0],
            None,
            1.0,
        );
        assert!(geometry.effect_rects.is_empty());
        assert_eq!(geometry.glyph_quads.len(), 2);
        assert_eq!(
            geometry.glyph_quads[0].0.corners,
            [[100.0, 50.0], [100.0, 58.0], [105.0, 58.0], [105.0, 50.0],]
        );
        // Second glyph pen-advanced by the plain advance (6).
        assert_eq!(geometry.glyph_quads[1].0.corners[0], [106.0, 50.0]);
        assert!(geometry
            .glyph_quads
            .iter()
            .all(|(_, tint)| *tint == [1.0, 0.5, 0.25, 1.0]));
    }

    #[test]
    fn inventory_text_input_layout_scrolls_to_cursor_with_width_budget() {
        let glyphs = styled_test_glyphs();
        let label = HudInventoryTextLabel {
            x: 0,
            y: 0,
            width: 12,
            text: "aaaaa".to_string(),
            tint: HUD_TINT_WHITE,
            background: None,
            input: None,
            shadow: false,
            runs: Vec::new(),
        };

        let layout = hud_inventory_text_input_layout(
            &label,
            HudInventoryTextInputDecoration {
                cursor: 5,
                selection: 5,
                scroll_to: 5,
                max_length: 50,
                cursor_visible: true,
                cursor_tint: HUD_TINT_WHITE,
                selection_tint: [0.0, 0.0, 1.0, 1.0],
            },
            &glyphs,
        );

        assert_eq!(layout.displayed_text, "aa");
        assert!(layout.cursor_on_screen);
        assert!(!layout.insert_cursor);
        assert_eq!(layout.cursor_x, 13.0);
        assert_eq!(layout.selection_rect, None);
    }

    #[test]
    fn inventory_text_input_layout_highlights_visible_selection_prefix() {
        let glyphs = styled_test_glyphs();
        let label = HudInventoryTextLabel {
            x: 0,
            y: 0,
            width: 12,
            text: "aaaaa".to_string(),
            tint: HUD_TINT_WHITE,
            background: None,
            input: None,
            shadow: false,
            runs: Vec::new(),
        };

        let layout = hud_inventory_text_input_layout(
            &label,
            HudInventoryTextInputDecoration {
                cursor: 5,
                selection: 0,
                scroll_to: 0,
                max_length: 50,
                cursor_visible: true,
                cursor_tint: HUD_TINT_WHITE,
                selection_tint: [0.0, 0.0, 1.0, 1.0],
            },
            &glyphs,
        );

        assert_eq!(layout.displayed_text, "aa");
        assert!(!layout.cursor_on_screen);
        assert_eq!(layout.selection_rect, Some((0.0, 12)));
    }

    #[test]
    fn styled_text_pass_geometry_bold_runs_double_quads_and_widen_the_pen() {
        let glyphs = styled_test_glyphs();
        let geometry = hud_styled_text_pass_geometry(
            &[HudStyledTextRun {
                text: "ab".to_string(),
                style: HudTextStyle {
                    bold: true,
                    ..Default::default()
                },
                color: None,
            }],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (100.0, 50.0),
            0.0,
            false,
            HUD_TINT_WHITE,
            None,
            1.0,
        );
        // Vanilla bold: each glyph renders twice, the second pass shifted by
        // getBoldOffset()=1.
        assert_eq!(geometry.glyph_quads.len(), 4);
        for pair in geometry.glyph_quads.chunks(2) {
            for (main_corner, bold_corner) in pair[0].0.corners.iter().zip(pair[1].0.corners.iter())
            {
                assert!((bold_corner[0] - main_corner[0] - HUD_FONT_BOLD_OFFSET).abs() < 1e-4);
                assert!((bold_corner[1] - main_corner[1]).abs() < 1e-4);
            }
        }
        // The pen advances by the bold-aware advance (6 + 1), with the
        // extraThickness=0.1 expansion on the cell.
        let second_char_left = geometry.glyph_quads[2].0.corners[0][0];
        assert!((second_char_left - (107.0 - HUD_FONT_BOLD_EXTRA_THICKNESS)).abs() < 1e-4);
    }

    #[test]
    fn styled_text_pass_geometry_emits_underline_and_strikethrough_bars_after_glyphs() {
        let glyphs = styled_test_glyphs();
        let geometry = hud_styled_text_pass_geometry(
            &[HudStyledTextRun {
                text: "a".to_string(),
                style: HudTextStyle {
                    underlined: true,
                    strikethrough: true,
                    ..Default::default()
                },
                color: None,
            }],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (100.0, 50.0),
            0.0,
            false,
            HUD_TINT_WHITE,
            None,
            1.0,
        );
        assert_eq!(geometry.glyph_quads.len(), 1);
        assert_eq!(geometry.effect_rects.len(), 2);
        let (strike, _) = &geometry.effect_rects[0];
        let (underline, _) = &geometry.effect_rects[1];
        assert_eq!(strike.kind, bbb_render_types::HudEffectKind::Strikethrough);
        assert_eq!(underline.kind, bbb_render_types::HudEffectKind::Underline);
        // First-in-line bars extend one pixel left (vanilla position == 0)
        // and span to x + advance.
        assert!((strike.x0 - 99.0).abs() < 1e-4);
        assert!((strike.x1 - 106.0).abs() < 1e-4);
        // Vanilla bar bands: strikethrough y+3.5..y+4.5, underline y+8..y+9.
        assert!((strike.y0 - 53.5).abs() < 1e-4);
        assert!((strike.y1 - 54.5).abs() < 1e-4);
        assert!((underline.y0 - 58.0).abs() < 1e-4);
        assert!((underline.y1 - 59.0).abs() < 1e-4);
    }

    #[test]
    fn styled_text_pass_geometry_shadow_pass_offsets_and_scales_the_run_color() {
        let glyphs = styled_test_glyphs();
        let run = HudStyledTextRun {
            text: "a".to_string(),
            style: HudTextStyle::default(),
            color: Some(0xFF_55_55), // ChatFormatting RED
        };
        let main = hud_styled_text_pass_geometry(
            &[run.clone()],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (100.0, 50.0),
            0.0,
            false,
            HUD_TINT_WHITE,
            None,
            1.0,
        );
        let shadow = hud_styled_text_pass_geometry(
            &[run],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (100.0, 50.0),
            1.0,
            true,
            HUD_TINT_WHITE,
            None,
            1.0,
        );
        // Run colour overrides the line tint on the main pass.
        assert_eq!(
            main.glyph_quads[0].1,
            [1.0, 85.0 / 255.0, 85.0 / 255.0, 1.0]
        );
        // Shadow pass: same glyph at +1,+1 in the vanilla shadow colour
        // (ARGB.scaleRGB(textColor, 0.25)).
        assert_eq!(
            shadow.glyph_quads[0].1,
            [0.25, 85.0 / 255.0 * 0.25, 85.0 / 255.0 * 0.25, 1.0]
        );
        for (shadow_corner, main_corner) in shadow.glyph_quads[0]
            .0
            .corners
            .iter()
            .zip(main.glyph_quads[0].0.corners.iter())
        {
            assert!((shadow_corner[0] - main_corner[0] - 1.0).abs() < 1e-4);
            assert!((shadow_corner[1] - main_corner[1] - 1.0).abs() < 1e-4);
        }
        // White text keeps the historical fixed shadow tint.
        assert_eq!(hud_text_shadow_tint(HUD_TINT_WHITE), HUD_TEXT_SHADOW_TINT);
    }

    #[test]
    fn styled_text_pass_geometry_honors_the_label_width_budget() {
        let glyphs = styled_test_glyphs();
        // Budget of 10: 'a' (cell 5 <= 10) draws, pen 6; 'b' cell needs
        // 6 + 5 = 11 > 10 so its quad is skipped, like the old label loop.
        let geometry = hud_styled_text_pass_geometry(
            &[HudStyledTextRun::plain("ab")],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (0.0, 0.0),
            0.0,
            false,
            HUD_TINT_WHITE,
            Some(10),
            1.0,
        );
        assert_eq!(geometry.glyph_quads.len(), 1);
        assert_eq!(geometry.glyph_quads[0].0.corners[0], [0.0, 0.0]);
    }

    #[test]
    fn styled_text_pass_geometry_scale_multiplies_pen_cells_shadow_and_effects() {
        // `scale` mirrors a vanilla pose scale around the origin: cells, the
        // pen advance, the shadow offset and effect bars all multiply.
        let glyphs = styled_test_glyphs();
        let geometry = hud_styled_text_pass_geometry(
            &[HudStyledTextRun::plain("ab")],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (100.0, 50.0),
            0.0,
            false,
            HUD_TINT_WHITE,
            None,
            4.0,
        );
        assert_eq!(geometry.glyph_quads.len(), 2);
        // 5x8 cell at the origin becomes 20x32.
        assert_eq!(
            geometry.glyph_quads[0].0.corners,
            [[100.0, 50.0], [100.0, 82.0], [120.0, 82.0], [120.0, 50.0]]
        );
        // Second glyph: pen advance 6 font px -> 24 HUD px.
        assert_eq!(geometry.glyph_quads[1].0.corners[0], [124.0, 50.0]);

        // The +1,+1 shadow offset rides the pose scale too (vanilla scales the
        // whole `textWithBackdrop` draw).
        let shadow = hud_styled_text_pass_geometry(
            &[HudStyledTextRun::plain("a")],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (100.0, 50.0),
            1.0,
            true,
            HUD_TINT_WHITE,
            None,
            4.0,
        );
        assert_eq!(shadow.glyph_quads[0].0.corners[0], [104.0, 54.0]);

        // Underline bar: first-in-line -1 font px, span to advance 6, band
        // y+8..y+9, all doubled at scale 2.
        let effects = hud_styled_text_pass_geometry(
            &[HudStyledTextRun {
                text: "a".to_string(),
                style: HudTextStyle {
                    underlined: true,
                    ..Default::default()
                },
                color: None,
            }],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (100.0, 50.0),
            0.0,
            false,
            HUD_TINT_WHITE,
            None,
            2.0,
        );
        let (bar, _) = &effects.effect_rects[0];
        assert_eq!((bar.x0, bar.x1), (98.0, 112.0));
        assert_eq!((bar.y0, bar.y1), (66.0, 68.0));
    }

    #[test]
    fn overlay_message_alpha_matches_vanilla_ramp_and_cap() {
        // Vanilla Gui.extractOverlayMessage:313-316: alpha = (int)(t*255/20),
        // capped at 255; the draw gate is alpha > 0.
        assert_eq!(hud_overlay_message_alpha(60.0), 255);
        assert_eq!(hud_overlay_message_alpha(20.0), 255);
        assert_eq!(hud_overlay_message_alpha(10.0), 127);
        assert_eq!(hud_overlay_message_alpha(0.05), 0);
        assert!(hud_overlay_message_alpha(-0.5) <= 0);
    }

    #[test]
    fn overlay_message_rainbow_rgb_is_deterministic_and_matches_the_hsv_quirk() {
        // hue = t / 50 (Gui.java:324): remaining-time driven, so the same
        // frame state always yields the same colour (no wall clock).
        let mid = hud_overlay_message_rainbow_rgb(25.0);
        // hue 0.5 -> h=3, f=0: (p, q, v) = (0.18, 0.6, 0.6) -> (45, 153, 153).
        assert_eq!(mid, [45.0 / 255.0, 153.0 / 255.0, 153.0 / 255.0]);
        assert_eq!(mid, hud_overlay_message_rainbow_rgb(25.0));
        // hue 1.2 -> hue*6 = 7.2: vanilla wraps h to 1 but keeps f = 6.2, so
        // q goes negative and clamps to 0 (Mth.hsvToArgb quirk kept verbatim).
        assert_eq!(
            hud_overlay_message_rainbow_rgb(60.0),
            [0.0, 153.0 / 255.0, 45.0 / 255.0]
        );
    }

    #[test]
    fn action_bar_draw_centers_above_the_hotbar_and_fades_out() {
        let glyphs = styled_test_glyphs();
        let surface = PhysicalSize::new(320, 240);
        let state = HudActionBarText {
            runs: vec![HudStyledTextRun::plain("ab")],
            remaining_ticks: 60,
            partial_tick: 0.5,
            animate_color: false,
        };
        let draw = hud_action_bar_text_draw(&state, &glyphs, surface).expect("visible");
        // (guiWidth/2 - width/2, guiHeight - 68 - 4) = (160 - 6, 240 - 72).
        assert_eq!(draw.origin, (154.0, 168.0));
        assert_eq!(draw.scale, 1.0);
        // t = 59.5 -> alpha 758 capped at 255 -> opaque white.
        assert_eq!(draw.tint, [1.0, 1.0, 1.0, 1.0]);

        // t = 10 -> alpha 127.
        let fading = HudActionBarText {
            remaining_ticks: 10,
            partial_tick: 0.0,
            ..state.clone()
        };
        assert_eq!(
            hud_action_bar_text_draw(&fading, &glyphs, surface)
                .expect("fading")
                .tint,
            [1.0, 1.0, 1.0, 127.0 / 255.0]
        );

        // alpha == 0 is dropped (vanilla `if (alpha > 0)`).
        let nearly_out = HudActionBarText {
            remaining_ticks: 1,
            partial_tick: 0.999,
            ..state.clone()
        };
        assert_eq!(
            hud_action_bar_text_draw(&nearly_out, &glyphs, surface),
            None
        );
        // An expired timer never draws.
        let expired = HudActionBarText {
            remaining_ticks: 0,
            partial_tick: 0.0,
            ..state.clone()
        };
        assert_eq!(hud_action_bar_text_draw(&expired, &glyphs, surface), None);

        // Jukebox rainbow: hue from remaining time, fade alpha kept.
        let rainbow = HudActionBarText {
            remaining_ticks: 25,
            partial_tick: 0.0,
            animate_color: true,
            ..state.clone()
        };
        assert_eq!(
            hud_action_bar_text_draw(&rainbow, &glyphs, surface)
                .expect("rainbow")
                .tint,
            [45.0 / 255.0, 153.0 / 255.0, 153.0 / 255.0, 1.0]
        );
    }

    #[test]
    fn title_draws_center_and_scale_title_4x_and_subtitle_2x() {
        let glyphs = styled_test_glyphs();
        let surface = PhysicalSize::new(320, 240);
        let state = HudTitleText {
            title_runs: vec![HudStyledTextRun::plain("ab")],
            subtitle_runs: vec![HudStyledTextRun::plain("a")],
            remaining_ticks: 50,
            fade_in: 10,
            stay: 70,
            fade_out: 20,
            partial_tick: 0.25,
        };
        let draws = hud_title_text_draws(&state, &glyphs, surface);
        assert_eq!(draws.len(), 2);
        // Title: center (160, 120) + 4 * (-12/2, -10) = (136, 80).
        assert_eq!(draws[0].origin, (136.0, 80.0));
        assert_eq!(draws[0].scale, 4.0);
        // Stay window -> full alpha.
        assert_eq!(draws[0].tint, [1.0, 1.0, 1.0, 1.0]);
        // Subtitle: center + 2 * (-6/2, 5) = (154, 130), same tint.
        assert_eq!(draws[1].origin, (154.0, 130.0));
        assert_eq!(draws[1].scale, 2.0);
        assert_eq!(draws[1].tint, draws[0].tint);

        // No subtitle set -> only the title line.
        let title_only = HudTitleText {
            subtitle_runs: Vec::new(),
            ..state.clone()
        };
        assert_eq!(hud_title_text_draws(&title_only, &glyphs, surface).len(), 1);

        // Expired timer -> nothing.
        let expired = HudTitleText {
            remaining_ticks: 0,
            ..state.clone()
        };
        assert!(hud_title_text_draws(&expired, &glyphs, surface).is_empty());

        // The very first frame after SetTitleText (t == fadeIn+stay+fadeOut at
        // partial 0) computes fade-in alpha 0 and is dropped, like vanilla.
        let first_frame = HudTitleText {
            remaining_ticks: 100,
            partial_tick: 0.0,
            ..state.clone()
        };
        assert!(hud_title_text_draws(&first_frame, &glyphs, surface).is_empty());
    }

    #[test]
    fn title_alpha_follows_the_fade_in_stay_fade_out_windows() {
        let base = HudTitleText {
            title_runs: vec![HudStyledTextRun::plain("ab")],
            subtitle_runs: Vec::new(),
            remaining_ticks: 50,
            fade_in: 10,
            stay: 70,
            fade_out: 20,
            partial_tick: 0.0,
        };
        // Stay window (fade_out < remaining <= fade_out + stay): full alpha.
        assert_eq!(hud_title_alpha(&base), 255);
        // Fade in (remaining > fade_out + stay): (total - t) * 255 / fade_in;
        // t = 94.5 -> 5.5 * 25.5 = 140.25 -> 140.
        let fade_in = HudTitleText {
            remaining_ticks: 95,
            partial_tick: 0.5,
            ..base.clone()
        };
        assert_eq!(hud_title_alpha(&fade_in), 140);
        // Fade out (remaining <= fade_out): t * 255 / fade_out;
        // t = 9.75 -> 9.75 * 12.75 = 124.3125 -> 124.
        let fade_out = HudTitleText {
            remaining_ticks: 10,
            partial_tick: 0.25,
            ..base.clone()
        };
        assert_eq!(hud_title_alpha(&fade_out), 124);
        // Freshly set title at partial 0: fade-in alpha starts at 0.
        let fresh = HudTitleText {
            remaining_ticks: 100,
            partial_tick: 0.0,
            ..base.clone()
        };
        assert_eq!(hud_title_alpha(&fresh), 0);
    }

    #[test]
    fn overlay_message_and_title_draws_submit_after_status_bars_and_below_screens() {
        // Vanilla `Gui.extractRenderState` submits the overlay message and the
        // title after the hotbar/status decorations (Gui.java:215-217); open
        // screens render in a later pass, so `collect_hud_draws` must push
        // these before the inventory-screen branch.
        let source = include_str!("hud.rs");
        let collect_start = source
            .find("fn collect_hud_draws(")
            .expect("collect_hud_draws is defined");
        let collect_source = &source[collect_start..];
        let food = collect_source
            .find("hud_food_fill(")
            .expect("food bar draws first");
        let overlay = collect_source
            .find("hud_action_bar_text_draw(")
            .expect("action bar draw is resolved");
        let title = collect_source
            .find("hud_title_text_draws(")
            .expect("title draws are resolved");
        let screen = collect_source
            .find("if let Some(screen) = &self.hud_inventory_screen")
            .expect("inventory screen branch follows");
        assert!(
            food < overlay && overlay < title && title < screen,
            "overlay message and title submit after status bars and before screen content"
        );
    }

    #[test]
    fn experience_level_text_submits_after_food_and_before_the_boss_overlay() {
        // Vanilla `Gui.extractRenderState` draws the level number between the
        // status bars and the boss overlay (Gui.java:532-535 then the boss
        // overlay stratum), so `collect_hud_draws` pushes it after the food row
        // and before the boss bars.
        let source = include_str!("hud.rs");
        let collect_start = source
            .find("fn collect_hud_draws(")
            .expect("collect_hud_draws is defined");
        let collect_source = &source[collect_start..];
        let food = collect_source
            .find("hud_food_fill(")
            .expect("food bar draws first");
        let level = collect_source
            .find("push_hud_experience_level_text(")
            .expect("experience level text is drawn");
        let boss = collect_source
            .find("hud_boss_bar_draws(")
            .expect("boss bars draw after");
        assert!(
            food < level && level < boss,
            "experience level submits after the food row and before the boss overlay"
        );
    }

    #[test]
    fn experience_level_projection_gates_on_a_positive_level() {
        // Vanilla `Gui.java:533` draws the level only when `experienceLevel > 0`.
        assert_eq!(hud_experience_level_projection(Some(30)), Some(30));
        assert_eq!(hud_experience_level_projection(Some(1)), Some(1));
        assert_eq!(hud_experience_level_projection(Some(0)), None);
        assert_eq!(hud_experience_level_projection(Some(-4)), None);
        assert_eq!(hud_experience_level_projection(None), None);
    }

    #[test]
    fn experience_level_outline_passes_match_vanilla_offsets_and_colors() {
        // ContextualBarRenderer.java:39-43: four black axis-offset copies drawn
        // in `(+1,0),(-1,0),(0,+1),(0,-1)` order, then the `0x80FF20` green
        // center last, so the fill sits on top of the outline.
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
        const GREEN: [f32; 4] = [128.0 / 255.0, 255.0 / 255.0, 32.0 / 255.0, 1.0];
        assert_eq!(
            HUD_EXPERIENCE_LEVEL_PASSES,
            [
                (1.0, 0.0, BLACK),
                (-1.0, 0.0, BLACK),
                (0.0, 1.0, BLACK),
                (0.0, -1.0, BLACK),
                (0.0, 0.0, GREEN),
            ]
        );
        assert!(HUD_EXPERIENCE_LEVEL_PASSES[..4]
            .iter()
            .all(|pass| pass.2 == BLACK));
        assert_eq!(HUD_EXPERIENCE_LEVEL_PASSES[4].2, GREEN);
    }

    #[test]
    fn food_sprite_variant_prefers_the_hunger_icon_only_under_the_effect() {
        // Under the Hunger effect the variant wins, with the base as fallback
        // when the variant is not loaded.
        assert_eq!(
            hud_food_sprite_variant(true, Some("hunger"), Some("base")),
            Some("hunger")
        );
        assert_eq!(
            hud_food_sprite_variant(true, None, Some("base")),
            Some("base")
        );
        assert_eq!(hud_food_sprite_variant::<&str>(true, None, None), None);
        // Without the effect the base sprite always wins, even if a hunger
        // variant happens to be loaded.
        assert_eq!(
            hud_food_sprite_variant(false, Some("hunger"), Some("base")),
            Some("base")
        );
        assert_eq!(hud_food_sprite_variant(false, Some("hunger"), None), None);
    }

    #[test]
    fn boss_bar_names_round_trip_the_vanilla_getname_vocabularies() {
        // Vanilla `BossEvent.BossBarColor`/`BossBarOverlay` getName strings
        // (BossEvent.java:90-97,122-127) — the same names the world stores.
        for color in HudBossBarColor::ALL {
            assert_eq!(HudBossBarColor::from_name(color.name()), Some(color));
        }
        assert_eq!(HudBossBarColor::from_name("magenta"), None);
        for overlay in [HudBossBarOverlay::Progress]
            .into_iter()
            .chain(HudBossBarOverlay::NOTCHED)
        {
            assert_eq!(HudBossBarOverlay::from_name(overlay.name()), Some(overlay));
        }
        assert_eq!(HudBossBarOverlay::from_name("notched_8"), None);

        // Notched sprite arrays index by `ordinal() - 1`
        // (BossHealthOverlay.java:103); Progress has no notched sheet.
        assert_eq!(HudBossBarOverlay::Progress.notched_index(), None);
        assert_eq!(HudBossBarOverlay::Notched6.notched_index(), Some(0));
        assert_eq!(HudBossBarOverlay::Notched20.notched_index(), Some(3));
    }

    #[test]
    fn sanitize_hud_boss_bar_clamps_progress_into_the_unit_range() {
        let bar = |progress| HudBossBar {
            name_runs: vec![HudStyledTextRun::plain("Wither")],
            progress,
            color: HudBossBarColor::Purple,
            overlay: HudBossBarOverlay::Progress,
        };
        assert_eq!(sanitize_hud_boss_bar(bar(0.25)).progress, 0.25);
        assert_eq!(sanitize_hud_boss_bar(bar(2.0)).progress, 1.0);
        assert_eq!(sanitize_hud_boss_bar(bar(-1.0)).progress, 0.0);
        assert_eq!(sanitize_hud_boss_bar(bar(f32::NAN)).progress, 0.0);
        // The rest of the bar passes through untouched.
        let sanitized = sanitize_hud_boss_bar(bar(f32::INFINITY));
        assert_eq!(sanitized.progress, 0.0);
        assert_eq!(sanitized.name_runs, vec![HudStyledTextRun::plain("Wither")]);
        assert_eq!(sanitized.color, HudBossBarColor::Purple);
    }

    #[test]
    fn boss_bar_layers_follow_vanilla_background_then_cropped_fill_order() {
        let plain = HudBossBar {
            name_runs: Vec::new(),
            progress: 0.5,
            color: HudBossBarColor::Red,
            overlay: HudBossBarOverlay::Progress,
        };
        // Progress overlay: colored background, then the fill cropped to
        // lerpDiscrete(0.5) = 91.
        assert_eq!(
            hud_boss_bar_layers(&plain),
            vec![
                HudBossBarLayer {
                    sheet: HudBossBarSheet::ColorBackground(HudBossBarColor::Red),
                    width: 182,
                },
                HudBossBarLayer {
                    sheet: HudBossBarSheet::ColorProgress(HudBossBarColor::Red),
                    width: 91,
                },
            ]
        );

        // Notched overlays double both the background and the fill
        // (BossHealthOverlay.java:101-103), sharing the same crop width.
        let notched = HudBossBar {
            overlay: HudBossBarOverlay::Notched10,
            ..plain.clone()
        };
        assert_eq!(
            hud_boss_bar_layers(&notched),
            vec![
                HudBossBarLayer {
                    sheet: HudBossBarSheet::ColorBackground(HudBossBarColor::Red),
                    width: 182,
                },
                HudBossBarLayer {
                    sheet: HudBossBarSheet::NotchedBackground(HudBossBarOverlay::Notched10),
                    width: 182,
                },
                HudBossBarLayer {
                    sheet: HudBossBarSheet::ColorProgress(HudBossBarColor::Red),
                    width: 91,
                },
                HudBossBarLayer {
                    sheet: HudBossBarSheet::NotchedProgress(HudBossBarOverlay::Notched10),
                    width: 91,
                },
            ]
        );

        // Zero progress skips both fill layers (vanilla `if (width > 0)`,
        // BossHealthOverlay.java:87), keeping the two backgrounds.
        let empty = HudBossBar {
            progress: 0.0,
            overlay: HudBossBarOverlay::Notched20,
            ..plain
        };
        let layers = hud_boss_bar_layers(&empty);
        assert_eq!(layers.len(), 2);
        assert_eq!(
            layers[1].sheet,
            HudBossBarSheet::NotchedBackground(HudBossBarOverlay::Notched20)
        );
    }

    #[test]
    fn boss_bar_draws_stack_rows_center_names_and_truncate() {
        let glyphs = styled_test_glyphs();
        let surface = PhysicalSize::new(320, 240);
        let bars = vec![
            HudBossBar {
                name_runs: vec![HudStyledTextRun::plain("ab")],
                progress: 1.0,
                color: HudBossBarColor::Purple,
                overlay: HudBossBarOverlay::Progress,
            };
            6
        ];
        let draws = hud_boss_bar_draws(&bars, &glyphs, surface);
        // guiHeight / 3 = 80: rows 12, 31, 50, 69 survive, bars 5-6 drop.
        assert_eq!(
            draws.iter().map(|draw| draw.y).collect::<Vec<_>>(),
            vec![12, 31, 50, 69]
        );
        // Name: centered `(guiWidth/2 - width/2, y - 9)` ("ab" is 12px), at
        // scale 1 in opaque white (vanilla colour -1 with drop shadow).
        assert_eq!(draws[0].name.origin, (154.0, 3.0));
        assert_eq!(draws[1].name.origin, (154.0, 22.0));
        assert_eq!(draws[0].name.scale, 1.0);
        assert_eq!(draws[0].name.tint, HUD_TINT_WHITE);
        // Full progress fills the whole sheet.
        assert_eq!(draws[0].layers.last().unwrap().width, 182);
        assert!(hud_boss_bar_draws(&[], &glyphs, surface).is_empty());
    }

    #[test]
    fn boss_bar_draws_submit_after_status_bars_and_before_the_overlay_message() {
        // Vanilla `Gui.extractRenderState` order: hotbar/status decorations,
        // then the boss overlay stratum, then the overlay message / title
        // (Gui.java:203-217).
        let source = include_str!("hud.rs");
        let collect_start = source
            .find("fn collect_hud_draws(")
            .expect("collect_hud_draws is defined");
        let collect_source = &source[collect_start..];
        let food = collect_source
            .find("hud_food_fill(")
            .expect("food bar draws first");
        let boss = collect_source
            .find("hud_boss_bar_draws(")
            .expect("boss bars are resolved");
        let overlay = collect_source
            .find("hud_action_bar_text_draw(")
            .expect("action bar draw is resolved");
        assert!(
            food < boss && boss < overlay,
            "boss bars submit after status bars and before the overlay message"
        );
    }

    /// Distinct-uv glyph map for obfuscated observability: three advance-6
    /// glyphs ('a'/'b'/'c') plus one advance-4 glyph ('d').
    fn obfuscated_test_glyphs() -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, uv_min, advance) in [
            ('a', 0.1, 6u32),
            ('b', 0.2, 6),
            ('c', 0.3, 6),
            ('d', 0.4, 4),
        ] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    uv: HudUvRect {
                        min: [uv_min, 0.0],
                        max: [uv_min + 0.05, 0.1],
                    },
                    advance,
                    width: 5,
                    height: 8,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        glyphs
    }

    #[test]
    fn styled_text_pass_geometry_italic_shears_through_the_mechanism_primitive() {
        // The live path now feeds `run.style` straight into `styled_quads`, so
        // an italic run's corners must equal the locked mechanism output for the
        // same glyph/pen — a direct comparison against `styled_quads`.
        let glyphs = styled_test_glyphs();
        let style = HudTextStyle {
            italic: true,
            ..Default::default()
        };
        let geometry = hud_styled_text_pass_geometry(
            &[HudStyledTextRun {
                text: "a".to_string(),
                style,
                color: None,
            }],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            0,
            (100.0, 50.0),
            0.0,
            false,
            HUD_TINT_WHITE,
            None,
            1.0,
        );
        let expected = hud_font_glyph('a', &glyphs).styled_quads(100.0, 50.0, style, false);
        assert_eq!(geometry.glyph_quads.len(), expected.len());
        for (drawn, mech) in geometry.glyph_quads.iter().zip(expected.iter()) {
            assert_eq!(drawn.0.corners, mech.corners);
        }
        // Shear is real: the top edge shifts right of the plain cell.
        let plain =
            hud_font_glyph('a', &glyphs).styled_quads(100.0, 50.0, HudTextStyle::default(), false);
        assert!(geometry.glyph_quads[0].0.corners[0][0] > plain[0].corners[0][0]);
    }

    #[test]
    fn styled_text_pass_geometry_non_italic_runs_do_not_regress() {
        // Releasing italic must leave every non-italic run byte-identical: a
        // bold+underlined run drawn upright equals the mechanism output.
        let glyphs = styled_test_glyphs();
        let style = HudTextStyle {
            bold: true,
            underlined: true,
            ..Default::default()
        };
        let geometry = hud_styled_text_pass_geometry(
            &[HudStyledTextRun {
                text: "a".to_string(),
                style,
                color: None,
            }],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            7,
            (100.0, 50.0),
            0.0,
            false,
            HUD_TINT_WHITE,
            None,
            1.0,
        );
        let glyph = hud_font_glyph('a', &glyphs);
        let expected = glyph.styled_quads(100.0, 50.0, style, false);
        for (drawn, mech) in geometry.glyph_quads.iter().zip(expected.iter()) {
            assert_eq!(drawn.0.corners, mech.corners);
        }
        // No shear on any corner (italic false).
        for quad in &geometry.glyph_quads {
            assert_eq!(quad.0.corners[0][0], quad.0.corners[1][0]);
        }
        // The seed is inert for non-obfuscated runs.
        let reseeded = hud_styled_text_pass_geometry(
            &[HudStyledTextRun {
                text: "a".to_string(),
                style,
                color: None,
            }],
            &glyphs,
            &HudObfuscatedGlyphPool::default(),
            999,
            (100.0, 50.0),
            0.0,
            false,
            HUD_TINT_WHITE,
            None,
            1.0,
        );
        assert_eq!(geometry, reseeded);
    }

    #[test]
    fn styled_text_pass_geometry_obfuscated_substitutes_equal_advance_glyphs() {
        let glyphs = obfuscated_test_glyphs();
        let pool = HudObfuscatedGlyphPool::from_glyph_map(&glyphs);
        let run = |text: &str| HudStyledTextRun {
            text: text.to_string(),
            style: HudTextStyle {
                obfuscated: true,
                ..Default::default()
            },
            color: None,
        };
        let pass = |seed: u64| {
            hud_styled_text_pass_geometry(
                &[run("aaaa")],
                &glyphs,
                &pool,
                seed,
                (0.0, 0.0),
                0.0,
                false,
                HUD_TINT_WHITE,
                None,
                1.0,
            )
        };
        // Fixed seed -> deterministic, reproducible glyph sequence.
        let first = pass(42);
        assert_eq!(first, pass(42));
        assert_eq!(first.glyph_quads.len(), 4);
        // Every substitute is an advance-6 pooled glyph (one of the three uvs);
        // pen positions stay on the advance-6 grid (0, 6, 12, 18), so obfuscation
        // never shifts layout.
        let advance6_uvs = [0.1_f32, 0.2, 0.3];
        for (index, (quad, _)) in first.glyph_quads.iter().enumerate() {
            assert!(advance6_uvs.contains(&quad.uv.min[0]));
            assert_eq!(quad.corners[0][0], (index as f32) * 6.0);
        }
        // The four draws are not all the same glyph (the LCG actually varies).
        let uvs: Vec<f32> = first.glyph_quads.iter().map(|(q, _)| q.uv.min[0]).collect();
        assert!(uvs.iter().any(|uv| *uv != uvs[0]));
        // A different frame seed changes the sequence (per-frame jitter).
        let other = pass(43);
        let other_uvs: Vec<f32> = other.glyph_quads.iter().map(|(q, _)| q.uv.min[0]).collect();
        assert_ne!(uvs, other_uvs);
    }

    #[test]
    fn styled_text_pass_geometry_obfuscated_never_substitutes_spaces() {
        // Vanilla `Font.getGlyph` guards `codepoint != 32`: spaces stay spaces.
        // Map: visible 'a' (advance 6) + empty space (advance 4) + a *visible*
        // advance-4 glyph 'd', so a broken guard would draw 'd' where the space
        // is. The space must stay invisible for every seed.
        let mut glyphs = HudFontGlyphMap::new();
        glyphs.insert_first_wins(
            'a',
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [0.1, 0.0],
                    max: [0.15, 0.1],
                },
                advance: 6,
                width: 5,
                height: 8,
                ..HudAsciiGlyph::default()
            },
        );
        glyphs.insert_first_wins(
            ' ',
            HudAsciiGlyph {
                advance: 4,
                width: 0,
                height: 0,
                ..HudAsciiGlyph::default()
            },
        );
        glyphs.insert_first_wins(
            'd',
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [0.4, 0.0],
                    max: [0.45, 0.1],
                },
                advance: 4,
                width: 5,
                height: 8,
                ..HudAsciiGlyph::default()
            },
        );
        let pool = HudObfuscatedGlyphPool::from_glyph_map(&glyphs);
        for seed in 0..16u64 {
            let geometry = hud_styled_text_pass_geometry(
                &[HudStyledTextRun {
                    text: "a a".to_string(),
                    style: HudTextStyle {
                        obfuscated: true,
                        ..Default::default()
                    },
                    color: None,
                }],
                &glyphs,
                &pool,
                seed,
                (0.0, 0.0),
                0.0,
                false,
                HUD_TINT_WHITE,
                None,
                1.0,
            );
            // Only the two visible 'a's draw; the space never becomes a glyph.
            assert_eq!(geometry.glyph_quads.len(), 2, "seed {seed}");
            // The space advances by 4, so the second glyph sits at pen 6 + 4.
            assert_eq!(geometry.glyph_quads[0].0.corners[0][0], 0.0);
            assert_eq!(geometry.glyph_quads[1].0.corners[0][0], 10.0);
        }
    }

    #[test]
    fn sanitize_hud_inventory_screen_keeps_slot_positions_and_sanitizes_icons() {
        let screen = sanitize_hud_inventory_screen(HudInventoryScreen {
            width: 0,
            height: 700,
            background_layers: vec![
                HudInventoryBackgroundLayer {
                    texture: HudInventoryBackgroundTexture::GenericContainer,
                    x: 0,
                    y: 0,
                    width: 176,
                    height: 125,
                    uv: HudUvRect {
                        min: [-1.0, 0.0],
                        max: [0.75, 2.0],
                    },
                },
                HudInventoryBackgroundLayer {
                    texture: HudInventoryBackgroundTexture::Inventory,
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 96,
                    uv: HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                },
            ],
            fill_layers: vec![
                HudInventoryFillLayer {
                    x: 8,
                    y: 84,
                    width: 16,
                    height: 16,
                    tint: [2.0, 0.5, -1.0, 0.5],
                    stage: HudInventoryFillStage::BeforeGhostItem,
                },
                HudInventoryFillLayer {
                    x: 26,
                    y: 84,
                    width: 0,
                    height: 16,
                    tint: [1.0, 1.0, 1.0, 1.0],
                    stage: HudInventoryFillStage::AfterGhostItem,
                },
            ],
            slots: vec![
                HudInventorySlot {
                    slot_id: 5,
                    x: 8,
                    y: 84,
                    icon: Some(HudItemIcon {
                        lighting: GuiItemLightingEntry::ItemsFlat,
                        layers: vec![HudIconLayer::new(
                            HudUvRect {
                                min: [1.25, 0.75],
                                max: [-0.5, 0.25],
                            },
                            [1.5, 0.25, -1.0, 1.0],
                        )],
                        foil: HudItemFoil::Standard,
                        count_label: Some(HudItemCountLabel::new("64")),
                        durability_bar: Some(HudItemDurabilityBar::new(99, [-1.0, 0.5, 1.5])),
                        cooldown_progress: Some(1.5),
                    }),
                    block_model: None,
                },
                HudInventorySlot {
                    slot_id: 6,
                    x: 26,
                    y: 84,
                    icon: Some(HudItemIcon {
                        lighting: GuiItemLightingEntry::ItemsFlat,
                        layers: vec![HudIconLayer::new(
                            HudUvRect {
                                min: [0.0, f32::NAN],
                                max: [1.0, 1.0],
                            },
                            [1.0, 1.0, 1.0, 1.0],
                        )],
                        foil: HudItemFoil::Standard,
                        count_label: Some(HudItemCountLabel::new("bad")),
                        durability_bar: None,
                        cooldown_progress: Some(f32::INFINITY),
                    }),
                    block_model: None,
                },
                HudInventorySlot {
                    slot_id: 7,
                    x: 44,
                    y: 84,
                    icon: None,
                    block_model: None,
                },
            ],
            floating_items: Vec::new(),
            ghost_items: Vec::new(),
            entity_previews: Vec::new(),
            text_labels: vec![
                HudInventoryTextLabel {
                    x: 62,
                    y: 24,
                    width: 103,
                    text: "Name\u{0007}".to_string(),
                    tint: [1.25, 0.5, -1.0, 1.0],
                    background: Some(HudInventoryTextBackground {
                        x: 60,
                        y: 22,
                        width: 120,
                        height: 12,
                        tint: [0.0, 0.0, 0.0, 1.5],
                    }),
                    input: None,
                    shadow: false,
                    runs: Vec::new(),
                },
                HudInventoryTextLabel {
                    x: 10,
                    y: 10,
                    width: 0,
                    text: "ignored".to_string(),
                    tint: HUD_TINT_WHITE,
                    background: None,
                    input: None,
                    shadow: true,
                    runs: Vec::new(),
                },
            ],
            hovered_slot_id: Some(7),
            tooltip: Some(HudInventoryTooltip {
                slot_id: 5,
                x: 8,
                y: 84,
                lines: vec![
                    HudInventoryTooltipLine {
                        text: "Diamond Sword".to_string(),
                        tint: [1.5, 1.0, 0.5, 1.0],
                        runs: Vec::new(),
                    },
                    HudInventoryTooltipLine {
                        text: String::new(),
                        tint: HUD_TINT_WHITE,
                        runs: Vec::new(),
                    },
                    HudInventoryTooltipLine {
                        text: "Attack\u{0007}Damage".to_string(),
                        tint: [0.25, 0.5, 0.75, 2.0],
                        runs: Vec::new(),
                    },
                ],
            }),
        });

        assert_eq!(screen.width, 1);
        assert_eq!(screen.height, 512);
        assert_eq!(screen.background_layers.len(), 1);
        assert_eq!(
            screen.background_layers[0].texture,
            HudInventoryBackgroundTexture::GenericContainer
        );
        assert_eq!(screen.background_layers[0].uv.min, [0.0, 0.0]);
        assert_eq!(screen.background_layers[0].uv.max, [0.75, 1.0]);
        assert_eq!(
            screen.fill_layers,
            vec![HudInventoryFillLayer {
                x: 8,
                y: 84,
                width: 16,
                height: 16,
                tint: [1.0, 0.5, 0.0, 0.5],
                stage: HudInventoryFillStage::BeforeGhostItem,
            }]
        );
        assert_eq!(screen.hovered_slot_id, Some(7));
        assert_eq!(screen.slots.len(), 3);
        assert_eq!(screen.slots[0].slot_id, 5);
        assert_eq!(screen.slots[0].x, 8);
        assert_eq!(screen.slots[0].y, 84);
        assert_eq!(
            screen.slots[0].icon,
            Some(HudItemIcon {
                lighting: GuiItemLightingEntry::ItemsFlat,
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, 0.25],
                        max: [1.0, 0.75],
                    },
                    [1.0, 0.25, 0.0, 1.0],
                )],
                foil: HudItemFoil::Standard,
                count_label: Some(HudItemCountLabel::new("64")),
                durability_bar: Some(HudItemDurabilityBar::new(13, [0.0, 0.5, 1.0])),
                cooldown_progress: Some(1.0),
            })
        );
        assert_eq!(screen.slots[1].slot_id, 6);
        assert_eq!(screen.slots[1].icon, None);
        assert_eq!(screen.slots[2].slot_id, 7);
        assert_eq!(screen.slots[2].icon, None);
        assert_eq!(
            screen.text_labels,
            vec![HudInventoryTextLabel {
                x: 62,
                y: 24,
                width: 103,
                text: "Name".to_string(),
                tint: [1.0, 0.5, 0.0, 1.0],
                background: Some(HudInventoryTextBackground {
                    x: 60,
                    y: 22,
                    width: 120,
                    height: 12,
                    tint: [0.0, 0.0, 0.0, 1.0],
                }),
                input: None,
                shadow: false,
                // Plain labels (empty runs in) synthesize one default-style
                // run from the sanitized text.
                runs: vec![HudStyledTextRun::plain("Name")],
            }]
        );
        assert_eq!(
            screen.tooltip,
            Some(HudInventoryTooltip {
                slot_id: 5,
                x: 8,
                y: 84,
                lines: vec![
                    HudInventoryTooltipLine {
                        text: "Diamond Sword".to_string(),
                        tint: [1.0, 1.0, 0.5, 1.0],
                        runs: vec![HudStyledTextRun::plain("Diamond Sword")],
                    },
                    HudInventoryTooltipLine {
                        text: "AttackDamage".to_string(),
                        tint: [0.25, 0.5, 0.75, 1.0],
                        runs: vec![HudStyledTextRun::plain("AttackDamage")],
                    },
                ],
            })
        );
    }

    #[test]
    fn sanitize_hud_inventory_text_label_keeps_empty_text_input_decoration() {
        let label = sanitize_hud_inventory_text_label(HudInventoryTextLabel {
            x: 29,
            y: 16,
            width: 73,
            text: String::new(),
            tint: [1.2, 1.0, -1.0, 1.0],
            background: None,
            input: Some(HudInventoryTextInputDecoration {
                cursor: 99,
                selection: 98,
                scroll_to: 97,
                max_length: 2_000,
                cursor_visible: true,
                cursor_tint: [2.0, 0.5, -1.0, 1.0],
                selection_tint: [0.0, 0.0, 2.0, 1.5],
            }),
            shadow: false,
            runs: Vec::new(),
        })
        .unwrap();

        assert_eq!(label.text, "");
        assert_eq!(label.tint, [1.0, 1.0, 0.0, 1.0]);
        assert_eq!(label.runs, vec![HudStyledTextRun::plain("")]);
        assert_eq!(
            label.input,
            Some(HudInventoryTextInputDecoration {
                cursor: 0,
                selection: 0,
                scroll_to: 0,
                max_length: 1024,
                cursor_visible: true,
                cursor_tint: [1.0, 0.5, 0.0, 1.0],
                selection_tint: [0.0, 0.0, 1.0, 1.0],
            })
        );
    }

    #[test]
    fn sanitize_hud_inventory_screen_keeps_sanitized_floating_items() {
        let screen = sanitize_hud_inventory_screen(HudInventoryScreen {
            width: 176,
            height: 166,
            background_layers: Vec::new(),
            fill_layers: Vec::new(),
            slots: Vec::new(),
            floating_items: vec![
                HudInventoryItem {
                    x: 33,
                    y: 19,
                    scale: 2.0,
                    scale_y: 20.0,
                    icon: HudItemIcon {
                        lighting: GuiItemLightingEntry::ItemsFlat,
                        layers: vec![HudIconLayer::new(
                            HudUvRect {
                                min: [1.25, 0.75],
                                max: [-0.5, 0.25],
                            },
                            [1.25, 0.5, -1.0, 1.0],
                        )],
                        foil: HudItemFoil::Standard,
                        count_label: Some(HudItemCountLabel::new("12")),
                        durability_bar: Some(HudItemDurabilityBar::new(99, [0.25, 2.0, -1.0])),
                        cooldown_progress: Some(1.5),
                    },
                    draw_decorations: true,
                    block_model: None,
                },
                HudInventoryItem {
                    x: 51,
                    y: 19,
                    scale: 1.0,
                    scale_y: 1.0,
                    icon: HudItemIcon {
                        lighting: GuiItemLightingEntry::ItemsFlat,
                        layers: vec![HudIconLayer::new(
                            HudUvRect {
                                min: [0.0, f32::NAN],
                                max: [1.0, 1.0],
                            },
                            [1.0, 1.0, 1.0, 1.0],
                        )],
                        foil: HudItemFoil::Standard,
                        count_label: Some(HudItemCountLabel::new("64")),
                        durability_bar: None,
                        cooldown_progress: None,
                    },
                    draw_decorations: true,
                    block_model: None,
                },
            ],
            ghost_items: Vec::new(),
            entity_previews: Vec::new(),
            hovered_slot_id: None,
            tooltip: None,
            text_labels: Vec::new(),
        });

        assert_eq!(screen.floating_items.len(), 1);
        assert_eq!(screen.floating_items[0].x, 33);
        assert_eq!(screen.floating_items[0].y, 19);
        assert_eq!(screen.floating_items[0].scale, 2.0);
        assert_eq!(screen.floating_items[0].scale_y, 16.0);
        assert!(screen.floating_items[0].draw_decorations);
        assert_eq!(
            screen.floating_items[0].icon,
            HudItemIcon {
                lighting: GuiItemLightingEntry::ItemsFlat,
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, 0.25],
                        max: [1.0, 0.75],
                    },
                    [1.0, 0.5, 0.0, 1.0],
                )],
                foil: HudItemFoil::Standard,
                count_label: Some(HudItemCountLabel::new("12")),
                durability_bar: Some(HudItemDurabilityBar::new(13, [0.25, 1.0, 0.0])),
                cooldown_progress: Some(1.0),
            }
        );
    }

    #[test]
    fn hud_entity_preview_blit_submits_after_background_and_before_slot_content() {
        // Vanilla submission order: `renderBg` blits the background, then the entity preview PIP
        // (`addBlitToCurrentLayer`), then slot highlights / items draw above. `collect_hud_draws`
        // must push the preview blit between the background layers and the hovered-slot lookup.
        let source = include_str!("hud.rs");
        let collect_start = source
            .find("fn collect_hud_draws(")
            .expect("collect_hud_draws is defined");
        let collect_source = &source[collect_start..];
        let background = collect_source
            .find("for layer in &screen.background_layers")
            .expect("background layers draw first");
        let blit = collect_source
            .find("HudDrawCommand::EntityPreviewBlit")
            .expect("preview blit command is pushed");
        let hovered = collect_source
            .find("let hovered_slot = screen")
            .expect("hovered slot content follows");
        assert!(
            background < blit && blit < hovered,
            "preview blit submits after backgrounds and before slot content"
        );
    }

    #[test]
    fn hud_entity_preview_blit_uv_is_identity_without_scissor() {
        let rect = HudEntityPreviewRect {
            x: 26,
            y: 8,
            width: 49,
            height: 70,
        };
        assert_eq!(
            hud_entity_preview_blit_uv(rect, rect),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            }
        );
    }

    #[test]
    fn hud_entity_preview_blit_uv_samples_scissored_sub_rect() {
        // rect 26..75 x 8..78 (49x70), visible sub-rect 30..40 x 20..32: the blit samples the
        // matching PIP texture fractions, with no vertical flip (wgpu row 0 is the texture top).
        let rect = HudEntityPreviewRect {
            x: 26,
            y: 8,
            width: 49,
            height: 70,
        };
        let visible = HudEntityPreviewRect {
            x: 30,
            y: 20,
            width: 10,
            height: 12,
        };
        let uv = hud_entity_preview_blit_uv(rect, visible);
        assert_eq!(uv.min, [4.0 / 49.0, 12.0 / 70.0]);
        assert_eq!(uv.max, [14.0 / 49.0, 24.0 / 70.0]);
    }

    #[test]
    fn sanitize_hud_entity_preview_rect_clamps_texture_dimensions() {
        assert_eq!(
            sanitize_hud_entity_preview_rect(HudEntityPreviewRect {
                x: 4,
                y: 6,
                width: 4096,
                height: 9999,
            }),
            Some(HudEntityPreviewRect {
                x: 4,
                y: 6,
                width: 512,
                height: 512,
            })
        );
    }

    /// End-to-end GPU proof of the entity-preview PIP consumer: bakes the vanilla smithing-screen
    /// armor-stand preview through the production `bake_hud_entity_preview_pip_geometry` path,
    /// renders it into an isolated 40x60 PIP color+depth target under the real
    /// [`CameraUniform::hud_entity_preview_pip`] camera (private depth cleared to 1.0 — the
    /// `depth_isolated` contract: no world depth is attached), then blits the PIP texture into a
    /// HUD frame through the real HUD sprite pipeline at the preview's screen rect. Asserts the
    /// projected first-face anchor shows the entity texture, the in-rect empty corner keeps the
    /// HUD background (transparent PIP pixels do not overwrite the frame), and pixels outside the
    /// rect stay background. Skips (no assertion) when no GPU adapter is available.
    #[test]
    fn hud_entity_preview_pip_renders_and_blits_isolated_entity_pixels() {
        use wgpu::util::DeviceExt;

        use crate::camera::{CameraUniform, LightmapEnvironment};
        use crate::entity_models::{
            armor_stand_entity_texture_refs, bake_hud_entity_preview_pip_geometry,
            build_entity_model_texture_atlas, create_entity_model_textured_pipeline,
            EntityModelTextureImage, EntityModelTextureRef, DEFAULT_ARMOR_STAND_MODEL_POSE,
        };
        use crate::gpu::{
            create_camera_buffer, create_depth_target, create_terrain_bind_group_layout,
        };
        use crate::lightmap::{
            create_lightmap_bind_group_layout, create_lightmap_gpu,
            create_lightmap_sample_bind_group_layout,
        };
        use glam::{Quat, Vec4};

        const FRAME_WIDTH: u32 = 320;
        const FRAME_HEIGHT: u32 = 240;
        const PIP_WIDTH: u32 = 40;
        const PIP_HEIGHT: u32 = 60;
        // Non-sRGB frame so readback bytes are the shader's output verbatim; `320 * 4 = 1280` is a
        // multiple of 256, so the copy needs no row padding.
        const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let Some(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
        else {
            // No GPU / software adapter on this machine — skip rather than fail the suite.
            return;
        };
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("bbb-hud-entity-preview-pip-test-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )) else {
            return;
        };

        // Entity atlas with the armor-stand texture filled opaque green, so every textured face
        // samples an unambiguous non-background colour.
        let green = |texture: EntityModelTextureRef| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            let mut rgba = vec![0u8; len];
            for pixel in rgba.chunks_exact_mut(4) {
                pixel.copy_from_slice(&[0, 255, 0, 255]);
            }
            EntityModelTextureImage::new(texture, rgba)
        };
        let images: Vec<EntityModelTextureImage> = armor_stand_entity_texture_refs()
            .iter()
            .copied()
            .map(green)
            .collect();
        let (atlas_layout, atlas_rgba) =
            build_entity_model_texture_atlas(&images).expect("entity atlas");

        // The vanilla smithing-screen preview: armor stand at the origin, bodyRot 210, head pitch
        // 25, rect 121..161 x 20..80 (40x60), scale 25, translation (0, 1, 0), rotation
        // rotX(0.43633232) * rotZ(PI) — the exact `hud_smithing_entity_preview` producer values.
        let entity = EntityModelInstance::armor_stand(
            -1,
            [0.0, 0.0, 0.0],
            210.0,
            false,
            true,
            false,
            DEFAULT_ARMOR_STAND_MODEL_POSE,
        )
        .with_head_look(0.0, 25.0);
        let rotation =
            Quat::from_rotation_x(0.43633232) * Quat::from_rotation_z(std::f32::consts::PI);
        let geometry = bake_hud_entity_preview_pip_geometry(&entity, &atlas_layout, None, None);
        assert!(
            !geometry.textured_indices.is_empty(),
            "armor stand preview bakes textured PIP geometry"
        );
        assert!(
            !geometry.textured_draws.is_empty(),
            "armor stand preview records PIP draw ranges"
        );
        let camera_uniform = CameraUniform::hud_entity_preview_pip(
            PIP_WIDTH as f32,
            PIP_HEIGHT as f32,
            25.0,
            [0.0, 1.0, 0.0],
            rotation.to_array(),
        );

        // Anchor: the first textured face's model-space center, projected through the same PIP
        // view-projection the shader uses — its own (green) face covers that pixel no matter which
        // equally-green part wins the depth test.
        let face_center = geometry
            .first_textured_face_center()
            .expect("baked geometry has a first face");
        let clip = camera_uniform.view_proj() * Vec4::from((face_center, 1.0));
        let ndc = clip.truncate() / clip.w;
        let anchor_px = ((ndc.x * 0.5 + 0.5) * PIP_WIDTH as f32).round() as u32;
        let anchor_py = ((0.5 - ndc.y * 0.5) * PIP_HEIGHT as f32).round() as u32;
        assert!(
            anchor_px < PIP_WIDTH && anchor_py < PIP_HEIGHT,
            "anchor projects into the PIP texture, got ({anchor_px},{anchor_py})"
        );

        // Entity atlas + PIP camera bind group (terrain layout: camera @0, texture @1, sampler @2).
        let bind_group_layout = create_terrain_bind_group_layout(&device);
        let camera_buffer = create_camera_buffer(&device);
        queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pip-test-atlas"),
            size: wgpu::Extent3d {
                width: atlas_layout.width,
                height: atlas_layout.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(atlas_layout.width * 4),
                rows_per_image: Some(atlas_layout.height),
            },
            wgpu::Extent3d {
                width: atlas_layout.width,
                height: atlas_layout.height,
                depth_or_array_layers: 1,
            },
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pip-test-atlas-sampler"),
            ..Default::default()
        });
        let entity_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pip-test-entity-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });
        let lightmap_bind_group_layout = create_lightmap_bind_group_layout(&device);
        let lightmap_sample_bind_group_layout = create_lightmap_sample_bind_group_layout(&device);
        let lightmap = create_lightmap_gpu(
            &device,
            &queue,
            &lightmap_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            LightmapEnvironment::default(),
        );
        // The armor stand's buckets are all cutout-family; one no-cull cutout pipeline covers the
        // whole concatenated index stream for the sentinel.
        let entity_pipeline = create_entity_model_textured_pipeline(
            &device,
            COLOR_FORMAT,
            &bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let entity_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pip-test-entity-vertices"),
            contents: geometry.textured_vertex_bytes(),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let entity_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pip-test-entity-indices"),
            contents: bytemuck::cast_slice(&geometry.textured_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Isolated PIP target: private color texture (sampled by the blit) + private depth.
        let pip_color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pip-test-color"),
            size: wgpu::Extent3d {
                width: PIP_WIDTH,
                height: PIP_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let pip_color_view = pip_color.create_view(&wgpu::TextureViewDescriptor::default());
        let pip_depth = create_depth_target(&device, PIP_WIDTH, PIP_HEIGHT);

        // HUD frame + blit resources: the production HUD sprite pipeline and vertex path.
        let hud_bind_group_layout = create_hud_bind_group_layout(&device);
        let hud_pipeline = create_hud_pipeline(&device, COLOR_FORMAT, &hud_bind_group_layout);
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pip-test-blit-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pip-test-blit-bind-group"),
            layout: &hud_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&pip_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&blit_sampler),
                },
            ],
        });
        // The preview blit rect at the smithing screen position, through the production screen
        // layout mapping (screen 176x166 centered; preview rect origin 121,20).
        let surface_size = PhysicalSize::new(FRAME_WIDTH, FRAME_HEIGHT);
        let blit_rect =
            inventory_background_hud_rect(surface_size, 176, 166, 121, 20, PIP_WIDTH, PIP_HEIGHT);
        let rect = HudEntityPreviewRect {
            x: 121,
            y: 20,
            width: PIP_WIDTH,
            height: PIP_HEIGHT,
        };
        let blit_vertices_data = hud_quad_vertices(
            surface_size,
            blit_rect,
            hud_entity_preview_blit_uv(rect, rect),
            [1.0, 1.0, 1.0, 1.0],
        );
        let blit_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pip-test-blit-vertices"),
            contents: bytemuck::cast_slice(&blit_vertices_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let frame_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pip-test-frame"),
            size: wgpu::Extent3d {
                width: FRAME_WIDTH,
                height: FRAME_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let frame_view = frame_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bytes_per_row = FRAME_WIDTH * 4;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pip-test-readback"),
            size: (bytes_per_row * FRAME_HEIGHT) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        // PIP pass: isolated color cleared to transparent + private depth cleared to 1.0 (the
        // depth_isolated contract — the frame's depth is never attached here).
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pip-test-pip-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &pip_color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &pip_depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&entity_pipeline);
            pass.set_bind_group(0, &entity_bind_group, &[]);
            pass.set_bind_group(1, &lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, entity_vertices.slice(..));
            pass.set_index_buffer(entity_indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..geometry.textured_indices.len() as u32, 0, 0..1);
        }
        // HUD pass: blue background, then the PIP blit quad through the HUD sprite pipeline.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pip-test-hud-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&hud_pipeline);
            pass.set_bind_group(0, &blit_bind_group, &[]);
            pass.set_vertex_buffer(0, blit_vertices.slice(..));
            pass.draw(0..blit_vertices_data.len() as u32, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &frame_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(FRAME_HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: FRAME_WIDTH,
                height: FRAME_HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);
        let data = slice.get_mapped_range();
        let pixel = |x: u32, y: u32| -> [u8; 4] {
            let o = (y * bytes_per_row + x * 4) as usize;
            [data[o], data[o + 1], data[o + 2], data[o + 3]]
        };
        // The screen origin is ((320-176)/2, (240-166)/2) = (72, 37); the preview rect lands at
        // (193, 57)..(233, 117) in frame pixels.
        let rect_origin_x = 72 + 121;
        let rect_origin_y = 37 + 20;
        let anchor_pixel = pixel(rect_origin_x + anchor_px, rect_origin_y + anchor_py);
        let in_rect_empty_pixel = pixel(rect_origin_x + 1, rect_origin_y + 1);
        let outside_pixel = pixel(0, 0);

        // The projected anchor shows the green entity texture through the blit.
        assert!(
            anchor_pixel[1] > 64
                && anchor_pixel[1] > anchor_pixel[2]
                && anchor_pixel[0] < anchor_pixel[1],
            "preview anchor should show the green entity, got {anchor_pixel:?}"
        );
        // Transparent PIP pixels inside the rect keep the HUD background (the blit alpha-blends
        // the isolated texture; it does not stamp the whole rect).
        assert!(
            in_rect_empty_pixel[2] > 128 && in_rect_empty_pixel[1] < 64,
            "empty preview pixel should keep the HUD background, got {in_rect_empty_pixel:?}"
        );
        // Pixels outside the preview rect stay background.
        assert!(
            outside_pixel[2] > 128 && outside_pixel[1] < 64,
            "outside pixel should stay background, got {outside_pixel:?}"
        );

        drop(data);
        readback.unmap();
    }

    fn hud_entity_preview_for_test() -> HudEntityPreview {
        HudEntityPreview {
            entity: EntityModelInstance::chicken(90, [0.0, 64.0, 0.0], 180.0, false)
                .with_light_coords(0)
                .with_outline_color(0xff00_ff00),
            lighting: GuiItemLightingEntry::EntityInUi,
            rect: HudEntityPreviewRect {
                x: 26,
                y: 8,
                width: 49,
                height: 70,
            },
            scissor: Some(HudEntityPreviewRect {
                x: 30,
                y: 20,
                width: 10,
                height: 12,
            }),
            translation: [0.0, 0.875, 0.0],
            rotation: [0.0, 0.0, 1.0, 0.0],
            override_camera_rotation: Some([0.125, 0.0, 0.0, 0.992_156_74]),
            scale: 30.0,
            depth_isolated: true,
            item_layers: vec![HudEntityPreviewItemLayer {
                slot: HudEntityPreviewItemSlot::LeftHand,
                display_context: HudEntityPreviewItemDisplayContext::ThirdPersonLeftHand,
                item_id: 12,
                count: 1,
                foil: true,
                light_coords: 0,
                overlay: [1.0, 2.0],
                order: 0,
                submit_sequence: 1,
            }],
        }
    }

    fn hud_inventory_screen_with_entity_previews(
        entity_previews: Vec<HudEntityPreview>,
    ) -> HudInventoryScreen {
        HudInventoryScreen {
            width: 176,
            height: 166,
            background_layers: Vec::new(),
            fill_layers: Vec::new(),
            slots: Vec::new(),
            floating_items: Vec::new(),
            ghost_items: Vec::new(),
            entity_previews,
            text_labels: Vec::new(),
            hovered_slot_id: None,
            tooltip: None,
        }
    }

    #[test]
    fn sanitize_inventory_keeps_entity_in_ui_preview_render_plan() {
        let screen =
            sanitize_hud_inventory_screen(hud_inventory_screen_with_entity_previews(vec![
                hud_entity_preview_for_test(),
            ]));

        assert_eq!(screen.entity_previews.len(), 1);
        let preview = &screen.entity_previews[0];
        assert_eq!(preview.lighting, GuiItemLightingEntry::EntityInUi);
        assert_eq!(
            preview.rect,
            HudEntityPreviewRect {
                x: 26,
                y: 8,
                width: 49,
                height: 70,
            }
        );
        assert_eq!(
            preview.visible_bounds(),
            Some(HudEntityPreviewRect {
                x: 30,
                y: 20,
                width: 10,
                height: 12,
            })
        );
        assert_eq!(preview.translation, [0.0, 0.875, 0.0]);
        assert_eq!(preview.rotation, [0.0, 0.0, 1.0, 0.0]);
        assert_eq!(preview.scale, 30.0);
        assert!(preview.depth_isolated);
        assert_eq!(
            preview.entity.render_state.light_coords,
            ENTITY_FULL_BRIGHT_LIGHT_COORDS
        );
        assert_eq!(preview.entity.render_state.outline_color, 0);
        assert!(!preview.entity.render_state.appears_glowing);
        assert_eq!(
            preview.item_layers,
            vec![HudEntityPreviewItemLayer {
                slot: HudEntityPreviewItemSlot::LeftHand,
                display_context: HudEntityPreviewItemDisplayContext::ThirdPersonLeftHand,
                item_id: 12,
                count: 1,
                foil: true,
                light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
                overlay: ITEM_MODEL_NO_OVERLAY,
                order: 0,
                submit_sequence: 1,
            }]
        );
    }

    #[test]
    fn sanitize_inventory_drops_invalid_entity_in_ui_previews() {
        let base = hud_entity_preview_for_test();
        let screen =
            sanitize_hud_inventory_screen(hud_inventory_screen_with_entity_previews(vec![
                HudEntityPreview {
                    lighting: GuiItemLightingEntry::Items3d,
                    ..base.clone()
                },
                HudEntityPreview {
                    depth_isolated: false,
                    ..base.clone()
                },
                HudEntityPreview {
                    rect: HudEntityPreviewRect {
                        width: 0,
                        ..base.rect
                    },
                    ..base.clone()
                },
                HudEntityPreview {
                    scissor: Some(HudEntityPreviewRect {
                        x: 200,
                        y: 200,
                        width: 10,
                        height: 10,
                    }),
                    ..base.clone()
                },
                HudEntityPreview {
                    translation: [0.0, f32::NAN, 0.0],
                    ..base.clone()
                },
                HudEntityPreview {
                    override_camera_rotation: Some([0.0, 0.0, f32::INFINITY, 1.0]),
                    ..base.clone()
                },
                HudEntityPreview { scale: 0.0, ..base },
            ]));

        assert!(screen.entity_previews.is_empty());
    }

    #[test]
    fn sanitize_entity_preview_allows_items_flat_only_for_gui_signs() {
        let base = hud_entity_preview_for_test();
        assert!(sanitize_hud_entity_preview(HudEntityPreview {
            lighting: GuiItemLightingEntry::ItemsFlat,
            ..base.clone()
        })
        .is_none());

        let gui_sign = HudEntityPreview {
            entity: EntityModelInstance::sign(
                -1,
                [0.0, 0.0, 0.0],
                0.0,
                SignModelWood::Oak,
                SignModelAttachment::Standing,
            ),
            lighting: GuiItemLightingEntry::ItemsFlat,
            rect: HudEntityPreviewRect {
                x: 0,
                y: 0,
                width: 96,
                height: 102,
            },
            scissor: None,
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            override_camera_rotation: None,
            scale: 62.500_004,
            depth_isolated: true,
            item_layers: Vec::new(),
        };
        let sanitized = sanitize_hud_entity_preview(gui_sign).expect("gui sign preview");
        assert_eq!(sanitized.lighting, GuiItemLightingEntry::ItemsFlat);
        assert_eq!(
            sanitized.entity.render_state.light_coords,
            ENTITY_FULL_BRIGHT_LIGHT_COORDS
        );
    }

    #[test]
    fn sanitize_sign_editor_screen_clamps_text_state_and_drops_hanging_preview() {
        let sign_preview = HudEntityPreview {
            entity: EntityModelInstance::sign(
                -1,
                [0.0, 0.0, 0.0],
                0.0,
                SignModelWood::Oak,
                SignModelAttachment::Standing,
            ),
            lighting: GuiItemLightingEntry::ItemsFlat,
            rect: HudEntityPreviewRect {
                x: 0,
                y: 0,
                width: 96,
                height: 102,
            },
            scissor: None,
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            override_camera_rotation: None,
            scale: 62.500_004,
            depth_isolated: true,
            item_layers: Vec::new(),
        };
        let standing = sanitize_hud_sign_editor_screen(HudSignEditorScreen {
            kind: HudSignEditorKind::Standing {
                wood: SignModelWood::Oak,
                attachment: SignModelAttachment::Standing,
            },
            sign_preview: Some(sign_preview.clone()),
            title: "Edit Sign Message".to_string(),
            lines: [
                String::new(),
                "front".to_string(),
                "third".to_string(),
                "fourth".to_string(),
            ],
            line: 7,
            cursor: 99,
            selection: 98,
            cursor_visible: true,
            text_tint: [2.0, 0.5, -1.0, 1.5],
        })
        .expect("standing sign editor");
        assert_eq!(standing.line, 3);
        assert_eq!(standing.cursor, "fourth".chars().count());
        assert_eq!(standing.selection, "fourth".chars().count());
        assert_eq!(standing.lines[0], "");
        assert_eq!(standing.text_tint, [1.0, 0.5, 0.0, 1.0]);
        assert!(standing.sign_preview.is_some());

        let hanging = sanitize_hud_sign_editor_screen(HudSignEditorScreen {
            kind: HudSignEditorKind::Hanging {
                wood: SignModelWood::Bamboo,
            },
            sign_preview: Some(sign_preview),
            title: "Edit Hanging Sign Message".to_string(),
            lines: std::array::from_fn(|_| String::new()),
            line: 0,
            cursor: 0,
            selection: 0,
            cursor_visible: true,
            text_tint: [1.0, 1.0, 1.0, 1.0],
        })
        .expect("hanging sign editor");
        assert!(hanging.sign_preview.is_none());
    }

    #[test]
    fn sanitize_inventory_keeps_renderable_block_models_and_drops_empty_ones() {
        let quad = crate::item_models::ItemModelQuad {
            corners: [[0.0, 0.0, 0.0]; 4],
            uvs: [[0.0, 0.0]; 4],
            tint: [1.0, 1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            shade: 1.0,
            translucent: false,
        };
        let model = |quads: Vec<crate::item_models::ItemModelQuad>| HudBlockItemModel {
            quads,
            gui_display: glam::Mat4::IDENTITY,
            lighting: GuiItemLightingEntry::Items3d,
            foil: false,
        };

        // A slot whose block model has geometry keeps it; one with no quads drops it (None).
        let kept = sanitize_hud_inventory_slot(HudInventorySlot {
            slot_id: 1,
            x: 0,
            y: 0,
            icon: None,
            block_model: Some(model(vec![quad])),
        });
        assert!(kept.block_model.is_some());

        let dropped = sanitize_hud_inventory_slot(HudInventorySlot {
            slot_id: 2,
            x: 0,
            y: 0,
            icon: None,
            block_model: Some(model(Vec::new())),
        });
        assert!(dropped.block_model.is_none());

        let wrong_lighting = sanitize_hud_inventory_slot(HudInventorySlot {
            slot_id: 3,
            x: 0,
            y: 0,
            icon: None,
            block_model: Some(HudBlockItemModel {
                quads: vec![quad],
                gui_display: glam::Mat4::IDENTITY,
                lighting: GuiItemLightingEntry::ItemsFlat,
                foil: false,
            }),
        });
        assert!(wrong_lighting.block_model.is_none());

        // The same filtering applies to floating (cursor / preview) items.
        let floating = sanitize_hud_inventory_item(HudInventoryItem {
            x: 0,
            y: 0,
            scale: 1.0,
            scale_y: 1.0,
            icon: HudItemIcon::single(HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            }),
            draw_decorations: true,
            block_model: Some(model(vec![quad])),
        })
        .unwrap();
        assert!(floating.block_model.is_some());
    }
}
