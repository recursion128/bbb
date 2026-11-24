use super::*;

pub const HUD_HOTBAR_SLOTS: usize = 9;

/// Vanilla `Options.textBackgroundOpacity` default
/// (`options.accessibility.text_background_opacity`).
pub const VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY: f32 = 0.5;

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

/// GUI sprite scaling selected by `assets/.../gui/sprites/**.png.mcmeta`, matching vanilla
/// `GuiSpriteScaling`. Custom tooltip background and frame sprites can independently use any of
/// the three variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudGuiSpriteScaling {
    Stretch,
    Tile { width: u32, height: u32 },
    NineSlice(HudNineSliceScaling),
}

/// One frame's `Gui.extractSelectedItemName` state: the
/// styled hover-name runs of `lastToolHighlight`, the post-tick
/// `toolHighlightTimer`, and whether `gameMode.canHurtPlayer()` is false (which
/// pushes the line 14 pixels down over the missing health row).
#[derive(Debug, Clone, PartialEq)]
pub struct HudSelectedItemName {
    pub runs: Vec<HudComponentTextRun>,
    pub highlight_ticks: i32,
    pub raised: bool,
}

/// One frame's action-bar overlay message state (vanilla
/// `Gui.extractOverlayMessage` inputs): the styled line, the post-tick
/// `overlayMessageTime`, the frame partial tick, and the jukebox rainbow flag
/// (`Gui.animateOverlayMessageColor`). The renderer derives fade alpha and
/// position per frame; it keeps no countdown state of its own.
#[derive(Debug, Clone, PartialEq)]
pub struct HudActionBarText {
    pub runs: Vec<HudComponentTextRun>,
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
    pub title_runs: Vec<HudComponentTextRun>,
    pub subtitle_runs: Vec<HudComponentTextRun>,
    /// Vanilla `titleTime` after `Gui.tick` (starts at fade_in+stay+fade_out).
    pub remaining_ticks: i32,
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
    pub partial_tick: f32,
}

/// Vanilla's default `AttackIndicatorStatus.CROSSHAIR` presentation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum HudCrosshairAttackIndicator {
    #[default]
    None,
    Full,
    Progress {
        strength_scale: f32,
    },
}

/// Vanilla `AttackIndicatorStatus.HOTBAR` presentation
/// (`Gui.extractItemHotbar`): the indicator is drawn beside
/// the hotbar only while the attack strength is still charging, so unlike the
/// crosshair variant it has no full-strength frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum HudHotbarAttackIndicator {
    #[default]
    None,
    Progress {
        strength_scale: f32,
        main_arm_left: bool,
    },
}

/// Vanilla `Gui.extractConfusionOverlay`: the nausea texture blown up about
/// the screen centre and blended additively at a strength-scaled green tint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudConfusionOverlay {
    /// `nauseaIntensity * (1 - screenEffectScale)`, the only input vanilla
    /// passes in.
    pub strength: f32,
}

impl HudConfusionOverlay {
    /// `Mth.lerp(strength, 2.0F, 1.0F)` — a weak effect covers twice the
    /// screen, a full one exactly fills it.
    pub fn scale(self) -> f32 {
        2.0 - self.strength
    }

    /// `ARGB.colorFromFloat(1.0F, 0.2F * strength, 0.4F * strength, 0.2F * strength)`.
    pub fn tint(self) -> [f32; 4] {
        [
            0.2 * self.strength,
            0.4 * self.strength,
            0.2 * self.strength,
            1.0,
        ]
    }
}

/// Vanilla `Gui.extractPortalOverlay`: the nether-portal particle sprite blit
/// over the whole GUI at the eased portal intensity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudPortalOverlay {
    /// Atlas-absolute UV span of
    /// `getParticleMaterial(Blocks.NETHER_PORTAL.defaultBlockState()).sprite()`.
    pub sprite_uv: HudUvRect,
    /// `Player.portalEffectIntensity` interpolated by the partial tick, before
    /// the vanilla easing.
    pub intensity: f32,
}

impl HudPortalOverlay {
    /// Vanilla `Gui.extractPortalOverlay`: a partial
    /// intensity is raised to the fourth power and then remapped into
    /// `[0.2, 1.0]`, so the overlay never fades below a fifth of its opacity
    /// while the portal effect is active.
    pub fn alpha(self) -> f32 {
        if self.intensity >= 1.0 {
            return 1.0;
        }
        let squared = self.intensity * self.intensity;
        squared * squared * 0.8 + 0.2
    }
}

/// One `Gui.extractTextureOverlay` call: a full-screen blit of a standalone
/// texture at `ARGB.white(alpha)`. Vanilla reaches it for an equipped item's
/// `Equippable.cameraOverlay` and for the powder-snow freeze outline.
#[derive(Debug, Clone, PartialEq)]
pub struct HudCameraOverlay {
    pub texture_id: String,
    pub alpha: f32,
}

/// Vanilla `Gui.extractItemHotbar`'s offhand slot: drawn only when the offhand
/// stack is non-empty, on the side opposite `Player.getMainArm()`.
#[derive(Debug, Clone, PartialEq)]
pub struct HudOffhandSlot {
    pub on_left: bool,
    pub icon: HudItemIcon,
}

/// One frame's food-bar effect inputs (vanilla `Gui.extractFood`):
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

/// One canonical local-player mob effect projected for vanilla HUD and
/// inventory presentation. Ordering and geometry remain renderer-owned;
/// native resolves catalog category/color plus localized text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudMobEffect {
    pub effect_id: i32,
    pub beneficial: bool,
    pub color: u32,
    pub amplifier: i32,
    pub duration_ticks: i32,
    pub ambient: bool,
    pub show_icon: bool,
    pub display_name: String,
    pub duration_text: String,
}

/// One frame's air-bubble row inputs (vanilla `Gui.extractAirBubbles`,
/// Gui): the synced air supply, the fixed max, the
/// `isEyeInFluid(FluidTags.WATER)` gate, and the client tick for the all-empty
/// wobble cadence. The renderer derives visibility and the per-bubble
/// full/popping/empty split per frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudAirSupply {
    /// Vanilla `player.getAirSupply()` (the synced `DATA_AIR_SUPPLY_ID` int).
    pub air: i32,
    /// Vanilla `player.getMaxAirSupply()` (300 for players).
    pub max_air: i32,
    /// Vanilla `player.isEyeInFluid(FluidTags.WATER)`.
    pub eye_in_water: bool,
    /// Client tick counter (vanilla `Gui.tickCount`) gating the empty-row wobble.
    pub tick_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudAirBubblePop {
    /// Vanilla's one-based `airBubble` loop index.
    pub bubble: i32,
    /// `emptyAirBubbles`, used by the sound volume and pitch formulas.
    pub empty_bubbles: i32,
}

/// One frame's vehicle-health inputs (vanilla `Gui.extractVehicleHealth` /
/// `getVehicleMaxHearts`, Gui): the living vehicle's
/// synced health and resolved MAX_HEALTH attribute value. `None` (no living
/// vehicle) keeps the food row; a non-zero heart count replaces it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudVehicleHealth {
    /// Vanilla `vehicle.getHealth()`; the draw ceils it.
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

/// The single contextual information bar selected by
/// `Gui.nextContextualInfoState`. Experience-level text is deliberately
/// separate because Vanilla extracts it independently.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum HudContextualBar {
    #[default]
    Empty,
    Experience {
        progress: f32,
    },
    Locator(HudLocatorBar),
    Jump(HudJumpBar),
}

/// Vanilla `Gui.HeartType`: the heart sprite family a
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
    pub const fn sprite_name(self, hardcore: bool, half: bool, blinking: bool) -> &'static str {
        match (self, hardcore, half, blinking) {
            (Self::Container, false, _, false) => "container",
            (Self::Container, false, _, true) => "container_blinking",
            (Self::Container, true, _, false) => "container_hardcore",
            (Self::Container, true, _, true) => "container_hardcore_blinking",

            (Self::Normal, false, false, false) => "full",
            (Self::Normal, false, false, true) => "full_blinking",
            (Self::Normal, false, true, false) => "half",
            (Self::Normal, false, true, true) => "half_blinking",
            (Self::Normal, true, false, false) => "hardcore_full",
            (Self::Normal, true, false, true) => "hardcore_full_blinking",
            (Self::Normal, true, true, false) => "hardcore_half",
            (Self::Normal, true, true, true) => "hardcore_half_blinking",

            (Self::Poisoned, false, false, false) => "poisoned_full",
            (Self::Poisoned, false, false, true) => "poisoned_full_blinking",
            (Self::Poisoned, false, true, false) => "poisoned_half",
            (Self::Poisoned, false, true, true) => "poisoned_half_blinking",
            (Self::Poisoned, true, false, false) => "poisoned_hardcore_full",
            (Self::Poisoned, true, false, true) => "poisoned_hardcore_full_blinking",
            (Self::Poisoned, true, true, false) => "poisoned_hardcore_half",
            (Self::Poisoned, true, true, true) => "poisoned_hardcore_half_blinking",

            (Self::Withered, false, false, false) => "withered_full",
            (Self::Withered, false, false, true) => "withered_full_blinking",
            (Self::Withered, false, true, false) => "withered_half",
            (Self::Withered, false, true, true) => "withered_half_blinking",
            (Self::Withered, true, false, false) => "withered_hardcore_full",
            (Self::Withered, true, false, true) => "withered_hardcore_full_blinking",
            (Self::Withered, true, true, false) => "withered_hardcore_half",
            (Self::Withered, true, true, true) => "withered_hardcore_half_blinking",

            (Self::Absorbing, false, false, false) => "absorbing_full",
            (Self::Absorbing, false, false, true) => "absorbing_full_blinking",
            (Self::Absorbing, false, true, false) => "absorbing_half",
            (Self::Absorbing, false, true, true) => "absorbing_half_blinking",
            (Self::Absorbing, true, false, false) => "absorbing_hardcore_full",
            (Self::Absorbing, true, false, true) => "absorbing_hardcore_full_blinking",
            (Self::Absorbing, true, true, false) => "absorbing_hardcore_half",
            (Self::Absorbing, true, true, true) => "absorbing_hardcore_half_blinking",

            (Self::Frozen, false, false, false) => "frozen_full",
            (Self::Frozen, false, false, true) => "frozen_full_blinking",
            (Self::Frozen, false, true, false) => "frozen_half",
            (Self::Frozen, false, true, true) => "frozen_half_blinking",
            (Self::Frozen, true, false, false) => "frozen_hardcore_full",
            (Self::Frozen, true, false, true) => "frozen_hardcore_full_blinking",
            (Self::Frozen, true, true, false) => "frozen_hardcore_half",
            (Self::Frozen, true, true, true) => "frozen_hardcore_half_blinking",
        }
    }
}

/// One frame's player-health inputs (vanilla `Gui.extractPlayerHealth` /
/// `extractHearts`, Gui): the synced current health, native-held
/// displayed health and blink phase, resolved MAX_HEALTH attribute, absorption
/// amount, base `HeartType`, hardcore flag, Regeneration wave gate, and client
/// tick. The renderer derives the ordered heart instances and row geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudPlayerHealth {
    /// Vanilla `player.getHealth()`; the draw ceils it.
    pub health: f32,
    /// Vanilla `Gui.displayHealth`, held for one wall-clock second after a
    /// damage/heal edge and drawn as the old blinking fill.
    pub display_health: i32,
    /// The frame phase derived from Vanilla `Gui.healthBlinkTime`.
    pub blinking: bool,
    /// Vanilla `player.getAttributeValue(Attributes.MAX_HEALTH)`.
    pub max_health: f32,
    /// Vanilla `player.getAbsorptionAmount()`; the draw ceils it.
    pub absorption: f32,
    /// Vanilla `HeartType.forPlayer(player)` base fill: one of
    /// `Normal`/`Poisoned`/`Withered`/`Frozen`.
    pub heart_type: HudHeartKind,
    /// Vanilla `player.level().getLevelData().isHardcore()`.
    pub hardcore: bool,
    /// Vanilla `player.hasEffect(MobEffects.REGENERATION)` —
    /// arms the per-heart lift wave.
    pub regen: bool,
    /// Client tick counter (vanilla `Gui.tickCount`) driving the regen wave
    /// index and the low-health shake seed.
    pub tick_count: u64,
}

/// Vanilla `Gui.extractSleepOverlay` full-screen fill. The RGB is fixed to
/// `0x101020`; native supplies the integer alpha derived from
/// `Player.sleepCounter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSleepOverlay {
    pub alpha: u8,
}

/// Vanilla `Gui.scopeScale` presentation value for the current first-person
/// spyglass frame. The renderer combines it with the selected-pack scope image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudSpyglassScope {
    pub scale: f32,
}

/// Vanilla `BossEvent.BossBarColor`: selects the
/// `boss_bar/{name}_background` / `boss_bar/{name}_progress` sprite pair.
/// Declaration order is the vanilla ordinal (the sprite-array index,
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

/// Vanilla `BossEvent.BossBarOverlay`: `Progress`
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
    /// BossHealthOverlay); `Progress` has no notched sheet.
    pub(super) fn notched_index(self) -> Option<usize> {
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

pub(super) const HUD_TINT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub(super) const HUD_MOB_EFFECT_OVERLAY_SPACING: i32 = 25;

pub(super) const HUD_MOB_EFFECT_OVERLAY_HARMFUL_Y: i32 = 27;

pub(super) const HUD_MOB_EFFECT_BACKGROUND_SIZE: u32 = 24;

pub(super) const HUD_MOB_EFFECT_ICON_SIZE: u32 = 18;

pub(super) const HUD_TEXT_SHADOW_TINT: [f32; 4] = [63.0 / 255.0, 63.0 / 255.0, 63.0 / 255.0, 1.0];

pub(super) const HUD_SOUND_SUBTITLE_BACKGROUND_TINT: [f32; 4] = [0.0, 0.0, 0.0, 0.8];

pub(super) const HUD_WIDGET_TEXT_LINE_HEIGHT: i32 = 9;

/// Which food icon a draw needs, so `hud_food_variant_sprite` can pick the base
/// or the Hunger-effect variant of that shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudFoodSprite {
    Empty,
    Half,
    Full,
}

/// Vanilla `ContextualBarRenderer.extractExperienceLevel` outline color
/// `-16777216` (0xFF000000 opaque black), drawn at the four axis offsets.
const HUD_EXPERIENCE_LEVEL_OUTLINE_TINT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// Vanilla `ContextualBarRenderer.extractExperienceLevel` center color
/// `-8323296` (0xFF80FF20 green), drawn last on top of the black outline.
const HUD_EXPERIENCE_LEVEL_FILL_TINT: [f32; 4] = [128.0 / 255.0, 1.0, 32.0 / 255.0, 1.0];

/// The five `extractExperienceLevel` passes in vanilla draw order:
/// four black `(±1)/(0,±1)` copies, then
/// the green center — each `dropShadow = false`.
pub(super) const HUD_EXPERIENCE_LEVEL_PASSES: [(f32, f32, [f32; 4]); 5] = [
    (1.0, 0.0, HUD_EXPERIENCE_LEVEL_OUTLINE_TINT),
    (-1.0, 0.0, HUD_EXPERIENCE_LEVEL_OUTLINE_TINT),
    (0.0, 1.0, HUD_EXPERIENCE_LEVEL_OUTLINE_TINT),
    (0.0, -1.0, HUD_EXPERIENCE_LEVEL_OUTLINE_TINT),
    (0.0, 0.0, HUD_EXPERIENCE_LEVEL_FILL_TINT),
];

pub(super) const HUD_ITEM_BAR_BACKGROUND_TINT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

pub(super) const HUD_ITEM_BAR_BACKGROUND_WIDTH: u32 = 13;

pub(super) const HUD_ITEM_BAR_BACKGROUND_HEIGHT: u32 = 2;

pub(super) const HUD_ITEM_BAR_FOREGROUND_HEIGHT: u32 = 1;

pub(super) const HUD_ITEM_COOLDOWN_TINT: [f32; 4] = [1.0, 1.0, 1.0, 127.0 / 255.0];

pub(super) const HUD_ITEM_SPECIAL_FOIL_GUI_SCALE: f32 = 0.5;

pub(super) const HUD_ITEM_SPECIAL_FOIL_TEXTURE_SCALE: f32 = 1.0 / 128.0;

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
    /// Vanilla `ItemStack.getPopTime()` minus this frame's partial tick. While
    /// positive, `Gui.extractSlot` squeezes the item quad; decorations are
    /// never posed.
    pub pop: f32,
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
            pop: 0.0,
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

/// Fixed Vanilla 26.1 sprite catalog consumed by the Smithing screen's three
/// `CyclingSlotBackground` instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HudSmithingSlotIcon {
    TemplateArmorTrim,
    TemplateNetheriteUpgrade,
    Helmet,
    Chestplate,
    Leggings,
    Boots,
    Hoe,
    Axe,
    Sword,
    Shovel,
    Spear,
    Pickaxe,
    Ingot,
    RedstoneDust,
    Quartz,
    Emerald,
    Diamond,
    LapisLazuli,
    AmethystShard,
    NautilusArmor,
}

impl HudSmithingSlotIcon {
    pub const ALL: [Self; 20] = [
        Self::TemplateArmorTrim,
        Self::TemplateNetheriteUpgrade,
        Self::Helmet,
        Self::Chestplate,
        Self::Leggings,
        Self::Boots,
        Self::Hoe,
        Self::Axe,
        Self::Sword,
        Self::Shovel,
        Self::Spear,
        Self::Pickaxe,
        Self::Ingot,
        Self::RedstoneDust,
        Self::Quartz,
        Self::Emerald,
        Self::Diamond,
        Self::LapisLazuli,
        Self::AmethystShard,
        Self::NautilusArmor,
    ];
    pub const COUNT: usize = Self::ALL.len();

    pub const fn sprite_path(self) -> &'static str {
        match self {
            Self::TemplateArmorTrim => "container/slot/smithing_template_armor_trim",
            Self::TemplateNetheriteUpgrade => "container/slot/smithing_template_netherite_upgrade",
            Self::Helmet => "container/slot/helmet",
            Self::Chestplate => "container/slot/chestplate",
            Self::Leggings => "container/slot/leggings",
            Self::Boots => "container/slot/boots",
            Self::Hoe => "container/slot/hoe",
            Self::Axe => "container/slot/axe",
            Self::Sword => "container/slot/sword",
            Self::Shovel => "container/slot/shovel",
            Self::Spear => "container/slot/spear",
            Self::Pickaxe => "container/slot/pickaxe",
            Self::Ingot => "container/slot/ingot",
            Self::RedstoneDust => "container/slot/redstone_dust",
            Self::Quartz => "container/slot/quartz",
            Self::Emerald => "container/slot/emerald",
            Self::Diamond => "container/slot/diamond",
            Self::LapisLazuli => "container/slot/lapis_lazuli",
            Self::AmethystShard => "container/slot/amethyst_shard",
            Self::NautilusArmor => "container/slot/nautilus_armor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudEntityPreviewRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl HudEntityPreviewRect {
    pub(super) fn right(self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    pub(super) fn bottom(self) -> i64 {
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
/// order. `item_layers` identifies hand/head item submissions resolved through resident item-model
/// owners and drawn into that same private target.
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
    /// Optional background-layer index after which the PIP blit is submitted.
    /// `None` preserves the standard after-all-background behavior.
    pub after_background_layer: Option<usize>,
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

pub(super) const MAX_HUD_INVENTORY_MAP_PREVIEWS: usize = 2;

pub(super) const MAX_HUD_INVENTORY_BANNER_PATTERN_PREVIEWS: usize = 16;

pub(crate) enum HudDrawCommand<'a> {
    Fill {
        start: u32,
        end: u32,
    },
    TextHighlight {
        invert_start: u32,
        additive_start: u32,
        end: u32,
    },
    Scissor {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Sprite {
        sprite: &'a HudSpriteGpu,
        start: u32,
        end: u32,
    },
    InvertingSprite {
        sprite: &'a HudSpriteGpu,
        start: u32,
        end: u32,
    },
    /// One Loom banner-pattern crop sampled directly from a resident entity
    /// texture atlas through the ordinary HUD sprite pipeline.
    BannerPattern {
        atlas: HudBannerPatternAtlas,
        start: u32,
        end: u32,
    },
    ItemGlint {
        mask: &'a HudSpriteGpu,
        start: u32,
        end: u32,
    },
    /// The additive `RenderPipelines.GUI_NAUSEA_OVERLAY` blit.
    NauseaOverlay {
        sprite: &'a HudSpriteGpu,
        start: u32,
        end: u32,
    },
    /// A HUD quad sampled straight from the resident blocks atlas, for
    /// `Gui.extractPortalOverlay`'s `blitSprite` of the nether-portal particle
    /// sprite (which keeps animating with the rest of the atlas).
    TerrainAtlas {
        start: u32,
        end: u32,
    },
    /// Complete dynamic map stack (pixels, decorations, and names) for one
    /// sanitized cartography preview. The renderer resolves this index against
    /// its resident GUI map targets.
    MapPreview {
        preview_index: u8,
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
    /// Vanilla F3+4 lightmap preview: blits the renderer-owned level lightmap
    /// texture into the bottom-right HUD corner after debug text.
    LightmapPreview {
        start: u32,
        end: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HudBannerPatternAtlas {
    StaticEntity,
    DynamicEntity,
}

pub(crate) struct HudDraws<'a> {
    pub(crate) vertices: Vec<HudVertex>,
    pub(crate) commands: Vec<HudDrawCommand<'a>>,
    pub(crate) post_gui_item_start: usize,
    pub(crate) screen_blur: Option<HudScreenBlurDraws>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudItemIconDrawStep {
    Layers,
    Glint,
    DurabilityBar,
    Cooldown,
    CountLabel,
}

impl HudDraws<'_> {
    pub(crate) fn project_vertices(&mut self, layout: PhysicalSize<u32>, extent: [f32; 2]) {
        let scale_x = layout.width.max(1) as f32 / extent[0];
        let scale_y = layout.height.max(1) as f32 / extent[1];
        // Layout-generated NDC crosses into the physical/scale projection once, before upload.
        for vertex in &mut self.vertices {
            vertex.position[0] = (vertex.position[0] + 1.0) * scale_x - 1.0;
            vertex.position[1] = (vertex.position[1] - 1.0) * scale_y + 1.0;
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct HudVertex {
    pub(super) position: [f32; 2],
    pub(super) uv: [f32; 2],
    pub(super) tint: [f32; 4],
    pub(super) local_uv: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudMobEffectOverlayInstance {
    pub(super) effect_id: i32,
    pub(super) ambient: bool,
    pub(super) background: HudRect,
    pub(super) icon: HudRect,
    pub(super) icon_alpha: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudMultilineLine {
    pub(super) begin: usize,
    pub(super) end: usize,
}

/// Which 182x5 sheet one bar layer samples (vanilla
/// `BAR_{BACKGROUND,PROGRESS}_SPRITES` / `OVERLAY_*_SPRITES`,
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudBossBarSheet {
    ColorBackground(HudBossBarColor),
    NotchedBackground(HudBossBarOverlay),
    ColorProgress(HudBossBarColor),
    NotchedProgress(HudBossBarOverlay),
}

/// One sprite layer of a bar: the sheet and the drawn width — 182 for
/// backgrounds, the discrete fill width for progress layers; the draw crops
/// the rect and UV to the left `width / 182` band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudBossBarLayer {
    pub(super) sheet: HudBossBarSheet,
    pub(super) width: u32,
}

/// One bar's resolved draw plan for this frame: the surviving stack row, the
/// sprite layers in vanilla submission order, and the centered name line.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct HudBossBarDraw<'a> {
    pub(super) y: i32,
    pub(super) layers: Vec<HudBossBarLayer>,
    pub(super) name: HudScreenTextDraw<'a>,
}

/// Glyphs in draw order, followed by underline/strikethrough effects. The
/// prepared bounds and glyph click areas are the vanilla `PreparedText` facts
/// the same pen walk produces; they stay empty unless the caller asks for them
/// (the hit geometry builder in `text_layout`), so per-frame draws pay nothing.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct HudStyledTextGeometry {
    pub(super) glyph_quads: Vec<(HudGlyphQuad, [f32; 4])>,
    pub(super) effect_rects: Vec<(HudEffectRect, [f32; 4])>,
    /// `PreparedText.bounds()` in font space, before the pose translation.
    pub(super) text_bounds: Option<[f32; 4]>,
    /// `acceptGlyph` order: glyphs with a bitmap and their interaction target.
    pub(super) glyph_areas: Vec<(HudGlyphArea, Option<usize>)>,
    /// `acceptEmptyArea` order, visited after every glyph instance.
    pub(super) empty_areas: Vec<(HudGlyphArea, Option<usize>)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudResolvedTextRun<'a> {
    pub(super) visual: &'a HudStyledTextRun,
    pub(super) glyphs: &'a HudFontGlyphMap,
    pub(super) obfuscated_pool: &'a HudObfuscatedGlyphPool,
    /// Component font id, which is part of vanilla `Style` and therefore of a
    /// `FormattedCharSequence` part boundary; `None` on the styled-run paths.
    pub(super) font: Option<&'a str>,
}

/// Vanilla `ItemStack.setPopTime(5)` is the only value the client ever writes,
/// so a projected pop can never exceed five ticks.
pub(super) const VANILLA_ITEM_POP_TIME_TICKS: f32 = 5.0;
