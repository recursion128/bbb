use anyhow::Result;
use winit::dpi::PhysicalSize;

use crate::item_models::HudBlockItemModel;
use crate::Renderer;

mod gpu;
mod layout;

pub(super) use self::gpu::{
    create_hud_bind_group_layout, create_hud_pipeline, create_hud_sprite_gpu, HudSpriteGpu,
};
use self::layout::{
    centered_hud_rect, experience_bar_hud_rect, food_hud_rect, gui_item_slot_placement,
    heart_hud_rect, hotbar_hud_rect, hotbar_item_hud_rect, hotbar_selection_hud_rect,
    hud_experience_progress_width, hud_food_fill, hud_heart_fill,
    hud_inventory_text_label_glyph_hud_rect, hud_inventory_tooltip_background_hud_rect,
    hud_inventory_tooltip_text_height, hud_inventory_tooltip_text_hud_rect, hud_item_cooldown_rect,
    hud_item_count_digit_hud_rect, hud_item_durability_bar_rect, hud_quad_vertices,
    inventory_background_hud_rect, inventory_slot_highlight_hud_rect, inventory_slot_item_hud_rect,
    HudIconFill, HudRect, HUD_FOOD_ICONS_PER_ROW, HUD_HEARTS_PER_ROW,
};

pub const HUD_HOTBAR_SLOTS: usize = 9;
pub const HUD_ASCII_FIRST_GLYPH: u8 = b' ';
pub const HUD_ASCII_LAST_GLYPH: u8 = b'~';
pub const HUD_ASCII_GLYPH_COUNT: usize =
    (HUD_ASCII_LAST_GLYPH - HUD_ASCII_FIRST_GLYPH + 1) as usize;
const HUD_TINT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const HUD_TEXT_SHADOW_TINT: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
const HUD_ITEM_BAR_BACKGROUND_TINT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const HUD_ITEM_BAR_BACKGROUND_WIDTH: u32 = 13;
const HUD_ITEM_BAR_BACKGROUND_HEIGHT: u32 = 2;
const HUD_ITEM_BAR_FOREGROUND_HEIGHT: u32 = 1;
const HUD_ITEM_COOLDOWN_TINT: [f32; 4] = [1.0, 1.0, 1.0, 127.0 / 255.0];
const HUD_TOOLTIP_BACKGROUND_TINT: [f32; 4] = [0.0625, 0.0, 0.0625, 0.94];
const HUD_ASCII_REPLACEMENT_GLYPH: char = '?';

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

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
    pub layers: Vec<HudIconLayer>,
    pub count_label: Option<HudItemCountLabel>,
    pub durability_bar: Option<HudItemDurabilityBar>,
    pub cooldown_progress: Option<f32>,
}

impl HudItemIcon {
    pub fn single(uv: HudUvRect) -> Self {
        Self {
            layers: vec![HudIconLayer::new(uv, HUD_TINT_WHITE)],
            count_label: None,
            durability_bar: None,
            cooldown_progress: None,
        }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudDigitGlyph {
    pub uv: HudUvRect,
    pub width: u32,
    pub height: u32,
    pub advance: u32,
}

impl Default for HudDigitGlyph {
    fn default() -> Self {
        Self {
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            width: 0,
            height: 0,
            advance: 0,
        }
    }
}

pub type HudAsciiGlyph = HudDigitGlyph;

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryTooltipLine {
    pub text: String,
    pub tint: [f32; 4],
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
    pub text_labels: Vec<HudInventoryTextLabel>,
    pub hovered_slot_id: Option<u16>,
    pub tooltip: Option<HudInventoryTooltip>,
}

pub(super) struct HudDrawCommand<'a> {
    pub(super) sprite: &'a HudSpriteGpu,
    pub(super) start: u32,
    pub(super) end: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct HudVertex {
    position: [f32; 2],
    uv: [f32; 2],
    tint: [f32; 4],
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

    pub fn upload_hud_ascii_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        glyphs: [HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT],
    ) -> Result<()> {
        self.hud_ascii_atlas = Some(self.upload_hud_sprite(width, height, rgba)?);
        self.hud_ascii_glyphs = glyphs;
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

    /// Bakes this frame's hotbar 3D block items into one item-model mesh (in GUI pixel space): each slot's
    /// block quads under its slot placement (`translate(slot_center)·scale(slot_px,-slot_px,slot_px)`)
    /// composed with the item's `gui` display transform. The GUI ortho camera projects it in the GUI item
    /// pass. Empty when no hotbar slot holds a 3D block item.
    pub(crate) fn collect_hud_block_item_mesh(&self) -> crate::item_models::ItemModelMesh {
        let surface_size = self.surface_size();
        let mut mesh = crate::item_models::ItemModelMesh::new();
        for (slot, model) in self.hud_hotbar_block_item_models.iter().enumerate() {
            if let Some(model) = model {
                let placement = gui_item_slot_placement(hotbar_item_hud_rect(surface_size, slot));
                mesh.append_quads(&model.quads, placement * model.gui_display);
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
                mesh.append_quads(&model.quads, placement * model.gui_display);
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
        mesh
    }

    pub(super) fn collect_hud_draws(&self) -> (Vec<HudVertex>, Vec<HudDrawCommand<'_>>) {
        let mut vertices = Vec::new();
        let mut commands = Vec::new();
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
                        renders_as_3d_block,
                    );
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
                            slot.block_model.is_some(),
                        );
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
                        item.block_model.is_some(),
                    );
                }
            }

            push_hud_inventory_text_labels(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.hud_ascii_atlas.as_ref(),
                &self.hud_ascii_glyphs,
                surface_size,
                screen,
            );

            if let (Some(slot), Some(highlight)) = (hovered_slot, &self.hud_slot_highlight_front) {
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

            push_hud_inventory_tooltip(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.hud_ascii_atlas.as_ref(),
                &self.hud_ascii_glyphs,
                surface_size,
                screen,
            );
        }

        if let Some(overlay) = &self.hud_code_of_conduct_overlay {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                overlay,
                surface_size,
                centered_hud_rect(surface_size, overlay.width, overlay.height),
            );
        }

        (vertices, commands)
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
    commands.push(HudDrawCommand { sprite, start, end });
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
    // flat block-texture stand-in that the 3D icon replaces — skip them, but keep the count / durability
    // / cooldown overlays, which the 3D pass does not draw.
    skip_layers: bool,
) {
    if !skip_layers {
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
    push_hud_item_durability_bar(
        vertices,
        commands,
        white_pixel,
        surface_size,
        item_rect,
        icon.durability_bar.as_ref(),
    );
    push_hud_item_cooldown(
        vertices,
        commands,
        white_pixel,
        surface_size,
        item_rect,
        icon.cooldown_progress,
    );
    push_hud_item_count_label(
        vertices,
        commands,
        digit_atlas,
        glyphs,
        surface_size,
        item_rect,
        icon.count_label.as_ref(),
    );
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

fn push_hud_inventory_text_labels<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    ascii_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT],
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
) {
    let Some(ascii_atlas) = ascii_atlas else {
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
        for (shadow_offset, tint) in label
            .shadow
            .then_some((1.0, HUD_TEXT_SHADOW_TINT))
            .into_iter()
            .chain(std::iter::once((0.0, label.tint)))
        {
            let mut pen_x = 0u32;
            for ch in label.text.chars() {
                let glyph = hud_ascii_glyph(ch, glyphs);
                if pen_x >= label.width {
                    break;
                }
                if glyph.width > 0
                    && glyph.height > 0
                    && pen_x.saturating_add(glyph.width) <= label.width
                {
                    push_hud_draw_with_uv_and_tint(
                        vertices,
                        commands,
                        ascii_atlas,
                        surface_size,
                        hud_inventory_text_label_glyph_hud_rect(
                            surface_size,
                            screen.width,
                            screen.height,
                            label.x,
                            label.y,
                            pen_x,
                            shadow_offset,
                            glyph,
                        ),
                        glyph.uv,
                        tint,
                    );
                }
                pen_x = pen_x.saturating_add(glyph.advance);
            }
        }
    }
}

fn push_hud_inventory_tooltip<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    ascii_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT],
    surface_size: PhysicalSize<u32>,
    screen: &HudInventoryScreen,
) {
    let (Some(ascii_atlas), Some(tooltip)) = (ascii_atlas, screen.tooltip.as_ref()) else {
        return;
    };
    let Some(text_height) = hud_inventory_tooltip_text_height(tooltip.lines.len()) else {
        return;
    };
    let Some(text_width) = tooltip
        .lines
        .iter()
        .filter_map(|line| hud_ascii_text_width(&line.text, glyphs))
        .max()
    else {
        return;
    };

    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        white_pixel,
        surface_size,
        hud_inventory_tooltip_background_hud_rect(
            surface_size,
            screen.width,
            screen.height,
            tooltip.x,
            tooltip.y,
            text_width,
            text_height,
        ),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        HUD_TOOLTIP_BACKGROUND_TINT,
    );

    for shadow_offset in [1.0, 0.0] {
        for (line_index, line) in tooltip.lines.iter().enumerate() {
            let tint = if shadow_offset > 0.0 {
                HUD_TEXT_SHADOW_TINT
            } else {
                line.tint
            };
            let mut pen_x = 0;
            for ch in line.text.chars() {
                let glyph = hud_ascii_glyph(ch, glyphs);
                if glyph.width > 0 && glyph.height > 0 {
                    push_hud_draw_with_uv_and_tint(
                        vertices,
                        commands,
                        ascii_atlas,
                        surface_size,
                        hud_inventory_tooltip_text_hud_rect(
                            surface_size,
                            screen.width,
                            screen.height,
                            tooltip.x,
                            tooltip.y,
                            text_width,
                            text_height,
                            line_index,
                            pen_x,
                            shadow_offset,
                            glyph,
                        ),
                        glyph.uv,
                        tint,
                    );
                }
                pen_x = pen_x.saturating_add(glyph.advance);
            }
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

fn hud_ascii_text_width(
    text: &str,
    glyphs: &[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT],
) -> Option<u32> {
    let mut width = 0u32;
    for ch in text.chars() {
        width = width.checked_add(hud_ascii_glyph(ch, glyphs).advance)?;
    }
    (width > 0).then_some(width)
}

fn hud_ascii_glyph(ch: char, glyphs: &[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT]) -> HudAsciiGlyph {
    let byte = if ch.is_ascii() {
        ch as u8
    } else {
        HUD_ASCII_REPLACEMENT_GLYPH as u8
    };
    let byte = if (HUD_ASCII_FIRST_GLYPH..=HUD_ASCII_LAST_GLYPH).contains(&byte) {
        byte
    } else {
        HUD_ASCII_REPLACEMENT_GLYPH as u8
    };
    glyphs[(byte - HUD_ASCII_FIRST_GLYPH) as usize]
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
    !model.quads.is_empty()
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
    (width > 0).then_some(HudInventoryTextLabel {
        x,
        y,
        width: width.min(512),
        text,
        tint: tint.map(|component| component.clamp(0.0, 1.0)),
        background,
        shadow: label.shadow,
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
    Some(HudInventoryTooltipLine {
        text: sanitize_hud_text_line(line.text)?,
        tint: line.tint.map(|component| component.clamp(0.0, 1.0)),
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

fn sanitize_hud_item_icon(icon: HudItemIcon) -> Option<HudItemIcon> {
    let layers = icon
        .layers
        .into_iter()
        .filter_map(sanitize_hud_icon_layer)
        .collect::<Vec<_>>();
    (!layers.is_empty()).then_some(HudItemIcon {
        layers,
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
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, 0.25],
                        max: [1.0, 0.75],
                    },
                    HUD_TINT_WHITE,
                )],
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
            layers: vec![first, second],
            count_label: Some(HudItemCountLabel::new("64")),
            durability_bar: Some(HudItemDurabilityBar::new(99, [-1.0, 0.5, 1.5])),
            cooldown_progress: Some(1.5),
        })
        .expect("valid icon layers should remain");

        assert_eq!(icon.layers.len(), 2);
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
    fn sanitize_hud_item_icon_discards_invalid_layers() {
        let icon = sanitize_hud_item_icon(HudItemIcon {
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
            count_label: Some(HudItemCountLabel::new("1x")),
            durability_bar: Some(HudItemDurabilityBar::new(10, [1.0, f32::NAN, 0.0])),
            cooldown_progress: Some(f32::NAN),
        })
        .expect("one valid layer should remain");

        assert_eq!(icon.layers.len(), 1);
        assert_eq!(icon.count_label, None);
        assert_eq!(icon.durability_bar, None);
        assert_eq!(icon.cooldown_progress, None);
        assert_eq!(icon.layers[0].uv.min, [0.25, 0.25]);
        assert_eq!(icon.layers[0].uv.max, [0.75, 0.75]);

        assert_eq!(
            sanitize_hud_item_icon(HudItemIcon {
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [f32::NAN, 0.0],
                        max: [1.0, 1.0],
                    },
                    [1.0, 1.0, 1.0, 1.0],
                )],
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
    fn hud_ascii_text_width_uses_printable_ascii_with_replacement_fallback() {
        let mut glyphs = [HudAsciiGlyph::default(); HUD_ASCII_GLYPH_COUNT];
        glyphs[(b'A' - HUD_ASCII_FIRST_GLYPH) as usize].advance = 6;
        glyphs[(b' ' - HUD_ASCII_FIRST_GLYPH) as usize].advance = 4;
        glyphs[(b'?' - HUD_ASCII_FIRST_GLYPH) as usize].advance = 5;

        assert_eq!(hud_ascii_text_width("A A", &glyphs), Some(16));
        assert_eq!(hud_ascii_text_width("A\u{0007}", &glyphs), Some(11));
        assert_eq!(hud_ascii_text_width("钻", &glyphs), Some(5));
        assert_eq!(hud_ascii_text_width("", &glyphs), None);
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
                        layers: vec![HudIconLayer::new(
                            HudUvRect {
                                min: [1.25, 0.75],
                                max: [-0.5, 0.25],
                            },
                            [1.5, 0.25, -1.0, 1.0],
                        )],
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
                        layers: vec![HudIconLayer::new(
                            HudUvRect {
                                min: [0.0, f32::NAN],
                                max: [1.0, 1.0],
                            },
                            [1.0, 1.0, 1.0, 1.0],
                        )],
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
                },
                HudInventoryTextLabel {
                    x: 10,
                    y: 10,
                    width: 0,
                    text: "ignored".to_string(),
                    tint: HUD_TINT_WHITE,
                    background: None,
                    shadow: true,
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
                    },
                    HudInventoryTooltipLine {
                        text: String::new(),
                        tint: HUD_TINT_WHITE,
                    },
                    HudInventoryTooltipLine {
                        text: "Attack\u{0007}Damage".to_string(),
                        tint: [0.25, 0.5, 0.75, 2.0],
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
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, 0.25],
                        max: [1.0, 0.75],
                    },
                    [1.0, 0.25, 0.0, 1.0],
                )],
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
                    },
                    HudInventoryTooltipLine {
                        text: "AttackDamage".to_string(),
                        tint: [0.25, 0.5, 0.75, 1.0],
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
                        layers: vec![HudIconLayer::new(
                            HudUvRect {
                                min: [1.25, 0.75],
                                max: [-0.5, 0.25],
                            },
                            [1.25, 0.5, -1.0, 1.0],
                        )],
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
                        layers: vec![HudIconLayer::new(
                            HudUvRect {
                                min: [0.0, f32::NAN],
                                max: [1.0, 1.0],
                            },
                            [1.0, 1.0, 1.0, 1.0],
                        )],
                        count_label: Some(HudItemCountLabel::new("64")),
                        durability_bar: None,
                        cooldown_progress: None,
                    },
                    block_model: None,
                },
            ],
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
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, 0.25],
                        max: [1.0, 0.75],
                    },
                    [1.0, 0.5, 0.0, 1.0],
                )],
                count_label: Some(HudItemCountLabel::new("12")),
                durability_bar: Some(HudItemDurabilityBar::new(13, [0.25, 1.0, 0.0])),
                cooldown_progress: Some(1.0),
            }
        );
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
