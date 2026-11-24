use std::borrow::Cow;

use bbb_protocol::mob_effect_types::VANILLA_BEACON_EFFECT_TIERS;

use super::{
    menu_screens_26_1::{vanilla_menu_screen_kind, VanillaMenuScreenKind},
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InventoryScreenBackground {
    LocalInventory,
    Generic9xRows {
        rows: u8,
    },
    Generic3x3,
    Anvil,
    Beacon,
    BlastFurnace,
    BrewingStand,
    CartographyTable,
    CraftingTable,
    Crafter,
    EnchantmentTable,
    Furnace,
    Grindstone,
    Hopper,
    Mount {
        kind: MountInventoryKind,
        inventory_columns: u8,
    },
    Lectern,
    Loom,
    Merchant,
    ShulkerBox,
    Smithing,
    Smoker,
    Stonecutter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InventoryScreenLayout {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) background: InventoryScreenBackground,
    pub(crate) recipe_book: Option<InventoryRecipeBookLayout>,
    pub(crate) slots: Cow<'static, [InventorySlotLayout]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InventoryRecipeBookLayout {
    pub(crate) x: i32,
    pub(crate) main_gui_x_offset: i32,
    pub(crate) narrow: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InventorySlotLayout {
    pub(crate) slot_id: i16,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

/// Vanilla `RecipeBookComponent.updateScreenPosition` seats the main 176px GUI
/// 149px to the right of the recipe-book panel in the non-narrow case
/// (`imageWidth == 176`): recipe book origin `(width - 147) / 2 - 86`,
/// main GUI origin `177 + (width - imageWidth - 200) / 2`.
pub(crate) const RECIPE_BOOK_MAIN_GUI_X_OFFSET: i32 = 149;
/// Virtual width whose centered origin matches the vanilla non-narrow
/// recipe-book origin for even GUI widths.
const RECIPE_BOOK_ORIGIN_WIDTH: i32 = 320;
const RECIPE_BOOK_NARROW_SCREEN_WIDTH: u32 = 379;
const RECIPE_BOOK_WIDTH: i32 = 147;
pub(crate) const RECIPE_BOOK_BUTTON_WIDTH: i32 = 20;
pub(crate) const RECIPE_BOOK_BUTTON_HEIGHT: i32 = 18;
pub(crate) const RECIPE_BOOK_FILTER_BUTTON_X: i32 = 110;
pub(crate) const RECIPE_BOOK_FILTER_BUTTON_Y: i32 = 12;
pub(crate) const RECIPE_BOOK_FILTER_BUTTON_WIDTH: i32 = 26;
pub(crate) const RECIPE_BOOK_FILTER_BUTTON_HEIGHT: i32 = 16;
pub(crate) const RECIPE_BOOK_SEARCH_BOX_X: i32 = 25;
pub(crate) const RECIPE_BOOK_SEARCH_BOX_Y: i32 = 13;
pub(crate) const RECIPE_BOOK_SEARCH_BOX_WIDTH: i32 = 81;
pub(crate) const RECIPE_BOOK_SEARCH_BOX_HEIGHT: i32 = 14;
pub(crate) const RECIPE_BOOK_SEARCH_TEXT_X_OFFSET: i32 = 4;
pub(crate) const RECIPE_BOOK_SEARCH_TEXT_Y_OFFSET: i32 = 3;
pub(crate) const RECIPE_BOOK_TAB_X: i32 = -30;
pub(crate) const RECIPE_BOOK_TAB_Y: i32 = 3;
pub(crate) const RECIPE_BOOK_TAB_WIDTH: i32 = 35;
pub(crate) const RECIPE_BOOK_TAB_HEIGHT: i32 = 27;
pub(crate) const RECIPE_BOOK_TAB_STRIDE_Y: i32 = 27;
pub(crate) const RECIPE_BOOK_SELECTED_TAB_X_OFFSET: i32 = -2;
pub(crate) const RECIPE_BOOK_PAGE_FORWARD_BUTTON_X: i32 = 93;
pub(crate) const RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X: i32 = 38;
pub(crate) const RECIPE_BOOK_PAGE_BUTTON_Y: i32 = 137;
pub(crate) const RECIPE_BOOK_PAGE_BUTTON_WIDTH: i32 = 12;
pub(crate) const RECIPE_BOOK_PAGE_BUTTON_HEIGHT: i32 = 17;
pub(crate) const RECIPE_BOOK_RECIPE_BUTTON_X: i32 = 11;
pub(crate) const RECIPE_BOOK_RECIPE_BUTTON_Y: i32 = 31;
pub(crate) const RECIPE_BOOK_RECIPE_BUTTON_SIZE: i32 = 25;
pub(crate) const RECIPE_BOOK_RECIPE_BUTTON_COLUMNS: usize = 5;

// LoomScreen and StonecutterScreen use these same coordinates for input hit
// testing and render extraction. Keep the intentional click/drag/sprite/item
// offsets explicit: vanilla does not use one shared Y coordinate for every
// interaction surface.
pub(crate) const LOOM_PATTERN_BUTTON_X: i32 = 60;
pub(crate) const LOOM_PATTERN_BUTTON_Y: i32 = 13;
pub(crate) const LOOM_PATTERN_BUTTON_COLUMNS: i32 = 4;
pub(crate) const LOOM_PATTERN_BUTTON_ROWS: i32 = 4;
pub(crate) const LOOM_PATTERN_BUTTON_SIZE: i32 = 14;
pub(crate) const LOOM_SCROLLER_X: i32 = 119;
pub(crate) const LOOM_SCROLLER_CLICK_Y: i32 = 9;
pub(crate) const LOOM_SCROLLER_Y: i32 = 13;
pub(crate) const LOOM_SCROLLER_DRAG_Y: i32 = LOOM_SCROLLER_Y;
pub(crate) const LOOM_SCROLLER_WIDTH: i32 = 12;
pub(crate) const LOOM_SCROLLER_HEIGHT: i32 = 15;
pub(crate) const LOOM_SCROLLER_FULL_HEIGHT: i32 = 56;
pub(crate) const LOOM_SCROLLER_MAX_OFFSET: i32 = 41;

pub(crate) const STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT: usize = 12;
pub(crate) const STONECUTTER_RECIPE_BUTTON_X: i32 = 52;
pub(crate) const STONECUTTER_RECIPE_BUTTON_Y: i32 = 14;
pub(crate) const STONECUTTER_RECIPE_BUTTON_SPRITE_Y: i32 = STONECUTTER_RECIPE_BUTTON_Y + 1;
pub(crate) const STONECUTTER_RECIPE_ITEM_Y: i32 = STONECUTTER_RECIPE_BUTTON_Y + 2;
pub(crate) const STONECUTTER_RECIPE_BUTTON_COLUMNS: i32 = 4;
pub(crate) const STONECUTTER_RECIPE_BUTTON_ROWS: i32 = 3;
pub(crate) const STONECUTTER_RECIPE_BUTTON_WIDTH: i32 = 16;
pub(crate) const STONECUTTER_RECIPE_BUTTON_HEIGHT: i32 = 18;
pub(crate) const STONECUTTER_SCROLLER_X: i32 = 119;
pub(crate) const STONECUTTER_SCROLLER_CLICK_Y: i32 = 9;
pub(crate) const STONECUTTER_SCROLLER_DRAG_Y: i32 = 14;
pub(crate) const STONECUTTER_SCROLLER_Y: i32 = 15;
pub(crate) const STONECUTTER_SCROLLER_WIDTH: i32 = 12;
pub(crate) const STONECUTTER_SCROLLER_HEIGHT: i32 = 15;
pub(crate) const STONECUTTER_SCROLLER_FULL_HEIGHT: i32 = 54;
pub(crate) const STONECUTTER_SCROLLER_MAX_OFFSET: i32 = 41;

pub(crate) const BEACON_EFFECT_BUTTON_SIZE: u32 = 22;
const BEACON_EFFECT_BUTTON_SPACING: i32 = 24;
const BEACON_PRIMARY_EFFECT_CENTER_X: i32 = 76;
const BEACON_PRIMARY_EFFECT_Y: i32 = 22;
const BEACON_PRIMARY_EFFECT_ROW_SPACING: i32 = 25;
const BEACON_SECONDARY_EFFECT_CENTER_X: i32 = 167;
const BEACON_SECONDARY_EFFECT_Y: i32 = 47;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BeaconEffectButton {
    pub(crate) primary: bool,
    pub(crate) upgrade: bool,
    pub(crate) tier: i16,
    pub(crate) effect_id: i32,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

pub(crate) fn beacon_effect_buttons(upgrade_effect: Option<i32>) -> Vec<BeaconEffectButton> {
    let mut buttons = Vec::with_capacity(7);
    for (tier, effects) in VANILLA_BEACON_EFFECT_TIERS[..3].iter().enumerate() {
        let total_width = effects.len() as i32 * BEACON_EFFECT_BUTTON_SIZE as i32
            + (effects.len() as i32 - 1) * 2;
        for (column, effect_id) in effects.iter().enumerate() {
            buttons.push(BeaconEffectButton {
                primary: true,
                upgrade: false,
                tier: tier as i16,
                effect_id: *effect_id,
                x: BEACON_PRIMARY_EFFECT_CENTER_X + column as i32 * BEACON_EFFECT_BUTTON_SPACING
                    - total_width / 2,
                y: BEACON_PRIMARY_EFFECT_Y + tier as i32 * BEACON_PRIMARY_EFFECT_ROW_SPACING,
            });
        }
    }

    let secondary_effects = VANILLA_BEACON_EFFECT_TIERS[3];
    let count = secondary_effects.len() as i32 + 1;
    let total_width = count * BEACON_EFFECT_BUTTON_SIZE as i32 + (count - 1) * 2;
    for (column, effect_id) in secondary_effects.iter().enumerate() {
        buttons.push(BeaconEffectButton {
            primary: false,
            upgrade: false,
            tier: 3,
            effect_id: *effect_id,
            x: BEACON_SECONDARY_EFFECT_CENTER_X + column as i32 * BEACON_EFFECT_BUTTON_SPACING
                - total_width / 2,
            y: BEACON_SECONDARY_EFFECT_Y,
        });
    }
    if let Some(effect_id) = upgrade_effect {
        buttons.push(BeaconEffectButton {
            primary: false,
            upgrade: true,
            tier: 3,
            effect_id,
            x: BEACON_SECONDARY_EFFECT_CENTER_X
                + secondary_effects.len() as i32 * BEACON_EFFECT_BUTTON_SPACING
                - total_width / 2,
            y: BEACON_SECONDARY_EFFECT_Y,
        });
    }
    buttons
}

pub(crate) fn recipe_book_recipe_button_position(index: usize) -> (i32, i32) {
    let column = index % RECIPE_BOOK_RECIPE_BUTTON_COLUMNS;
    let row = index / RECIPE_BOOK_RECIPE_BUTTON_COLUMNS;
    (
        RECIPE_BOOK_RECIPE_BUTTON_X
            + i32::try_from(column).unwrap_or_default() * RECIPE_BOOK_RECIPE_BUTTON_SIZE,
        RECIPE_BOOK_RECIPE_BUTTON_Y
            + i32::try_from(row).unwrap_or_default() * RECIPE_BOOK_RECIPE_BUTTON_SIZE,
    )
}

pub(crate) fn loom_pattern_row_count(selectable_count: i32) -> i32 {
    if selectable_count <= 0 {
        0
    } else {
        (selectable_count + LOOM_PATTERN_BUTTON_COLUMNS - 1) / LOOM_PATTERN_BUTTON_COLUMNS
    }
}

pub(crate) fn local_inventory_slot_layouts() -> &'static [InventorySlotLayout] {
    LOCAL_INVENTORY_SLOTS
}

pub(crate) fn inventory_screen_layout(world: &WorldStore) -> Option<InventoryScreenLayout> {
    inventory_screen_layout_with_recipe_book_mode(world, false)
}

pub(crate) fn inventory_screen_layout_for_surface(
    world: &WorldStore,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<InventoryScreenLayout> {
    inventory_screen_layout_with_recipe_book_mode(
        world,
        surface_size.width < RECIPE_BOOK_NARROW_SCREEN_WIDTH,
    )
}

fn inventory_screen_layout_with_recipe_book_mode(
    world: &WorldStore,
    recipe_book_narrow: bool,
) -> Option<InventoryScreenLayout> {
    if world.local_inventory_is_open() {
        return Some(recipe_book_overlay_layout(
            world,
            InventoryScreenLayout {
                width: INVENTORY_SCREEN_WIDTH,
                height: INVENTORY_SCREEN_HEIGHT,
                background: InventoryScreenBackground::LocalInventory,
                recipe_book: None,
                slots: Cow::Borrowed(local_inventory_slot_layouts()),
            },
            recipe_book_narrow,
        ));
    }

    let container = world.inventory().open_container.as_ref()?;
    if let Some(mount) = container.mount {
        let kind = world.open_mount_inventory_kind()?;
        let inventory_columns = match kind {
            MountInventoryKind::Horse => clamped_mount_inventory_columns(mount.inventory_columns),
            MountInventoryKind::Nautilus => 0,
        };
        let equipment_slots = world.open_mount_equipment_slot_visibility()?;
        return Some(InventoryScreenLayout {
            width: MOUNT_SCREEN_WIDTH,
            height: MOUNT_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Mount {
                kind,
                inventory_columns,
            },
            recipe_book: None,
            slots: Cow::Owned(mount_inventory_slot_layouts(
                inventory_columns,
                equipment_slots,
            )),
        });
    }
    let menu_type_id = container.menu_type_id?;
    let screen_kind = vanilla_menu_screen_kind(menu_type_id)?;
    Some(match screen_kind {
        VanillaMenuScreenKind::Container => {
            let rows = generic_container_rows(menu_type_id)?;
            InventoryScreenLayout {
                width: GENERIC_CONTAINER_WIDTH,
                height: GENERIC_CONTAINER_BASE_HEIGHT
                    + i32::from(rows) * GENERIC_CONTAINER_ROW_HEIGHT,
                background: InventoryScreenBackground::Generic9xRows { rows },
                recipe_book: None,
                slots: Cow::Borrowed(generic_container_slot_layouts(rows)),
            }
        }
        VanillaMenuScreenKind::Dispenser => InventoryScreenLayout {
            width: GENERIC_3X3_SCREEN_WIDTH,
            height: GENERIC_3X3_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Generic3x3,
            recipe_book: None,
            slots: Cow::Borrowed(GENERIC_3X3_SLOTS),
        },
        VanillaMenuScreenKind::Crafter => InventoryScreenLayout {
            width: CRAFTER_SCREEN_WIDTH,
            height: CRAFTER_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Crafter,
            recipe_book: None,
            slots: Cow::Borrowed(CRAFTER_SLOTS),
        },
        VanillaMenuScreenKind::Crafting => recipe_book_overlay_layout(
            world,
            InventoryScreenLayout {
                width: CRAFTING_SCREEN_WIDTH,
                height: CRAFTING_SCREEN_HEIGHT,
                background: InventoryScreenBackground::CraftingTable,
                recipe_book: None,
                slots: Cow::Borrowed(CRAFTING_TABLE_SLOTS),
            },
            recipe_book_narrow,
        ),
        VanillaMenuScreenKind::Enchantment => InventoryScreenLayout {
            width: ENCHANTMENT_SCREEN_WIDTH,
            height: ENCHANTMENT_SCREEN_HEIGHT,
            background: InventoryScreenBackground::EnchantmentTable,
            recipe_book: None,
            slots: Cow::Borrowed(ENCHANTMENT_TABLE_SLOTS),
        },
        VanillaMenuScreenKind::Anvil => InventoryScreenLayout {
            width: ANVIL_SCREEN_WIDTH,
            height: ANVIL_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Anvil,
            recipe_book: None,
            slots: Cow::Borrowed(ANVIL_SLOTS),
        },
        VanillaMenuScreenKind::Beacon => InventoryScreenLayout {
            width: BEACON_SCREEN_WIDTH,
            height: BEACON_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Beacon,
            recipe_book: None,
            slots: Cow::Borrowed(BEACON_SLOTS),
        },
        VanillaMenuScreenKind::BrewingStand => InventoryScreenLayout {
            width: BREWING_STAND_SCREEN_WIDTH,
            height: BREWING_STAND_SCREEN_HEIGHT,
            background: InventoryScreenBackground::BrewingStand,
            recipe_book: None,
            slots: Cow::Borrowed(BREWING_STAND_SLOTS),
        },
        VanillaMenuScreenKind::BlastFurnace
        | VanillaMenuScreenKind::Furnace
        | VanillaMenuScreenKind::Smoker => recipe_book_overlay_layout(
            world,
            InventoryScreenLayout {
                width: FURNACE_SCREEN_WIDTH,
                height: FURNACE_SCREEN_HEIGHT,
                background: match screen_kind {
                    VanillaMenuScreenKind::BlastFurnace => InventoryScreenBackground::BlastFurnace,
                    VanillaMenuScreenKind::Furnace => InventoryScreenBackground::Furnace,
                    VanillaMenuScreenKind::Smoker => InventoryScreenBackground::Smoker,
                    _ => unreachable!("furnace-family arm excludes other screen kinds"),
                },
                recipe_book: None,
                slots: Cow::Borrowed(FURNACE_SLOTS),
            },
            recipe_book_narrow,
        ),
        VanillaMenuScreenKind::Grindstone => InventoryScreenLayout {
            width: GRINDSTONE_SCREEN_WIDTH,
            height: GRINDSTONE_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Grindstone,
            recipe_book: None,
            slots: Cow::Borrowed(GRINDSTONE_SLOTS),
        },
        VanillaMenuScreenKind::Hopper => InventoryScreenLayout {
            width: HOPPER_SCREEN_WIDTH,
            height: HOPPER_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Hopper,
            recipe_book: None,
            slots: Cow::Borrowed(HOPPER_SLOTS),
        },
        VanillaMenuScreenKind::Lectern => InventoryScreenLayout {
            width: LECTERN_SCREEN_WIDTH,
            height: LECTERN_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Lectern,
            recipe_book: None,
            slots: Cow::Borrowed(&[]),
        },
        VanillaMenuScreenKind::Loom => InventoryScreenLayout {
            width: LOOM_SCREEN_WIDTH,
            height: LOOM_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Loom,
            recipe_book: None,
            slots: Cow::Borrowed(LOOM_SLOTS),
        },
        VanillaMenuScreenKind::Merchant => InventoryScreenLayout {
            width: MERCHANT_SCREEN_WIDTH,
            height: MERCHANT_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Merchant,
            recipe_book: None,
            slots: Cow::Borrowed(MERCHANT_SLOTS),
        },
        VanillaMenuScreenKind::ShulkerBox => InventoryScreenLayout {
            width: SHULKER_BOX_SCREEN_WIDTH,
            height: SHULKER_BOX_SCREEN_HEIGHT,
            background: InventoryScreenBackground::ShulkerBox,
            recipe_book: None,
            slots: Cow::Borrowed(SHULKER_BOX_SLOTS),
        },
        VanillaMenuScreenKind::Smithing => InventoryScreenLayout {
            width: SMITHING_SCREEN_WIDTH,
            height: SMITHING_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Smithing,
            recipe_book: None,
            slots: Cow::Borrowed(SMITHING_SLOTS),
        },
        VanillaMenuScreenKind::CartographyTable => InventoryScreenLayout {
            width: CARTOGRAPHY_TABLE_SCREEN_WIDTH,
            height: CARTOGRAPHY_TABLE_SCREEN_HEIGHT,
            background: InventoryScreenBackground::CartographyTable,
            recipe_book: None,
            slots: Cow::Borrowed(CARTOGRAPHY_TABLE_SLOTS),
        },
        VanillaMenuScreenKind::Stonecutter => InventoryScreenLayout {
            width: STONECUTTER_SCREEN_WIDTH,
            height: STONECUTTER_SCREEN_HEIGHT,
            background: InventoryScreenBackground::Stonecutter,
            recipe_book: None,
            slots: Cow::Borrowed(STONECUTTER_SLOTS),
        },
    })
}

pub(crate) fn recipe_book_main_gui_offset(
    world: &WorldStore,
    background: InventoryScreenBackground,
) -> i32 {
    if recipe_book_is_open_for_background(world, background) {
        RECIPE_BOOK_MAIN_GUI_X_OFFSET
    } else {
        0
    }
}

pub(crate) fn recipe_book_button_position(
    background: InventoryScreenBackground,
) -> Option<(i32, i32)> {
    match background {
        InventoryScreenBackground::LocalInventory => Some((104, 61)),
        InventoryScreenBackground::CraftingTable => Some((5, 34)),
        InventoryScreenBackground::Furnace
        | InventoryScreenBackground::BlastFurnace
        | InventoryScreenBackground::Smoker => Some((20, 34)),
        _ => None,
    }
}

pub(crate) fn recipe_book_type_for_background(
    background: InventoryScreenBackground,
) -> Option<bbb_protocol::packets::RecipeBookType> {
    match background {
        InventoryScreenBackground::LocalInventory | InventoryScreenBackground::CraftingTable => {
            Some(bbb_protocol::packets::RecipeBookType::Crafting)
        }
        InventoryScreenBackground::Furnace => Some(bbb_protocol::packets::RecipeBookType::Furnace),
        InventoryScreenBackground::BlastFurnace => {
            Some(bbb_protocol::packets::RecipeBookType::BlastFurnace)
        }
        InventoryScreenBackground::Smoker => Some(bbb_protocol::packets::RecipeBookType::Smoker),
        _ => None,
    }
}

pub(crate) fn recipe_book_type_settings(
    world: &WorldStore,
    book_type: bbb_protocol::packets::RecipeBookType,
) -> bbb_protocol::packets::RecipeBookTypeSettings {
    let settings = &world.recipe_book().settings;
    match book_type {
        bbb_protocol::packets::RecipeBookType::Crafting => settings.crafting,
        bbb_protocol::packets::RecipeBookType::Furnace => settings.furnace,
        bbb_protocol::packets::RecipeBookType::BlastFurnace => settings.blast_furnace,
        bbb_protocol::packets::RecipeBookType::Smoker => settings.smoker,
    }
}

pub(crate) fn recipe_book_tab_count_for_background(
    background: InventoryScreenBackground,
) -> Option<usize> {
    match background {
        InventoryScreenBackground::LocalInventory | InventoryScreenBackground::CraftingTable => {
            Some(5)
        }
        InventoryScreenBackground::Furnace => Some(4),
        InventoryScreenBackground::BlastFurnace => Some(3),
        InventoryScreenBackground::Smoker => Some(2),
        _ => None,
    }
}

fn recipe_book_is_open_for_background(
    world: &WorldStore,
    background: InventoryScreenBackground,
) -> bool {
    recipe_book_type_for_background(background)
        .is_some_and(|book_type| recipe_book_type_settings(world, book_type).open)
}

fn recipe_book_overlay_layout(
    world: &WorldStore,
    mut layout: InventoryScreenLayout,
    narrow: bool,
) -> InventoryScreenLayout {
    if !recipe_book_is_open_for_background(world, layout.background) {
        return layout;
    }

    if narrow {
        layout.recipe_book = Some(InventoryRecipeBookLayout {
            x: (layout.width - RECIPE_BOOK_WIDTH) / 2,
            main_gui_x_offset: 0,
            narrow: true,
        });
    } else {
        layout.width = RECIPE_BOOK_ORIGIN_WIDTH;
        layout.slots = Cow::Borrowed(match layout.background {
            InventoryScreenBackground::LocalInventory => LOCAL_INVENTORY_BOOK_SLOTS,
            InventoryScreenBackground::CraftingTable => CRAFTING_TABLE_BOOK_SLOTS,
            InventoryScreenBackground::BlastFurnace
            | InventoryScreenBackground::Furnace
            | InventoryScreenBackground::Smoker => FURNACE_BOOK_SLOTS,
            _ => unreachable!("only recipe-book screens reach this branch"),
        });
        layout.recipe_book = Some(InventoryRecipeBookLayout {
            x: 0,
            main_gui_x_offset: RECIPE_BOOK_MAIN_GUI_X_OFFSET,
            narrow: false,
        });
    }
    layout
}

pub(crate) fn inventory_screen_selected_hotbar_slot_id(world: &WorldStore) -> Option<i16> {
    let selected_hotbar_slot = i16::from(world.local_player().selected_hotbar_slot.min(8));
    if world.local_inventory_is_open() {
        return Some(36 + selected_hotbar_slot);
    }

    let container = world.inventory().open_container.as_ref()?;
    if let Some(mount) = container.mount {
        let kind = world.open_mount_inventory_kind()?;
        let inventory_columns = match kind {
            MountInventoryKind::Horse => clamped_mount_inventory_columns(mount.inventory_columns),
            MountInventoryKind::Nautilus => 0,
        };
        let mount_inventory_slots =
            i16::try_from(i32::from(inventory_columns) * MOUNT_INVENTORY_ROWS).ok()?;
        return Some(
            MOUNT_EQUIPMENT_SLOT_COUNT + mount_inventory_slots + 27 + selected_hotbar_slot,
        );
    }

    inventory_screen_hotbar_start(container.menu_type_id?).map(|start| start + selected_hotbar_slot)
}

fn inventory_screen_hotbar_start(menu_type_id: i32) -> Option<i16> {
    if let Some(rows) = generic_container_rows(menu_type_id) {
        return Some(i16::from(rows) * GENERIC_CONTAINER_SLOT_COUNT_PER_ROW + 27);
    }

    match menu_type_id {
        VANILLA_MENU_TYPE_GENERIC_3X3_ID => Some(GENERIC_3X3_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_CRAFTER_3X3_ID => Some(CRAFTER_GRID_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_CRAFTING_ID => Some(CRAFTING_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_ENCHANTMENT_ID => Some(ENCHANTMENT_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_ANVIL_ID => Some(ANVIL_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_BEACON_ID => Some(BEACON_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_BREWING_STAND_ID => Some(BREWING_STAND_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID
        | VANILLA_MENU_TYPE_FURNACE_ID
        | VANILLA_MENU_TYPE_SMOKER_ID => Some(FURNACE_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_GRINDSTONE_ID => Some(GRINDSTONE_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_HOPPER_ID => Some(HOPPER_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_LOOM_ID => Some(LOOM_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_MERCHANT_ID => Some(MERCHANT_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_SHULKER_BOX_ID => Some(SHULKER_BOX_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_SMITHING_ID => Some(SMITHING_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID => Some(CARTOGRAPHY_TABLE_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_STONECUTTER_ID => Some(STONECUTTER_SLOT_COUNT + 27),
        VANILLA_MENU_TYPE_LECTERN_ID => None,
        _ => None,
    }
}

fn generic_container_rows(menu_type_id: i32) -> Option<u8> {
    (VANILLA_MENU_TYPE_GENERIC_9X1_ID..=VANILLA_MENU_TYPE_GENERIC_9X6_ID)
        .contains(&menu_type_id)
        .then(|| (menu_type_id - VANILLA_MENU_TYPE_GENERIC_9X1_ID + 1) as u8)
}

const fn slot(slot_id: i16, x: i32, y: i32) -> InventorySlotLayout {
    InventorySlotLayout { slot_id, x, y }
}

const fn slot_grid(
    slots: &mut [InventorySlotLayout],
    start: usize,
    count: usize,
    columns: usize,
    x: i32,
    y: i32,
) {
    let mut index = 0;
    while index < count {
        slots[start + index] = slot(
            (start + index) as i16,
            x + (index % columns) as i32 * 18,
            y + (index / columns) as i32 * 18,
        );
        index += 1;
    }
}

const fn player_inventory_slots(slots: &mut [InventorySlotLayout], start: usize, x: i32, y: i32) {
    slot_grid(
        slots,
        start,
        27,
        GENERIC_CONTAINER_SLOT_COLUMNS as usize,
        x,
        y,
    );
    slot_grid(
        slots,
        start + 27,
        9,
        GENERIC_CONTAINER_SLOT_COLUMNS as usize,
        x,
        y + 58,
    );
}

const fn menu_slots<const N: usize>(
    menu: &[(i32, i32)],
    player_x: i32,
    player_y: i32,
) -> [InventorySlotLayout; N] {
    let mut slots = [slot(0, 0, 0); N];
    let mut index = 0;
    while index < menu.len() {
        slots[index] = slot(index as i16, menu[index].0, menu[index].1);
        index += 1;
    }
    player_inventory_slots(&mut slots, menu.len(), player_x, player_y);
    slots
}

const LOCAL_INVENTORY_SLOTS: &[InventorySlotLayout] = &{
    let mut slots = [slot(0, 154, 28); 46];
    slot_grid(&mut slots, 1, 4, 2, 98, 18);
    slot_grid(&mut slots, 5, 4, 1, 8, 8);
    player_inventory_slots(&mut slots, 9, 8, 84);
    slots[45] = slot(45, 77, 62);
    slots
};

const fn generic_container_slots<const N: usize>(rows: usize) -> [InventorySlotLayout; N] {
    let mut slots = [slot(0, 0, 0); N];
    let count = rows * GENERIC_CONTAINER_SLOT_COUNT_PER_ROW as usize;
    slot_grid(
        &mut slots,
        0,
        count,
        GENERIC_CONTAINER_SLOT_COLUMNS as usize,
        8,
        18,
    );
    player_inventory_slots(
        &mut slots,
        count,
        8,
        31 + rows as i32 * GENERIC_CONTAINER_ROW_HEIGHT,
    );
    slots
}

fn generic_container_slot_layouts(rows: u8) -> &'static [InventorySlotLayout] {
    const ROWS: [&[InventorySlotLayout]; 6] = [
        &generic_container_slots::<45>(1),
        &generic_container_slots::<54>(2),
        &generic_container_slots::<63>(3),
        &generic_container_slots::<72>(4),
        &generic_container_slots::<81>(5),
        &generic_container_slots::<90>(6),
    ];
    ROWS[usize::from(rows) - 1]
}

const GENERIC_3X3_SLOTS: &[InventorySlotLayout] = &{
    let mut slots = [slot(0, 0, 0); GENERIC_3X3_SLOT_COUNT as usize + 36];
    slot_grid(
        &mut slots,
        0,
        GENERIC_3X3_SLOT_COUNT as usize,
        GENERIC_3X3_SLOT_COLUMNS as usize,
        62,
        17,
    );
    player_inventory_slots(&mut slots, GENERIC_3X3_SLOT_COUNT as usize, 8, 84);
    slots
};
const CRAFTER_SLOTS: &[InventorySlotLayout] = &{
    let mut slots = [slot(0, 0, 0); CRAFTER_TOTAL_SLOT_COUNT as usize];
    slot_grid(
        &mut slots,
        0,
        CRAFTER_GRID_SLOT_COUNT as usize,
        CRAFTER_GRID_SLOT_COLUMNS as usize,
        26,
        17,
    );
    player_inventory_slots(&mut slots, CRAFTER_GRID_SLOT_COUNT as usize, 8, 84);
    slots[CRAFTER_RESULT_SLOT as usize] = slot(CRAFTER_RESULT_SLOT, 134, 35);
    slots
};
const CRAFTING_TABLE_SLOTS: &[InventorySlotLayout] = &{
    let mut slots = [slot(0, 124, 35); CRAFTING_SLOT_COUNT as usize + 36];
    slot_grid(
        &mut slots,
        1,
        CRAFTING_SLOT_COUNT as usize - 1,
        CRAFTING_GRID_SLOT_COLUMNS as usize,
        30,
        17,
    );
    player_inventory_slots(&mut slots, CRAFTING_SLOT_COUNT as usize, 8, 84);
    slots
};
const ANVIL_SLOTS: &[InventorySlotLayout] =
    &menu_slots::<{ ANVIL_SLOT_COUNT as usize + 36 }>(&[(27, 47), (76, 47), (134, 47)], 8, 84);
const ENCHANTMENT_TABLE_SLOTS: &[InventorySlotLayout] =
    &menu_slots::<{ ENCHANTMENT_SLOT_COUNT as usize + 36 }>(&[(15, 47), (35, 47)], 8, 84);
const BEACON_SLOTS: &[InventorySlotLayout] =
    &menu_slots::<{ BEACON_SLOT_COUNT as usize + 36 }>(&[(136, 110)], 36, 137);
const BREWING_STAND_SLOTS: &[InventorySlotLayout] =
    &menu_slots::<{ BREWING_STAND_SLOT_COUNT as usize + 36 }>(
        &[(56, 51), (79, 58), (102, 51), (79, 17), (17, 17)],
        8,
        84,
    );
const HOPPER_SLOTS: &[InventorySlotLayout] = &{
    let mut slots = [slot(0, 0, 0); HOPPER_SLOT_COUNT as usize + 36];
    slot_grid(
        &mut slots,
        0,
        HOPPER_SLOT_COUNT as usize,
        HOPPER_SLOT_COUNT as usize,
        44,
        20,
    );
    player_inventory_slots(&mut slots, HOPPER_SLOT_COUNT as usize, 8, 51);
    slots
};
const FURNACE_SLOTS: &[InventorySlotLayout] =
    &menu_slots::<{ FURNACE_SLOT_COUNT as usize + 36 }>(&[(56, 17), (56, 53), (116, 35)], 8, 84);
const GRINDSTONE_SLOTS: &[InventorySlotLayout] =
    &menu_slots::<{ GRINDSTONE_SLOT_COUNT as usize + 36 }>(&[(49, 19), (49, 40), (129, 34)], 8, 84);
const SHULKER_BOX_SLOTS: &[InventorySlotLayout] = &{
    let mut slots = [slot(0, 0, 0); SHULKER_BOX_SLOT_COUNT as usize + 36];
    slot_grid(
        &mut slots,
        0,
        SHULKER_BOX_SLOT_COUNT as usize,
        GENERIC_CONTAINER_SLOT_COLUMNS as usize,
        8,
        18,
    );
    player_inventory_slots(&mut slots, SHULKER_BOX_SLOT_COUNT as usize, 8, 84);
    slots
};
const LOOM_SLOTS: &[InventorySlotLayout] = &menu_slots::<{ LOOM_SLOT_COUNT as usize + 36 }>(
    &[(13, 26), (33, 26), (23, 45), (143, 57)],
    8,
    84,
);
const MERCHANT_SLOTS: &[InventorySlotLayout] = &menu_slots::<{ MERCHANT_SLOT_COUNT as usize + 36 }>(
    &[(136, 37), (162, 37), (220, 37)],
    108,
    84,
);
const CARTOGRAPHY_TABLE_SLOTS: &[InventorySlotLayout] = &menu_slots::<
    { CARTOGRAPHY_TABLE_SLOT_COUNT as usize + 36 },
>(&[(15, 15), (15, 52), (145, 39)], 8, 84);
const SMITHING_SLOTS: &[InventorySlotLayout] = &menu_slots::<{ SMITHING_SLOT_COUNT as usize + 36 }>(
    &[(8, 48), (26, 48), (44, 48), (98, 48)],
    8,
    84,
);
const STONECUTTER_SLOTS: &[InventorySlotLayout] =
    &menu_slots::<{ STONECUTTER_SLOT_COUNT as usize + 36 }>(&[(20, 33), (143, 33)], 8, 84);

const fn recipe_book_slots<const N: usize>(
    slots: &[InventorySlotLayout],
) -> [InventorySlotLayout; N] {
    let mut shifted = [slot(0, 0, 0); N];
    let mut index = 0;
    while index < N {
        shifted[index] = slot(
            slots[index].slot_id,
            slots[index].x + RECIPE_BOOK_MAIN_GUI_X_OFFSET,
            slots[index].y,
        );
        index += 1;
    }
    shifted
}

const LOCAL_INVENTORY_BOOK_SLOTS: &[InventorySlotLayout] =
    &recipe_book_slots::<46>(LOCAL_INVENTORY_SLOTS);
const CRAFTING_TABLE_BOOK_SLOTS: &[InventorySlotLayout] =
    &recipe_book_slots::<{ CRAFTING_SLOT_COUNT as usize + 36 }>(CRAFTING_TABLE_SLOTS);
const FURNACE_BOOK_SLOTS: &[InventorySlotLayout] =
    &recipe_book_slots::<{ FURNACE_SLOT_COUNT as usize + 36 }>(FURNACE_SLOTS);

fn clamped_mount_inventory_columns(inventory_columns: i32) -> u8 {
    inventory_columns.clamp(0, MOUNT_MAX_INVENTORY_COLUMNS) as u8
}

fn mount_inventory_slot_layouts(
    inventory_columns: u8,
    equipment_slots: MountEquipmentSlotVisibility,
) -> Vec<InventorySlotLayout> {
    let inventory_columns = i32::from(inventory_columns);
    let mount_inventory_slot_count = inventory_columns * MOUNT_INVENTORY_ROWS;
    let player_inventory_start =
        MOUNT_EQUIPMENT_SLOT_COUNT + i16::try_from(mount_inventory_slot_count).unwrap_or_default();
    let mut slots = Vec::with_capacity(
        MOUNT_EQUIPMENT_SLOT_COUNT as usize + mount_inventory_slot_count as usize + 36,
    );
    if equipment_slots.saddle {
        slots.push(InventorySlotLayout {
            slot_id: 0,
            x: 8,
            y: 18,
        });
    }
    if equipment_slots.body.is_some() {
        slots.push(InventorySlotLayout {
            slot_id: 1,
            x: 8,
            y: 36,
        });
    }
    for y in 0..MOUNT_INVENTORY_ROWS {
        for x in 0..inventory_columns {
            slots.push(InventorySlotLayout {
                slot_id: MOUNT_EQUIPMENT_SLOT_COUNT
                    + i16::try_from(x + y * inventory_columns).unwrap_or_default(),
                x: 80 + x * 18,
                y: 18 + y * 18,
            });
        }
    }
    for y in 0..3 {
        for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
            slots.push(InventorySlotLayout {
                slot_id: player_inventory_start + (x + y * GENERIC_CONTAINER_SLOT_COLUMNS) as i16,
                x: 8 + x * 18,
                y: 84 + y * 18,
            });
        }
    }
    for x in 0..GENERIC_CONTAINER_SLOT_COLUMNS {
        slots.push(InventorySlotLayout {
            slot_id: player_inventory_start + 27 + x as i16,
            x: 8 + x * 18,
            y: 142,
        });
    }

    slots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loom_grid_and_scroller_layout_match_vanilla_screen() {
        assert_eq!(LOOM_PATTERN_BUTTON_COLUMNS * LOOM_PATTERN_BUTTON_ROWS, 16);
        assert_eq!(
            (
                LOOM_PATTERN_BUTTON_X,
                LOOM_PATTERN_BUTTON_Y,
                LOOM_PATTERN_BUTTON_SIZE
            ),
            (60, 13, 14)
        );
        assert_eq!(
            (
                LOOM_SCROLLER_X,
                LOOM_SCROLLER_CLICK_Y,
                LOOM_SCROLLER_DRAG_Y,
                LOOM_SCROLLER_Y,
            ),
            (119, 9, 13, 13)
        );
        assert_eq!(LOOM_SCROLLER_MAX_OFFSET, 41);
        assert_eq!(
            LOOM_SCROLLER_FULL_HEIGHT - LOOM_SCROLLER_HEIGHT,
            LOOM_SCROLLER_MAX_OFFSET
        );
    }

    #[test]
    fn stonecutter_layout_preserves_vanilla_surface_offsets() {
        assert_eq!(
            STONECUTTER_RECIPE_BUTTON_COLUMNS * STONECUTTER_RECIPE_BUTTON_ROWS,
            STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT as i32
        );
        assert_eq!(
            (
                STONECUTTER_RECIPE_BUTTON_X,
                STONECUTTER_RECIPE_BUTTON_Y,
                STONECUTTER_RECIPE_BUTTON_SPRITE_Y,
                STONECUTTER_RECIPE_ITEM_Y,
            ),
            (52, 14, 15, 16)
        );
        assert_eq!(
            (
                STONECUTTER_SCROLLER_X,
                STONECUTTER_SCROLLER_CLICK_Y,
                STONECUTTER_SCROLLER_DRAG_Y,
                STONECUTTER_SCROLLER_Y,
            ),
            (119, 9, 14, 15)
        );
        assert_eq!(STONECUTTER_SCROLLER_MAX_OFFSET, 41);
        assert_eq!(
            STONECUTTER_SCROLLER_FULL_HEIGHT - STONECUTTER_SCROLLER_HEIGHT,
            39
        );
    }
}
