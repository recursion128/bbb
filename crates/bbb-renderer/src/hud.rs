use anyhow::Result;
use winit::dpi::PhysicalSize;

use crate::entity_models::{EntityModelInstance, ENTITY_FULL_BRIGHT_LIGHT_COORDS};
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
    centered_hud_rect, experience_bar_hud_rect, food_hud_rect, gui_item_slot_placement,
    heart_hud_rect, hotbar_hud_rect, hotbar_item_hud_rect, hotbar_selection_hud_rect,
    hud_experience_progress_width, hud_food_fill, hud_heart_fill, hud_inventory_text_label_origin,
    hud_inventory_tooltip_background_hud_rect, hud_inventory_tooltip_line_origin,
    hud_inventory_tooltip_sprite_segments, hud_inventory_tooltip_text_height,
    hud_item_cooldown_rect, hud_item_count_digit_hud_rect, hud_item_durability_bar_rect,
    hud_quad_vertices, hud_styled_quad_vertices, inventory_background_hud_rect,
    inventory_slot_highlight_hud_rect, inventory_slot_item_hud_rect, HudIconFill, HudRect,
    HudTooltipSpriteLayer, HUD_FOOD_ICONS_PER_ROW, HUD_HEARTS_PER_ROW,
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

const HUD_TINT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const HUD_TEXT_SHADOW_TINT: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
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
    pub icon: HudItemIcon,
    /// The item's 3D block-item model (vanilla 3D inventory icon), when it is a block. See
    /// [`HudInventorySlot::block_model`].
    pub block_model: Option<HudBlockItemModel>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryTextBackground {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub tint: [f32; 4],
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
    /// Slots for the currently open inventory container.
    pub slots: Vec<HudInventorySlot>,
    /// Item icons drawn by the inventory screen that are not container slots.
    pub floating_items: Vec<HudInventoryItem>,
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

    pub fn upload_hud_heart_container(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_heart_container = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_full(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_heart_full = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_half(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_heart_half = Some(self.upload_hud_sprite(width, height, rgba)?);
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

    pub fn set_hud_health(&mut self, health: Option<f32>) {
        self.hud_health = health.filter(|health| health.is_finite());
    }

    pub fn set_hud_food(&mut self, food: Option<i32>) {
        self.hud_food = food;
    }

    pub fn set_hud_experience_progress(&mut self, progress: Option<f32>) {
        self.hud_experience_progress_value = progress
            .filter(|progress| progress.is_finite())
            .map(|progress| progress.clamp(0.0, 1.0));
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

    pub fn clear_hud_inventory_screen(&mut self) {
        self.hud_inventory_screen = None;
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
            let mut append = |model: &HudBlockItemModel, x: i32, y: i32| {
                let placement = gui_item_slot_placement(inventory_slot_item_hud_rect(
                    surface_size,
                    screen.width,
                    screen.height,
                    x,
                    y,
                ));
                append_model(model, placement);
            };
            for slot in &screen.slots {
                if let Some(model) = &slot.block_model {
                    append(model, slot.x, slot.y);
                }
            }
            for item in &screen.floating_items {
                if let Some(model) = &item.block_model {
                    append(model, item.x, item.y);
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

        if let (Some(progress), Some(background)) = (
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

            let progress_width = hud_experience_progress_width(progress);
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

        if let (Some(health), Some(container)) = (self.hud_health, &self.hud_heart_container) {
            for index in 0..HUD_HEARTS_PER_ROW {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    container,
                    surface_size,
                    heart_hud_rect(surface_size, index, container.width, container.height),
                );
            }

            for index in 0..HUD_HEARTS_PER_ROW {
                let sprite = match hud_heart_fill(health, index) {
                    HudIconFill::Empty => None,
                    HudIconFill::Half => self.hud_heart_half.as_ref(),
                    HudIconFill::Full => self.hud_heart_full.as_ref(),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        heart_hud_rect(surface_size, index, sprite.width, sprite.height),
                    );
                }
            }
        }

        if let (Some(food), Some(empty)) = (self.hud_food, &self.hud_food_empty) {
            for index in 0..HUD_FOOD_ICONS_PER_ROW {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    empty,
                    surface_size,
                    food_hud_rect(surface_size, index, empty.width, empty.height),
                );
            }

            for index in 0..HUD_FOOD_ICONS_PER_ROW {
                let sprite = match hud_food_fill(food, index) {
                    HudIconFill::Empty => None,
                    HudIconFill::Half => self.hud_food_half.as_ref(),
                    HudIconFill::Full => self.hud_food_full.as_ref(),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        food_hud_rect(surface_size, index, sprite.width, sprite.height),
                    );
                }
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

            if let Some(atlas) = &self.hud_item_atlas {
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
                for item in &screen.floating_items {
                    let item_rect = inventory_slot_item_hud_rect(
                        surface_size,
                        screen.width,
                        screen.height,
                        item.x,
                        item.y,
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
                        &item.icon,
                        item.block_model.is_none(),
                        item.block_model.is_none(),
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
                            true,
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
/// when its cell/advance still fits.
///
/// Italic runs are sheared through the locked `styled_quads` primitive (top
/// edge `1-0.25*up`, bottom `1-0.25*down`). Obfuscated (`§k`) non-space glyphs
/// draw a random equal-advance substitute (`FontSet.getRandomGlyph`) picked
/// from `obfuscated_pool`; `obfuscated_seed` (the render frame counter) seeds a
/// deterministic per-pass LCG so the jitter is reproducible and the shadow pass
/// picks the same substitutes as the main pass. The pen advance always follows
/// the original glyph, so obfuscation and italic never shift layout.
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
            let x = origin.0 + pen_x as f32 + shadow_offset;
            let y = origin.1 + shadow_offset;
            if glyph.width > 0
                && glyph.height > 0
                && width_limit.is_none_or(|limit| pen_x.saturating_add(glyph.width) <= limit)
            {
                for quad in glyph.styled_quads(x, y, run.style, false) {
                    geometry.glyph_quads.push((quad, tint));
                }
            }
            // Underline/strikethrough bars follow the advance, unaffected by the
            // obfuscated bitmap swap, so they read the original glyph.
            if width_limit.is_none_or(|limit| pen_x.saturating_add(advance) <= limit) {
                for rect in base_glyph.styled_effect_rects(x, y, run.style, first_in_line) {
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

fn sanitize_hud_inventory_background_layer(
    layer: HudInventoryBackgroundLayer,
) -> Option<HudInventoryBackgroundLayer> {
    let uv = sanitize_hud_uv_rect(layer.uv)?;
    (layer.width > 0 && layer.height > 0).then_some(HudInventoryBackgroundLayer { uv, ..layer })
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
    Some(HudInventoryItem {
        x: item.x,
        y: item.y,
        icon: sanitize_hud_item_icon(item.icon)?,
        block_model: item.block_model.filter(hud_block_item_model_is_renderable),
    })
}

/// A 3D block-item icon is only worth carrying when it has geometry to draw.
fn hud_block_item_model_is_renderable(model: &HudBlockItemModel) -> bool {
    model.lighting == GuiItemLightingEntry::Items3d && !model.quads.is_empty()
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
    if preview.lighting != GuiItemLightingEntry::EntityInUi || !preview.depth_isolated {
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
    let text = sanitize_hud_text_line(label.text)?;
    let runs = sanitize_hud_styled_runs(label.runs, &text);
    (width > 0).then_some(HudInventoryTextLabel {
        x,
        y,
        width: width.min(512),
        text,
        tint: tint.map(|component| component.clamp(0.0, 1.0)),
        background,
        shadow: label.shadow,
        runs,
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

    /// End-to-end GPU proof of the HUD 3D block-item consumer: bakes a block item's quad at hotbar slot
    /// 4's pixel rect (via the real [`gui_item_slot_placement`]), renders it through the actual item-model
    /// pipeline under the GUI ortho camera, reads the framebuffer back, and asserts the slot center shows
    /// the block (non-background) while a far corner stays background. Skips (no assertion) when no GPU
    /// adapter is available, so it never fails the suite on adapter-less machines.
    #[test]
    fn hud_block_item_renders_visible_pixels_in_its_slot() {
        use wgpu::util::DeviceExt;

        use crate::camera::{CameraUniform, LightmapEnvironment};
        use crate::gpu::{
            create_camera_buffer, create_depth_target, create_terrain_atlas_gpu,
            create_terrain_bind_group, create_terrain_bind_group_layout,
        };
        use crate::item_models::{bake_item_model_mesh, create_item_model_pipeline, ItemModelQuad};
        use crate::lightmap::{
            create_lightmap_bind_group_layout, create_lightmap_gpu,
            create_lightmap_sample_bind_group_layout,
        };
        use glam::{Mat4, Vec3, Vec4};

        const WIDTH: u32 = 320;
        const HEIGHT: u32 = 240;
        // Non-sRGB target so the readback bytes are the shader's linear output verbatim (no color
        // conversion to reason about). `320 * 4 = 1280 = 5 * 256` so the copy needs no row padding.
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
                label: Some("bbb-hud-item-test-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )) else {
            return;
        };

        // A 1x1 opaque-red atlas: every UV samples red, so the baked quad is unambiguously visible.
        let atlas =
            create_terrain_atlas_gpu(&device, &queue, 1, 1, &[255, 0, 0, 255]).expect("1x1 atlas");
        let bind_group_layout = create_terrain_bind_group_layout(&device);
        let camera_buffer = create_camera_buffer(&device);
        queue.write_buffer(
            &camera_buffer,
            0,
            bytemuck::bytes_of(&CameraUniform::gui_ortho(WIDTH as f32, HEIGHT as f32)),
        );
        let bind_group =
            create_terrain_bind_group(&device, &bind_group_layout, &camera_buffer, &atlas);
        let lightmap_bind_group_layout = create_lightmap_bind_group_layout(&device);
        let lightmap_sample_bind_group_layout = create_lightmap_sample_bind_group_layout(&device);
        let lightmap = create_lightmap_gpu(
            &device,
            &queue,
            &lightmap_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            LightmapEnvironment::default(),
        );
        let pipeline = create_item_model_pipeline(
            &device,
            COLOR_FORMAT,
            &bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );

        // Bake one full-slot front-facing quad at hotbar slot 4, centered in the slot exactly as vanilla's
        // display transform centers the model (`gui_display = T(-0.5)`), so its pixels fill the slot rect.
        let surface_size = PhysicalSize::new(WIDTH, HEIGHT);
        let slot = 4;
        let placement = gui_item_slot_placement(hotbar_item_hud_rect(surface_size, slot));
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
        let mesh = bake_item_model_mesh(&[quad], placement * gui_display);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test-vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test-indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // The slot-center pixel (framebuffer col,row from top-left) is where `placement` seats the model
        // origin; pixel (0,0) is far from the bottom-centered hotbar, so it stays background.
        let center = placement * gui_display * Vec4::new(0.5, 0.5, 0.5, 1.0);
        let center_px = center.x.round() as u32;
        let center_py = center.y.round() as u32;
        assert!(
            center_px < WIDTH && center_py < HEIGHT,
            "slot center in bounds"
        );

        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test-color"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = create_depth_target(&device, WIDTH, HEIGHT);

        let bytes_per_row = WIDTH * 4;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test-readback"),
            size: (bytes_per_row * HEIGHT) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("test-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Blue background — distinct from the red block icon.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_bind_group(1, &lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
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
        let center_pixel = pixel(center_px, center_py);
        let corner_pixel = pixel(0, 0);

        // The slot center shows the red block icon (R high, B low); the far corner stays blue background.
        assert!(
            center_pixel[0] > 128 && center_pixel[2] < 128,
            "slot center should show the block item, got {center_pixel:?}"
        );
        assert!(
            corner_pixel[2] > 128 && corner_pixel[0] < 128,
            "corner should stay background, got {corner_pixel:?}"
        );

        drop(data);
        readback.unmap();
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
        );
        assert_eq!(geometry.glyph_quads.len(), 1);
        assert_eq!(geometry.glyph_quads[0].0.corners[0], [0.0, 0.0]);
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
    fn sanitize_hud_inventory_screen_keeps_sanitized_floating_items() {
        let screen = sanitize_hud_inventory_screen(HudInventoryScreen {
            width: 176,
            height: 166,
            background_layers: Vec::new(),
            slots: Vec::new(),
            floating_items: vec![
                HudInventoryItem {
                    x: 33,
                    y: 19,
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
                    block_model: None,
                },
                HudInventoryItem {
                    x: 51,
                    y: 19,
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
                    block_model: None,
                },
            ],
            entity_previews: Vec::new(),
            hovered_slot_id: None,
            tooltip: None,
            text_labels: Vec::new(),
        });

        assert_eq!(screen.floating_items.len(), 1);
        assert_eq!(screen.floating_items[0].x, 33);
        assert_eq!(screen.floating_items[0].y, 19);
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
            slots: Vec::new(),
            floating_items: Vec::new(),
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
            icon: HudItemIcon::single(HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            }),
            block_model: Some(model(vec![quad])),
        })
        .unwrap();
        assert!(floating.block_model.is_some());
    }
}
