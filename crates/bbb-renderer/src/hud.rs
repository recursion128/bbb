use anyhow::Result;
use winit::dpi::PhysicalSize;

use crate::Renderer;

mod gpu;
mod layout;

pub(super) use self::gpu::{
    create_hud_bind_group_layout, create_hud_pipeline, create_hud_sprite_gpu, HudSpriteGpu,
};
use self::layout::{
    centered_hud_rect, experience_bar_hud_rect, food_hud_rect, heart_hud_rect, hotbar_hud_rect,
    hotbar_item_hud_rect, hotbar_selection_hud_rect, hud_experience_progress_width, hud_food_fill,
    hud_heart_fill, hud_item_cooldown_rect, hud_item_count_digit_hud_rect,
    hud_item_durability_bar_rect, hud_quad_vertices, inventory_background_hud_rect,
    inventory_slot_highlight_hud_rect, inventory_slot_item_hud_rect, HudIconFill, HudRect,
    HUD_FOOD_ICONS_PER_ROW, HUD_HEARTS_PER_ROW,
};

pub const HUD_HOTBAR_SLOTS: usize = 9;
const HUD_TINT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const HUD_TEXT_SHADOW_TINT: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
const HUD_ITEM_BAR_BACKGROUND_TINT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const HUD_ITEM_BAR_BACKGROUND_WIDTH: u32 = 13;
const HUD_ITEM_BAR_BACKGROUND_HEIGHT: u32 = 2;
const HUD_ITEM_BAR_FOREGROUND_HEIGHT: u32 = 1;
const HUD_ITEM_COOLDOWN_TINT: [f32; 4] = [1.0, 1.0, 1.0, 127.0 / 255.0];

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryItem {
    /// Item icon x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Item icon y position relative to the centered inventory screen origin.
    pub y: i32,
    pub icon: HudItemIcon,
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
    pub hovered_slot_id: Option<u16>,
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
                    );
                }
            }

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
) {
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
        hovered_slot_id: screen.hovered_slot_id,
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
    }
}

fn sanitize_hud_inventory_item(item: HudInventoryItem) -> Option<HudInventoryItem> {
    Some(HudInventoryItem {
        x: item.x,
        y: item.y,
        icon: sanitize_hud_item_icon(item.icon)?,
    })
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
                },
                HudInventorySlot {
                    slot_id: 7,
                    x: 44,
                    y: 84,
                    icon: None,
                },
            ],
            floating_items: Vec::new(),
            hovered_slot_id: Some(7),
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
                },
            ],
            hovered_slot_id: None,
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
}
